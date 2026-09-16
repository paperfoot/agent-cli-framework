# Build instructions

Build a CLI an agent can discover and use through its executable. **Rust is
preferred, not required.** Choose the language that fits the product and meets
its measured speed, CPU, memory, and distribution needs. Preserve an existing
stack when it meets those needs. Implement only the patterns the product uses.

This file is the build specification. JSON shapes are defined in `schemas/`.
The example is executable reference code. A disagreement is a bug to fix in all
three, not a reason to invent another convention.

## Read in this order

1. This file.
2. The relevant modules in the Rust reference, `example/src/`, and their tests.
   Copy them for Rust; implement equivalent behavior in another language.
3. Only the reference your task needs:
   - [Runtime contracts](docs/contracts.md): discovery, output, errors, compatibility.
   - [Command design](docs/command-design.md): bounded reads, writes, batches, jobs.
   - [Implementation](docs/implementation.md): config, secrets, concurrency, HTTP.
   - [Update standard](docs/update-standard.md): distribution and releases.
   - [Evaluation](docs/evaluation.md): measurements and agent task trials.
   - [Performance](docs/performance.md): language choice and resource budgets.

Do not load every reference up front. Do not add model names, provider-specific
prompt tricks, a server, or a protocol layer to the core.

## Required contracts

1. **Stdout is data.** Piped/redirected output is one compact JSON document.
   A terminal gets readable output. Global `--json` forces JSON; global `--quiet`
   suppresses human informational output, never JSON or errors.
2. **Help and version succeed.** Own the parser's result. Pre-scan `--json` up to
   the `--` delimiter. Wrap help/version in a success envelope when piped; exit 0.
   Parse errors go through the framework and exit 3. In Rust, use
   `Cli::try_parse()`; never let a parser choose an incompatible error exit code.
3. **Failures go to stderr.** Return one error envelope, a nonzero code, and no
   success envelope. Serialize before writing. Diagnostic reports on failure
   belong in `error.details`. No raw logs in JSON mode.
4. **Exit codes are fixed.** `0` success; `1` runtime/transient; `2` config/auth;
   `3` invalid input/conflict; `4` rate limited. Exit 1 is not permission to replay
   a write. Recovery depends on the operation and whether its outcome is known.
5. **Discovery matches execution.** `agent-info` (alias `info`) emits the raw
   canonical manifest. `commands` is an object; every entry has `description`,
   `args`, `options`. Aliases are arrays; global flags live in `global_flags`;
   config metadata lives in `config`. Generate syntax from the command parser.
   Test defaults, enums, aliases, examples, and command coverage in both directions.
6. **Discovery is scoped and local.** `agent-info --command "resource action"`
   returns the same manifest with only that command; a group selects descendants.
   Unknown paths fail with exit 3. Help, discovery, `config path`, and pure commands
   work offline with absent or malformed config. Do not run doctor automatically.
7. **No implicit interaction.** No prompts, pagers, stdin reads, or automatic
   upgrades. Destructive commands require `--confirm`. `--force` only bypasses
   the documented duplicate guard; it never substitutes for confirmation.
8. **Secrets stay secret.** Resolve explicit value → environment → config, first
   non-empty wins. Mask displays. Do not include credentials, raw auth headers,
   or secret-bearing provider/config errors in logs, state, or suggestions.
9. **Recovery is executable.** Every error supplies an exit code, stable error code,
   and recovery suggestion (methods in the Rust reference). Suggestions must be
   correct, actionable, and tested. Never
   suggest changing installation channel or blindly repeating an uncertain write.
10. **Completion is observable.** Return stable IDs, resulting state, and artifact
    paths when relevant. Accepted/queued means accepted/queued, not completed.

Success (stdout):

```json
{"version":"1","status":"success","data":{}}
```

Failure (stderr):

```json
{"version":"1","status":"error","error":{"code":"invalid_input","message":"Name cannot be empty","suggestion":"Provide a non-empty name"}}
```

`agent-info` is the raw-JSON exception. Data is an object or array. Keep envelope
version `1` for compatible additions; document migrations for breaking changes.

## Architecture

Keep parsing, configuration, errors, output, and domain logic separate in any
language. The Rust reference uses this layout; filenames and libraries are not
requirements for other implementations.

```text
src/
  main.rs             parse, detect format, dispatch, exit
  cli.rs              Clap derives and contextual help
  config.rs           AppConfig and lazy figment loading
  error.rs            AppError and recovery instructions
  output.rs           Format, Ctx, fallible output helpers
  guard.rs            required for expensive/irreversible work
  commands/
    mod.rs
    <domain>.rs       one module per domain
    agent_info.rs     discovery derived from command grammar
    skill.rs          embedded skill install/status
    config.rs         show/path
    doctor.rs         required for external dependencies
    update.rs         required for distribution
tests/                integration contracts, outside src/
```

