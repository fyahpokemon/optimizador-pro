//! Limpieza de temporales. Solo archivos con más de 24 h: los recientes pueden estar en uso
//! por instaladores o actualizaciones en curso. Los archivos bloqueados se omiten.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const MIN_AGE: Duration = Duration::from_secs(24 * 3600);

fn temp_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![std::env::temp_dir()];
    if let Ok(windir) = std::env::var("SystemRoot") {
        dirs.push(PathBuf::from(windir).join("Temp"));
    }
    // El servicio corre como SYSTEM: también los %TEMP% de cada perfil de usuario.
    if let Ok(entries) = std::fs::read_dir(r"C:\Users") {
        for e in entries.flatten() {
            let p = e.path().join(r"AppData\Local\Temp");
            if p.is_dir() {
                dirs.push(p);
            }
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

fn old_files() -> impl Iterator<Item = (PathBuf, u64)> {
    let cutoff = SystemTime::now() - MIN_AGE;
    temp_dirs().into_iter().flat_map(move |dir| {
        walkdir::WalkDir::new(dir)
            .min_depth(1)
            .follow_links(false) // nunca seguir enlaces fuera de las carpetas temporales
            .into_iter()
            .flatten()
            .filter_map(move |e| {
                let md = e.metadata().ok()?;
                (md.is_file() && md.modified().ok()? < cutoff).then(|| (e.into_path(), md.len()))
            })
    })
}

/// Bytes que se liberarían (para el escaneo).
pub fn temp_size() -> u64 {
    old_files().map(|(_, len)| len).sum()
}

/// Borra y devuelve los bytes liberados.
pub fn clean_temp() -> anyhow::Result<u64> {
    let mut freed = 0;
    for (path, len) in old_files() {
        if std::fs::remove_file(&path).is_ok() {
            freed += len;
        }
    }
    Ok(freed)
}
