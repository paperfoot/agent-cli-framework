# Implementation notes

Copy working modules from [example/src](../example/src/). Keep contracts in one
implementation instead of maintaining divergent code snippets in documentation.
These are Rust implementation notes. Other languages use equivalent behavior
with native libraries; see [performance](performance.md) for language choice.

## Entry point and output

[main.rs](../example/src/main.rs) owns parsing, routing, and exit codes.
[output.rs](../example/src/output.rs) owns serialization and streams. Commands
return `Result<(), AppError>` and propagate `print_success_or(...)` failures.
Keep the machine output path fallible; do not use `println!` for arbitrary JSON.
Human rendering stays inside the output helper's closure.

[agent_info.rs](../example/src/commands/agent_info.rs) walks the built Clap tree.
When adding a command, add its semantic annotation and contract examples. The
manifest should never query a provider. Avoid timestamps and live environment
values in capability descriptions so repeated discovery is stable.

## Configuration and secrets

[config.rs](../example/src/config.rs) uses figment for defaults → TOML → prefixed
environment. Apply explicit command values where that command supports them.
The Rust reference uses `__` between nested fields, preserving `_` inside names:
`GREETER_UPDATE__INSTALL_SOURCE=cargo`. Hyphens in the package name become
underscores in the uppercase prefix. Greeting style is a command option, not a
configuration setting.
Use `directories::ProjectDirs`; let `config path` report the platform location.
Linux uses XDG paths; macOS uses its platform directories. Config is settings,
state is operational data, and cache is disposable. Never suggest deleting all
three as a routine recovery step.

For secret-bearing tools, resolve explicit value → environment → config, taking
the first non-empty value. Do not persist a secret in state or caches. Prefer
credential handles or provider-supported secure storage when available. Mask
values in effective config and diagnostics; redact whole values when too short
to reveal a safe prefix/suffix. Do not format a parser or HTTP error verbatim if
it can contain a secret. The greeter has no secret-bearing fields.

Do not silently interpret stale config examples as active settings. If a setting
is advertised, test that changing it changes the intended behavior. Separate
configuration metadata from per-command defaults when they serve different uses.

## Dependencies and startup

Keep pure commands synchronous and local. Initialize an HTTP client, async runtime,
or database only for commands that use it. Reuse one client within a process.
Prefer domain-specific deadlines and bounded concurrency over an unbounded task
spawn. Avoid API calls just to render help or discover the config path.

The example uses only its required dependencies. It returns update instructions
rather than bundling an unverified downloader. Add a network stack or updater
when the tool needs one, not simply because the framework once had one.

The release profile enables LTO, one codegen unit, stripping, and optimization.
Measure startup and binary size on supported platforms. Compilation speed, binary
startup, provider latency, and model latency are separate measurements.

## Duplicate guard

[guard.rs](../example/src/guard.rs) demonstrates a nonblocking advisory `flock`
on macOS/Linux. The file lives under the application's state directory, keyed
by an internal operation/resource identity. Never build its filename directly
from an untrusted path, account name, or payload; derive a safe stable key.

```rust
let mut guard = DuplicateGuard::new(&config::data_dir(), "publish-resource-42");
guard.acquire(force)?;
// Perform the guarded work while `guard` remains in scope.
```

The file descriptor owns the lock. Dropping a rejected caller cannot release
another owner. Closing the owning descriptor or process death releases it. Keep
the lock file in place: deleting it allows a second inode to be locked while the
first process is still active. PID/timestamp metadata is informational and may
remain after completion. `--force` performs a documented bypass without writing
or deleting the current owner's file.

Do not expire a live lock based solely on time. Do not use a read-then-write PID
file as a concurrency primitive. The tests cover contention, independent resource
keys, abandoned metadata, and failed/forced callers. For Windows, implement and
test an equivalent native lock before claiming Windows support. Network filesystems
and distributed workers need an explicitly supported coordination mechanism.

A local lock ends when the operation ends. Provider idempotency and reconciliation
are separate requirements for safe retries; see [command design](command-design.md).

## Network work

When adding a provider, define a finite total deadline including retries and
backoff. Reuse connections, limit parallel requests, and respect shared quotas.
Classify failures from typed status codes; expose a concise safe error message.
Retry known-safe operations with bounded exponential backoff and jitter. A write
with an unknown outcome needs reconciliation, not a new request identity.

Test with controlled provider fixtures: a connection failure before sending,
a timeout after commit, a rate limit with a retry delay, and a partial response.
Do not treat a toy greeting benchmark as validation of network behavior.
