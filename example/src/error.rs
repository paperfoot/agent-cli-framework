use std::borrow::Cow;

/// Error types with semantic exit codes.
///
/// Every error maps to an exit code (1-4), a machine-readable code, and a
/// recovery suggestion that agents can follow literally.

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // Some variants demonstrate the full exit code contract (0-4)
pub enum AppError {
    #[error("Could not serialize command output")]
    Serialization,

    #[error("doctor found failing checks")]
    Diagnostics(serde_json::Value),

    #[error("Operation already running")]
    OperationBusy,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
    /// A parse error inside a known command; `command` is its canonical path.
    #[error("Invalid input: {message}")]
    InvalidArguments { message: String, command: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Transient(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Update failed: {0}")]
    Update(String),
}

impl AppError {
    pub fn details(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Diagnostics(report) => Some(report),
            _ => None,
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) | Self::InvalidArguments { .. } | Self::OperationBusy => 3,
            Self::Config(_) | Self::Diagnostics(_) => 2,
            Self::RateLimited(_) => 4,
            Self::Transient(_) | Self::Io(_) | Self::Update(_) | Self::Serialization => 1,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::Serialization => "serialization_error",
            Self::Diagnostics(_) => "config_error",
            Self::OperationBusy => "operation_busy",
            Self::InvalidInput(_) | Self::InvalidArguments { .. } => "invalid_input",
            Self::Config(_) => "config_error",
            Self::Transient(_) => "transient_error",
            Self::RateLimited(_) => "rate_limited",
            Self::Io(_) => "io_error",
            Self::Update(_) => "update_error",
        }
    }

    pub fn suggestion(&self) -> Cow<'static, str> {
        match self {
            Self::Serialization => {
                "Report this output serialization failure to the tool maintainer".into()
            }
            Self::Diagnostics(_) => {
                "Fix the failed checks listed in error.details, then run doctor again".into()
            }
            Self::OperationBusy => "Operation already running. Use --force to override.".into(),
            Self::InvalidInput(_) => {
                concat!("Check arguments with: ", env!("CARGO_PKG_NAME"), " --help").into()
            }
            // Point at the failing command's own contract, not the root help,
            // so one scoped lookup fixes the retry.
            Self::InvalidArguments { command, .. } => format!(
                "See valid arguments with: {} agent-info --command \"{command}\"",
                env!("CARGO_PKG_NAME")
            )
            .into(),
            Self::Config(_) => {
                concat!("Check config with: ", env!("CARGO_PKG_NAME"), " config path").into()
            }
            Self::Transient(_) | Self::Io(_) => {
                "Inspect the operation result before retrying; a failed response does not prove a write failed".into()
            }
            Self::RateLimited(_) => {
                "Respect the provider backoff; retry only when the operation is safe to repeat".into()
            }
            Self::Update(_) => concat!(
                "Inspect the installed version and update instructions with: ",
                env!("CARGO_PKG_NAME"),
                " update --check"
            )
            .into(),
        }
    }
}
