//! Syntax comes from the same Clap tree used to execute commands.
//! Semantic annotations (effects, examples, result fields) remain explicit.
//!
//! The default manifest lists each command as one generated signature line;
//! `--command PATH` returns the selected commands in full.
use std::fmt::Write;

use clap::{Arg, ArgAction, Command, CommandFactory};
use serde_json::{Map, Value, json};

use crate::cli::{Cli, own_args};
use crate::{error::AppError, output};

fn argument(arg: &Arg) -> Value {
    let boolean = matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse);
    let mut value = json!({
        "name": arg.get_long().map(|s| format!("--{s}"))
            .unwrap_or_else(|| arg.get_id().to_string()),
        "type": if boolean { "bool" } else { "string" },
        "required": arg.is_required_set(),
        "description": arg.get_help().map(ToString::to_string).unwrap_or_default(),
    });
    if arg.is_positional() {
        value["kind"] = json!("positional");
    }
    if let Some(short) = arg.get_short() {
        value["short"] = json!(format!("-{short}"));
    }
    if let Some(default) = arg.get_default_values().first() {
        let default = default.to_string_lossy();
        value["default"] = if boolean {
            json!(default == "true")
        } else {
            json!(default)
        };
    }
    // SetTrue/SetFalse are switches, not value-taking boolean options.
    if let Some(values) = arg
        .get_value_parser()
        .possible_values()
        .filter(|_| !boolean)
    {
        value["values"] = values
            .filter(|v| !v.is_hide_set())
            .map(|v| json!(v.get_name()))
            .collect();
    }
    value
}

/// Every public leaf command by canonical path ("config show"), in Clap order.
fn leaves<'a>(root: &'a Command, prefix: &str, result: &mut Vec<(String, &'a Command)>) {
    for command in root.get_subcommands().filter(|c| !c.is_hide_set()) {
        // Clap's generated help command is presentation, not a domain command.
        if command.get_name() == "help" {
            continue;
        }
        let path = format!("{prefix}{}", command.get_name());
        if command.has_subcommands() {
            leaves(command, &format!("{path} "), result);
        } else {
            result.push((path, command));
        }
    }
}

/// The command in full: description, args, options, defaults, values, aliases.
fn full(command: &Command) -> Value {
    let (args, options): (Vec<&Arg>, Vec<&Arg>) =
        own_args(command).partition(|a| a.is_positional());
    let mut value = json!({
        "description": command.get_about().map(ToString::to_string).unwrap_or_default(),
        "args": args.into_iter().map(argument).collect::<Vec<_>>(),
        "options": options.into_iter().map(argument).collect::<Vec<_>>(),
    });
    let aliases: Vec<_> = command.get_visible_aliases().collect();
    if !aliases.is_empty() {
        value["aliases"] = json!(aliases);
    }
    value
}

/// One line for the default manifest: `<name> --style=: Greet someone`.
/// `<x>` required, `[x]` optional, `...` repeats, `--flag=` takes a value,
/// `--flag=<flag>` is a required flag, `--flag` alone is a switch.
pub fn line(command: &Command) -> String {
    let mut line = String::new();
    for arg in own_args(command) {
        let id = arg.get_id().as_str();
        let many = if matches!(arg.get_action(), ArgAction::Append) {
            "..."
        } else {
            ""
        };
        if arg.is_positional() {
            let (open, close) = if arg.is_required_set() {
                ('<', '>')
            } else {
                ('[', ']')
            };
            let _ = write!(line, "{open}{id}{many}{close} ");
        } else if let Some(long) = arg.get_long() {
            let value = match (arg.get_action().takes_values(), arg.is_required_set()) {
                (false, _) => String::new(),
                (true, false) => "=".into(),
                (true, true) => format!("=<{id}>"),
            };
            let _ = write!(line, "--{long}{value}{many} ");
        }
    }
    let about = command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut line = match (line.trim_end(), about.as_str()) {
        ("", about) => about.to_string(),
        (syntax, "") => syntax.to_string(),
        (syntax, about) => format!("{syntax}: {about}"),
    };
    let aliases: Vec<_> = command.get_visible_aliases().collect();
    if !aliases.is_empty() {
        let _ = write!(line, " (alias: {})", aliases.join(", "));
    }
    line
}

// REPLACE: annotate every public leaf command. These are claims about behavior,
// not syntax. Tests verify coverage and execute the harmless examples.
fn annotations() -> Value {
    json!({
        "hello": {
            "effect": "read", "idempotent": true,
            "examples": [["hello", "Ada", "--style", "pirate"]],
            "output_fields": ["name", "style", "message"]
        },
        "agent-info": {
            "effect": "read", "idempotent": true,
            "examples": [["agent-info", "--command", "hello"], ["agent-info", "--command", "config"]]
        },
        "config show": {"effect": "read", "idempotent": true, "examples": [["config", "show"]]},
        "config path": {"effect": "read", "idempotent": true, "examples": [["config", "path"]]},
        "skill status": {"effect": "read", "idempotent": true, "examples": [["skill", "status"]]},
        "skill install": {
            "effect": "write", "idempotent": true,
            "examples": [["skill", "install"]],
            "effect_detail": "Writes the embedded skill to the documented agent platform directories"
        },
        "doctor": {
            "effect": "read", "idempotent": true,
            "examples": [["doctor"]],
            "exit_behavior": "0 with no failed checks; 2 with the report in error.details"
        },
        "update": {
            "effect": "read", "idempotent": true,
            "examples": [["update", "--check"]],
            "effect_detail": "This scaffold returns installation instructions; it does not query releases or replace binaries",
            "install_sources": ["standalone", "homebrew", "cargo", "cargo_binstall", "npm", "bun", "uv_tool", "pipx", "winget", "scoop", "apt", "managed", "unknown"],
            "data_fields": ["current_version", "latest_version", "status", "install_source", "update_mode", "upgrade_command", "release_url", "requires_skill_reinstall"]
        }
    })
}

