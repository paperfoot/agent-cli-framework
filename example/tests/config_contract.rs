//! Configuration should change real behavior, including underscore-bearing keys.
mod common;
use common::{greeter_in, write_config_in};

#[test]
fn nested_environment_overrides_file_without_splitting_field_names() {
    let home = tempfile::tempdir().unwrap();
    write_config_in(
        home.path(),
        "[update]\ninstall_source = \"homebrew\"\ncrate_name = \"from-file\"\n",
    );
    let out = greeter_in(home.path())
        .env("GREETER_UPDATE__INSTALL_SOURCE", "cargo")
        .env("GREETER_UPDATE__CRATE_NAME", "from-env")
        .args(["update", "--check"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["data"]["install_source"], "cargo");
    assert_eq!(
        value["data"]["upgrade_command"],
        "cargo install --locked --force from-env"
    );
}

#[test]
fn environment_can_disable_updates() {
    let home = tempfile::tempdir().unwrap();
    let out = greeter_in(home.path())
        .env("GREETER_UPDATE__ENABLED", "false")
        .args(["update"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["data"]["status"], "disabled");
    assert!(value["data"]["upgrade_command"].is_null());
}
