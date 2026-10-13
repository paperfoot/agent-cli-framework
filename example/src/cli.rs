use clap::{Parser, Subcommand, ValueEnum};

// REPLACE: rewrite Tips and Examples for your domain. Agents read --help to
// bootstrap usage; keep 3-8 tips and 3-5 real, copy-pasteable examples.
const HELP_FOOTER: &str = "\
Tips:
  • Run `greeter agent-info --command hello` for one command; omit --command for all
  • Piped results use JSON envelopes; agent-info is raw JSON
  • Run `greeter doctor` to diagnose setup failures
  • Find platform config with `greeter config path`; defaults < file < GREETER_*
  • --quiet suppresses human output; JSON always emits

Examples:
  greeter hello Ada --style pirate
    Greet Ada like a pirate

  greeter hello Ada | jq -r '.data.message'
    Extract just the greeting text as plain text

  greeter update --check
    Safe update check (no mutation), reports the owning install channel";

#[derive(Parser)]
#[command(
    name = "greeter",
    version,
    about = "Minimal agent-friendly CLI",
    after_long_help = HELP_FOOTER
)]
pub struct Cli {
    /// Force JSON output even in a terminal
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress informational output
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Greeting style. Use ValueEnum so clap rejects invalid values with a clear
/// error instead of silently accepting arbitrary strings.
#[derive(Clone, Copy, ValueEnum, serde::Serialize)]
pub enum Style {
    Friendly,
    Formal,
    Pirate,
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Friendly => write!(f, "friendly"),
            Self::Formal => write!(f, "formal"),
            Self::Pirate => write!(f, "pirate"),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    // REPLACE: placeholder domain command. Doc comments are what agents read.
    /// Greet someone
    Hello {
        /// Name to greet
        name: String,
        /// Greeting style
        #[arg(long, value_enum, default_value = "friendly")]
        style: Style,
    },
    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo {
        /// Inspect a canonical command or group, e.g. "hello" or "config show"
        #[arg(long, value_name = "PATH")]
        command: Option<String>,
    },
    /// Manage skill file installation
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Check external dependencies and configuration health
    Doctor,
    /// Distribution-aware update check/apply
    Update {
        /// Check only, don't install
        #[arg(long)]
        check: bool,
        /// Bypass duplicate guard (no effect for instructions-only updates)
        #[arg(long)]
        force: bool,
    },
    /// Hidden: deterministic exit-code trigger for contract tests
    #[command(hide = true)]
    Contract {
        /// Exit code to trigger (0-4)
        code: i32,
    },
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// Write skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Display effective merged configuration
    Show,
    /// Print configuration file path
    Path,
}

/// A command's own arguments: no globals, hidden args, or help/version.
pub fn own_args(command: &clap::Command) -> impl Iterator<Item = &clap::Arg> {
    use clap::ArgAction;
    command.get_arguments().filter(|a| {
        !a.is_hide_set()
            && !a.is_global_set()
            && !matches!(
                a.get_action(),
                ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
            )
    })
}

/// Longest flag or subcommand list a parse-error hint spells out; past it,
/// the hint names the scoped `agent-info` lookup instead.
const INLINE_LIMIT: usize = 12;

/// What the command at `path` accepts, in one sentence: its flags, or the
/// subcommands of a group, so a retry needs no lookup.
pub fn accepts(command: &clap::Command, path: &str) -> String {
    let bin = env!("CARGO_PKG_NAME");
    let shown = format!("{bin} {path}").trim_end().to_string();
    let (kind, names): (&str, Vec<String>) = if command.has_subcommands() {
        let names = command
            .get_subcommands()
            .filter(|c| !c.is_hide_set() && c.get_name() != "help")
            .map(|c| c.get_name().to_string());
        ("Commands", names.collect())
    } else {
        let names = own_args(command).filter_map(|a| {
            let long = a.get_long()?;
            Some(match a.get_short() {
                Some(short) => format!("-{short}/--{long}"),
                None => format!("--{long}"),
            })
        });
        ("Flags", names.collect())
    };
    match names.len() {
        0 if kind == "Flags" => format!("`{shown}` takes no flags."),
        n if n <= INLINE_LIMIT => format!("{kind} for `{shown}`: {}.", names.join(", ")),
        _ if path.is_empty() => format!("List commands with: {bin} agent-info"),
        _ => format!(
            "See all {}: {bin} agent-info --command \"{path}\"",
            kind.to_lowercase()
        ),
    }
}

/// A parse error as `(message, suggestion)` for the error envelope. The
/// message is clap's text without its `error:` prefix, usage block and
/// `--help` footer; the suggestion names what the deepest command in `args`
/// accepts, plus its usage line. Aliases resolve to canonical names.
pub fn explain_parse_error(err: &clap::Error, args: &[String]) -> (String, String) {
    use clap::CommandFactory;
    let mut root = Cli::command();
    root.build();
    let mut command = &root;
    let mut path = Vec::new();
    for arg in args {
        if arg == "--" || !command.has_subcommands() {
            break;
        }
        if let Some(sub) = command.find_subcommand(arg) {
            path.push(sub.get_name());
            command = sub;
        }
    }
    let path = path.join(" ");

    let text = err.to_string();
    let message = if err.kind() == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    {
        // clap's text here is the whole help page; say what is missing.
        format!("`{} {path}` needs a subcommand", env!("CARGO_PKG_NAME"))
    } else {
        let mut message = String::new();
        for line in text
            .lines()
            .map(str::trim)
            .take_while(|l| !l.starts_with("Usage:"))
            .filter(|l| !l.is_empty() && !l.starts_with("For more information"))
        {
            if !message.is_empty() {
                message.push_str(if message.ends_with(':') { " " } else { "; " });
            }
            message.push_str(line.strip_prefix("error: ").unwrap_or(line));
        }
        message
    };

    let mut suggestion = accepts(command, &path);
    let usage = text.lines().find_map(|l| l.trim().strip_prefix("Usage: "));
    if let Some(usage) = usage.filter(|_| !command.has_subcommands()) {
        suggestion.push_str(&format!(" Usage: {usage}"));
    }
    (message, suggestion)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, ArgAction, Command};

    #[test]
    fn long_flag_lists_point_at_scoped_discovery() {
        let mut wide = Command::new("wide");
        for i in 0..=INLINE_LIMIT {
            let name: &'static str = format!("f{i}").leak();
            wide = wide.arg(Arg::new(name).long(name).action(ArgAction::SetTrue));
        }
        assert_eq!(
            accepts(&wide, "wide"),
            "See all flags: greeter agent-info --command \"wide\""
        );
        let narrow = Command::new("narrow").arg(Arg::new("n").long("limit").short('n'));
        assert_eq!(
            accepts(&narrow, "narrow"),
            "Flags for `greeter narrow`: -n/--limit."
        );
        assert_eq!(
            accepts(&Command::new("bare"), "bare"),
            "`greeter bare` takes no flags."
        );
    }
}
