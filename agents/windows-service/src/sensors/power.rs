//! Planes de energía, alimentación y pantalla.

use anyhow::{bail, Result};
use windows::core::GUID;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, PowerGetActiveScheme, PowerSetActiveScheme, SYSTEM_POWER_STATUS,
};

pub const BALANCED: GUID = GUID::from_u128(0x381b4222_f694_41f0_9685_ff5bb260df2e);
pub const HIGH_PERFORMANCE: GUID = GUID::from_u128(0x8c5e7fda_e8bf_4a96_9a85_a6e23a8c635c);
pub const POWER_SAVER: GUID = GUID::from_u128(0xa1841308_3541_4fab_bc81_f71556f20b4a);
pub const ULTIMATE: GUID = GUID::from_u128(0xe9a42b02_d5df_448d_aa00_03f14749eb61);

/// Plan activo como (nombre normalizado, GUID en texto "XXXXXXXX-XXXX-...").
pub fn active_plan() -> Result<(String, String)> {
    unsafe {
        let mut ptr: *mut GUID = std::ptr::null_mut();
        PowerGetActiveScheme(None, &mut ptr).ok()?;
        let guid = *ptr;
        let _ = LocalFree(HLOCAL(ptr as _));
        let name = match guid {
            g if g == BALANCED => "balanced",
            g if g == HIGH_PERFORMANCE || g == ULTIMATE => "high_performance",
            g if g == POWER_SAVER => "power_saver",
            _ => "other", // plan del fabricante o del usuario: no lo pisamos con cargador
        };
        Ok((name.to_string(), format!("{guid:?}")))
    }
}

fn set_guid(guid: &GUID) -> Result<()> {
    unsafe { PowerSetActiveScheme(None, Some(guid as *const GUID)).ok()? };
    Ok(())
}

pub fn set_plan(name: &str) -> Result<()> {
    set_guid(match name {
        "high_performance" => &HIGH_PERFORMANCE,
        "power_saver" => &POWER_SAVER,
        _ => &BALANCED,
    })
}

/// Restaura el plan guardado por `active_plan`. `GUID::from(&str)` hace panic con texto
/// inválido, así que se valida antes.
pub fn set_plan_by_guid(text: &str) -> Result<()> {
    let s = text.trim_matches(|c| c == '{' || c == '}');
    let shape_ok = s.len() == 36
        && s.char_indices().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() });
    if !shape_ok {
        bail!("GUID inválido: {text}");
    }
    set_guid(&GUID::from(s))
}

/// En equipos con Modern Standby solo existe "Equilibrado": comprobar que el plan existe.
pub fn plan_available(name: &str) -> bool {
    let Ok(out) = std::process::Command::new("powercfg").arg("/list").output() else { return false };
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    match name {
        "high_performance" => text.contains("8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c"),
        _ => true,
    }
}

fn status() -> Option<SYSTEM_POWER_STATUS> {
    let mut s = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    Some(s)
}

pub fn on_battery() -> bool {
    status().is_some_and(|s| s.ACLineStatus == 0)
}

pub fn has_battery() -> bool {
    // BatteryFlag 128 = sin batería; 255 = desconocido.
    status().is_some_and(|s| s.BatteryFlag != 128 && s.BatteryFlag != 255)
}

pub fn refresh_rate_hz() -> Option<u16> {
    let mut dm = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
    unsafe { EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut dm).as_bool() }
        .then_some(dm.dmDisplayFrequency as u16)
        .filter(|hz| *hz > 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_text_roundtrip() {
        let text = format!("{HIGH_PERFORMANCE:?}");
        assert_eq!(text.to_lowercase(), "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c");
        assert_eq!(GUID::from(text.as_str()), HIGH_PERFORMANCE);
    }
}
