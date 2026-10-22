//! Temperatura de CPU.
//!
//! Windows no ofrece una API pública fiable. Orden de preferencia:
//! 1. LibreHardwareMonitor (si el usuario lo instala o se usa el sidecar opcional): expone
//!    sensores por WMI en `root\LibreHardwareMonitor`.
//! 2. ACPI `MSAcpi_ThermalZoneTemperature` (`root\WMI`): disponible en muchos portátiles,
//!    a menudo ausente o fija en sobremesa.
//!
//! Sin ninguno: `None`, y el estado térmico se infiere del throttling (ver `sensors::mod`).

use std::sync::OnceLock;

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct Sensor {
    SensorType: String,
    Name: String,
    Value: f32,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case, non_camel_case_types)]
struct MSAcpi_ThermalZoneTemperature {
    CurrentTemperature: u32, // décimas de Kelvin
}

pub fn cpu_celsius() -> Option<f32> {
    lhm().or_else(acpi)
}

fn connection(namespace: &str) -> Option<wmi::WMIConnection> {
    let com = wmi::COMLibrary::new().ok()?;
    wmi::WMIConnection::with_namespace_path(namespace, com).ok()
}

fn lhm() -> Option<f32> {
    // Si no existe el espacio de nombres, no volver a intentarlo en cada tick.
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    if !*AVAILABLE.get_or_init(|| connection(r"root\LibreHardwareMonitor").is_some()) {
        return None;
    }
    let con = connection(r"root\LibreHardwareMonitor")?;
    let sensors: Vec<Sensor> =
        con.raw_query("SELECT SensorType, Name, Value FROM Sensor WHERE SensorType = 'Temperature'").ok()?;
    sensors
        .into_iter()
        .filter(|s| s.SensorType == "Temperature")
        .filter(|s| s.Name.contains("Package") || s.Name.contains("Tctl") || s.Name.contains("Core (Tctl/Tdie)"))
        .map(|s| s.Value)
        .reduce(f32::max)
}

fn acpi() -> Option<f32> {
    let con = connection(r"root\WMI")?;
    let zones: Vec<MSAcpi_ThermalZoneTemperature> = con.query().ok()?;
    zones
        .into_iter()
        .map(|z| z.CurrentTemperature as f32 / 10.0 - 273.15)
        .filter(|c| (20.0..115.0).contains(c)) // descarta valores fijos o absurdos
        .reduce(f32::max)
}
