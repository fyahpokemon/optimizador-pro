//! Journal de reversión.
//!
//! Antes de aplicar un ajuste se guarda el valor anterior; "Deshacer" recorre la sesión
//! en orden inverso y restaura cada valor. El journal sobrevive a reinicios (SQLite), de
//! modo que si la app o el equipo se cierran durante una partida, el siguiente arranque
//! puede deshacer lo pendiente.

use anyhow::{Context as _, Result};
use model::TweakKind;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: i64,
    pub session: i64,
    pub ts_ms: i64,
    pub tweak: TweakKind,
    pub previous: Value,
    pub reverted: bool,
}

/// Lo implementa cada agente nativo (Windows, Android).
pub trait Applier {
    /// Lee el valor actual que `apply` va a cambiar.
    fn read_current(&self, t: &TweakKind) -> Result<Value>;
    fn apply(&self, t: &TweakKind) -> Result<()>;
    fn restore(&self, t: &TweakKind, previous: &Value) -> Result<()>;
}

pub struct Journal {
    conn: Connection,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct ApplyReport {
    pub applied: usize,
    pub failed: Vec<(TweakKind, String)>,
}

impl Journal {
    pub fn open(path: &str) -> Result<Self> {
        Self::init(Connection::open(path).with_context(|| format!("abrir journal en {path}"))?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        // `journal_mode` devuelve una fila: se lee con query_row (execute fallaría).
        conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 started_ms INTEGER NOT NULL,
                 label TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS entries (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session INTEGER NOT NULL REFERENCES sessions(id),
                 ts_ms INTEGER NOT NULL,
                 tweak TEXT NOT NULL,
                 previous TEXT NOT NULL,
                 reverted INTEGER NOT NULL DEFAULT 0
             );",
        )?;
        Ok(Journal { conn })
    }

    pub fn begin_session(&self, label: &str) -> Result<i64> {
        self.conn.execute("INSERT INTO sessions (started_ms, label) VALUES (?1, ?2)", params![now_ms(), label])?;
        Ok(self.conn.last_insert_rowid())
    }

