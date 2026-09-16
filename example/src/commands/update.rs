use serde::Serialize;
use std::path::Path;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct UpdateResult {
    current_version: String,
    latest_version: Option<String>,
    status: String,
    install_source: String,
    update_mode: String,
    upgrade_command: Option<String>,
    release_url: Option<String>,
    requires_skill_reinstall: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallSource {
    Auto,
    Standalone,
    Homebrew,
    Cargo,
    CargoBinstall,
    Npm,
    Bun,
    UvTool,
    Pipx,
    Winget,
    Scoop,
    Apt,
    Managed,
    Unknown,
}

impl InstallSource {
    fn parse(raw: &str) -> Result<Self, AppError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "standalone" => Ok(Self::Standalone),
            "homebrew" | "brew" => Ok(Self::Homebrew),
            "cargo" => Ok(Self::Cargo),
            "cargo_binstall" | "cargo-binstall" | "binstall" => Ok(Self::CargoBinstall),
            "npm" => Ok(Self::Npm),
            "bun" => Ok(Self::Bun),
            "uv_tool" | "uv-tool" | "uv" => Ok(Self::UvTool),
            "pipx" => Ok(Self::Pipx),
            "winget" => Ok(Self::Winget),
            "scoop" => Ok(Self::Scoop),
            "apt" => Ok(Self::Apt),
            "managed" => Ok(Self::Managed),
            "unknown" => Ok(Self::Unknown),
            other => Err(AppError::Config(format!(
                "invalid update.install_source '{other}' (expected auto, standalone, homebrew, cargo, cargo_binstall, npm, bun, uv_tool, pipx, winget, scoop, apt, managed, or unknown)"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Standalone => "standalone",
            Self::Homebrew => "homebrew",
            Self::Cargo => "cargo",
            Self::CargoBinstall => "cargo_binstall",
            Self::Npm => "npm",
            Self::Bun => "bun",
            Self::UvTool => "uv_tool",
            Self::Pipx => "pipx",
            Self::Winget => "winget",
            Self::Scoop => "scoop",
            Self::Apt => "apt",
            Self::Managed => "managed",
            Self::Unknown => "unknown",
        }
    }
}

fn detect_install_source(config: &AppConfig) -> Result<InstallSource, AppError> {
    let configured = InstallSource::parse(&config.update.install_source)?;
    if configured != InstallSource::Auto {
        return Ok(configured);
    }

    if let Some(source) = option_env!("ACF_INSTALL_SOURCE") {
        let source = InstallSource::parse(source)?;
        if source != InstallSource::Auto {
            return Ok(source);
        }
    }

    let exe = std::env::current_exe().map_err(AppError::Io)?;
    let path = exe.to_string_lossy();

    if path.contains("/Cellar/") || path.starts_with("/opt/homebrew/bin/") {
        return Ok(InstallSource::Homebrew);
    }

    let cargo_bin = std::env::var_os("CARGO_HOME")
        .map(|p| Path::new(&p).join("bin"))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| Path::new(&home).join(".cargo").join("bin"))
        });
    if let Some(cargo_bin) = cargo_bin {
        if exe.starts_with(cargo_bin) {
            return Ok(InstallSource::Cargo);
        }
    }

    if path.contains("/node_modules/.bin/") {
        return Ok(InstallSource::Npm);
    }

    if path.contains("/.bun/bin/") {
        return Ok(InstallSource::Bun);
    }

    if path.contains("/uv/tools/") || path.contains("/.local/share/uv/tools/") {
        return Ok(InstallSource::UvTool);
    }

    Ok(InstallSource::Unknown)
}

fn upgrade_command(source: InstallSource, config: &AppConfig) -> Option<String> {
    let crate_name = &config.update.crate_name;
    let formula = &config.update.formula;
    match source {
        InstallSource::Homebrew => {
            if config.update.tap.trim().is_empty() {
                Some(format!("brew upgrade {formula}"))
            } else {
                Some(format!("brew upgrade {}/{formula}", config.update.tap))
            }
        }
        InstallSource::Cargo => Some(format!("cargo install --locked --force {crate_name}")),
        InstallSource::CargoBinstall => Some(format!("cargo binstall --no-confirm {crate_name}")),
        InstallSource::Npm => Some(format!("npm update -g {crate_name}")),
        InstallSource::Bun => Some(format!("bun update --global {crate_name}")),
        InstallSource::UvTool => Some(format!("uv tool upgrade {crate_name}")),
        InstallSource::Pipx => Some(format!("pipx upgrade {crate_name}")),
        InstallSource::Winget => Some(format!("winget upgrade --id {crate_name}")),
        InstallSource::Scoop => Some(format!("scoop update {crate_name}")),
        InstallSource::Apt => Some(format!(
            "sudo apt update && sudo apt install --only-upgrade {crate_name}"
        )),
        InstallSource::Auto
        | InstallSource::Standalone
        | InstallSource::Managed
        | InstallSource::Unknown => None,
    }
}

fn validate_package_identifier(value: &str) -> Result<(), AppError> {
    if value.is_empty()
        || value.starts_with('-')
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@/._+-".contains(c))
    {
        return Err(AppError::Config(
            "invalid update package identifier; use a package name without whitespace, shell syntax, or leading '-'".into(),
        ));
    }
    Ok(())
}

fn managed_result(
    current: &str,
    source: InstallSource,
    config: &AppConfig,
    status: &str,
) -> UpdateResult {
    UpdateResult {
        current_version: current.into(),
        latest_version: None,
        status: status.into(),
        install_source: source.as_str().into(),
        update_mode: match source {
            InstallSource::Managed => "disabled",
            InstallSource::Unknown | InstallSource::Standalone => "instructions_only",
            _ => "package_manager",
        }
        .into(),
        upgrade_command: upgrade_command(source, config),
        release_url: Some(format!(
            "https://github.com/{}/{}/releases/latest",
            config.update.owner, config.update.repo
        )),
        requires_skill_reinstall: false,
    }
}

/// This scaffold has no verified release downloader. Return the owning channel
/// honestly rather than querying a placeholder repo or replacing a binary.
/// REPLACE: implement docs/update-standard.md before enabling self-replacement.
pub fn run(ctx: Ctx, _check: bool, _force: bool, config: &AppConfig) -> Result<(), AppError> {
    let source = detect_install_source(config)?;
    if config.update.enabled {
        match source {
            InstallSource::Homebrew => {
                validate_package_identifier(&config.update.formula)?;
                if !config.update.tap.is_empty() {
                    validate_package_identifier(&config.update.tap)?;
                }
            }
            InstallSource::Auto
            | InstallSource::Standalone
            | InstallSource::Managed
            | InstallSource::Unknown => {}
            _ => validate_package_identifier(&config.update.crate_name)?,
        }
    }
    let mut result = managed_result(env!("CARGO_PKG_VERSION"), source, config, "not_checked");
    if !config.update.enabled || source == InstallSource::Managed {
        result.status = "disabled".into();
        result.update_mode = "disabled".into();
        result.upgrade_command = None;
    }
    output::print_success_or(ctx, &result, |r| {
        if r.status == "disabled" {
            println!("Updates are disabled by configuration or the installation owner");
            return;
        }
        println!(
            "Installed via {}; latest version has not been checked",
            r.install_source
        );
        if let Some(command) = &r.upgrade_command {
            println!("Update with: {command}");
        } else if let Some(url) = &r.release_url {
            println!("Release instructions: {url}");
        }
    })
}
