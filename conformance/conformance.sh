#!/usr/bin/env bash
# Conformance probe for Agent CLI Framework binaries.
#
# Usage: conformance/conformance.sh <path-to-binary>
#
# Probes a built binary against the behavioral contract in README/AGENTS.md:
# manifest shape, command routability, help/version behavior, stderr
# discipline, and (when the hidden `contract` hook exists) every documented
# exit code. The JSON Schemas in schemas/ are the normative shape definitions;
# this script checks the load-bearing subset with jq so it runs anywhere.
#
# Requires: bash, jq. Exit 0 = conformant, 1 = violations found.

set -u

BIN="${1:?usage: conformance.sh <path-to-binary>}"
FAIL=0

pass() { printf '  \033[32m✓\033[0m %s\n' "$1"; }
fail() { printf '  \033[31m✗\033[0m %s\n' "$1"; FAIL=1; }
note() { printf '  \033[33m·\033[0m %s\n' "$1"; }

# check_json <description> <json> <jq-boolean-filter> [jq options...]
check_json() {
  local desc="$1" json="$2" filter="$3"
  shift 3
  if printf '%s' "$json" | jq -e "$@" "$filter" >/dev/null 2>&1; then
    pass "$desc"
  else
    fail "$desc"
  fi
}

if ! command -v jq >/dev/null 2>&1; then
  echo "conformance.sh requires jq" >&2
  exit 1
fi
if [ ! -x "$BIN" ]; then
  echo "not an executable: $BIN" >&2
  exit 1
fi

echo "== agent-info manifest =="
if INFO="$("$BIN" agent-info 2>/dev/null)"; then
  pass "agent-info exits 0"
else
  fail "agent-info exits 0"
fi
if printf '%s' "$INFO" | jq -e . >/dev/null 2>&1; then
  pass "agent-info emits valid JSON"
else
  fail "agent-info emits valid JSON"
  INFO='{}'
fi
check_json "manifest is raw, not wrapped in the envelope" "$INFO" \
  '(has("data") and has("status")) | not'
check_json "has name, version, description" "$INFO" \
  '(.name|type=="string") and (.version|type=="string") and (.description|type=="string")'
check_json "has non-empty commands object" "$INFO" \
  '(.commands|type=="object") and ((.commands|length) > 0)'
check_json "every command is a signature line or an object with description/args/options" "$INFO" \
  '[.commands[] | (type=="string" and length > 0) or ((type=="object") and has("description") and has("args") and has("options"))] | all'
if printf '%s' "$INFO" | jq -e '[.commands[] | type=="object"] | any' >/dev/null 2>&1; then
  note "default manifest lists full command objects; one signature line each costs fewer tokens (full object via --command)"
fi
check_json "global_flags documents --json and --quiet" "$INFO" \
  '(.global_flags["--json"]|type=="object") and (.global_flags["--quiet"]|type=="object")'
check_json "exit codes 0-4 documented" "$INFO" \
  '[.exit_codes["0","1","2","3","4"]] | all(type=="string")'
check_json "other exit codes are documented integers >= 5" "$INFO" \
  '[.exit_codes | keys[] | test("^(0|[1-9][0-9]*)$")] | all'
check_json "envelope shape documented" "$INFO" \
  '(.envelope.version=="1") and (.envelope|has("success")) and (.envelope|has("error"))'
check_json "config path and env_prefix documented" "$INFO" \
  '(.config.path|type=="string") and (.config.env_prefix|test("^[A-Z][A-Z0-9_]*_$"))'
check_json "auto_json_when_piped declared" "$INFO" \
  '.auto_json_when_piped == true'

