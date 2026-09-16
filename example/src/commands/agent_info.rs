//! Syntax comes from the same Clap tree used to execute commands.
//! Semantic annotations (effects, examples, result fields) remain explicit.
use clap::{Arg, ArgAction, Command, CommandFactory};
use serde_json::{Map, Value, json};

use crate::{cli::Cli, error::AppError, output};

fn argument(arg: &Arg) -> Value {
    let boolean = matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse);
    let mut value = json!({
        "name": arg.get_long().map(|s| format!("--{s}"))
            .unwrap_or_else(|| arg.get_id().to_string()),
        "type": if boolean { "bool" } else { "string" },
        "required": arg.is_required_set(),
        "description": arg.get_help().map(ToString::to_string).unwrap_or_default(),
    });
    if arg.get_index().is_some() {
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

fn commands(root: &Command, prefix: &str, result: &mut Map<String, Value>) {
    for command in root.get_subcommands().filter(|c| !c.is_hide_set()) {
        // Clap's generated help command is presentation, not a domain command.
        if command.get_name() == "help" {
            continue;
        }
        let path = format!("{prefix}{}", command.get_name());
        if command.get_subcommands().next().is_some() {
            commands(command, &format!("{path} "), result);
            continue;
        }
        let mut args = Vec::new();
        let mut options = Vec::new();
        for arg in command
            .get_arguments()
            .filter(|a| !a.is_hide_set() && !a.is_global_set())
        {
            if matches!(
                arg.get_action(),
                ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
            ) {
                continue;
            }
            if arg.get_index().is_some() {
                args.push(argument(arg));
            } else {
                options.push(argument(arg));
            }
        }
        let mut value = json!({
            "description": command.get_about().map(ToString::to_string).unwrap_or_default(),
            "args": args,
            "options": options,
        });
        let aliases: Vec<_> = command.get_visible_aliases().collect();
        if !aliases.is_empty() {
            value["aliases"] = json!(aliases);
        }
        result.insert(path, value);
    }
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

pub fn manifest(filter: Option<&str>) -> Result<Value, AppError> {
    let mut root = Cli::command();
    root.build();
    let mut entries = Map::new();
    commands(&root, "", &mut entries);
    let metadata = annotations();
    for (path, value) in metadata.as_object().expect("static annotation object") {
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
    if let Some(filter) = filter {
        let path = filter.split_whitespace().collect::<Vec<_>>().join(" ");
        if path.is_empty() {
            return Err(AppError::InvalidInput(
                "command path cannot be empty".into(),
            ));
        }
        let prefix = format!("{path} ");
        entries.retain(|key, _| key == &path || key.starts_with(&prefix));
        if entries.is_empty() {
            return Err(AppError::InvalidInput(format!(
                "unknown command path: {path}; use a canonical path from --help"
            )));
        }
    }
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
        "commands": entries,
        "global_flags": globals,
        "exit_codes": {
            "0": "Success", "1": "Runtime/transient failure; inspect outcome before retrying writes",
            "2": "Configuration/authentication; fix setup", "3": "Invalid input/conflict; fix request",
            "4": "Rate limited; respect backoff and operation retry semantics"
        },
        "envelope": {"version": "1", "success": "{ version, status, data }", "error": "{ version, status, error: { code, message, suggestion, details? } }"},
        "config": {"path": crate::config::config_path().display().to_string(), "env_prefix": format!("{}_", env!("CARGO_PKG_NAME").to_uppercase().replace('-', "_"))},
        "auto_json_when_piped": true
    }))
}

pub fn run(filter: Option<&str>) -> Result<(), AppError> {
    output::print_json(&manifest(filter)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn boolean_switches_do_not_advertise_value_syntax() {
        let value = manifest(None).unwrap();
        for flag in value["global_flags"].as_object().unwrap().values() {
            assert!(flag.get("values").is_none());
        }
        for flag in value["commands"]["update"]["options"].as_array().unwrap() {
            assert_eq!(flag["type"], "bool");
            assert!(flag.get("values").is_none());
        }
        assert!(Cli::try_parse_from(["greeter", "update", "--check"]).is_ok());
        assert!(Cli::try_parse_from(["greeter", "update", "--check=true"]).is_err());
    }

    #[test]
    fn every_example_parses_against_the_real_cli() {
        for entry in manifest(None).unwrap()["commands"]
            .as_object()
            .unwrap()
            .values()
        {
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
}
