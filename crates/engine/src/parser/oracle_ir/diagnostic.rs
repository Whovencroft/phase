//! Typed Oracle parse diagnostics (Phase 50, D-04).
//!
//! Replaces thread-local `push_warning` string accumulation with
//! machine-readable diagnostics carrying severity and source provenance.

use std::fmt;

/// Severity level for parse diagnostics (D-05).
/// Derived from the variant — not stored as a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Which cascade slot was lost in a cascade-diff diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum CascadeSlot {
    Optional,
    OpponentMay,
    Condition,
    RepeatFor,
    PlayerScope,
    Duration,
}

/// Typed Oracle parse diagnostic (D-04).
///
/// Every variant carries `line_index` for source provenance (D-06).
/// Severity is determined by variant via `severity()` method (D-05).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum OracleDiagnostic {
    /// Parser fell back to a degraded target filter (TargetFilter::Any or similar).
    /// Covers both target-fallback and bare-filter-fallback categories.
    TargetFallback {
        context: String,
        text: String,
        line_index: usize,
    },

    /// Text remained after a successful parse that was silently discarded.
    IgnoredRemainder {
        text: String,
        parser: String,
        line_index: usize,
    },

    /// Swallow-check detector found Oracle text not represented in parsed output.
    SwallowedClause {
        detector: String,
        description: String,
        line_index: usize,
    },

    /// Cascade-diff: a cascade slot was populated but did not land on the final def.
    CascadeLoss {
        slot: CascadeSlot,
        effect_name: String,
        line_index: usize,
    },
}

impl OracleDiagnostic {
    /// Severity level, determined by variant (D-05).
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::TargetFallback { .. } => DiagnosticSeverity::Warning,
            Self::IgnoredRemainder { .. } => DiagnosticSeverity::Info,
            Self::SwallowedClause { .. } => DiagnosticSeverity::Warning,
            Self::CascadeLoss { .. } => DiagnosticSeverity::Warning,
        }
    }

    /// Oracle text line index (D-06 provenance).
    pub fn line_index(&self) -> usize {
        match self {
            Self::TargetFallback { line_index, .. }
            | Self::IgnoredRemainder { line_index, .. }
            | Self::SwallowedClause { line_index, .. }
            | Self::CascadeLoss { line_index, .. } => *line_index,
        }
    }

    /// Diagnostic category name for regression tracking (D-08).
    pub fn category_name(&self) -> &'static str {
        match self {
            Self::TargetFallback { .. } => "target-fallback",
            Self::IgnoredRemainder { .. } => "ignored-remainder",
            Self::SwallowedClause { .. } => "swallowed-clause",
            Self::CascadeLoss { .. } => "cascade-loss",
        }
    }
}

/// Display impl uses structured [severity:category] prefix format (D-11).
impl fmt::Display for OracleDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let severity = match self.severity() {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Info => "info",
        };
        let category = self.category_name();
        match self {
            Self::TargetFallback { context, text, .. } => {
                write!(f, "[{severity}:{category}] {context} '{text}'")
            }
            Self::IgnoredRemainder { text, parser, .. } => {
                write!(f, "[{severity}:{category}] ({parser}) '{text}'")
            }
            Self::SwallowedClause {
                detector,
                description,
                ..
            } => {
                write!(f, "[{severity}:{category}] {detector} — {description}")
            }
            Self::CascadeLoss {
                slot, effect_name, ..
            } => {
                write!(
                    f,
                    "[{severity}:{category}] {slot:?} lost (effect={effect_name})"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_mapping() {
        let diag = OracleDiagnostic::TargetFallback {
            context: "test".into(),
            text: "foo".into(),
            line_index: 0,
        };
        assert_eq!(diag.severity(), DiagnosticSeverity::Warning);

        let diag = OracleDiagnostic::IgnoredRemainder {
            text: "bar".into(),
            parser: "test".into(),
            line_index: 0,
        };
        assert_eq!(diag.severity(), DiagnosticSeverity::Info);
    }

    #[test]
    fn line_index_accessor() {
        let diag = OracleDiagnostic::CascadeLoss {
            slot: CascadeSlot::Condition,
            effect_name: "DealDamage".into(),
            line_index: 5,
        };
        assert_eq!(diag.line_index(), 5);
    }
}
