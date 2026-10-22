//! Motor de reglas. Cada regla es una función pura `(&Snapshot, &Context) -> Option<Issue>`;
//! añadir una regla no toca el resto del sistema.

use model::*;
#[cfg(feature = "serde")]
use serde::Deserialize;

/// Listas de procesos (ver `data/process_rules.json`) y umbrales del perfil activo.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
pub struct Context {
    /// Nunca tocar: antivirus, drivers, audio, anti-cheat, accesibilidad.
    pub whitelist: Vec<String>,
    /// Lastre conocido: updaters, launchers en reposo, sincronizadores.
    pub known_heavy: Vec<String>,
    /// Umbral de CPU de un proceso de fondo para considerarlo lastre.
    #[cfg_attr(feature = "serde", serde(default = "default_bg_threshold"))]
    pub bg_cpu_threshold_pct: f32,
    /// Juego activo (si hay), para no frenarlo nunca.
    #[cfg_attr(feature = "serde", serde(default))]
    pub active_game: Option<String>,
}

fn default_bg_threshold() -> f32 {
    5.0
}

impl Context {
    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    fn is_whitelisted(&self, name: &str) -> bool {
        self.whitelist.iter().any(|w| name.eq_ignore_ascii_case(w))
            || self.active_game.as_deref().is_some_and(|g| name.eq_ignore_ascii_case(g))
    }

    fn is_known_heavy(&self, name: &str) -> bool {
        self.known_heavy.iter().any(|h| name.eq_ignore_ascii_case(h))
    }
}

impl Default for Context {
    fn default() -> Self {
        Context {
            whitelist: vec![],
            known_heavy: vec![],
            bg_cpu_threshold_pct: default_bg_threshold(),
            active_game: None,
        }
    }
}

pub type Rule = fn(&Snapshot, &Context) -> Option<Issue>;

pub const RULES: &[Rule] = &[
    power_plan_rule,
    android_power_save_rule,
    game_mode_rule,
    background_hogs_rule,
    thermal_rule,
    memory_pressure_rule,
    low_storage_rule,
    temp_files_rule,
    battery_optimizer_rule,
];

fn issue(id: &str, severity: u8, fix: Vec<TweakKind>, one_click: bool) -> Issue {
    Issue { id: id.into(), severity, title_key: format!("issue.{id}"), fix, safe_for_one_click: one_click }
}

/// Windows: con cargador, alto rendimiento; con batería, equilibrado (evita calor y throttling).
pub fn power_plan_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    if s.platform != Platform::Windows {
        return None;
    }
    let plan = s.power_plan.as_deref()?;
    let want = if s.on_battery { "balanced" } else { "high_performance" };
    if plan == want || (plan == "other" && !s.on_battery) {
        return None; // "other" = plan personalizado del usuario o del fabricante: no lo pisamos
    }
    Some(issue("power_plan", 6, vec![TweakKind::SetPowerPlan { plan: want.into() }], true))
}

/// Android: el ahorro de batería limita la CPU/GPU; se guía al usuario a desactivarlo para jugar.
pub fn android_power_save_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    if s.platform != Platform::Android || s.power_save_mode != Some(true) {
        return None;
    }
    Some(issue("power_save", 5, vec![TweakKind::OpenSystemScreen { screen: "battery_saver".into() }], false))
}

pub fn game_mode_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    (s.game_mode_on == Some(false)).then(|| issue("game_mode", 4, vec![TweakKind::EnableGameMode], true))
}

/// Procesos de fondo que gastan CPU: se frenan (prioridad baja / EcoQoS), nunca se matan.
pub fn background_hogs_rule(s: &Snapshot, c: &Context) -> Option<Issue> {
    let fixes: Vec<_> = s
        .processes
        .iter()
        .filter(|p| !p.is_foreground && !p.is_system && !c.is_whitelisted(&p.name))
        .filter(|p| p.cpu_pct > c.bg_cpu_threshold_pct || (c.is_known_heavy(&p.name) && p.cpu_pct > 0.5))
        .map(|p| TweakKind::ThrottleBackground { pid: p.pid, name: p.name.clone() })
        .collect();
    (!fixes.is_empty()).then(|| issue("background_apps", 7, fixes, true))
}

/// Calor o throttling. Sin arreglo directo: lo gestiona el controlador del modo juego.
pub fn thermal_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    let limit = if s.hw.has_battery { Thermal::Light } else { Thermal::Moderate };
    let throttled = s.cpu_freq_ratio.is_some_and(|r| r < 0.7) && s.cpu_load_pct > 50.0;
    let hot = s.thermal >= limit || s.thermal_headroom.is_some_and(|h| h > 0.85);
    (hot || throttled).then(|| issue("overheating", 9, vec![], true))
}

pub fn memory_pressure_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    (s.ram_used_pct > 90.0).then(|| issue("memory_full", 7, vec![], true))
}

pub fn low_storage_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    (s.storage_free_pct < 10.0)
        .then(|| issue("low_storage", 5, vec![TweakKind::OpenSystemScreen { screen: "storage".into() }], false))
}

pub fn temp_files_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    const LIMIT: u64 = 500 * 1024 * 1024;
    (s.temp_bytes > LIMIT).then(|| issue("temp_files", 2, vec![TweakKind::CleanTemp], true))
}

