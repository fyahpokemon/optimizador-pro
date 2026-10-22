//! Análisis de frametimes.
//!
//! La métrica que importa para la sensación de fluidez no es el FPS medio sino la
//! regularidad: el 1 % de frames más lentos, los tirones y la variación del frametime.

#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct FrameStats {
    pub frames: usize,
    pub avg_fps: f32,
    /// FPS equivalentes al percentil 99 del frametime ("1 % low").
    pub low1_fps: f32,
    pub stutters: usize,
    pub stutters_per_min: f32,
    /// Coeficiente de variación del frametime (desviación / media).
    pub pacing_cv: f32,
}

/// Mínimo de frames para que el análisis tenga sentido (~0,5 s a 60 FPS).
pub const MIN_FRAMES: usize = 30;
/// Ventana de la mediana móvil para detectar tirones.
const WINDOW: usize = 15;
/// Un tirón dura más del doble de la mediana local…
const STUTTER_RATIO: f32 = 2.0;
/// …y al menos 8 ms más (evita falsos positivos a FPS muy altos).
const STUTTER_MIN_EXTRA_MS: f32 = 8.0;

/// Analiza una serie de frametimes en milisegundos. La duración de la ventana se
/// deduce de la suma de frametimes.
pub fn analyze(ft_ms: &[f32]) -> Option<FrameStats> {
    let ft: Vec<f32> = ft_ms.iter().copied().filter(|x| x.is_finite() && *x > 0.0).collect();
    if ft.len() < MIN_FRAMES {
        return None;
    }
    let n = ft.len() as f32;
    let total_ms: f32 = ft.iter().sum();
    let mean = total_ms / n;

    let mut sorted = ft.clone();
    sorted.sort_by(f32::total_cmp);
    let p99_idx = ((n * 0.99).ceil() as usize).clamp(1, ft.len()) - 1;
    let p99 = sorted[p99_idx];

    let var = ft.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;

    let mut stutters = 0;
    let mut window = Vec::with_capacity(WINDOW);
    for i in WINDOW..ft.len() {
        window.clear();
        window.extend_from_slice(&ft[i - WINDOW..i]);
        window.sort_by(f32::total_cmp);
        let med = window[WINDOW / 2];
        if ft[i] > med * STUTTER_RATIO && ft[i] - med > STUTTER_MIN_EXTRA_MS {
            stutters += 1;
        }
    }

    let minutes = total_ms / 60_000.0;
    Some(FrameStats {
        frames: ft.len(),
        avg_fps: 1000.0 / mean,
        low1_fps: 1000.0 / p99,
        stutters,
        stutters_per_min: stutters as f32 / minutes,
        pacing_cv: var.sqrt() / mean,
    })
}

/// Criterio de "estable": el 1 % bajo cerca de la media, casi sin tirones y ritmo regular.
pub fn is_stable(f: &FrameStats) -> bool {
    f.low1_fps >= f.avg_fps * 0.75 && f.stutters_per_min < 2.0 && f.pacing_cv < 0.15
}

/// Buffer circular de frametimes para el bucle de control (últimos N segundos).
#[derive(Debug)]
pub struct FrameBuffer {
    data: std::collections::VecDeque<f32>,
    max_ms: f32,
    sum_ms: f32,
}

impl FrameBuffer {
    pub fn new(seconds: f32) -> Self {
        FrameBuffer { data: Default::default(), max_ms: seconds * 1000.0, sum_ms: 0.0 }
    }

    pub fn push(&mut self, ft_ms: f32) {
        if !(ft_ms.is_finite() && ft_ms > 0.0) {
            return;
        }
        self.data.push_back(ft_ms);
        self.sum_ms += ft_ms;
        while self.sum_ms > self.max_ms {
            match self.data.pop_front() {
                Some(old) => self.sum_ms -= old,
                None => break,
            }
        }
    }

    pub fn stats(&self) -> Option<FrameStats> {
        let v: Vec<f32> = self.data.iter().copied().collect();
        analyze(&v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steady(fps: f32, secs: f32) -> Vec<f32> {
        vec![1000.0 / fps; (fps * secs) as usize]
    }

    #[test]
    fn too_few_frames() {
        assert!(analyze(&[16.6; 10]).is_none());
    }

    #[test]
    fn steady_60_is_stable() {
        let f = analyze(&steady(60.0, 10.0)).unwrap();
        assert!((f.avg_fps - 60.0).abs() < 0.1);
        assert_eq!(f.stutters, 0);
        assert!(is_stable(&f));
    }

    #[test]
    fn spikes_are_counted_as_stutters() {
        let mut ft = steady(60.0, 10.0);
        for i in (60..ft.len()).step_by(60) {
            ft[i] = 50.0; // un tirón de 50 ms cada segundo
        }
        let f = analyze(&ft).unwrap();
        assert_eq!(f.stutters, 9);
        assert!(f.stutters_per_min > 50.0);
        assert!(!is_stable(&f));
    }

    #[test]
    fn small_jitter_at_high_fps_is_not_stutter() {
        // 240 FPS con picos a 9 ms: más del doble, pero < 8 ms extra.
        let mut ft = steady(240.0, 5.0);
        for i in (100..ft.len()).step_by(100) {
            ft[i] = 9.0;
        }
        assert_eq!(analyze(&ft).unwrap().stutters, 0);
    }

    #[test]
    fn oscillating_fps_is_unstable_even_without_spikes() {
        // Alterna 40 y 75 FPS cada medio segundo.
        let mut ft = Vec::new();
        for k in 0..20 {
            let fps = if k % 2 == 0 { 40.0 } else { 75.0 };
            ft.extend(steady(fps, 0.5));
        }
        let f = analyze(&ft).unwrap();
        assert!(f.pacing_cv > 0.15);
        assert!(!is_stable(&f));
    }

    #[test]
    fn buffer_keeps_only_window() {
        let mut b = FrameBuffer::new(2.0);
        for _ in 0..600 {
            b.push(10.0);
        }
        let s = b.stats().unwrap();
        assert_eq!(s.frames, 200);
    }

    #[test]
    fn ignores_invalid_values() {
        let mut ft = steady(60.0, 2.0);
        ft.push(f32::NAN);
        ft.push(-1.0);
        assert_eq!(analyze(&ft).unwrap().frames, 120);
    }
}
