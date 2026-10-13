//! Verify JSON envelope structure on stdout/stderr.
//!
//! assert_cmd runs the binary in a pipe (not a TTY), so JSON auto-detection
//! kicks in — no need for `--json`.

use assert_cmd::Command;

fn greeter() -> Command {
    Command::cargo_bin("greeter").unwrap()
}

// ── Success envelope ───────────────────────────────────────────────────────

#[test]
fn success_envelope_shape() {
    let out = greeter().args(["hello", "World"]).output().unwrap();

    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout should be valid JSON");

    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "success");
    assert!(
        json["data"].is_object(),
        "envelope must have a 'data' field"
    );
    assert_eq!(json["data"]["name"], "World");
}

#[test]
fn success_envelope_with_json_flag() {
    let out = greeter()
        .args(["hello", "World", "--json"])
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--json stdout should be valid JSON");

    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["style"], "friendly");
}

// ── Error envelope ─────────────────────────────────────────────────────────

#[test]
fn error_envelope_shape() {
    let out = greeter().args(["contract", "3"]).output().unwrap();

    assert!(!out.status.success());

    // Error envelopes go to stderr.
    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("stderr should be valid JSON");

    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "error");
    assert!(
        json["error"].is_object(),
        "error envelope must have 'error' field"
    );
    assert!(json["error"]["code"].is_string(), "error must have 'code'");
    assert!(
        json["error"]["message"].is_string(),
        "error must have 'message'"
    );
    assert!(
        json["error"]["suggestion"].is_string(),
        "error must have 'suggestion'"
    );
}

#[test]
fn error_code_matches_variant() {
    // Exit 2 = config_error
    let out = greeter().args(["contract", "2"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "config_error");

    // Exit 4 = rate_limited
    let out = greeter().args(["contract", "4"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "rate_limited");
}

// ── Help/version wrapping ──────────────────────────────────────────────────

#[test]
fn piped_help_and_version_are_plain_text() {
    // Agents read help; escaping it into one JSON string costs tokens and
    // loses the column layout. Plain text even when piped.
    for flag in ["--help", "--version"] {
        let out = greeter().arg(flag).output().unwrap();
        assert!(out.status.success());
        assert!(
            serde_json::from_slice::<serde_json::Value>(&out.stdout).is_err(),
            "piped {flag} should be plain text"
        );
    }
}

#[test]
fn help_and_version_wrapped_with_json_flag() {
    for flag in ["--help", "--version"] {
        let out = greeter().args(["--json", flag]).output().unwrap();
        assert!(out.status.success());
        let json: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|_| panic!("--json {flag} should be JSON"));
        assert_eq!(json["status"], "success");
    }
}

#[test]
fn long_help_includes_tips_and_examples() {
    // --help carries a Tips and an Examples section so agents can bootstrap
    // usage without external docs.
    let out = greeter().arg("--help").output().unwrap();
    assert!(out.status.success());
    let usage = String::from_utf8(out.stdout).unwrap();
    assert!(
        usage.contains("Tips:"),
        "--help must include a Tips section"
    );
    assert!(
        usage.contains("Examples:"),
        "--help must include an Examples section"
    );
}

// ── Quiet flag ─────────────────────────────────────────────────────────────

#[test]
fn quiet_still_emits_json() {
    // --quiet should not suppress JSON output (agents need the envelope).
    let out = greeter()
        .args(["hello", "World", "--json", "--quiet"])
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--quiet should not suppress JSON");

    assert_eq!(json["status"], "success");
}

// ── Parse error envelope ───────────────────────────────────────────────────

#[test]
fn parse_error_wrapped_in_envelope() {
    let out = greeter()
        .arg("hello") // missing required <name>
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(3));

    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("parse error should be JSON on stderr");

    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "invalid_input");
    // The flags and usage are inline: the retry needs no lookup.
    assert_eq!(
        json["error"]["suggestion"],
        "Flags for `greeter hello`: --style. Usage: greeter hello <NAME>"
    );
    assert_eq!(json["error"]["retryable"], false);
}

#[test]
fn rate_limit_error_says_it_is_retryable() {
    let out = greeter().args(["contract", "4"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["retryable"], true);
    // An uncertain outcome omits the field rather than guessing.
    let out = greeter().args(["contract", "1"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(json["error"].get("retryable").is_none());
}
