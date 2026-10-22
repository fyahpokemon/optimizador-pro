//! Contadores de rendimiento (PDH).
//!
//! En windows 0.58 los handles de PDH son `isize` (no hay tipos PDH_HQUERY/PDH_HCOUNTER).

use anyhow::{bail, Result};
use std::sync::{Mutex, MutexGuard, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::System::Performance::{
    PdhAddEnglishCounterW, PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue,
    PdhOpenQueryW, PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
};

/// Consulta PDH persistente: los contadores de porcentaje necesitan dos recogidas.
struct Query {
    query: isize,
    perf: isize,
    gpu: isize,
}

const ERROR_SUCCESS: u32 = 0;

fn query() -> Result<&'static Mutex<Query>> {
    static Q: OnceLock<Option<Mutex<Query>>> = OnceLock::new();
    let q = Q.get_or_init(|| unsafe {
        let mut query = 0isize;
        if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != ERROR_SUCCESS {
            return None;
        }
        let mut perf = 0isize;
        let mut gpu = 0isize;
        PdhAddEnglishCounterW(query, w!(r"\Processor Information(_Total)\% Processor Performance"), 0, &mut perf);
        PdhAddEnglishCounterW(query, w!(r"\GPU Engine(*engtype_3D)\Utilization Percentage"), 0, &mut gpu);
        PdhCollectQueryData(query);
        Some(Mutex::new(Query { query, perf, gpu }))
    });
    match q {
        Some(m) => Ok(m),
        None => bail!("PDH no disponible"),
    }
}

fn collect() -> Result<MutexGuard<'static, Query>> {
    let q = query()?.lock().unwrap();
    unsafe { PdhCollectQueryData(q.query) };
    Ok(q)
}

/// Frecuencia efectiva / nominal (1.0 = nominal; > 1 con turbo; < 0.7 bajo carga = throttling).
pub fn processor_performance_ratio() -> Result<f32> {
    let q = collect()?;
    if q.perf == 0 {
        bail!("contador de rendimiento no disponible");
    }
    let mut v = PDH_FMT_COUNTERVALUE::default();
    let rc = unsafe { PdhGetFormattedCounterValue(q.perf, PDH_FMT_DOUBLE, None, &mut v) };
    if rc != ERROR_SUCCESS {
        bail!("PDH perf {rc:#x}");
    }
    Ok((unsafe { v.Anonymous.doubleValue } / 100.0) as f32)
}

/// Suma de la utilización 3D de todas las instancias del motor (0..100).
pub fn gpu_3d_utilization() -> Result<f32> {
    let q = collect()?;
    if q.gpu == 0 {
        bail!("contador de GPU no disponible");
    }
    unsafe {
        let mut size = 0u32;
        let mut count = 0u32;
        // Primera llamada: tamaño necesario (devuelve PDH_MORE_DATA).
        PdhGetFormattedCounterArrayW(q.gpu, PDH_FMT_DOUBLE, &mut size, &mut count, None);
        if size == 0 {
            bail!("sin instancias de GPU");
        }
        // Búfer alineado para PDH_FMT_COUNTERVALUE_ITEM_W (contiene f64 y punteros).
        let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
        let mut buf: Vec<PDH_FMT_COUNTERVALUE_ITEM_W> = Vec::with_capacity((size as usize).div_ceil(item));
        let items = buf.as_mut_ptr();
        let rc = PdhGetFormattedCounterArrayW(q.gpu, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items));
        if rc != ERROR_SUCCESS {
            bail!("PDH gpu {rc:#x}");
        }
        let total: f64 = (0..count as usize).map(|i| (*items.add(i)).FmtValue.Anonymous.doubleValue).sum();
        Ok(total.min(100.0) as f32)
    }
}
