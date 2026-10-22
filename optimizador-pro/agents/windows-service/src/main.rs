//! Servicio "OptimizadorPro" (LocalSystem).
//!
//! La UI corre sin privilegios y le habla por el named pipe `\\.\pipe\optimizador-pro`
//! (ACL: SYSTEM y usuarios interactivos). Protocolo: una petición JSON por línea, una
//! respuesta JSON por línea. Ver `protocol.rs`.
//!
//! Uso:
//!   optimizador-service.exe --console   # depuración, sin registrarse como servicio
//!   optimizador-service.exe             # lo arranca el SCM (instalado por el MSIX)

mod protocol;

#[cfg(windows)]
mod frames;
#[cfg(windows)]
mod ipc;
#[cfg(windows)]
mod pipe;
#[cfg(windows)]
mod sensors;
#[cfg(windows)]
mod tweaks;

#[cfg(windows)]
fn main() -> anyhow::Result<()> {
    if std::env::args().any(|a| a == "--console") {
        return ipc::serve_forever(service_state()?);
    }
    win_service::run()
}

#[cfg(not(windows))]
fn main() {
    eprintln!("optimizador-service solo funciona en Windows. El núcleo se prueba con `cargo test`.");
    std::process::exit(1);
}

/// Estado compartido del servicio: journal persistente y contexto de reglas.
#[cfg(windows)]
pub struct ServiceState {
    pub journal: journal::Journal,
    pub applier: tweaks::WinApplier,
    pub hw: model::HardwareInfo,
}

#[cfg(windows)]
fn journal_path() -> anyhow::Result<String> {
    let dir = std::path::PathBuf::from(std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into()))
        .join("OptimizadorPro");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("journal.db").to_string_lossy().into_owned())
}

#[cfg(windows)]
fn service_state() -> anyhow::Result<ServiceState> {
    let journal = journal::Journal::open(&journal_path()?)?;
    let applier = tweaks::WinApplier;

    // Si el servicio o el equipo se cerraron en mitad de una partida, se deshace lo pendiente.
    undo_game_sessions(&journal, &applier);
    Ok(ServiceState { journal, applier, hw: sensors::hardware_info()? })
}

/// Deshace las sesiones de modo juego que sigan activas. Las de "1 clic" se mantienen.
#[cfg(windows)]
pub fn undo_game_sessions(journal: &journal::Journal, applier: &tweaks::WinApplier) {
    match journal.pending_sessions(Some(ipc::GAME_LABEL)) {
        Ok(sessions) => {
            for session in sessions {
                if let Err(e) = journal.undo_session(applier, session) {
                    eprintln!("No se pudo deshacer la sesión {session}: {e:#}");
                }
            }
        }
        Err(e) => eprintln!("Journal ilegible: {e:#}"),
    }
}

#[cfg(windows)]
mod win_service {
    use std::ffi::OsString;
    use std::sync::mpsc;
    use std::time::Duration;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::{define_windows_service, service_dispatcher};

    pub const SERVICE_NAME: &str = "OptimizadorPro";

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> anyhow::Result<()> {
        service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
        Ok(())
    }

    fn service_main(_args: Vec<OsString>) {
        if let Err(e) = run_service() {
            eprintln!("{e:#}");
        }
    }

    fn run_service() -> anyhow::Result<()> {
        let (stop_tx, stop_rx) = mpsc::channel();
        let handler = move |event| match event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        };
        let status = service_control_handler::register(SERVICE_NAME, handler)?;
        let set = |state, accept| {
            status.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: state,
                controls_accepted: accept,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::from_secs(5),
                process_id: None,
            })
        };
        set(ServiceState::Running, ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN)?;

        let state = super::service_state()?;
        std::thread::spawn(move || {
            if let Err(e) = super::ipc::serve_forever(state) {
                eprintln!("IPC terminó: {e:#}");
            }
        });
        let _ = stop_rx.recv();

        // Al parar (apagado del equipo o actualización): deshacer la partida en curso.
        // Se abre otra conexión al journal: SQLite en WAL admite varios lectores/escritores.
        if let Ok(path) = super::journal_path() {
            if let Ok(j) = journal::Journal::open(&path) {
                super::undo_game_sessions(&j, &super::tweaks::WinApplier);
            }
        }
        set(ServiceState::Stopped, ServiceControlAccept::empty())?;
        Ok(())
    }
}
