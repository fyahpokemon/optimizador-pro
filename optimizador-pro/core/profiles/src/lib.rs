//! Perfiles de partida por nivel de hardware y controlador adaptativo del modo juego.

pub mod controller;

use model::{HardwareInfo, HardwareTier, Platform, Profile, Thermal};

/// Clasifica el equipo en un nivel a partir del hardware detectado.
pub fn detect_tier(hw: &HardwareInfo, platform: Platform) -> HardwareTier {
    let ram_gb = hw.ram_total_mb as f32 / 1024.0;
    let cores = hw.logical_cores;
    let dedicated_gpu = hw.gpu_vram_mb.is_some_and(|v| v >= 4096);

    match platform {
        Platform::Windows => {
            if cores <= 4 || ram_gb <= 4.5 {
                HardwareTier::Basic
            } else if cores >= 8 && ram_gb >= 15.5 && dedicated_gpu {
                HardwareTier::High
            } else {
                HardwareTier::Mid
            }
        }
        // En móvil la RAM es el mejor indicador disponible sin listas de SoC.
        Platform::Android => {
            if ram_gb <= 4.5 || cores <= 4 {
                HardwareTier::Basic
            } else if ram_gb >= 11.5 {
                HardwareTier::High
            } else {
                HardwareTier::Mid
            }
        }
    }
}

/// Perfil de partida. El controlador lo ajusta en caliente durante la partida.
pub fn profile_for(tier: HardwareTier, hw: &HardwareInfo) -> Profile {
    let refresh = hw.refresh_hz.unwrap_or(60);
    let mut p = match tier {
        HardwareTier::Basic => Profile {
            tier,
            plugged_power_plan: "balanced".into(),
            bg_cpu_threshold_pct: 2.0,
            initial_fps_cap: Some(if refresh >= 90 { 45 } else { 30 }),
            thermal_threshold: Thermal::Moderate,
            clean_cache_before_play: true,
        },
        HardwareTier::Mid => Profile {
            tier,
            plugged_power_plan: "high_performance".into(),
            bg_cpu_threshold_pct: 3.5,
            initial_fps_cap: Some(if refresh >= 120 { (refresh / 2).max(60) } else { 60 }),
            thermal_threshold: Thermal::Moderate,
            clean_cache_before_play: false,
        },
        HardwareTier::High => Profile {
            tier,
            plugged_power_plan: "high_performance".into(),
            bg_cpu_threshold_pct: 5.0,
            initial_fps_cap: None,
            thermal_threshold: Thermal::Moderate,
            clean_cache_before_play: false,
        },
    };
    // Modificador para portátiles y móviles.
    if hw.has_battery {
        p.thermal_threshold = Thermal::Light;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hw(cores: u16, ram_gb: u64, vram_gb: Option<u64>) -> HardwareInfo {
        HardwareInfo {
            logical_cores: cores,
            ram_total_mb: ram_gb * 1024,
            gpu_vram_mb: vram_gb.map(|v| v * 1024),
            ..Default::default()
        }
    }

    #[test]
    fn tiers_windows() {
        assert_eq!(detect_tier(&hw(4, 8, None), Platform::Windows), HardwareTier::Basic);
        assert_eq!(detect_tier(&hw(8, 16, None), Platform::Windows), HardwareTier::Mid);
        assert_eq!(detect_tier(&hw(16, 32, Some(8)), Platform::Windows), HardwareTier::High);
    }

    #[test]
    fn tiers_android() {
        assert_eq!(detect_tier(&hw(8, 4, None), Platform::Android), HardwareTier::Basic);
        assert_eq!(detect_tier(&hw(8, 8, None), Platform::Android), HardwareTier::Mid);
        assert_eq!(detect_tier(&hw(8, 12, None), Platform::Android), HardwareTier::High);
    }

    #[test]
    fn battery_lowers_thermal_threshold() {
        let mut h = hw(8, 16, None);
        h.has_battery = true;
        assert_eq!(profile_for(HardwareTier::Mid, &h).thermal_threshold, Thermal::Light);
    }

    #[test]
    fn mid_cap_follows_refresh_rate() {
        let mut h = hw(8, 16, None);
        h.refresh_hz = Some(144);
        assert_eq!(profile_for(HardwareTier::Mid, &h).initial_fps_cap, Some(72));
        h.refresh_hz = Some(60);
        assert_eq!(profile_for(HardwareTier::Mid, &h).initial_fps_cap, Some(60));
    }
}
