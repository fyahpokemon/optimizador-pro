//! Nombre y memoria del adaptador gráfico principal (WMI).

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(non_snake_case, non_camel_case_types)]
struct Win32_VideoController {
    Name: Option<String>,
    AdapterRAM: Option<u32>,
}

pub fn adapter() -> Result<(Option<String>, Option<u64>)> {
    let con = wmi::WMIConnection::new(wmi::COMLibrary::new()?)?;
    let list: Vec<Win32_VideoController> = con.query()?;
    // AdapterRAM es u32 (satura en 4 GB): basta para distinguir integrada / dedicada.
    let best = list.into_iter().max_by_key(|g| g.AdapterRAM.unwrap_or(0));
    Ok(best.map_or((None, None), |g| (g.Name, g.AdapterRAM.map(|b| u64::from(b) / 1_048_576))))
}
