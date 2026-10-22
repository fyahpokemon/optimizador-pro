//! Bucle de control del modo juego: medir → ¿estable? → clasificar la causa → corregir
//! solo esa causa, con histéresis para no oscilar.

use model::{Profile, Snapshot, TweakKind};
#[cfg(feature = "serde")]
use serde::Serialize;
use stutter::{is_stable, FrameStats};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Cause {
    Thermal,
    CpuContention,
    MemoryPressure,
    GpuBound,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "action", rename_all = "snake_case"))]
pub enum Action {
    /// Aplicar (o recomendar, si la plataforma no puede) un tope de FPS.
    SetFpsCap {
        fps: u16,
    },
    /// Quitar el tope: el equipo ha demostrado que puede con más.
    RaiseFpsCap {
        fps: Option<u16>,
    },
    Tweak(TweakKind),
    /// Android elevado: restringir segundo plano de apps lastre durante la partida.
    RestrictHeavyBackgroundApps,
    /// No entendemos la causa: no tocar, registrar para el informe.
    LogForReport {
        cause: Cause,
    },
}

/// Topes de FPS habituales (todos dividen bien 60/120/144/240 Hz o son estándar en juegos).
pub const CAPS: [u16; 8] = [30, 40, 45, 60, 72, 90, 120, 144];

/// Tiempo mínimo entre dos correcciones.
pub const MIN_SECONDS_BETWEEN_CHANGES: f64 = 5.0;
/// Tiempo estable y frío necesario para intentar subir el tope.
pub const SECONDS_STABLE_BEFORE_RAISE: f64 = 60.0;

pub fn classify(f: &FrameStats, s: &Snapshot, p: &Profile) -> Cause {
    let near_limit = s.thermal >= p.thermal_threshold
        || s.thermal_headroom.is_some_and(|h| h > 0.85)
        || (s.cpu_freq_ratio.is_some_and(|r| r < 0.7) && s.cpu_load_pct > 50.0);
    if near_limit {
        return Cause::Thermal;
    }
    if s.ram_used_pct > 90.0 {
        return Cause::MemoryPressure;
    }
    let game_cpu = s.foreground().map_or(0.0, |g| g.cpu_pct);
    let others: f32 = s.processes.iter().filter(|p| !p.is_foreground && !p.is_system).map(|p| p.cpu_pct).sum();
    if s.cpu_load_pct > 85.0 && others > game_cpu * 0.3 {
        return Cause::CpuContention;
    }
    if s.gpu_load_pct.is_some_and(|g| g > 95.0) {
        return Cause::GpuBound;
    }
    let _ = f; // reservado: patrones de frametime por causa (p. ej. compilación de shaders)
    Cause::Unknown
}

/// Mayor tope de la lista que el equipo sostiene: algo por debajo del 1 % bajo actual,
/// de modo que casi todos los frames lleguen a tiempo.
pub fn sustainable_cap(f: &FrameStats) -> u16 {
    CAPS.iter().rev().copied().find(|c| f32::from(*c) <= f.low1_fps * 1.1).unwrap_or(CAPS[0])
}

#[derive(Debug)]
pub struct Controller {
    profile: Profile,
    last_change_s: f64,
    stable_since_s: Option<f64>,
    fps_cap: Option<u16>,
}

impl Controller {
    pub fn new(profile: Profile) -> Self {
        let fps_cap = profile.initial_fps_cap;
        Controller { profile, last_change_s: f64::NEG_INFINITY, stable_since_s: None, fps_cap }
    }

    pub fn fps_cap(&self) -> Option<u16> {
        self.fps_cap
    }

    /// Acciones para el estado de inicio de partida.
    pub fn start(&self) -> Vec<Action> {
        let mut v = Vec::new();
        if let Some(fps) = self.fps_cap {
            v.push(Action::SetFpsCap { fps });
        }
        if self.profile.clean_cache_before_play {
            v.push(Action::Tweak(TweakKind::CleanTemp));
        }
        v
    }

    /// Un paso del bucle (se llama cada segundo). `now_s` es un reloj monótono en segundos.
    pub fn step(&mut self, now_s: f64, f: &FrameStats, s: &Snapshot, ctx: &rules::Context) -> Vec<Action> {
        let stable = is_stable(f);
        let cool = s.thermal < self.profile.thermal_threshold && s.thermal_headroom.is_none_or(|h| h < 0.7);

        if stable {
            let since = *self.stable_since_s.get_or_insert(now_s);
            // Recuperación: tras un minuto estable y frío, se prueba un escalón más.
            if cool && now_s - since >= SECONDS_STABLE_BEFORE_RAISE && self.can_change(now_s) {
                if let Some(cap) = self.fps_cap {
                    let next = CAPS.iter().copied().find(|c| *c > cap);
                    let limit = self.profile.initial_fps_cap;
                    let next = match (next, limit) {
                        (Some(n), Some(l)) if n > l => None, // no pasar del perfil
                        (n, _) => n,
                    };
                    if next != Some(cap) && (next.is_some() || limit.is_none()) {
                        self.fps_cap = next;
                        self.mark_change(now_s);
                        self.stable_since_s = Some(now_s);
                        return vec![Action::RaiseFpsCap { fps: next }];
                    }
                }
            }
            return vec![];
        }

        self.stable_since_s = None;
        if !self.can_change(now_s) {
            return vec![];
        }

        let actions = match classify(f, s, &self.profile) {
            Cause::Thermal | Cause::GpuBound => {
                let target = sustainable_cap(f);
                // Nunca subir el tope cuando hay calor; solo bajarlo.
                let target = self.fps_cap.map_or(target, |c| target.min(c));
                if Some(target) == self.fps_cap {
                    let lower = CAPS.iter().rev().copied().find(|c| *c < target).unwrap_or(CAPS[0]);
                    self.fps_cap = Some(lower);
                } else {
                    self.fps_cap = Some(target);
                }
                vec![Action::SetFpsCap { fps: self.fps_cap.unwrap() }]
            }
            Cause::CpuContention => {
                let mut c = ctx.clone();
                c.bg_cpu_threshold_pct = self.profile.bg_cpu_threshold_pct;
                rules::background_hogs_rule(s, &c)
                    .map(|i| i.fix.into_iter().map(Action::Tweak).collect())
                    .unwrap_or_default()
            }
            Cause::MemoryPressure => vec![Action::RestrictHeavyBackgroundApps],
            cause @ Cause::Unknown => vec![Action::LogForReport { cause }],
        };
        if !actions.is_empty() {
            self.mark_change(now_s);
        }
        actions
    }

