//! Un solo número 0-100 para la pantalla principal.
//!
//! Cada problema resta el cuadrado de su severidad: un problema grave (térmico, 9² = 81)
//! pesa más que varios leves juntos, que es como lo percibe quien juega.

use model::Issue;
#[cfg(feature = "serde")]
use serde::Serialize;

/// Penalización que lleva la puntuación a 0.
const FULL_PENALTY: u32 = 180;

pub fn health_score(issues: &[Issue]) -> u8 {
    let penalty: u32 = issues.iter().map(|i| u32::from(i.severity.min(10)).pow(2)).sum();
    100u32.saturating_sub(penalty * 100 / FULL_PENALTY) as u8
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum HealthLabel {
    Excellent,
    Good,
    NeedsAttention,
    Poor,
}

impl HealthLabel {
    pub fn for_score(score: u8) -> Self {
        match score {
            90..=100 => HealthLabel::Excellent,
            70..=89 => HealthLabel::Good,
            40..=69 => HealthLabel::NeedsAttention,
            _ => HealthLabel::Poor,
        }
    }

    /// Clave i18n para la UI.
    pub fn key(self) -> &'static str {
        match self {
            HealthLabel::Excellent => "health.excellent",
            HealthLabel::Good => "health.good",
            HealthLabel::NeedsAttention => "health.needs_attention",
            HealthLabel::Poor => "health.poor",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issue(sev: u8) -> Issue {
        Issue { id: "x".into(), severity: sev, title_key: "k".into(), fix: vec![], safe_for_one_click: true }
    }

    #[test]
    fn no_issues_is_100() {
        assert_eq!(health_score(&[]), 100);
        assert_eq!(HealthLabel::for_score(100), HealthLabel::Excellent);
    }

    #[test]
    fn thermal_alone_needs_attention() {
        let s = health_score(&[issue(9)]);
        assert_eq!(s, 55);
        assert_eq!(HealthLabel::for_score(s), HealthLabel::NeedsAttention);
    }

    #[test]
    fn minor_issues_stay_good() {
        let s = health_score(&[issue(2), issue(4)]);
        assert!(s >= 85, "{s}");
    }

    #[test]
    fn many_issues_floor_at_zero() {
        assert_eq!(health_score(&[issue(10), issue(10)]), 0);
    }
}
