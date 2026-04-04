//! greeter -- minimal agent-friendly CLI.
//!
//! Demonstrates every pattern from the agent-cli-framework:
//!   - Modular structure (cli, error, output, commands/)
//!   - JSON envelope on stdout, coloured table on TTY
//!   - Semantic exit codes (0-4)
//!   - `agent-info` for machine-readable capability discovery
//!   - `skill install` to register with AI agent platforms
//!   - `update` for self-update via GitHub Releases

mod cli;
mod commands;
mod error;
mod output;

use clap::Parser;

use cli::{Cli, Commands, SkillAction};
use output::Format;

/// Pre-scan argv for --json before clap parses. This ensures --json is
/// honored even on help, version, and parse-error paths where clap hasn't
/// populated the Cli struct yet.
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();

    // Use try_parse so clap errors go through the JSON envelope instead of
    // printing human-only text that breaks agent pipelines.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Help and --version are NOT errors. Exit 0.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => {
                        output::print_help_json(e);
                        std::process::exit(0);
                    }
                    Format::Human => e.exit(), // clap prints coloured help, exits 0
                }
            }

            // Actual parse errors -- always exit 3, never let clap own the exit.
            let format = Format::detect(json_flag);
            output::print_clap_error(format, &e);
            std::process::exit(3);
        }
    };

    let format = Format::detect(cli.json);

    let result = match cli.command {
        Commands::Hello { name, style } => commands::hello::run(format, name, style),
        Commands::AgentInfo => {
            commands::agent_info::run();
            Ok(())
        }
        Commands::Skill { action } => match action {
            SkillAction::Install => commands::skill::install(format),
            SkillAction::Status => commands::skill::status(format),
        },
        Commands::Update { check } => commands::update::run(format, check),
    };

    if let Err(e) = result {
        output::print_error(format, &e);
        std::process::exit(e.exit_code());
    }
}
