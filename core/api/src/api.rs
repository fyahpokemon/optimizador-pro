//! Funciones expuestas a Flutter. `flutter_rust_bridge_codegen generate` lee este módulo
//! (`rust_input: crate::api`) y genera `app/lib/bridge/api.dart`.

use std::sync::Mutex;

use model::{HardwareTier, Profile, Snapshot, TweakKind};
use profiles::controller::{Action, Controller};
use serde::Serialize;

/// Contexto por defecto empaquetado con la app (actualizable en caliente).
const DEFAULT_RULES: &str = include_str!("../../../data/process_rules.json");

/// Solo se serializa: no es `pub` para que flutter_rust_bridge no intente traducirlo
/// (un `&'static str` en un struct público rompe la generación de bindings).
#[derive(Debug, Serialize)]
struct ScanResult {
    pub score: u8,
    pub label_key: &'static str,
    /// Clave de la frase principal ("Tu equipo se está calentando").
    pub headline_key: String,
    pub issues: Vec<model::Issue>,
    pub one_click: Vec<TweakKind>,
    pub guided: Vec<TweakKind>,
    pub tier: HardwareTier,
}

pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn context(rules_json: Option<&str>) -> anyhow::Result<rules::Context> {
    Ok(rules::Context::from_json(rules_json.unwrap_or(DEFAULT_RULES))?)
}

/// Diagnóstico completo de una instantánea. Entrada y salida en JSON.
pub fn scan(snapshot_json: String, rules_json: Option<String>) -> anyhow::Result<String> {
    let s: Snapshot = serde_json::from_str(&snapshot_json)?;
    let mut ctx = context(rules_json.as_deref())?;
    let tier = profiles::detect_tier(&s.hw, s.platform);
    // El umbral de "proceso lastre" depende del equipo: más estricto en gama baja.
    ctx.bg_cpu_threshold_pct = profiles::profile_for(tier, &s.hw).bg_cpu_threshold_pct;

    let issues = rules::diagnose(&s, &ctx);
    let score = scoring::health_score(&issues);
    let result = ScanResult {
        score,
        label_key: scoring::HealthLabel::for_score(score).key(),
        headline_key: issues.first().map_or_else(|| "headline.all_good".to_string(), |i| format!("headline.{}", i.id)),
        one_click: rules::one_click_plan(&issues),
        guided: rules::guided_steps(&issues),
        issues,
        tier,
    };
    Ok(serde_json::to_string(&result)?)
}

/// Perfil de partida para el hardware detectado.
pub fn profile(snapshot_json: String) -> anyhow::Result<String> {
    let s: Snapshot = serde_json::from_str(&snapshot_json)?;
    let tier = profiles::detect_tier(&s.hw, s.platform);
    Ok(serde_json::to_string(&profiles::profile_for(tier, &s.hw))?)
}

/// Estado de una partida en curso (una a la vez).
struct GameSession {
    controller: Controller,
    buffer: stutter::FrameBuffer,
    ctx: rules::Context,
    started: std::time::Instant,
}

static SESSION: Mutex<Option<GameSession>> = Mutex::new(None);

/// Empieza el modo juego. Devuelve las acciones iniciales (JSON).
pub fn game_start(profile_json: String, game: String, rules_json: Option<String>) -> anyhow::Result<String> {
    let profile: Profile = serde_json::from_str(&profile_json)?;
    let mut ctx = context(rules_json.as_deref())?;
    ctx.active_game = Some(game);
    let controller = Controller::new(profile);
    let actions = controller.start();
    *SESSION.lock().unwrap() = Some(GameSession {
        controller,
        buffer: stutter::FrameBuffer::new(10.0),
        ctx,
        started: std::time::Instant::now(),
    });
    Ok(serde_json::to_string(&actions)?)
}

/// Añade frametimes (ms) medidos por el agente desde la última llamada.
pub fn game_push_frames(frametimes_ms: Vec<f32>) {
    if let Some(s) = SESSION.lock().unwrap().as_mut() {
        frametimes_ms.into_iter().for_each(|f| s.buffer.push(f));
    }
}

#[derive(Serialize)]
struct Tick {
    pub stats: Option<stutter::FrameStats>,
    pub fps_cap: Option<u16>,
    pub actions: Vec<Action>,
}

/// Un paso del bucle de control (cada segundo) con la instantánea actual.
pub fn game_tick(snapshot_json: String) -> anyhow::Result<String> {
    let snap: Snapshot = serde_json::from_str(&snapshot_json)?;
    let mut guard = SESSION.lock().unwrap();
    let Some(s) = guard.as_mut() else { anyhow::bail!("no hay partida en curso") };
    let stats = s.buffer.stats();
    let actions = match &stats {
        Some(f) => s.controller.step(s.started.elapsed().as_secs_f64(), f, &snap, &s.ctx),
        None => vec![],
    };
    Ok(serde_json::to_string(&Tick { stats, fps_cap: s.controller.fps_cap(), actions })?)
}

pub fn game_stop() {
    *SESSION.lock().unwrap() = None;
}