echo "== scoped discovery =="
FIRST="$(printf '%s' "$INFO" | jq -r '.commands | keys[0] // empty' 2>/dev/null)"
if [ -n "$FIRST" ] && SCOPED="$("$BIN" agent-info --command "$FIRST" 2>/dev/null)"; then
  check_json "agent-info --command returns name, version and the selected commands" "$SCOPED" \
    '(.name|type=="string") and (.version|type=="string") and ((.commands|length) > 0) and ([.commands | keys[] | . == $p or startswith($p + " ")] | all)' \
    --arg p "$FIRST"
  check_json "scoped commands are full objects with description/args/options" "$SCOPED" \
    '[.commands[] | (type=="object") and has("description") and has("args") and has("options")] | all'
  if printf '%s' "$SCOPED" | jq -e 'keys - ["name","version","commands"] | length > 0' >/dev/null 2>&1; then
    note "scoped output repeats manifest fields; name, version and commands are enough"
  fi
else
  note "no agent-info --command -- scoped discovery not probed"
fi

echo "== help and version =="
"$BIN" --help >/dev/null 2>&1 && pass "--help exits 0" || fail "--help exits 0"
"$BIN" --version >/dev/null 2>&1 && pass "--version exits 0" || fail "--version exits 0"

# Help is read, not parsed: plain text when piped; --json wraps it.
HELP_OUT="$("$BIN" --help 2>/dev/null)"
if printf '%s' "$HELP_OUT" | jq -e . >/dev/null 2>&1; then
  note "piped --help is JSON; plain text reads better and costs fewer tokens (envelope only with --json)"
  HELP_TEXT="$(printf '%s' "$HELP_OUT" | jq -r '.data.usage // empty')"
else
  pass "piped --help is plain text"
  HELP_TEXT="$HELP_OUT"
fi
case "$HELP_TEXT" in
  *Tips:*Examples:*|*Examples:*Tips:*) pass "--help teaches usage (Tips + Examples sections)" ;;
  *) fail "--help teaches usage (Tips + Examples sections)" ;;
esac
check_json "--json --help is wrapped in a success envelope" "$("$BIN" --json --help 2>/dev/null)" \
  '(.status=="success") and (.version=="1") and (.data.usage | type=="string")'

echo "== command routability =="
# Every command key in the manifest (possibly multi-word, e.g. "config show")
# must accept --help with exit 0.
while IFS= read -r cmd; do
  read -r -a cmd_parts <<< "$cmd"
  if "$BIN" "${cmd_parts[@]}" --help >/dev/null 2>&1; then
    pass "routable: $cmd"
  else
    fail "routable: $cmd"
  fi
done < <(printf '%s' "$INFO" | jq -r '.commands | keys[]' 2>/dev/null)

echo "== error discipline =="
BAD_STDOUT="$("$BIN" definitely-not-a-command-xyz 2>/dev/null)"
BAD_STDERR="$("$BIN" definitely-not-a-command-xyz 2>&1 >/dev/null)"
BAD_CODE=$("$BIN" definitely-not-a-command-xyz >/dev/null 2>&1; echo $?)

[ "$BAD_CODE" = "3" ] && pass "unknown command exits 3" || fail "unknown command exits 3 (got $BAD_CODE)"
[ -z "$BAD_STDOUT" ] && pass "unknown command writes nothing to stdout" || fail "unknown command writes nothing to stdout"
check_json "unknown command emits JSON error envelope on stderr" "$BAD_STDERR" \
  '(.status=="error") and (.error.code|type=="string") and (.error.message|type=="string") and (.error.suggestion|type=="string")'

echo "== exit-code contract =="
# The hidden `contract <code>` hook deterministically triggers each exit code.
# Recommended in every framework CLI; probed only when present.
if "$BIN" contract 0 >/dev/null 2>&1; then
  for code in $(printf '%s' "$INFO" | jq -r '.exit_codes | keys[] | select(. != "0")'); do
    GOT=$("$BIN" contract "$code" >/dev/null 2>&1; echo $?)
    [ "$GOT" = "$code" ] && pass "contract $code exits $code" || fail "contract $code exits $code (got $GOT)"
  done
else
  note "no hidden 'contract' hook -- documented exit codes not probed (add the hook; see example/)"
fi

echo
if [ "$FAIL" = "0" ]; then
  echo "CONFORMANT: $BIN"
  exit 0
else
  echo "VIOLATIONS FOUND: $BIN"
  exit 1
fi
