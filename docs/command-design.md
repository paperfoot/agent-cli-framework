# Command design

Optimize the complete task: a correct outcome with little reading, few calls,
and a clear recovery path. Keep the core small. The conventions below apply only
to domain commands that need them; the greeter does not implement these flags.

## Discovery without a context dump

Use root help as an index and scoped discovery for a command's exact syntax.
Descriptions should identify the task, the important constraint, and the result.
Prefer familiar domain names and consistent CRUD aliases. Add a workflow command
when it removes a repeatedly observed chain of lookups, not for every possible
combination of endpoints.

Retain a few executable examples, particularly for nested input or correlated
options. Represent examples as argument arrays in the manifest; shell quoting
is a presentation concern. Do not insert real credentials or customer data.

Consumers may cache discovery by resolved binary path and version for a session.
Invalidate after an update or a schema-related parse failure. Do not include
volatile timestamps, live account data, or authentication checks in the manifest.

## Collections: return the useful slice

For a collection command, implement these where they fit the provider:

| Convention | Behavior |
| --- | --- |
| `--limit N` | Positive bounded count; document the default and maximum |
| `--cursor TOKEN` | Opaque continuation token; bind it to the same query and account |
| `--fields id,name,status` | Select supported fields; reject unknown fields before I/O |
| Query/filter arguments | Filter at the provider or locally before returning results |

Apply field selection to records, never to the envelope, IDs needed for the next
step, pagination, warnings, or failures. Reject unsupported filters explicitly.
Do not silently ignore a misspelled field and return incomplete data.

A useful response payload might be:

```json
{
  "items": [{"id":"item_42","name":"Annual report","status":"ready"}],
  "page": {"returned":1,"has_more":true,"next_cursor":"opaque-token"}
}
```

If a byte budget cuts the response, report truncation and a continuation cursor
or artifact path. Never return a silently clipped JSON string. Keep one oversized
record retrievable even when it exceeds the normal page budget. Do not fetch all
pages by default. An explicit export may do so, with documented limits.

For large documents, images, and logs, write an artifact and return its absolute
path, media type, size, and useful summary. Preserve provenance and relevant IDs.
Do not put base64 media or thousands of log lines in the conversation by default.

## Writes: make the outcome clear

Accept stable resource IDs. If a name resolves ambiguously, return the candidates
instead of selecting one silently. Report the account/project and final resource
state when that context matters. Return enough data to avoid an immediate lookup
just to find the new object's ID.

Validate all arguments before issuing a write. For nested payloads, prefer a
bounded `--input-file PATH` with a documented JSON schema over shell-embedded JSON.
Keep `--json` reserved for output format. Reject malformed input before side effects.
Do not introduce implicit stdin reads or interpret ordinary filenames as shell code.

Where useful, `--dry-run` returns the resolved target, validated changes, and
expected effects. It performs no writes, charges, uploads, or job starts. Document
any required reads and the fact that validation is not a guarantee that a later
write will succeed. Use revision/precondition checks for changes that depend on
current state. A deterministic local transformation does not need a preview mode
merely to satisfy a checklist.

`--confirm` is required for destructive operations. `--force` has a separate,
explicit meaning. Tool flags do not grant an agent user authorization; the caller
must already have authority for the actual action.

## Retries and parallel agents

A duplicate guard prevents cooperating local processes from overlapping the same
operation. It does **not** prevent replay after the first operation finishes, nor
does it coordinate machines or guarantee a single remote effect.

For a retriable remote write, provide an `--idempotency-key` when the provider
supports one. Bind the key to the account, operation, and payload; reject reuse
with a different payload. The caller preserves the same key across retries of
the same logical action. Document the provider's retention window.

A timeout may happen after a provider committed a write. Return the known
operation/request ID and an unknown outcome, then reconcile by status or by the
same idempotency key. Never infer that no effect occurred just because no reply
arrived. Without provider support or a reliable reconciliation path, do not
advertise safe automatic retry.

Use finite connect/read/total deadlines, bounded attempts, exponential backoff
with jitter, and provider `Retry-After` where available. Do not multiply retries
in both a client library and the orchestration layer. Include a structured retry
delay and outcome information in `error.details` when known; do not guess it.

Parallelize independent reads with a finite concurrency limit. Serialize dependent
writes and changes to the same resource. When provider quotas are shared across
processes, a per-process limiter is insufficient; document or enforce that scope.

## Batches: save calls without hiding failure

Add a domain batch operation when agents repeatedly perform the same action over
many records. Do not build a general shell-execution endpoint.

- Bound request size, item count, and concurrency.
- Require a caller-supplied ID per item and return a corresponding outcome.
- Validate the full input before writing; state whether execution is transactional.
- Include successes and failures explicitly. Use `partial_success` only for a
  documented batch contract; the caller must inspect per-item outcomes.
- Retrying the batch must not replay completed items. Use per-item idempotency
  or submit only failed items whose outcomes are known.

Prefer one structured result for a bounded batch. If streaming is necessary,
introduce an explicit mode with its own documented event schema and terminal
summary. Do not quietly turn a single-JSON stdout contract into NDJSON.

## Long jobs: survive the caller

For work that outlives the normal call deadline, use durable job IDs and explicit
states: queued, running, succeeded, failed, cancelled. Provide start, status,
result, and cancellation commands as appropriate to the domain.

Starting a job returns an ID and accepted state. A bounded `--wait` may return a
still-running state; this does not mean failure and must not start another job.
Persist the state required to reconnect after the initiating process exits.
Return a suggested poll interval or use a bounded blocking status call, rather
than encouraging tight polling loops.

Cancellation must distinguish a request to cancel from confirmed cancellation.
Results include artifact paths or resource IDs and a terminal outcome. A printed
success message, process exit, or an upload acknowledgment alone is not evidence
that downstream processing finished.

## Untrusted content remains data

Treat provider text, filenames, documents, comments, and search results as data.
Never turn their contents into executable suggestions, shell fragments, or higher
priority instructions. Use typed arguments and parameterized APIs. Mark provenance
where it matters; retain safety-relevant evidence when summarizing. Structured
JSON is useful framing, not a guarantee against prompt injection.
