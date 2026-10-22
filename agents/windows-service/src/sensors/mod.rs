//! Sensores de Windows → `model::Snapshot`.
//!
//! - CPU y procesos: sysinfo (dos muestras; el uso de CPU es una diferencia entre lecturas).
//! - Frecuencia real: PDH `% Processor Performance` (detecta throttling).
//! - GPU: PDH `GPU Engine(*engtype_3D)\Utilization Percentage`.
//! - Temperatura: WMI (ACPI) o LibreHardwareMonitor si está instalado.
//! - Energía: plan activo y alimentación.

mod foreground;
mod gpu;
mod pdh;
mod power;
mod thermal;

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use anyhow::Result;
use model::*;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

use foreground::foreground_pid;

pub use power::{active_plan, plan_available as power_plan_available, set_plan, set_plan_by_guid};

/// `System` persistente: cada refresco calcula el uso desde la lectura anterior.
fn system() -> &'static Mutex<System> {
    static SYS: OnceLock<Mutex<System>> = OnceLock::new();
    SYS.get_or_init(|| {
        Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()).with_memory(MemoryRefreshKind::everything()),
        ))
    })
}

pub fn hardware_info() -> Result<HardwareInfo> {
    let mut sys = system().lock().unwrap();
    sys.refresh_all();
    let cpu_model = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    let (gpu_model, gpu_vram_mb) = gpu::adapter().unwrap_or((None, None));
    Ok(HardwareInfo {
        cpu_model,
        logical_cores: sys.cpus().len() as u16,
        ram_total_mb: sys.total_memory() / 1_048_576,
        gpu_model,
        gpu_vram_mb,
        storage_is_ssd: true, // refinado en la UI con IOCTL_STORAGE_QUERY_PROPERTY si hace falta
        has_battery: power::has_battery(),
        refresh_hz: power::refresh_rate_hz(),
    })
}

/// Escaneo completo (~1,5 s): dos muestras de CPU para medir procesos con precisión.
pub fn take_snapshot(hw: &HardwareInfo) -> Result<Snapshot> {
    {
        let mut sys = system().lock().unwrap();
        sys.refresh_cpu_all();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());
    }
    std::thread::sleep(Duration::from_millis(1500));
    build_snapshot(hw, None)
}

/// Instantánea para el bucle del modo juego (cada segundo, sin espera: usa la lectura anterior).
pub fn take_snapshot_fast(hw: &HardwareInfo, game_exe: &str) -> Result<Snapshot> {
    build_snapshot(hw, Some(game_exe))
}

fn build_snapshot(hw: &HardwareInfo, game_exe: Option<&str>) -> Result<Snapshot> {
    let mut sys = system().lock().unwrap();
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::everything());

    let fg_pid = foreground_pid();
    let cores = hw.logical_cores.max(1) as f32;
    let processes = sys
        .processes()
        .values()
        .map(|p| {
            let name = p.name().to_string_lossy().into_owned();
            let is_game = game_exe.is_some_and(|g| name.eq_ignore_ascii_case(g));
            ProcessInfo {
                pid: p.pid().as_u32(),
                cpu_pct: p.cpu_usage() / cores,
                ram_mb: p.memory() / 1_048_576,
                is_foreground: is_game || Some(p.pid().as_u32()) == fg_pid,
                is_system: is_protected(&name, p.pid().as_u32()),
                name,
            }
        })
        .collect();

    let disks = Disks::new_with_refreshed_list();
    let sys_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let storage_free_pct = disks
        .iter()
        .find(|d| d.mount_point().to_string_lossy().starts_with(&sys_drive))
        .map(|d| d.available_space() as f32 / d.total_space().max(1) as f32 * 100.0)
        .unwrap_or(100.0);

    let cpu_freq_ratio = pdh::processor_performance_ratio().ok();
    let cpu_celsius = thermal::cpu_celsius();
    let mut thermal = cpu_celsius.map(Thermal::from_cpu_celsius).unwrap_or_default();
    // Sin sensor fiable: el throttling sostenido es la mejor señal de calor.
    if cpu_celsius.is_none() && cpu_freq_ratio.is_some_and(|r| r < 0.6) && sys.global_cpu_usage() > 50.0 {
        thermal = Thermal::Moderate;
    }

    Ok(Snapshot {
        platform: Platform::Windows,
        hw: hw.clone(),
        cpu_load_pct: sys.global_cpu_usage(),
        cpu_freq_ratio,
        gpu_load_pct: pdh::gpu_3d_utilization().ok(),
        ram_used_pct: sys.used_memory() as f32 / sys.total_memory().max(1) as f32 * 100.0,
        storage_free_pct,
        thermal,
        thermal_headroom: None,
        on_battery: power::on_battery(),
        power_plan: Some(power::active_plan()?.0),
        power_save_mode: None,
        game_mode_on: None, // HKCU: lo rellena la UI
        ignoring_battery_opt: None,
        processes,
        temp_bytes: if game_exe.is_some() { 0 } else { crate::tweaks::cleanup::temp_size() },
    })
}

/// Procesos que nunca se tocan, independientemente de las listas de datos.
fn is_protected(name: &str, pid: u32) -> bool {
    const CRITICAL: &[&str] = &[
        "System",
        "Registry",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "winlogon.exe",
        "services.exe",
        "lsass.exe",
        "svchost.exe",
        "dwm.exe",
        "explorer.exe",
        "audiodg.exe",
        "MsMpEng.exe",
        "optimizador-service.exe",
        "optimizador_pro.exe",
    ];
    pid <= 4 || CRITICAL.iter().any(|c| name.eq_ignore_ascii_case(c))
}
