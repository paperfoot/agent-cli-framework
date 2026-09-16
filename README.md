# Agent CLI Framework

**Build tools agents can discover quickly, use correctly, and recover with.**

[![CI](https://github.com/paperfoot/agent-cli-framework/actions/workflows/ci.yml/badge.svg)](https://github.com/paperfoot/agent-cli-framework/actions/workflows/ci.yml)
[![MIT](https://img.shields.io/badge/license-MIT-lightgrey)](LICENSE)

A tested set of language-neutral contracts with a small Rust reference. Rust is
preferred for efficient native tools, not required. Choose the language that
meets the product's measured needs. One executable, ordinary shell commands,
structured results. No service to run alongside it.

**Giving this link to a coding agent?** Tell it:

> Build this CLI using https://github.com/paperfoot/agent-cli-framework.
> Read AGENTS.md first. Rust is preferred, not required; use example/ as the
> behavioral reference. Implement only relevant patterns, measure speed and
> resource use, and run tests and conformance checks before handing it back.

[Build instructions](AGENTS.md) · [Example](example/) · [Command design](docs/command-design.md) · [Research](docs/research-2026-09.md)

## The shortest useful path

An agent should be able to discover a command, execute it, and use its result.
It should not need to read an entire manual before doing useful work.

```bash
greeter --help                         # Small command index
greeter agent-info --command hello     # Just this command's contract
greeter hello Ada --style pirate       # Execute
```

When piped, the last command returns:

```json
{"version":"1","status":"success","data":{"name":"Ada","style":"pirate","message":"Ahoy, Ada! Welcome aboard!"}}
```

The same command in a terminal prints a readable greeting. `--json` makes the
machine format explicit. `agent-info` without a filter still returns the full
manifest, and `agent-info --command "config show"` handles nested commands.
Already know the command and its contract? Use it directly.

## What makes an agent fast

| Friction | Framework response |
| --- | --- |
| Reading every tool definition | Short help; discovery scoped to one command or group |
| Guessing flags and defaults | Generate syntax from Clap; test semantic metadata |
| Reading thousands of irrelevant results | Search, limits, cursors, and field selection where the domain needs them |
| Repeated lookups before an action | Return stable identifiers and useful next-step data |
| Retrying an uncertain write | Distinguish a local lock from provider idempotency and reconciliation |
| Waiting through dozens of calls | Bounded batches and resumable jobs when justified |
| Relearning after an update | Stable envelopes, explicit capabilities, compatible additions |
| Excess CPU and memory | Lazy initialization, bounded work, and resource measurements |
| Optimizing the wrong thing | Measure completed tasks, calls, bytes, and latency together |

These choices apply across model families. Model and harness changes belong in
[evaluations](docs/evaluation.md), not hardcoded model-specific instructions.

## A small core

Every CLI has:

- **`agent-info` / `info`** — raw JSON describing real commands, arguments, defaults,
  examples, and effects; `--command` narrows discovery without changing its shape.
- **Structured output** — compact JSON when piped; readable output in a terminal;
  failures on stderr; `--quiet` suppresses human informational output only.
- **Semantic exit codes** — `0` success, `1` runtime/transient failure, `2` setup,
  `3` input, `4` rate limit. Help and version exit `0`.
- **`config show` / `config path`** — lazy configuration loading and masked secrets.
- **`skill install` / `skill status`** — a short embedded signpost to the binary.

Add `doctor` for external dependencies, `update` for distributed tools, and a
concurrency guard for expensive or irreversible operations. Destructive commands
require `--confirm`. No interactive prompts or implicit stdin reads.

The example demonstrates the core, scoped discovery, structured diagnostic
failures, and a kernel-backed duplicate guard. The
[optional command patterns](docs/command-design.md) describe domain features to
implement and test when needed; they are not extra commands in the greeter.

## Build your own

For a Rust CLI, copy the reference:

```bash
git clone https://github.com/paperfoot/agent-cli-framework.git
cp -R agent-cli-framework/example my-cli
cd my-cli
```

Rename the package, replace the `REPLACE` placeholders, and add your domain
commands. Keep the modules small:

```text
src/
  main.rs          parse, dispatch, exit
  cli.rs           command grammar and help
  config.rs        defaults → file → environment
  error.rs         codes and recovery instructions
  output.rs        JSON and human output
  guard.rs         exclusion for concurrent operations
  commands/        one module per domain
tests/             observable behavior and contracts
```

Then validate the built tool:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
cargo build --release --locked
../agent-cli-framework/conformance/conformance.sh ./target/release/my-cli
```

Replace `my-cli` with your actual binary name. The probe uses Bash and `jq`.
The repository CI also validates emitted JSON against the published schemas.
Other languages use their native build/test tools and the same conformance and
schema checks. Keep the behavior; adapt the implementation.

## Read only what you need

| Working on | Read |
| --- | --- |
| A new CLI | [AGENTS.md](AGENTS.md), then [example/](example/) |
| Discovery, output, errors, compatibility | [Runtime contracts](docs/contracts.md) and [schemas/](schemas/) |
| Search, writes, batches, long jobs | [Command design](docs/command-design.md) |
| Config, secrets, dependencies, concurrency | [Implementation notes](docs/implementation.md) |
| Releases and package managers | [Update standard](docs/update-standard.md) |
| Measuring improvements across agents | [Evaluation](docs/evaluation.md) |
| Language choice, CPU, memory, startup | [Performance](docs/performance.md) |
| Updating an existing framework CLI | [Migration notes](docs/migration-2026-09.md) |
| Why these choices | [Research and source links](docs/research-2026-09.md) |

## Evidence over model folklore

[SWE-agent](https://arxiv.org/abs/2405.15793) studied how interface design affects
agent behavior. More recently, a
[controlled CLI/MCP comparison](https://arxiv.org/abs/2608.08654) found widely
varying costs across scaffoldings on one software task. Neither establishes a
universal winning interface or a speedup for this framework.

The practical lesson is to keep interfaces discoverable, responses relevant, and
results verifiable. This repository chooses a local CLI as its interface; it does
not require claims that every alternative is slower.

Use the included [measurement tool](conformance/measure.py) for output bytes and
process latency. Use paired task evaluations for claims about agent performance.

## Built with this approach

[search-cli](https://github.com/paperfoot/search-cli) ·
[autoresearch](https://github.com/paperfoot/autoresearch-cli) ·
[xmaster](https://github.com/paperfoot/xmaster-cli) ·
[email-cli](https://github.com/paperfoot/email-cli)

[Contributing](CONTRIBUTING.md) · [MIT license](LICENSE)

Built by [Boris Djordjevic](https://github.com/longevityboris) at
[199 Biotechnologies](https://github.com/199-biotechnologies) and
[Paperfoot AI](https://paperfoot.ai).
