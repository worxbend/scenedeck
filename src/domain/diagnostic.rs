#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl DiagnosticSeverity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Warning => "Warnings",
            Self::Error => "Errors",
        }
    }

    pub const fn icon_name(self) -> &'static str {
        match self {
            Self::Info => "dialog-information-symbolic",
            Self::Warning => "dialog-warning-symbolic",
            Self::Error => "dialog-error-symbolic",
        }
    }

    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Info => "diag-info",
            Self::Warning => "diag-warning",
            Self::Error => "diag-error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    /// Scene name this diagnostic refers to, if any.
    pub scene: Option<String>,
    pub message: String,
    /// Short human-readable fix suggestion.
    pub suggestion: Option<String>,
}