/// Android: si la propia app está sometida a la optimización de batería, el modo juego
/// en segundo plano puede morir. Se guía al usuario a la lista de exenciones.
pub fn battery_optimizer_rule(s: &Snapshot, _: &Context) -> Option<Issue> {
    if s.platform != Platform::Android || s.ignoring_battery_opt != Some(false) {
        return None;
    }
    Some(issue(
        "battery_optimizer",
        3,
        vec![TweakKind::OpenSystemScreen { screen: "battery_optimization".into() }],
        false,
    ))
}

/// Diagnóstico completo, ordenado de más a menos grave.
pub fn diagnose(s: &Snapshot, c: &Context) -> Vec<Issue> {
    let mut v: Vec<_> = RULES.iter().filter_map(|r| r(s, c)).collect();
    v.sort_by(|a, b| b.severity.cmp(&a.severity).then_with(|| a.id.cmp(&b.id)));
    v
}

/// Ajustes que el botón "Optimizar con 1 clic" aplica sin preguntar.
pub fn one_click_plan(issues: &[Issue]) -> Vec<TweakKind> {
    issues
        .iter()
        .filter(|i| i.safe_for_one_click)
        .flat_map(|i| i.fix.iter().cloned())
        .filter(|t| !t.is_guided())
        .collect()
}

/// Pasos que requieren al usuario (pantallas del sistema).
pub fn guided_steps(issues: &[Issue]) -> Vec<TweakKind> {
    issues.iter().flat_map(|i| i.fix.iter().cloned()).filter(|t| t.is_guided()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, name: &str, cpu: f32) -> ProcessInfo {
        ProcessInfo { pid, name: name.into(), cpu_pct: cpu, ram_mb: 100, is_foreground: false, is_system: false }
    }

    fn ctx() -> Context {
        Context {
            whitelist: vec!["MsMpEng.exe".into()],
            known_heavy: vec!["OneDrive.exe".into()],
            ..Default::default()
        }
    }

    #[test]
    fn healthy_windows_has_no_issues() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.power_plan = Some("high_performance".into());
        s.game_mode_on = Some(true);
        assert!(diagnose(&s, &ctx()).is_empty());
    }

    #[test]
    fn power_plan_depends_on_battery() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.power_plan = Some("high_performance".into());
        s.on_battery = true;
        let i = power_plan_rule(&s, &ctx()).unwrap();
        assert_eq!(i.fix, vec![TweakKind::SetPowerPlan { plan: "balanced".into() }]);
    }

    #[test]
    fn custom_plan_is_respected_when_plugged() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.power_plan = Some("other".into());
        assert!(power_plan_rule(&s, &ctx()).is_none());
    }

    #[test]
    fn hogs_are_throttled_not_whitelisted() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.processes = vec![
            proc(1, "MsMpEng.exe", 30.0), // antivirus: nunca
            proc(2, "updater.exe", 12.0), // lastre por CPU
            proc(3, "OneDrive.exe", 1.0), // lastre conocido
            proc(4, "notepad.exe", 0.1),  // inofensivo
        ];
        let mut fg = proc(5, "game.exe", 40.0);
        fg.is_foreground = true;
        s.processes.push(fg);
        let i = background_hogs_rule(&s, &ctx()).unwrap();
        let pids: Vec<u32> = i
            .fix
            .iter()
            .map(|t| match t {
                TweakKind::ThrottleBackground { pid, .. } => *pid,
                _ => 0,
            })
            .collect();
        assert_eq!(pids, vec![2, 3]);
    }

    #[test]
    fn active_game_is_never_throttled() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.processes = vec![proc(9, "Game.exe", 50.0)]; // en fondo (alt-tab) pero es el juego
        let c = Context { active_game: Some("game.exe".into()), ..ctx() };
        assert!(background_hogs_rule(&s, &c).is_none());
    }

    #[test]
    fn thermal_is_stricter_with_battery() {
        let mut s = Snapshot::empty(Platform::Android);
        s.thermal = Thermal::Light;
        assert!(thermal_rule(&s, &ctx()).is_none());
        s.hw.has_battery = true;
        assert!(thermal_rule(&s, &ctx()).is_some());
    }

    #[test]
    fn throttling_detected_by_frequency() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.cpu_freq_ratio = Some(0.55);
        s.cpu_load_pct = 80.0;
        assert!(thermal_rule(&s, &ctx()).is_some());
    }

    #[test]
    fn one_click_excludes_guided_steps() {
        let mut s = Snapshot::empty(Platform::Android);
        s.power_save_mode = Some(true);
        s.game_mode_on = Some(false);
        s.storage_free_pct = 5.0;
        let issues = diagnose(&s, &ctx());
        assert_eq!(one_click_plan(&issues), vec![TweakKind::EnableGameMode]);
        assert_eq!(guided_steps(&issues).len(), 2);
    }

    #[test]
    fn diagnose_sorted_by_severity() {
        let mut s = Snapshot::empty(Platform::Windows);
        s.temp_bytes = 2 << 30;
        s.thermal = Thermal::Severe;
        s.game_mode_on = Some(false);
        let ids: Vec<_> = diagnose(&s, &ctx()).into_iter().map(|i| i.id).collect();
        assert_eq!(ids, vec!["overheating", "game_mode", "temp_files"]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn context_loads_from_shipped_json() {
        let json = include_str!("../../../data/process_rules.json");
        let c = Context::from_json(json).unwrap();
        assert!(c.whitelist.iter().any(|w| w == "MsMpEng.exe"));
    }
}
