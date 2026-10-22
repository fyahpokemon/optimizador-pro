//! Servicios "lastre". Solo se pausan servicios de una lista permitida y nunca se cambia su
//! tipo de inicio: al deshacer (o al reiniciar) vuelven a su estado normal.

use anyhow::{bail, Result};
use windows_service::service::{ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

/// Servicios que se pueden pausar durante una partida sin romper nada visible.
const ALLOWED: &[&str] = &[
    "SysMain",         // precarga (en SSD aporta poco y genera E/S durante el juego)
    "WSearch",         // indexador de búsqueda
    "DiagTrack",       // telemetría
    "MapsBroker",      // mapas sin conexión
    "edgeupdate",      // actualizador de Edge
    "gupdate",         // actualizador de Google
    "AdobeARMservice", // actualizador de Adobe
];

fn manager() -> Result<ServiceManager> {
    Ok(ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?)
}

pub fn is_running(name: &str) -> Result<bool> {
    let svc = manager()?.open_service(name, ServiceAccess::QUERY_STATUS)?;
    Ok(svc.query_status()?.current_state == ServiceState::Running)
}

pub fn stop_if_allowed(name: &str) -> Result<()> {
    if !ALLOWED.iter().any(|a| a.eq_ignore_ascii_case(name)) {
        bail!("el servicio {name} no está en la lista permitida");
    }
    let svc = manager()?.open_service(name, ServiceAccess::QUERY_STATUS | ServiceAccess::STOP)?;
    if svc.query_status()?.current_state == ServiceState::Running {
        svc.stop()?;
    }
    Ok(())
}

pub fn start(name: &str) -> Result<()> {
    let svc = manager()?.open_service(name, ServiceAccess::QUERY_STATUS | ServiceAccess::START)?;
    if svc.query_status()?.current_state != ServiceState::Running {
        svc.start::<&str>(&[])?;
    }
    Ok(())
}