    fn can_change(&self, now_s: f64) -> bool {
        now_s - self.last_change_s >= MIN_SECONDS_BETWEEN_CHANGES
    }

    fn mark_change(&mut self, now_s: f64) {
        self.last_change_s = now_s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_for;
    use model::{HardwareInfo, HardwareTier, Platform, ProcessInfo, Thermal};

    fn stats(avg: f32, low1: f32, spm: f32) -> FrameStats {
        FrameStats { frames: 600, avg_fps: avg, low1_fps: low1, stutters: 0, stutters_per_min: spm, pacing_cv: 0.05 }
    }

    fn unstable() -> FrameStats {
        stats(70.0, 41.0, 12.0)
    }

    fn snap() -> Snapshot {
        let mut s = Snapshot::empty(Platform::Windows);
        s.processes.push(ProcessInfo {
            pid: 1,
            name: "game.exe".into(),
            cpu_pct: 40.0,
            ram_mb: 4000,
            is_foreground: true,
            is_system: false,
        });
        s
    }

    fn controller(tier: HardwareTier) -> Controller {
        Controller::new(profile_for(tier, &HardwareInfo::default()))
    }

    #[test]
    fn sustainable_cap_below_low1() {
        assert_eq!(sustainable_cap(&stats(70.0, 41.0, 0.0)), 45);
        assert_eq!(sustainable_cap(&stats(70.0, 56.0, 0.0)), 60);
        assert_eq!(sustainable_cap(&stats(20.0, 15.0, 0.0)), 30);
    }

    #[test]
    fn thermal_lowers_cap() {
        let mut c = controller(HardwareTier::High);
        let mut s = snap();
        s.thermal = Thermal::Severe;
        let a = c.step(0.0, &unstable(), &s, &rules::Context::default());
        assert_eq!(a, vec![Action::SetFpsCap { fps: 45 }]);
        assert_eq!(c.fps_cap(), Some(45));
    }

    #[test]
    fn hysteresis_limits_changes() {
        let mut c = controller(HardwareTier::High);
        let mut s = snap();
        s.thermal = Thermal::Severe;
        assert!(!c.step(0.0, &unstable(), &s, &Default::default()).is_empty());
        assert!(c.step(2.0, &unstable(), &s, &Default::default()).is_empty());
        // Sigue inestable con el mismo tope: baja un escalón más.
        assert_eq!(c.step(6.0, &unstable(), &s, &Default::default()), vec![Action::SetFpsCap { fps: 40 }]);
    }

    #[test]
    fn cpu_contention_throttles_hogs() {
        let mut c = controller(HardwareTier::Mid);
        let mut s = snap();
        s.cpu_load_pct = 95.0;
        s.processes.push(ProcessInfo {
            pid: 7,
            name: "updater.exe".into(),
            cpu_pct: 20.0,
            ram_mb: 50,
            is_foreground: false,
            is_system: false,
        });
        let a = c.step(0.0, &unstable(), &s, &Default::default());
        assert_eq!(a, vec![Action::Tweak(TweakKind::ThrottleBackground { pid: 7, name: "updater.exe".into() })]);
    }

    #[test]
    fn unknown_cause_touches_nothing() {
        let mut c = controller(HardwareTier::Mid);
        let a = c.step(0.0, &unstable(), &snap(), &Default::default());
        assert_eq!(a, vec![Action::LogForReport { cause: Cause::Unknown }]);
    }

    #[test]
    fn stable_and_cool_raises_cap_back_to_profile() {
        let mut c = controller(HardwareTier::Mid); // perfil: 60
        let mut hot = snap();
        hot.thermal = Thermal::Severe;
        c.step(0.0, &unstable(), &hot, &Default::default()); // baja a 45
        assert_eq!(c.fps_cap(), Some(45));

        let cool = snap();
        let ok = stats(45.0, 44.0, 0.0);
        assert!(c.step(10.0, &ok, &cool, &Default::default()).is_empty());
        assert!(c.step(50.0, &ok, &cool, &Default::default()).is_empty());
        assert_eq!(c.step(70.0, &ok, &cool, &Default::default()), vec![Action::RaiseFpsCap { fps: Some(60) }]);
        // Ya en el tope del perfil: no sube más.
        assert!(c.step(200.0, &ok, &cool, &Default::default()).is_empty());
        assert_eq!(c.fps_cap(), Some(60));
    }

    #[test]
    fn start_applies_profile() {
        let c = controller(HardwareTier::Basic);
        let a = c.start();
        assert!(a.contains(&Action::SetFpsCap { fps: 30 }));
        assert!(a.contains(&Action::Tweak(TweakKind::CleanTemp)));
    }
}