/// Every public leaf command in full, with its semantic annotations.
fn entries(root: &Command) -> Result<Map<String, Value>, AppError> {
    let mut paths = Vec::new();
    leaves(root, "", &mut paths);
    let mut entries: Map<String, Value> = paths
        .into_iter()
        .map(|(path, command)| (path, full(command)))
        .collect();
    for (path, value) in annotations().as_object().expect("static annotation object") {
        let entry = entries
            .get_mut(path)
            .ok_or_else(|| AppError::Config(format!("metadata names missing command: {path}")))?;
        entry
            .as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
    }
    if entries.values().any(|entry| entry.get("effect").is_none()) {
        return Err(AppError::Config(
            "add effect metadata for every public command".into(),
        ));
    }
    Ok(entries)
}

fn root() -> Command {
    let mut root = Cli::command();
    root.build();
    root
}

/// `agent-info --command PATH`: name, version, and the selected commands in
/// full. A leaf path selects itself; a group selects its descendants.
pub fn scoped(filter: &str) -> Result<Value, AppError> {
    let path = filter.split_whitespace().collect::<Vec<_>>().join(" ");
    if path.is_empty() {
        return Err(AppError::InvalidInput(
            "command path cannot be empty".into(),
        ));
    }
    let mut entries = entries(&root())?;
    let prefix = format!("{path} ");
    entries.retain(|key, _| key == &path || key.starts_with(&prefix));
    if entries.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "unknown command path: {path}; use a key from `{} agent-info`",
            env!("CARGO_PKG_NAME")
        )));
    }
    Ok(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "commands": entries,
    }))
}

/// The default manifest: everything an agent needs to start, with one
/// generated line per command.
pub fn manifest() -> Result<Value, AppError> {
    let root = root();
    entries(&root)?; // annotation coverage holds in both views
    let mut paths = Vec::new();
    leaves(&root, "", &mut paths);
    let lines: Map<String, Value> = paths
        .into_iter()
        .map(|(path, command)| (path, json!(line(command))))
        .collect();
    let globals: Map<String, Value> = root
        .get_arguments()
        .filter(|a| a.is_global_set())
        .map(|a| {
            let mut value = argument(a);
            let name = value
                .as_object_mut()
                .unwrap()
                .remove("name")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            (name, value)
        })
        .collect();
    Ok(json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "commands": lines,
        "global_flags": globals,
        "exit_codes": {
            "0": "Success", "1": "Runtime/transient failure; inspect outcome before retrying writes",
            "2": "Configuration/authentication; fix setup", "3": "Invalid input/conflict; fix request",
            "4": "Rate limited; respect backoff and operation retry semantics"
        },
        "envelope": {"version": "1", "success": "{ version, status, data }", "error": "{ version, status, error: { code, message, suggestion, retryable?, retry_after_secs?, details? } }"},
        "config": {"path": crate::config::config_path().display().to_string(), "env_prefix": format!("{}_", env!("CARGO_PKG_NAME").to_uppercase().replace('-', "_"))},
        "auto_json_when_piped": true
    }))
}

pub fn run(filter: Option<&str>) -> Result<(), AppError> {
    match filter {
        Some(path) => output::print_json(&scoped(path)?),
        None => output::print_json(&manifest()?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn boolean_switches_do_not_advertise_value_syntax() {
        let value = manifest().unwrap();
        for flag in value["global_flags"].as_object().unwrap().values() {
            assert!(flag.get("values").is_none());
        }
        assert_eq!(
            value["commands"]["update"],
            "--check --force: Distribution-aware update check/apply"
        );
        for flag in entries(&root()).unwrap()["update"]["options"]
            .as_array()
            .unwrap()
        {
            assert_eq!(flag["type"], "bool");
            assert!(flag.get("values").is_none());
        }
        assert!(Cli::try_parse_from(["greeter", "update", "--check"]).is_ok());
        assert!(Cli::try_parse_from(["greeter", "update", "--check=true"]).is_err());
    }

    #[test]
    fn every_example_parses_against_the_real_cli() {
        for entry in entries(&root()).unwrap().values() {
            for example in entry["examples"].as_array().unwrap() {
                let argv = std::iter::once("greeter").chain(
                    example
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap()),
                );
                assert!(Cli::try_parse_from(argv).is_ok(), "bad example: {example}");
            }
        }
    }

    #[test]
    fn lines_render_required_optional_repeated_and_switch_syntax() {
        let command = Command::new("send")
            .about("Send a message")
            .visible_alias("s")
            .arg(Arg::new("to").required(true))
            .arg(Arg::new("files").action(ArgAction::Append))
            .arg(Arg::new("channel").long("channel").required(true))
            .arg(Arg::new("tag").long("tag").action(ArgAction::Append))
            .arg(Arg::new("limit").long("limit").short('n'))
            .arg(Arg::new("dry").long("dry-run").action(ArgAction::SetTrue));
        assert_eq!(
            line(&command),
            "<to> [files...] --channel=<channel> --tag=... --limit= --dry-run: Send a message (alias: s)"
        );
    }
}
