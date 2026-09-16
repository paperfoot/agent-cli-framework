//! Compact JSON for pipes; human output for terminals. Serialize before writing.
use serde::Serialize;
use std::io::{IsTerminal, Write};

use crate::error::AppError;

#[derive(Clone, Copy)]
pub enum Format {
    Json,
    Human,
}

impl Format {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Self::Json
        } else {
            Self::Human
        }
    }
}

#[derive(Clone, Copy)]
pub struct Ctx {
    pub format: Format,
    pub quiet: bool,
}

impl Ctx {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        Self {
            format: Format::detect(json_flag),
            quiet,
        }
    }
}

fn write_json<T: Serialize>(mut writer: impl Write, value: &T) -> Result<(), AppError> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| AppError::Serialization)?;
    bytes.push(b'\n');
    writer.write_all(&bytes)?;
    Ok(())
}

pub fn print_json<T: Serialize>(value: &T) -> Result<(), AppError> {
    write_json(std::io::stdout().lock(), value)
}

pub fn print_success_or<T: Serialize, F: FnOnce(&T)>(
    ctx: Ctx,
    data: &T,
    human: F,
) -> Result<(), AppError> {
    match ctx.format {
        Format::Json => {
            // json! on a failing Serialize implementation would panic. Convert
            // explicitly so no success bytes escape before serialization succeeds.
            let data = serde_json::to_value(data).map_err(|_| AppError::Serialization)?;
            print_json(&serde_json::json!({"version": "1", "status": "success", "data": data}))?;
        }
        Format::Human if !ctx.quiet => human(data),
        Format::Human => {}
    }
    Ok(())
}

pub fn print_error(format: Format, err: &AppError) {
    let mut error = serde_json::json!({
        "code": err.error_code(), "message": err.to_string(), "suggestion": err.suggestion(),
    });
    if let Some(details) = err.details() {
        error["details"] = details.clone();
    }
    match format {
        Format::Json => {
            // If stderr itself is closed, there is nowhere else safe to report.
            let _ = write_json(
                std::io::stderr().lock(),
                &serde_json::json!({
                    "version": "1", "status": "error", "error": error,
                }),
            );
        }
        Format::Human => {
            use owo_colors::OwoColorize;
            let mut stderr = std::io::stderr().lock();
            let _ = writeln!(
                stderr,
                "{} {}\n  {}",
                "error:".red().bold(),
                err,
                err.suggestion()
            );
            if let Some(details) = err.details() {
                let _ = writeln!(stderr, "{details:#}");
            }
        }
    }
}

pub fn print_help_json(err: clap::Error) -> Result<(), AppError> {
    print_json(&serde_json::json!({
        "version": "1", "status": "success", "data": {"usage": err.to_string().trim_end()},
    }))
}

pub fn print_clap_error(format: Format, err: &clap::Error) {
    print_error(format, &AppError::InvalidInput(err.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CannotSerialize;
    impl Serialize for CannotSerialize {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("do not expose internal data"))
        }
    }

    #[test]
    fn failed_serialization_writes_nothing_and_returns_framework_error() {
        let mut bytes = Vec::new();
        let error = write_json(&mut bytes, &CannotSerialize).unwrap_err();
        assert!(bytes.is_empty());
        assert_eq!(error.exit_code(), 1);
        assert_eq!(error.error_code(), "serialization_error");
        let error = print_success_or(Ctx::new(true, false), &CannotSerialize, |_| {}).unwrap_err();
        assert_eq!(error.error_code(), "serialization_error");
    }

    #[test]
    fn closed_output_is_an_error_without_a_panic() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(
            write_json(Closed, &serde_json::json!({}))
                .unwrap_err()
                .exit_code(),
            1
        );
    }
}
