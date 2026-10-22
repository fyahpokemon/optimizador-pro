//! Modelo de datos compartido entre el núcleo, los agentes nativos y la UI.
//!
//! El núcleo nunca toca el sistema operativo: recibe un [`Snapshot`] de un agente,
//! devuelve [`Issue`]s con sus [`TweakKind`]s y el agente los aplica.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Platform {
    Windows,
    Android,
}

/// Estado térmico normalizado. En Android coincide con `PowerManager.THERMAL_STATUS_*`
/// (0..=6); en Windows se deriva de temperaturas y del ratio de frecuencia.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Thermal {
    #[default]
    Normal,
    Light,
    Moderate,
    Severe,
    Critical,
}

impl Thermal {
    /// Convierte `PowerManager.getCurrentThermalStatus()` de Android.
    pub fn from_android_status(status: i32) -> Self {
        match status {
            i32::MIN..=0 => Thermal::Normal,
            1 => Thermal::Light,
            2 => Thermal::Moderate,
            3 => Thermal::Severe,
            _ => Thermal::Critical, // 4 CRITICAL, 5 EMERGENCY, 6 SHUTDOWN
        }
    }

    /// Clasifica por temperatura de CPU en ºC (Windows).
    pub fn from_cpu_celsius(c: f32) -> Self {
        match c {
            t if t < 75.0 => Thermal::Normal,
            t if t < 85.0 => Thermal::Light,
            t if t < 92.0 => Thermal::Moderate,
            t if t < 98.0 => Thermal::Severe,
            _ => Thermal::Critical,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub logical_cores: u16,
    pub ram_total_mb: u64,
    pub gpu_model: Option<String>,
    pub gpu_vram_mb: Option<u64>,
    pub storage_is_ssd: bool,
    /// Tiene batería: límites térmicos más estrictos y nunca "alto rendimiento" sin cargador.
    pub has_battery: bool,
    /// Tasa de refresco de la pantalla principal en Hz.
    pub refresh_hz: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    /// Uso de CPU normalizado al total de núcleos (0..=100).
    pub cpu_pct: f32,
    pub ram_mb: u64,
    pub is_foreground: bool,
    /// Proceso crítico del sistema: nunca se toca.
    pub is_system: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Snapshot {
    pub platform: Platform,
    pub hw: HardwareInfo,
    pub cpu_load_pct: f32,
    /// Frecuencia actual / nominal. < 0.7 sostenido bajo carga = throttling.
    pub cpu_freq_ratio: Option<f32>,
    pub gpu_load_pct: Option<f32>,
    pub ram_used_pct: f32,
    pub storage_free_pct: f32,
    pub thermal: Thermal,
    /// Android (API 30+): previsión de margen térmico; 1.0 = a punto de limitar.
    pub thermal_headroom: Option<f32>,
    pub on_battery: bool,
    /// Windows: identificador del plan activo ("balanced", "high_performance", "power_saver", "other").
    pub power_plan: Option<String>,
    /// Android: ahorro de batería del sistema activo.
    pub power_save_mode: Option<bool>,
    pub game_mode_on: Option<bool>,
    /// Android: la app está exenta de optimización de batería.
    pub ignoring_battery_opt: Option<bool>,
    pub processes: Vec<ProcessInfo>,
    /// Bytes en carpetas temporales / cachés limpiables.
    pub temp_bytes: u64,
}

impl Snapshot {
    /// Instantánea mínima útil para tests y para la UI antes del primer escaneo.
    pub fn empty(platform: Platform) -> Self {
        Snapshot {
            platform,
            hw: HardwareInfo::default(),
            cpu_load_pct: 0.0,
            cpu_freq_ratio: None,
            gpu_load_pct: None,
            ram_used_pct: 0.0,
            storage_free_pct: 100.0,
            thermal: Thermal::Normal,
            thermal_headroom: None,
            on_battery: false,
            power_plan: None,
            power_save_mode: None,
            game_mode_on: None,
            ignoring_battery_opt: None,
            processes: Vec::new(),
            temp_bytes: 0,
        }
    }

    pub fn foreground(&self) -> Option<&ProcessInfo> {
        self.processes.iter().find(|p| p.is_foreground)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Priority {
    Low,
    BelowNormal,
    Normal,
    AboveNormal,
    High, // nunca Realtime: congela entrada y audio
}

/// Un cambio concreto y reversible que un agente sabe aplicar.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum TweakKind {
    SetPowerPlan {
        plan: String,
    },
    EnableGameMode,
    SetPriority {
        pid: u32,
        level: Priority,
    },
    /// Bajar prioridad + EcoQoS (Windows) o restringir segundo plano (Android elevado).
    ThrottleBackground {
        pid: u32,
        name: String,
    },
    StopService {
        name: String,
    },
    CleanTemp,
    /// Android: exentar un paquete de Doze / optimización de batería.
    ExemptFromBatteryOpt {
        package: String,
    },
    /// Acción guiada: abrir una pantalla del sistema; el usuario confirma allí.
    OpenSystemScreen {
        screen: String,
    },
    /// Tope de FPS para un juego (perfil del driver o `cmd game set --fps`).
    SetFpsCap {
        target: String,
        fps: u16,
    },
}

impl TweakKind {
    /// Un ajuste guiado no lo aplica el agente: lo hace el usuario.
    pub fn is_guided(&self) -> bool {
        matches!(self, TweakKind::OpenSystemScreen { .. })
    }

    /// Algunos cambios no se pueden deshacer (borrar temporales): la UI avisa antes.
    pub fn is_reversible(&self) -> bool {
        !matches!(self, TweakKind::CleanTemp | TweakKind::OpenSystemScreen { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Issue {
    /// Identificador estable (también usado en telemetría y tests).
    pub id: String,
    /// 1..=10; pondera la puntuación de salud.
    pub severity: u8,
    /// Clave i18n con el texto en lenguaje cotidiano.
    pub title_key: String,
    pub fix: Vec<TweakKind>,
    /// Se puede incluir en "Optimizar con 1 clic" sin preguntar.
    pub safe_for_one_click: bool,
}

/// Nivel de hardware que decide el perfil de partida.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum HardwareTier {
    Basic,
    Mid,
    High,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Profile {
    pub tier: HardwareTier,
    /// Plan deseado con cargador; con batería siempre "balanced".
    pub plugged_power_plan: String,
    /// Umbral de CPU (%) a partir del cual un proceso de fondo se frena.
    pub bg_cpu_threshold_pct: f32,
    /// Tope de FPS inicial (None = sin tope salvo calor).
    pub initial_fps_cap: Option<u16>,
    /// Estado térmico a partir del cual se considera "calor".
    pub thermal_threshold: Thermal,
    pub clean_cache_before_play: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_from_android() {
        assert_eq!(Thermal::from_android_status(0), Thermal::Normal);
        assert_eq!(Thermal::from_android_status(2), Thermal::Moderate);
        assert_eq!(Thermal::from_android_status(6), Thermal::Critical);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn tweak_serializes_tagged() {
        let t = TweakKind::SetPowerPlan { plan: "balanced".into() };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"kind":"set_power_plan","plan":"balanced"}"#);
        let back: TweakKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn reversibility() {
        assert!(!TweakKind::CleanTemp.is_reversible());
        assert!(TweakKind::EnableGameMode.is_reversible());
    }
}
