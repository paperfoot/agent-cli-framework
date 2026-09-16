# Fast execution, bounded resources

Rust is a preference, not a requirement. The framework specifies observable
behavior; its reference happens to use Rust. Go, Python, TypeScript, Swift, and
other implementations can satisfy the same contracts.

Choose for the workload, deployment environment, existing libraries, and measured
cost. A native executable can be useful for frequent short invocations and easy
distribution. A runtime-based tool may be the practical choice for an existing
codebase or specialist library. Neither the language name nor a compiled bundle
proves acceptable startup, CPU, or memory use. Do not rewrite a working tool
without evidence that the change will pay for itself.

## Set budgets around real tasks

Before tuning, define representative small, typical, and large workloads. Record
input size, machine, OS, production build settings, runtime version, and baseline
revision. Choose project-specific ceilings for:

| Metric | What it catches |
| --- | --- |
| Wall time, including startup | Slow imports, discovery, I/O, or sequential round trips |
| User + system CPU time | Busy loops, repeated parsing, unnecessary computation |
| Peak resident memory | Eager SDK loading, retained buffers, oversized batches |
| Bytes returned to the agent | Irrelevant context and avoidable reading |
| External requests and process launches | Work hidden behind one apparent command |
| Correct final state | Skipped work, truncation, lost results, and duplicate effects |

Measure repeated short invocations as well as one larger task. Distinguish warm
OS caches from cold starts. Report both p50 and tail behavior. Compare the same
inputs and outcomes; preserve the baseline's quality and error handling. A tiny
hello command measures overhead, not your provider or compute workload.

## Keep the common path small

- Parse and dispatch before loading optional clients, SDKs, databases, or runtimes.
  In dynamic languages, put heavyweight imports behind the relevant command.
- Help, version, and discovery must not query a provider or initialize unused
  services. Avoid background update checks, telemetry flushes, or daemon startup
  on those paths.
- Reuse connections and parsed data within one invocation. Use a bounded batch
  instead of spawning a new process for every item when the domain supports it.
- Keep a daemon or persistent worker only when repeated expensive setup warrants
  its idle memory, lifecycle, and isolation costs. It is not a core requirement.
- Optimize the measured bottleneck. Shaving parser microseconds has little effect
  when redundant remote requests dominate the task.

## Bound work and memory

Filter and project near the data source. Return a page instead of downloading a
whole collection to slice locally. Cap batch size and concurrency; more workers
can increase memory, rate limiting, and contention without reducing wall time.
Use backpressure so producers cannot outrun consumers.

The reference serializes a complete response before writing, which gives one
valid success or serialization error. This buffers the payload. Keep normal
JSON results bounded; use an explicit streaming or artifact-file contract for
large outputs instead of building two giant in-memory copies. Document its
completion, failure, and cleanup behavior.

Apply finite deadlines, retry counts, and backoff. Prefer status/result retrieval
or bounded waits to busy polling. Cancel work that no longer has a consumer when
safe; do not cancel a committed write merely because its output pipe closed.
Cache only when hit rate warrants it, with a bounded size, identity, and invalidation
policy. Never trade stale/wrong results for an attractive timing number.

## Measure and profile

[measure.py](../conformance/measure.py) records process latency and output bytes.
On macOS/Linux, add `--resources` to record per-invocation CPU time and peak RSS:

```bash
python3 conformance/measure.py /path/to/tool --command hello --resources
```

Replace `hello` with a real command path. This inspects its schema; it does not
execute the domain operation. Resource sampling uses one fresh helper and one
target process; helper overhead is excluded from target timing. CPU includes
user/system time accounted to that target; peak RSS is not an allocation total,
an idle-memory measurement, or the simultaneous sum of a worker tree. Profile
multi-process workloads with an appropriate process-tree profiler as well.

For a regression, use the language's profiler to attribute CPU and allocations,
then rerun the unchanged workload. Keep heavyweight profiling out of ordinary
execution. CI should enforce deterministic contracts and deliberate resource
budgets on controlled runners; noisy shared-runner timings are useful evidence,
not a universal performance gate.
