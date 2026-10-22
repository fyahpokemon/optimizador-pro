//! Servidor del named pipe `\\.\pipe\optimizador-pro`.
//!
//! - ACL: SYSTEM control total; usuarios interactivos lectura/escritura; nadie remoto.
//! - Un cliente a la vez (la UI); petición y respuesta JSON delimitadas por salto de línea.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::{AsRawHandle, FromRawHandle};

use anyhow::Result;
use model::TweakKind;
use optimizador_core as oc;
use serde::Deserialize;
use windows::Win32::Foundation::HANDLE;

use crate::protocol::{Request, Response};
use crate::{frames, pipe, sensors, ServiceState};

pub const GAME_LABEL: &str = "game";

/// Partida en curso.
struct GameRun {
    session: i64,
    frames: frames::PresentMon,
    exe: String,
}

/// Acciones del controlador tal como llegan del núcleo (JSON de `profiles::controller::Action`).
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum CoreAction {
    SetFpsCap {
        fps: u16,
    },
    RaiseFpsCap {
        fps: Option<u16>,
    },
    Tweak(TweakKind),
    RestrictHeavyBackgroundApps,
    #[allow(dead_code)]
    LogForReport {
        cause: String,
    },
}

pub fn serve_forever(state: ServiceState) -> Result<()> {
    let mut game: Option<GameRun> = None;
    loop {
        let h = pipe::create()?;
        if !pipe::wait_for_client(h) {
            pipe::close(h); // antes se perdía el handle en este camino
            continue;
        }
        // `File` pasa a ser el dueño del handle y lo cierra al soltarse.
        let file = unsafe { File::from_raw_handle(h.0 as _) };
        if let Err(e) = handle_client(&file, &state, &mut game) {
            eprintln!("cliente desconectado: {e:#}");
        }
        pipe::disconnect(HANDLE(file.as_raw_handle() as _));
        drop(file);

        // Si la UI se cierra en mitad de una partida, se deshace todo.
        if let Some(run) = game.take() {
            stop_game(&state, run);
        }
    }
}

fn handle_client(file: &File, state: &ServiceState, game: &mut Option<GameRun>) -> Result<()> {
    let mut reader = BufReader::new(file);
    let mut writer = file;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            return Ok(()); // la UI cerró
        }
        let response = match serde_json::from_str::<Request>(line.trim()) {
            Ok(req) => dispatch(req, state, game).unwrap_or_else(|e| Response::err(format!("{e:#}"))),
            Err(e) => Response::err(format!("petición inválida: {e}")),
        };
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        writer.write_all(out.as_bytes())?;
        writer.flush()?;
    }
}

fn dispatch(req: Request, state: &ServiceState, game: &mut Option<GameRun>) -> Result<Response> {
    Ok(match req {
        Request::Ping => Response::ok(oc::core_version()),
        Request::Scan { game_mode_on } => {
            let mut snap = sensors::take_snapshot(&state.hw)?;
            snap.game_mode_on = game_mode_on;
            let snap_json = serde_json::to_string(&snap)?;
            let scan: serde_json::Value = serde_json::from_str(&oc::scan(snap_json, None)?)?;
            Response::ok(serde_json::json!({ "snapshot": snap, "scan": scan }))
        }
        Request::Apply { label, tweaks } => {
            // Game Mode vive en HKCU: lo aplica la UI en el contexto del usuario.
            let tweaks: Vec<TweakKind> =
                tweaks.into_iter().filter(|t| !matches!(t, TweakKind::EnableGameMode)).collect();
            let session = state.journal.begin_session(&label)?;
            let report = state.journal.apply_all(&state.applier, session, &tweaks);
            Response::ok(serde_json::json!({ "session": session, "report": report }))
        }
        Request::Undo { session } => Response::ok(state.journal.undo_session(&state.applier, session)?),
        Request::GameStart { pid, exe } => {
            if let Some(run) = game.take() {
                stop_game(state, run);
            }
            let snap_json = serde_json::to_string(&sensors::take_snapshot(&state.hw)?)?;
            let profile = oc::profile(snap_json)?;
            let actions: Vec<CoreAction> = serde_json::from_str(&oc::game_start(profile, exe.clone(), None)?)?;
            let session = state.journal.begin_session(GAME_LABEL)?;
            let recommendations = apply_actions(state, session, actions);
            let frames = frames::PresentMon::start(pid)?;
            *game = Some(GameRun { session, frames, exe });
            Response::ok(serde_json::json!({ "session": session, "recommendations": recommendations }))
        }
        Request::GameTick => {
            let Some(run) = game.as_mut() else { anyhow::bail!("no hay partida en curso") };
            oc::game_push_frames(run.frames.drain());
            let snap = sensors::take_snapshot_fast(&state.hw, &run.exe)?;
            let tick: serde_json::Value = serde_json::from_str(&oc::game_tick(serde_json::to_string(&snap)?)?)?;
            let actions: Vec<CoreAction> = serde_json::from_value(tick["actions"].clone()).unwrap_or_default();
            let recommendations = apply_actions(state, run.session, actions);
            Response::ok(serde_json::json!({
                "stats": tick["stats"],
                "fps_cap": tick["fps_cap"],
                "thermal": snap.thermal,
                "recommendations": recommendations,
            }))
        }
        Request::GameStop => {
            let restored = game.take().map(|run| stop_game(state, run)).unwrap_or(0);
            Response::ok(serde_json::json!({ "restored": restored }))
        }
    })
}

/// Aplica en el journal lo que el agente sabe hacer; el resto vuelve a la UI como recomendación
/// (p. ej. un tope de FPS que en Windows se fija en el juego o en el driver).
fn apply_actions(state: &ServiceState, session: i64, actions: Vec<CoreAction>) -> Vec<serde_json::Value> {
    let mut recommendations = Vec::new();
    let mut tweaks = Vec::new();
    for a in actions {
        match a {
            CoreAction::Tweak(t) => tweaks.push(t),
            CoreAction::SetFpsCap { fps } => recommendations.push(serde_json::json!({ "fps_cap": fps })),
            CoreAction::RaiseFpsCap { fps } => recommendations.push(serde_json::json!({ "fps_cap": fps })),
            CoreAction::RestrictHeavyBackgroundApps => {
                recommendations.push(serde_json::json!({ "close_heavy_apps": true }))
            }
            CoreAction::LogForReport { .. } => {}
        }
    }
    if !tweaks.is_empty() {
        let report = state.journal.apply_all(&state.applier, session, &tweaks);
        for (t, err) in report.failed {
            eprintln!("no aplicado {t:?}: {err}");
        }
    }
    recommendations
}

fn stop_game(state: &ServiceState, mut run: GameRun) -> usize {
    run.frames.stop();
    oc::game_stop();
    state.journal.undo_session(&state.applier, run.session).unwrap_or_else(|e| {
        eprintln!("deshacer partida: {e:#}");
        0
    })
}
