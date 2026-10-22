//! Lógica detrás de `crate::api`. Vive fuera de ese módulo a propósito:
//! `flutter_rust_bridge_codegen` recorre `crate::api` entero y genera código Dart para cada
//! tipo que encuentra (también los privados), así que aquí van los tipos internos.

use std::sync::Mutex;

use model::{HardwareTier, Profile, Snapshot, TweakKind};
use profiles::controller::{Action, Controller};
use serde::Serialize;

/// Contexto por defecto empaquetado con la app (actualizable en caliente).
const DEFAULT_RULES: &str = include_str!("../../../data/process_rules.json");

#[derive(Debug, Serialize)]
struct ScanResult {
    score: u8,
    label_key: &'static str,
    /// Clave de la frase principal ("Tu equipo se está calentando").
    headline_key: String,
    issues: Vec<model::Issue>,
    one_click: Vec<TweakKind>,
    guided: Vec<TweakKind>,
    tier: HardwareTier,
}

fn context(rules_json: Option<&str>) -> anyhow::Result<rules::Context> {
    Ok(rules::Context::from_json(rules_json.unwrap_or(DEFAULT_RULES))?)
}

pub(crate) fn scan(snapshot_json: &str, rules_json: Option<&str>) -> anyhow::Result<String> {
    let s: Snapshot = serde_json::from_str(snapshot_json)?;
    let mut ctx = context(rules_json)?;
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

pub(crate) fn profile(snapshot_json: &str) -> anyhow::Result<String> {
    let s: Snapshot = serde_json::from_str(snapshot_json)?;
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

pub(crate) fn game_start(profile_json: &str, game: String, rules_json: Option<&str>) -> anyhow::Result<String> {
    let profile: Profile = serde_json::from_str(profile_json)?;
    let mut ctx = context(rules_json)?;
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

pub(crate) fn game_push_frames(frametimes_ms: Vec<f32>) {
    if let Some(s) = SESSION.lock().unwrap().as_mut() {
        frametimes_ms.into_iter().for_each(|f| s.buffer.push(f));
    }
}

#[derive(Serialize)]
struct Tick {
    stats: Option<stutter::FrameStats>,
    fps_cap: Option<u16>,
    actions: Vec<Action>,
}

pub(crate) fn game_tick(snapshot_json: &str) -> anyhow::Result<String> {
    let snap: Snapshot = serde_json::from_str(snapshot_json)?;
    let mut guard = SESSION.lock().unwrap();
    let Some(s) = guard.as_mut() else { anyhow::bail!("no hay partida en curso") };
    let stats = s.buffer.stats();
    let actions = match &stats {
        Some(f) => s.controller.step(s.started.elapsed().as_secs_f64(), f, &snap, &s.ctx),
        None => vec![],
    };
    Ok(serde_json::to_string(&Tick { stats, fps_cap: s.controller.fps_cap(), actions })?)
}

pub(crate) fn game_stop() {
    *SESSION.lock().unwrap() = None;
}
