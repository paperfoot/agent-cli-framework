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
use std::io::IsTerminal;

use cli::{Cli, Commands, SkillAction};
use output::Format;

fn main() {
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
                if !std::io::stdout().is_terminal() {
                    let envelope = serde_json::json!({
                        "version": "1",
                        "status": "success",
                        "data": { "usage": e.to_string().trim_end() },
                    });
                    println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
                    std::process::exit(0);
                }
                e.exit(); // clap prints coloured help and exits 0
            }

            // Actual parse errors -- wrap in envelope when piped.
            let format = Format::detect(false);
            output::print_clap_error(format, e);
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
