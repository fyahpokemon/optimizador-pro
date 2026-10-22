//! Prioridad y EcoQoS de procesos. Solo depende de `windows` y `model`.

use anyhow::{bail, Result};
use model::Priority;
use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE};
use windows::Win32::System::Threading::{
    GetPriorityClass, OpenProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
    ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
    NORMAL_PRIORITY_CLASS, PROCESS_ACCESS_RIGHTS, PROCESS_CREATION_FLAGS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION,
};

pub const NORMAL_CLASS: u32 = NORMAL_PRIORITY_CLASS.0;
pub const BELOW_NORMAL_CLASS: u32 = BELOW_NORMAL_PRIORITY_CLASS.0;

pub fn class_for(p: Priority) -> u32 {
    match p {
        Priority::Low => IDLE_PRIORITY_CLASS.0,
        Priority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS.0,
        Priority::Normal => NORMAL_PRIORITY_CLASS.0,
        Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS.0,
        // Nunca REALTIME_PRIORITY_CLASS: puede congelar entrada, audio y el propio sistema.
        Priority::High => HIGH_PRIORITY_CLASS.0,
    }
}

/// Handle de proceso que se cierra solo.
struct ProcHandle(HANDLE);

impl ProcHandle {
    fn open(pid: u32, access: PROCESS_ACCESS_RIGHTS) -> Result<Self> {
        Ok(ProcHandle(unsafe { OpenProcess(access, BOOL::from(false), pid)? }))
    }
}

impl Drop for ProcHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub fn get_priority_class(pid: u32) -> Result<u32> {
    let h = ProcHandle::open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let c = unsafe { GetPriorityClass(h.0) };
    if c == 0 {
        bail!("GetPriorityClass({pid}) falló");
    }
    Ok(c)
}

pub fn set_priority_class(pid: u32, class: u32) -> Result<()> {
    let h = ProcHandle::open(pid, PROCESS_SET_INFORMATION)?;
    unsafe { SetPriorityClass(h.0, PROCESS_CREATION_FLAGS(class))? };
    Ok(())
}

/// EcoQoS (Windows 11): núcleos eficientes y menor frecuencia para ese proceso.
pub fn set_eco_qos(pid: u32, on: bool) -> Result<()> {
    let h = ProcHandle::open(pid, PROCESS_SET_INFORMATION)?;
    let state = PROCESS_POWER_THROTTLING_STATE {
        Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
        ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        StateMask: if on { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
    };
    unsafe {
        SetProcessInformation(
            h.0,
            ProcessPowerThrottling,
            &state as *const PROCESS_POWER_THROTTLING_STATE as *const std::ffi::c_void,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        )?
    };
    Ok(())
}
