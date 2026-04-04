use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "greeter", version, about = "Minimal agent-friendly CLI")]
pub struct Cli {
    /// Force JSON output even in a terminal
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Greet someone (the actual domain command)
    Hello {
        /// Name to greet
        name: String,
        /// Greeting style: friendly, formal, pirate
        #[arg(long, default_value = "friendly")]
        style: String,
    },
    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo,
    /// Manage skill file installation
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Self-update from GitHub Releases
    Update {
        /// Check only, don't install
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// Write skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}
