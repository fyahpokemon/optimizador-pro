//! Contrato del named pipe. Una petición JSON por línea → una respuesta JSON por línea.
//! El mismo contrato lo implementa `app/lib/platform/windows_agent.dart`.

use model::TweakKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
#[allow(dead_code)] // en Linux solo se compila el contrato
pub enum Request {
    /// Escaneo completo: instantánea + diagnóstico del núcleo. La UI aporta el estado de
    /// Game Mode porque vive en el registro del usuario (HKCU), no en el del servicio.
    Scan {
        #[serde(default)]
        game_mode_on: Option<bool>,
    },
    /// Aplica ajustes en una sesión nueva; devuelve el id de sesión.
    Apply {
        label: String,
        tweaks: Vec<TweakKind>,
    },
    Undo {
        session: i64,
    },
    /// Modo juego: empieza a medir frametimes del proceso y a controlar.
    GameStart {
        pid: u32,
        exe: String,
    },
    /// Un paso del bucle (la UI lo pide cada segundo mientras dura la partida).
    GameTick,
    GameStop,
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Response {
    Ok { data: serde_json::Value },
    Error { message: String },
}

#[allow(dead_code)]
impl Response {
    pub fn ok(data: impl Serialize) -> Self {
        Response::Ok { data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null) }
    }
    pub fn err(e: impl std::fmt::Display) -> Self {
        Response::Error { message: e.to_string() }
    }
}
