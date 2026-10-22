//! Creación del named pipe con su ACL. Separado de `ipc.rs` para poder comprobarlo de forma
//! aislada (solo depende del crate `windows`).

use anyhow::{bail, Context, Result};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, LocalFree, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_WAIT,
};

pub const PIPE_NAME: PCWSTR = w!(r"\\.\pipe\optimizador-pro");
/// SYSTEM: todo. Usuarios interactivos (IU): leer y escribir. Sin acceso de red.
const PIPE_SDDL: PCWSTR = w!("D:P(A;;GA;;;SY)(A;;GRGW;;;IU)");

/// Crea una instancia del pipe (una sola a la vez: la UI).
pub fn create() -> Result<HANDLE> {
    unsafe {
        let mut sd = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(PIPE_SDDL, SDDL_REVISION_1, &mut sd, None)
            .context("SDDL del pipe")?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd.0,
            bInheritHandle: false.into(),
        };
        // CreateNamedPipeW devuelve HANDLE (no Result) en windows 0.58: se valida a mano.
        let h = CreateNamedPipeW(
            PIPE_NAME,
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            64 * 1024,
            64 * 1024,
            0,
            Some(&sa as *const SECURITY_ATTRIBUTES),
        );
        let err = GetLastError();
        let _ = LocalFree(HLOCAL(sd.0));
        if h.is_invalid() {
            bail!("CreateNamedPipeW: {err:?}");
        }
        Ok(h)
    }
}

/// Bloquea hasta que un cliente se conecta. `true` si hay cliente.
pub fn wait_for_client(h: HANDLE) -> bool {
    unsafe {
        match ConnectNamedPipe(h, None) {
            Ok(()) => true,
            // El cliente se conectó entre CreateNamedPipe y ConnectNamedPipe: también vale.
            Err(_) => GetLastError() == ERROR_PIPE_CONNECTED,
        }
    }
}

pub fn disconnect(h: HANDLE) {
    unsafe {
        let _ = DisconnectNamedPipe(h);
    }
}

pub fn close(h: HANDLE) {
    unsafe {
        let _ = CloseHandle(h);
    }
}
