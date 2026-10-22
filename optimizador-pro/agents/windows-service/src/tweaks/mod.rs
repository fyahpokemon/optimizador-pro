//! Ajustes de Windows con su lectura previa y su reversión (`journal::Applier`).

pub mod cleanup;
mod process;
mod services;

use anyhow::{bail, Result};
use model::TweakKind;
use serde_json::{json, Value};

use crate::sensors;
use process::{class_for, get_priority_class, set_eco_qos, set_priority_class, BELOW_NORMAL_CLASS, NORMAL_CLASS};

pub struct WinApplier;

impl journal::Applier for WinApplier {
    fn read_current(&self, t: &TweakKind) -> Result<Value> {
        Ok(match t {
            TweakKind::SetPowerPlan { .. } => json!(sensors::active_plan()?.1),
            TweakKind::SetPriority { pid, .. } => json!({ "class": get_priority_class(*pid)? }),
            TweakKind::ThrottleBackground { pid, .. } => {
                json!({ "class": get_priority_class(*pid)?, "eco": false })
            }
            TweakKind::StopService { name } => json!(services::is_running(name)?),
            _ => Value::Null,
        })
    }

    fn apply(&self, t: &TweakKind) -> Result<()> {
        match t {
            TweakKind::SetPowerPlan { plan } => {
                if !sensors::power_plan_available(plan) {
                    bail!("el plan {plan} no existe en este equipo (Modern Standby)");
                }
                sensors::set_plan(plan)
            }
            TweakKind::SetPriority { pid, level } => set_priority_class(*pid, class_for(*level)),
            TweakKind::ThrottleBackground { pid, .. } => {
                set_priority_class(*pid, BELOW_NORMAL_CLASS)?;
                // EcoQoS (Windows 11): núcleos eficientes y menor frecuencia para ese proceso.
                // En Windows 10 falla sin consecuencias: basta con la prioridad.
                let _ = set_eco_qos(*pid, true);
                Ok(())
            }
            TweakKind::StopService { name } => services::stop_if_allowed(name),
            TweakKind::CleanTemp => cleanup::clean_temp().map(|_| ()),
            TweakKind::EnableGameMode => bail!("Game Mode se aplica desde la app (registro del usuario)"),
            TweakKind::SetFpsCap { .. } => bail!("el tope de FPS se fija en el juego o en el driver"),
            TweakKind::ExemptFromBatteryOpt { .. } | TweakKind::OpenSystemScreen { .. } => {
                bail!("no aplica en Windows")
            }
        }
    }

    fn restore(&self, t: &TweakKind, previous: &Value) -> Result<()> {
        match t {
            TweakKind::SetPowerPlan { .. } => match previous.as_str() {
                Some(guid) => sensors::set_plan_by_guid(guid),
                None => Ok(()),
            },
            TweakKind::SetPriority { pid, .. } | TweakKind::ThrottleBackground { pid, .. } => {
                let _ = set_eco_qos(*pid, false);
                let class = previous["class"].as_u64().map_or(NORMAL_CLASS, |c| c as u32);
                // El proceso puede haber terminado: no es un error.
                let _ = set_priority_class(*pid, class);
                Ok(())
            }
            TweakKind::StopService { name } => {
                if previous.as_bool() == Some(true) {
                    services::start(name)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