    fn record(&self, session: i64, t: &TweakKind, previous: &Value) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO entries (session, ts_ms, tweak, previous) VALUES (?1, ?2, ?3, ?4)",
            params![session, now_ms(), serde_json::to_string(t)?, previous.to_string()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn forget(&self, entry: i64) -> Result<()> {
        self.conn.execute("DELETE FROM entries WHERE id = ?1", params![entry])?;
        Ok(())
    }

    pub fn entries(&self, session: i64) -> Result<Vec<Entry>> {
        let mut st = self.conn.prepare(
            "SELECT id, session, ts_ms, tweak, previous, reverted FROM entries WHERE session = ?1 ORDER BY id",
        )?;
        let rows = st.query_map(params![session], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, bool>(5)?,
            ))
        })?;
        rows.map(|row| {
            let (id, session, ts_ms, tweak, previous, reverted) = row?;
            Ok(Entry {
                id,
                session,
                ts_ms,
                tweak: serde_json::from_str(&tweak)?,
                previous: serde_json::from_str(&previous)?,
                reverted,
            })
        })
        .collect()
    }

    /// Sesiones con cambios sin deshacer, opcionalmente filtradas por etiqueta. Al arrancar,
    /// el agente deshace las de etiqueta `"game"` (partida interrumpida); las de "Optimizar
    /// con 1 clic" se mantienen hasta que el usuario pulse "Deshacer".
    pub fn pending_sessions(&self, label: Option<&str>) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(
            "SELECT DISTINCT e.session FROM entries e JOIN sessions s ON s.id = e.session
             WHERE e.reverted = 0 AND (?1 IS NULL OR s.label = ?1) ORDER BY e.session",
        )?;
        let v = st.query_map(params![label], |r| r.get(0))?.collect::<rusqlite::Result<Vec<i64>>>()?;
        Ok(v)
    }

    /// Aplica los ajustes registrando antes el valor previo. Un fallo no detiene el resto.
    pub fn apply_all(&self, a: &dyn Applier, session: i64, tweaks: &[TweakKind]) -> ApplyReport {
        let mut report = ApplyReport::default();
        for t in tweaks {
            let res = (|| -> Result<()> {
                let prev = if t.is_reversible() { a.read_current(t)? } else { Value::Null };
                let entry = self.record(session, t, &prev)?;
                if let Err(e) = a.apply(t) {
                    self.forget(entry)?; // nada que deshacer
                    return Err(e);
                }
                Ok(())
            })();
            match res {
                Ok(()) => report.applied += 1,
                Err(e) => report.failed.push((t.clone(), format!("{e:#}"))),
            }
        }
        report
    }

    /// Deshace una sesión en orden inverso. Devuelve cuántos cambios se restauraron.
    pub fn undo_session(&self, a: &dyn Applier, session: i64) -> Result<usize> {
        let mut restored = 0;
        let mut errors = Vec::new();
        for e in self.entries(session)?.into_iter().rev().filter(|e| !e.reverted) {
            if e.tweak.is_reversible() {
                if let Err(err) = a.restore(&e.tweak, &e.previous) {
                    errors.push(format!("{:?}: {err:#}", e.tweak));
                    continue;
                }
                restored += 1;
            }
            self.conn.execute("UPDATE entries SET reverted = 1 WHERE id = ?1", params![e.id])?;
        }
        if errors.is_empty() {
            Ok(restored)
        } else {
            anyhow::bail!("{} cambios no se pudieron restaurar: {}", errors.len(), errors.join("; "))
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Sistema simulado: un mapa clave → valor.
    #[derive(Default)]
    struct FakeSystem {
        state: RefCell<HashMap<String, Value>>,
        fail_on: Option<String>,
    }

    fn key(t: &TweakKind) -> String {
        match t {
            TweakKind::SetPowerPlan { .. } => "plan".into(),
            TweakKind::EnableGameMode => "game_mode".into(),
            TweakKind::ThrottleBackground { pid, .. } => format!("prio:{pid}"),
            other => format!("{other:?}"),
        }
    }

    fn target(t: &TweakKind) -> Value {
        match t {
            TweakKind::SetPowerPlan { plan } => Value::from(plan.clone()),
            TweakKind::EnableGameMode => Value::from(true),
            TweakKind::ThrottleBackground { .. } => Value::from("below_normal"),
            _ => Value::from("done"),
        }
    }

    impl Applier for FakeSystem {
        fn read_current(&self, t: &TweakKind) -> Result<Value> {
            Ok(self.state.borrow().get(&key(t)).cloned().unwrap_or(Value::Null))
        }
        fn apply(&self, t: &TweakKind) -> Result<()> {
            if self.fail_on.as_deref() == Some(key(t).as_str()) {
                anyhow::bail!("acceso denegado");
            }
            self.state.borrow_mut().insert(key(t), target(t));
            Ok(())
        }
        fn restore(&self, t: &TweakKind, previous: &Value) -> Result<()> {
            self.state.borrow_mut().insert(key(t), previous.clone());
            Ok(())
        }
    }

    fn sys() -> FakeSystem {
        let s = FakeSystem::default();
        s.state.borrow_mut().insert("plan".into(), Value::from("balanced"));
        s.state.borrow_mut().insert("game_mode".into(), Value::from(false));
        s.state.borrow_mut().insert("prio:7".into(), Value::from("normal"));
        s
    }

    fn tweaks() -> Vec<TweakKind> {
        vec![
            TweakKind::SetPowerPlan { plan: "high_performance".into() },
            TweakKind::EnableGameMode,
            TweakKind::ThrottleBackground { pid: 7, name: "updater.exe".into() },
            TweakKind::CleanTemp,
        ]
    }

    #[test]
    fn apply_and_undo_restores_everything() {
        let j = Journal::in_memory().unwrap();
        let s = sys();
        let before = s.state.borrow().clone();
        let session = j.begin_session("one_click").unwrap();
        let r = j.apply_all(&s, session, &tweaks());
        assert_eq!(r.applied, 4);
        assert_ne!(s.state.borrow().get("plan"), before.get("plan"));

        assert_eq!(j.undo_session(&s, session).unwrap(), 3); // CleanTemp no se revierte
        for k in ["plan", "game_mode", "prio:7"] {
            assert_eq!(s.state.borrow().get(k), before.get(k), "{k}");
        }
        assert!(j.pending_sessions(None).unwrap().is_empty());
    }

    #[test]
    fn hundred_cycles_leave_no_residue() {
        let j = Journal::in_memory().unwrap();
        let s = sys();
        let before: Vec<_> =
            ["plan", "game_mode", "prio:7"].iter().map(|k| s.state.borrow().get(*k).cloned()).collect();
        for _ in 0..100 {
            let id = j.begin_session("cycle").unwrap();
            j.apply_all(&s, id, &tweaks());
            j.undo_session(&s, id).unwrap();
        }
        let after: Vec<_> = ["plan", "game_mode", "prio:7"].iter().map(|k| s.state.borrow().get(*k).cloned()).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn failed_apply_is_reported_and_not_journaled() {
        let j = Journal::in_memory().unwrap();
        let s = FakeSystem { fail_on: Some("game_mode".into()), ..sys() };
        let id = j.begin_session("one_click").unwrap();
        let r = j.apply_all(&s, id, &tweaks());
        assert_eq!(r.applied, 3);
        assert_eq!(r.failed.len(), 1);
        assert!(j.entries(id).unwrap().iter().all(|e| e.tweak != TweakKind::EnableGameMode));
    }

    #[test]
    fn pending_sessions_survive_until_undone() {
        let j = Journal::in_memory().unwrap();
        let s = sys();
        let id = j.begin_session("game").unwrap();
        j.apply_all(&s, id, &tweaks()[..1]);
        assert_eq!(j.pending_sessions(None).unwrap(), vec![id]);
        j.undo_session(&s, id).unwrap();
        assert!(j.pending_sessions(None).unwrap().is_empty());
    }

    #[test]
    fn pending_sessions_filter_by_label() {
        let j = Journal::in_memory().unwrap();
        let s = sys();
        let one_click = j.begin_session("one_click").unwrap();
        j.apply_all(&s, one_click, &tweaks()[..1]);
        let game = j.begin_session("game").unwrap();
        j.apply_all(&s, game, &tweaks()[1..2]);
        assert_eq!(j.pending_sessions(Some("game")).unwrap(), vec![game]);
        assert_eq!(j.pending_sessions(None).unwrap(), vec![one_click, game]);
    }
}
