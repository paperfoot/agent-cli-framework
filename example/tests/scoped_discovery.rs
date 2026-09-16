//! Scoped discovery is a projection of the canonical, executable manifest.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{greeter_in, write_config_in};
use serde_json::Value;

fn run_json(home: &Path, args: &[&str]) -> Value {
    let out = greeter_in(home).args(args).output().unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "{args:?} wrote stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("command should emit one JSON document")
}

fn commands(value: &Value) -> BTreeSet<&str> {
    value["commands"]
        .as_object()
        .expect("commands should be an object")
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn full_manifest_is_raw_and_uses_canonical_leaf_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let info = run_json(tmp.path(), &["agent-info"]);

    assert!(info.is_object());
    assert!(info.get("status").is_none(), "agent-info is raw JSON");
    assert_eq!(
        commands(&info),
        BTreeSet::from([
            "agent-info",
            "config path",
            "config show",
            "doctor",
            "hello",
            "skill install",
            "skill status",
            "update",
        ])
    );
    for (path, command) in info["commands"].as_object().unwrap() {
        assert!(command["description"].is_string(), "{path}: description");
        assert!(command["args"].is_array(), "{path}: args");
        assert!(command["options"].is_array(), "{path}: options");
    }
}

#[test]
fn hello_scope_is_an_exact_projection_of_the_full_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    let full = run_json(tmp.path(), &["agent-info"]);
    let scoped = run_json(tmp.path(), &["agent-info", "--command", "hello"]);

    assert_eq!(commands(&scoped), BTreeSet::from(["hello"]));
    assert_eq!(scoped["commands"]["hello"], full["commands"]["hello"]);

    let mut full_metadata = full.clone();
    let mut scoped_metadata = scoped.clone();
    full_metadata.as_object_mut().unwrap().remove("commands");
    scoped_metadata.as_object_mut().unwrap().remove("commands");
    assert_eq!(scoped_metadata, full_metadata);
}

#[test]
fn group_and_nested_scopes_select_only_the_requested_commands() {
    let tmp = tempfile::tempdir().unwrap();
    let full = run_json(tmp.path(), &["agent-info"]);
    let config = run_json(tmp.path(), &["agent-info", "--command", "config"]);
    let show = run_json(tmp.path(), &["agent-info", "--command", "config show"]);

    assert_eq!(
        commands(&config),
        BTreeSet::from(["config path", "config show"])
    );
    assert_eq!(commands(&show), BTreeSet::from(["config show"]));
    assert_eq!(
        show["commands"]["config show"],
        full["commands"]["config show"]
    );
}

#[test]
fn info_alias_returns_the_same_manifest() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(
        run_json(tmp.path(), &["info"]),
        run_json(tmp.path(), &["agent-info"])
    );
}

#[test]
fn invalid_scopes_are_json_input_errors_with_no_stdout() {
    for path in ["", "unknown", "con", "configx", "contract"] {
        let tmp = tempfile::tempdir().unwrap();
        let out = greeter_in(tmp.path())
            .args(["agent-info", "--command", path])
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(3), "scope {path:?}");
        assert!(out.stdout.is_empty(), "scope {path:?} wrote stdout");
        let error: Value = serde_json::from_slice(&out.stderr)
            .unwrap_or_else(|_| panic!("scope {path:?} did not emit JSON stderr"));
        assert_eq!(error["status"], "error", "scope {path:?}");
        assert_eq!(error["error"]["code"], "invalid_input", "scope {path:?}");
    }
}

#[test]
fn malformed_config_does_not_block_scoped_discovery_or_help() {
    let tmp = tempfile::tempdir().unwrap();
    write_config_in(tmp.path(), "{{invalid toml");

    let scoped = run_json(tmp.path(), &["agent-info", "--command", "hello"]);
    assert_eq!(commands(&scoped), BTreeSet::from(["hello"]));

    let help = run_json(tmp.path(), &["--json", "--help"]);
    assert_eq!(help["status"], "success");
    assert!(help["data"]["usage"].as_str().unwrap().contains("Usage:"));
}

#[test]
fn global_output_flags_do_not_suppress_discovery() {
    let tmp = tempfile::tempdir().unwrap();
    let expected = run_json(tmp.path(), &["agent-info", "--command", "hello"]);
    let flagged = run_json(
        tmp.path(),
        &["--json", "--quiet", "agent-info", "--command", "hello"],
    );
    assert_eq!(flagged, expected);
}

#[test]
fn every_advertised_example_executes_in_an_isolated_home() {
    let manifest_home = tempfile::tempdir().unwrap();
    let manifest = run_json(manifest_home.path(), &["agent-info"]);

    for (path, command) in manifest["commands"].as_object().unwrap() {
        let examples = command["examples"]
            .as_array()
            .unwrap_or_else(|| panic!("{path} should advertise examples"));
        assert!(!examples.is_empty(), "{path} should advertise an example");

        for example in examples {
            let args: Vec<&str> = example
                .as_array()
                .unwrap()
                .iter()
                .map(|arg| arg.as_str().unwrap())
                .collect();
            let tmp = tempfile::tempdir().unwrap();
            let result = run_json(tmp.path(), &args);

            if args.first() == Some(&"hello") {
                assert_eq!(result["status"], "success");
                assert_eq!(result["data"]["name"], "Ada");
                assert_eq!(result["data"]["style"], "pirate");
                assert_eq!(result["data"]["message"], "Ahoy, Ada! Welcome aboard!");
            }
        }
    }
}
