# Evaluate the complete task

The goal is a correct result with less time and effort. A smaller response is a
useful mechanism, not proof that an agent finishes faster.

## Local measurements

From the framework root, after building the example:

```bash
python3 conformance/measure.py example/target/release/greeter --command hello
```

The standard-library script measures root help, version, full discovery, and an
optional scoped command. It launches argument arrays without a shell, closes
stdin, isolates home/config/state/cache paths, and enforces a ten-second timeout.
It does not execute domain commands or advertised examples. Use a binary you
trust; directory isolation is not an operating-system sandbox.

Each case has one warmup and ten measured process launches by default. The JSON
report contains maximum stdout bytes and p50/p95 wall time. This includes process
startup with warm OS caches, not a cold boot or an in-process function benchmark.
No token estimates or cost projections are produced.

Add `--resources` on macOS/Linux to measure CPU time and peak resident memory
through a fresh child process. See [performance](performance.md) for the metric
definitions and limits. The same probe works across implementation languages.

```bash
python3 conformance/measure.py /path/to/old-cli > before.json
python3 conformance/measure.py /path/to/new-cli --command hello --baseline before.json > after.json
```

The comparison covers output bytes for matching argument lists, excluding the
executable path. New cases remain visible without an invented baseline. Record
revisions, build profiles, machine, and environment; config path lengths can
change manifest bytes. For latency claims, interleave paired runs under comparable
load with more samples. CI uploads reports as artifacts; it does not enforce a
machine-dependent latency threshold.

## Agent task evaluations

Compare the same task suite against the old and new CLI with everything else
held constant. Evaluate each model and harness separately; aggregate only after
showing individual results. Pin exact model identifiers and tool settings.

Use fixtures with observable final state:

| Task | Success check |
| --- | --- |
| Discover and execute an unfamiliar command | Correct arguments and expected result |
| Find a record in a large collection | Correct stable ID; no hidden truncation |
| Recover from invalid input | Corrected call succeeds without unrelated actions |
| Diagnose missing credentials | Identifies required setup without retrying unchanged |
| Repeat a write after timeout | One intended effect, reconciled through the same request identity |
| Resume a partially failed batch | Only failed/unprocessed items are retried |
| Two workers target the same resource | Defined conflict or serialization; no duplicate effect |

Collection, provider-write, batch, and job cases belong in CLIs implementing
those features. The greeter cannot validate their behavior. Keep a held-out task
set so changes are not tuned solely to the examples in the skill.

For every run, retain task ID, model ID, harness version, framework revision,
fixture revision, seed when supported, tool-call transcript, and final-state
check. Record task success, unsafe/duplicate effects, elapsed time, command count,
failed calls, retries, input/output tokens reported by the harness, and actual
provider cost when available. Handle cached tokens consistently.

Randomize old/new order and repeat enough times to expose variance. Compare cost
and latency on both all tasks and paired successful tasks; cheap failures must
not look like improvements. Report uncertainty and regressions, not just the
best run. Accept an optimization only when task quality is maintained.

## What this revision establishes

A [local snapshot from 11 September 2026](measurements/2026-09-11.json) records
ten samples per case on an Apple M4 Max. It includes binary hashes, build profiles,
raw reports, and scope notes, with local paths removed.

| Discovery response | Stdout bytes |
| --- | ---: |
| Previous full manifest | 3,452 |
| Revised full manifest, including semantic annotations | 3,558 |
| Revised `agent-info --command hello` | 1,394 |

The scoped response is 59.6% smaller than the previous full manifest and 60.8%
smaller than the current full manifest. This compares bytes needed to inspect
one command, not equivalent full-manifest content. The richer full manifest is
slightly larger. It remains available when the task needs it.

For full discovery, observed median CPU time was 11.35 → 10.64 ms and median peak
RSS was 7.35 → 6.99 MB (decimal). These are small local observations, not established
gains: runs were sequential on a shared workstation, build profiles differ, and
wall-time variance was large. Do not use this snapshot to claim an agent speedup.
The release executable decreased from 7.29 MB to 1.30 MB with the dependency and
release-profile changes. Binary size is separate from resident memory.

The repository checks parser/discovery agreement, output and exit semantics,
diagnostic failures, locking, and local discovery measurements. It does not
claim a measured end-to-end speedup on a current commercial model. The
[research notes](research-2026-09.md) explain the hypotheses to test.
