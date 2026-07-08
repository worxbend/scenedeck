//! Domain diagnostics produced by hardening checks.

/// Severity used to sort and group diagnostics in the Doctor page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    /// Informational issue that does not block normal workflows.
    Info,
    /// Suspicious setup that is allowed but likely worth correcting.
    Warning,
    /// Structural issue that can break or confuse scene operation.
    Error,
}

impl DiagnosticSeverity {
    /// Display order for summaries and grouped diagnostic rows.
    pub const DISPLAY_ORDER: [Self; 3] = [Self::Error, Self::Warning, Self::Info];

    /// User-facing group label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }

    /// Symbolic icon name for rows carrying this severity.
    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Info => "dialog-information-symbolic",
            Self::Warning => "dialog-warning-symbolic",
            Self::Error => "dialog-error-symbolic",
        }
    }

    /// CSS class applied to the severity icon.
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Info => "diag-info",
            Self::Warning => "diag-warning",
            Self::Error => "diag-error",
        }
    }

    /// Count diagnostics with this severity.
    pub fn count_in(self, diagnostics: &[Diagnostic]) -> usize {
        diagnostics.iter().filter(|d| d.severity == self).count()
    }

    /// User-facing count label with correct singular/plural wording.
    pub fn format_count(self, count: usize) -> String {
        let noun = if count == 1 {
            self.singular_count_label()
        } else {
            self.plural_count_label()
        };
        format!("{count} {noun}")
    }

    const fn singular_count_label(self) -> &'static str {
        match self {
            Self::Info => "info item",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    const fn plural_count_label(self) -> &'static str {
        match self {
            Self::Info => "info items",
            Self::Warning => "warnings",
            Self::Error => "errors",
        }
    }
}

/// A single architecture or configuration finding shown in the Doctor page.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Importance and grouping for the finding.
    pub severity: DiagnosticSeverity,
    /// Scene name this diagnostic refers to, if any.
    pub scene: Option<String>,
    /// Short user-facing description of the problem.
    pub message: String,
    /// Short human-readable fix suggestion.
    pub suggestion: Option<String>,
}

impl Diagnostic {
    /// User-facing row title combining scene context and message.
    pub fn title(&self) -> String {
        self.scene
            .as_deref()
            .map(|scene| format!("{scene}: {}", self.message))
            .unwrap_or_else(|| self.message.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_order_prioritizes_actionable_findings() {
        assert_eq!(
            DiagnosticSeverity::DISPLAY_ORDER,
            [
                DiagnosticSeverity::Error,
                DiagnosticSeverity::Warning,
                DiagnosticSeverity::Info,
            ]
        );
    }

    #[test]
    fn count_labels_use_singular_and_plural_wording() {
        assert_eq!(DiagnosticSeverity::Error.format_count(1), "1 error");
        assert_eq!(DiagnosticSeverity::Error.format_count(2), "2 errors");
        assert_eq!(DiagnosticSeverity::Warning.format_count(1), "1 warning");
        assert_eq!(DiagnosticSeverity::Warning.format_count(3), "3 warnings");
        assert_eq!(DiagnosticSeverity::Info.format_count(1), "1 info item");
        assert_eq!(DiagnosticSeverity::Info.format_count(0), "0 info items");
    }

    #[test]
    fn count_in_filters_by_severity() {
        let diagnostics = vec![
            diagnostic(DiagnosticSeverity::Error),
            diagnostic(DiagnosticSeverity::Warning),
            diagnostic(DiagnosticSeverity::Warning),
        ];

        assert_eq!(DiagnosticSeverity::Error.count_in(&diagnostics), 1);
        assert_eq!(DiagnosticSeverity::Warning.count_in(&diagnostics), 2);
        assert_eq!(DiagnosticSeverity::Info.count_in(&diagnostics), 0);
    }

    #[test]
    fn diagnostic_title_includes_scene_context_when_present() {
        let diagnostic = Diagnostic {
            severity: DiagnosticSeverity::Warning,
            scene: Some("Main".to_string()),
            message: "depends on Raw".to_string(),
            suggestion: None,
        };

        assert_eq!(diagnostic.title(), "Main: depends on Raw");
    }

    #[test]
    fn diagnostic_title_uses_message_for_global_findings() {
        let diagnostic = Diagnostic {
            severity: DiagnosticSeverity::Info,
            scene: None,
            message: "No scene role assigned".to_string(),
            suggestion: None,
        };

        assert_eq!(diagnostic.title(), "No scene role assigned");
    }

    fn diagnostic(severity: DiagnosticSeverity) -> Diagnostic {
        Diagnostic {
            severity,
            scene: None,
            message: String::new(),
            suggestion: None,
        }
    }
}
