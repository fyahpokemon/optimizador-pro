//! Funciones expuestas a Flutter. `flutter_rust_bridge_codegen generate` lee este módulo
//! (`rust_input: crate::api`) y genera `app/lib/bridge/api.dart`.
//!
//! Solo funciones con tipos simples (String, Vec<f32>, Option): todo el intercambio va en
//! JSON. No declares structs ni estado aquí: el generador traduciría también los privados.
//! La lógica está en `crate::engine`.

use crate::engine;

pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Diagnóstico completo de una instantánea. Entrada y salida en JSON.
pub fn scan(snapshot_json: String, rules_json: Option<String>) -> anyhow::Result<String> {
    engine::scan(&snapshot_json, rules_json.as_deref())
}

/// Perfil de partida para el hardware detectado.
pub fn profile(snapshot_json: String) -> anyhow::Result<String> {
    engine::profile(&snapshot_json)
}

/// Empieza el modo juego. Devuelve las acciones iniciales (JSON).
pub fn game_start(profile_json: String, game: String, rules_json: Option<String>) -> anyhow::Result<String> {
    engine::game_start(&profile_json, game, rules_json.as_deref())
}

/// Añade frametimes (ms) medidos por el agente desde la última llamada.
pub fn game_push_frames(frametimes_ms: Vec<f32>) {
    engine::game_push_frames(frametimes_ms)
}

/// Un paso del bucle de control (cada segundo) con la instantánea actual.
pub fn game_tick(snapshot_json: String) -> anyhow::Result<String> {
    engine::game_tick(&snapshot_json)
}

pub fn game_stop() {
    engine::game_stop()
}
