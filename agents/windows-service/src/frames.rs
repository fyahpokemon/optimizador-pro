//! Frametimes de un proceso mediante PresentMon (Intel, licencia MIT), incluido como sidecar
//! en `bin/PresentMon.exe`. PresentMon lee los eventos ETW de presentación de DXGI/D3D9/
//! Vulkan/OpenGL sin inyectar nada en el juego (seguro con anti-cheat).
//!
//! Se lanza en modo consola con salida CSV y se lee la columna de intervalo entre
//! presentaciones (`MsBetweenPresents`, o `FrameTime` en PresentMon 2.x).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

pub struct PresentMon {
    child: Option<Child>,
    buffer: Arc<Mutex<Vec<f32>>>,
}

fn presentmon_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("bin").join("PresentMon.exe")))
        .unwrap_or_else(|| "PresentMon.exe".into())
}

impl PresentMon {
    pub fn start(pid: u32) -> Result<Self> {
        let mut child = Command::new(presentmon_path())
            .args([
                "--process_id",
                &pid.to_string(),
                "--output_stdout",
                "--no_console_stats",
                "--stop_existing_session",
                "--session_name",
                "OptimizadorPro",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("no se pudo lanzar PresentMon")?;

        let buffer = Arc::new(Mutex::new(Vec::with_capacity(512)));
        let out = child.stdout.take().context("PresentMon sin stdout")?;
        let sink = buffer.clone();
        std::thread::spawn(move || {
            let mut column: Option<usize> = None;
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let fields: Vec<&str> = line.split(',').collect();
                match column {
                    None => {
                        column = fields.iter().position(|f| *f == "MsBetweenPresents" || *f == "FrameTime");
                    }
                    Some(i) => {
                        if let Some(ms) = fields.get(i).and_then(|v| v.trim().parse::<f32>().ok()) {
                            let mut b = sink.lock().unwrap();
                            if b.len() < 10_000 {
                                b.push(ms);
                            }
                        }
                    }
                }
            }
        });
        Ok(PresentMon { child: Some(child), buffer })
    }

    /// Frametimes recibidos desde la última llamada.
    pub fn drain(&mut self) -> Vec<f32> {
        std::mem::take(&mut *self.buffer.lock().unwrap())
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for PresentMon {
    fn drop(&mut self) {
        self.stop();
    }
}
