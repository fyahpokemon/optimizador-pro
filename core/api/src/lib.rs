//! API pública del núcleo.
//!
//! Los tipos cruzan la frontera como JSON para que el mismo contrato sirva a la UI
//! (flutter_rust_bridge), al servicio de Windows (named pipe) y al módulo de Android.

pub mod api;
mod engine;
pub use api::*;
pub use model;

// Código generado por flutter_rust_bridge (solo al compilar para la app).
#[cfg(feature = "frb")]
mod frb_generated;

#[cfg(test)]
mod tests {
    use super::*;
    use model::{Platform, Snapshot};

    fn snapshot_json() -> String {
        let mut s = Snapshot::empty(Platform::Windows);
        s.hw.logical_cores = 8;
        s.hw.ram_total_mb = 16 * 1024;
        s.power_plan = Some("balanced".into());
        s.game_mode_on = Some(false);
        s.thermal = model::Thermal::Moderate;
        serde_json::to_string(&s).unwrap()
    }

    #[test]
    fn scan_roundtrip() {
        let out: serde_json::Value = serde_json::from_str(&scan(snapshot_json(), None).unwrap()).unwrap();
        assert_eq!(out["headline_key"], "headline.overheating");
        assert_eq!(out["tier"], "mid");
        assert!(out["score"].as_u64().unwrap() < 70);
        assert_eq!(out["one_click"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn game_session_flow() {
        let p = profile(snapshot_json()).unwrap();
        let start: serde_json::Value = serde_json::from_str(&game_start(p, "game.exe".into(), None).unwrap()).unwrap();
        assert_eq!(start[0]["action"], "set_fps_cap");
        game_push_frames(vec![16.6; 120]);
        let tick: serde_json::Value = serde_json::from_str(&game_tick(snapshot_json()).unwrap()).unwrap();
        assert!(tick["stats"]["avg_fps"].as_f64().unwrap() > 59.0);
        game_stop();
        assert!(game_tick(snapshot_json()).is_err());
    }
}