In Rust, `main.rs` stays small. Pass `Ctx { format, quiet }` to commands. Commands return
`Result<(), AppError>` and propagate output errors. Load config only inside
commands that need it. Add only dependencies the product uses. For Rust, copy the
locked example crate rather than guessing versions. Other languages use their
native parser, serializer, configuration, and error-handling equivalents.

- `clap` derives for parsing; `serde` / `serde_json` for data; `thiserror` for errors.
- `figment` merges defaults → TOML → prefixed environment variables.
- `directories` resolves platform paths. `config path` is authoritative; macOS
  uses platform-native paths, Linux uses XDG. Never hardcode Linux paths on macOS.
- Use the release profile in `example/Cargo.toml`. CI builds release artifacts.

## Performance is part of correctness

Set workload-specific budgets for startup, CPU time, peak memory, output bytes,
and external calls. Measure release/production builds against unchanged tasks.
Keep help and discovery local. Load runtimes, SDKs, and clients only on the paths
that need them. Avoid idle daemons, process-per-item work, busy polling, unbounded
concurrency, and loading whole datasets to return a small page.

Bound input, result size, workers, retries, and total time. Reuse connections
within a command. Stream large artifacts to files through an explicit bounded
contract. A faster failure or truncated answer is not an optimization. See
[performance](docs/performance.md) for profiling and language tradeoffs.

## Commands and help

Always provide `agent-info` / `info`, `skill install`, `skill status`,
`config show`, and `config path`. Add `doctor` when external binaries, endpoints,
or credentials are required: structured pass/warn/fail checks, exit 0 with no
failures, exit 2 otherwise. Warnings are allowed.

Alias CRUD verbs consistently: `list` / `ls`, `create` / `new`, `delete` / `rm`,
`show` / `get` (`visible_alias` in Clap). Use parser-enforced closed vocabularies
(`ValueEnum` in Rust). Root `--help` has 3–8 concise Tips and real Examples
(`after_long_help` in Clap).
Describe useful tasks, defaults, effects, and the result; avoid redundant prose.

Keep the installed skill a short signpost: when to use the CLI, one useful
example, and how to inspect one command. Preserve the example's platform targets.
Do not dump the full manifest into every skill or tell agents to rediscover it
before each call.

## Optional patterns: implement only when relevant

- Collections: bounded `--limit`, opaque `--cursor`, explicit truncation and
  continuation; `--fields` for wide records. Filter before returning data.
- Writes: deterministic validation, `--dry-run` where meaningful, stable resource
  IDs, explicit effects. Preview is a capability, not a mandatory extra call on
  every authorized action.
- Retriable writes: provider-backed idempotency keys and reconciliation after
  uncertain outcomes. Local duplicate locks cannot guarantee exactly-once work.
- Batches: bounded input and concurrency, per-item outcomes and IDs; no hidden
  replay of successful items.
- Long jobs: durable IDs, status/result/cancel and bounded waits; never rerun the
  start operation just because the agent's shell call timed out.

[Command design](docs/command-design.md) defines these contracts. The greeter does
not implement these domain-specific flags; do not advertise them until yours do.

## Concurrent work and updates

Use an OS-backed nonblocking lock scoped to the resource/operation. The reference
uses `flock` on macOS/Linux. Only the owner releases its handle; keep the lock file
in place to avoid inode races. The kernel releases locks on process death. A PID
or a one-hour timestamp alone is not a safe ownership test. `--force` bypasses the
guard without touching another owner's lock. See [implementation notes](docs/implementation.md).

`update --check` performs no filesystem or package-manager mutation and exits 0
when the check completes, including when a newer release exists. `update` respects
the owning installation channel. Unknown sources return `instructions_only`.
Standalone replacement requires exact asset selection, HTTPS, checksum verification,
staging, version validation, and atomic replacement. The example deliberately
returns instructions for standalone installs until that verified updater exists.
Follow the [update standard](docs/update-standard.md), including its JSON fields.

## Before handing back

For Rust, run from `example/` (substitute your binary and framework path):

```bash
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo build --release --locked
../conformance/conformance.sh ./target/release/greeter
```

Other languages run their native formatter, static checks, tests, production
build/package step, and the same language-neutral conformance probe. Run the full
schema validator as described in [CONTRIBUTING.md](CONTRIBUTING.md).
Tests use isolated home/config/state directories and controlled provider fixtures.
Never trigger real writes merely to check routing. Verify normal use, bad input,
malformed config, and relevant retries/concurrency. Test actual resulting state.
Report what changed, which checks passed, and any unimplemented domain features.
