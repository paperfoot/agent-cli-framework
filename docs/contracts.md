# Runtime contracts

The binary is the public interface. These contracts survive model and harness
changes. `schemas/` defines JSON shapes; integration tests define behavior.

## Discovery

`agent-info` and its alias `info` emit raw JSON, with no success envelope.

```bash
greeter agent-info
greeter agent-info --command hello
greeter agent-info --command "config show"
greeter agent-info --command config
```

The default manifest contains `name`, binary `version`, `description`,
`commands`, `global_flags`, `exit_codes`, `envelope`, `config`, and
`auto_json_when_piped`. `commands` maps each canonical path to one line
generated from the parser:

```json
{"hello":"<name> --style=: Greet someone","update":"--check --force: Distribution-aware update check/apply"}
```

`<x>` is required, `[x]` optional, `...` repeats, `--flag=` takes a value,
`--flag=<flag>` is a required flag, and a bare `--flag` is a switch. A large
group may take one line naming its subcommands if its key is a valid
`--command` path. Full command objects in the default manifest remain valid but
cost more tokens (32.6 KB against 7.0 KB as lines on a 101-command CLI).

`--command PATH` returns only `name`, `version`, and `commands`, where each
selected command is a full object: `description`, `args`, `options` (with
types, defaults, allowed values, short forms), `aliases`, and annotations. A
canonical leaf path selects that command; a group selects descendants. Empty or
unknown paths fail with exit 3. Command aliases execute normally but filters use
canonical paths. Filtering never executes the selected command. Piped, TTY,
`--json`, and `--quiet` discovery all produce JSON.

The reference derives syntax from Clap and adds these semantic annotations:

| Field | Meaning |
| --- | --- |
| `effect` | `read`, `write`, or `mixed`; describes application state effects |
| `effect_detail` | Explains scope or option-dependent effects when needed |
| `idempotent` | Repeating the same request preserves its intended effects |
| `examples` | Arrays of arguments, excluding the executable |
| `output_fields` | Useful fields in the result payload, where specified |

An idempotent read can return different data as the world changes. An idempotent
write can still fail, and idempotence does not imply permission to execute it.
Semantic metadata is a claim to test; the parser cannot infer it.

When extending the scaffold with different argument types, arity, aliases,
conflicts, or nested groups, extend introspection and its tests together. Do not
silently publish a simplified schema that accepts requests the parser rejects.
Only advertise optional capabilities that the installed binary actually implements.

## Output

All piped stdout is one compact UTF-8 JSON document followed by a newline, unless
an explicitly selected streaming mode defines otherwise. Human output is for
terminals. Every command receives a `Ctx` with format and quiet state.

```json
{"version":"1","status":"success","data":{"id":"item_42"}}
```

`data` is an object or array. Format choice does not change business behavior.
`--quiet` suppresses only informational human output. Do not add banners, update
checks, progress bars, or debug logs to normal JSON output. Consumers can pretty
print with `jq`; JSON key order and whitespace are not API contracts.

Serialize successfully before writing. A serialization failure returns a framework
error, not an error-shaped value on stdout with exit 0. A closed output pipe must
not cause a panic. All JSON write failures map to the normal runtime error path.

## Errors and status

A failed command returns nonzero, no success envelope, and one JSON error on
stderr in machine mode:

```json
{
  "version":"1",
  "status":"error",
  "error":{
    "code":"config_error",
    "message":"doctor found failing checks",
    "suggestion":"Fix the failed checks listed in error.details, then run doctor again",
    "details":{"checks":[],"summary":{"pass":0,"warn":0,"fail":1}}
  }
}
```

`error.details` is an optional object or array for structured diagnostics. For
provider failures it can carry known request IDs or outcome state. Optional
`error.retryable` says whether repeating the same request unchanged can succeed,
and `error.retry_after_secs` how long to wait first. Omit `retryable` when the
outcome is uncertain, such as a write that may have landed.
Never expose secrets or raw untrusted provider instructions in recovery fields.

| Exit | Meaning | Recovery |
| --- | --- | --- |
| 0 | Success | Inspect the result, including per-item status where relevant |
| 1 | Runtime/transient failure | Determine whether retry is useful and safe |
| 2 | Configuration/authentication | Fix setup; do not retry unchanged |
| 3 | Invalid input or conflict | Correct arguments or resolve the conflict |
| 4 | Rate limit | Honor backoff; retain the same logical request identity |

A tool may add domain codes from 5 up when each is documented in the manifest's
`exit_codes` and the contract hook triggers it (xmaster: 5 = blocked by the
provider; do not retry unchanged). Host termination signals are outside the CLI's
ability to promise an envelope. A killed process or timeout does not establish
whether a remote action happened.

For explicit multi-source/batch contracts, `partial_success` and `no_results`
remain supported envelope values. Define per-item errors, continuation, and exit
semantics in that command. The generic success helper emits only `success`.
`all_failed` remains accepted by the schema for legacy consumers; new operations
should use a nonzero error envelope with diagnostic details when nothing succeeds.
Do not claim these optional states are implemented merely because a schema allows them.

`doctor` has a precise single-result contract: no failed checks means a success
report on stdout and exit 0; any failed check means an error with the complete
report in `error.details` on stderr and exit 2. Missing optional config is a warning.

## Help, configuration, and dependencies

Root and command `--help` and root `--version` exit 0 and print plain text, even
when piped: agents read help rather than parse it. With `--json` their text is in
`data.usage` of a success envelope. A parse error's suggestion lists what the
failing command accepts, inline: its flags (``Flags for `greeter hello`: --style.``)
or a group's subcommands, then its usage line, so the retry needs no lookup.
Past 12 entries it names `agent-info --command "<path>"` instead. Global
`--json` works before or after a command. Tokens after
`--` are positional data; do not scan them as flags. Unknown flags, invalid enum
values, and missing required arguments return exit 3.

Discovery, help, version, `config path`, skill status, and pure domain commands
must not require credentials, network access, or parseable configuration.
`config show` loads effective config lazily and masks secrets. `config path`
reports the actual platform location. Doctor is explicit, never a prerequisite
for every operation. Setup failures should identify the smallest useful fix.

## Compatibility

Keep the envelope version separate from the binary version. Preserve existing
commands, aliases, types, and defaults unless a documented migration justifies a
breaking change. Add semantic fields to the manifest without changing its
canonical shape. Consumers should ignore unknown manifest fields.

The envelope schema deliberately rejects undeclared fields. Extend that schema
and its tests before emitting new envelope fields. Use `error.details` or a
command's `data` for domain-specific extensions. Do not introduce a second
competing envelope, rename `--json` to mean input, or replace JSON with a novel
encoding without a measured need and an explicit opt-in contract.
