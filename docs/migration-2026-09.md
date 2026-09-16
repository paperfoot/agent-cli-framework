# September 2026 migration

Existing CLIs can adopt these changes incrementally. Keep public behavior stable
while updating the scaffold. This revision makes no automatic changes to tools
already built from the framework.

## Compatible additions

- Add `agent-info --command PATH` as a projection of the existing manifest. Keep
  full discovery and the `info` alias. Exact commands and groups use canonical
  paths; invalid filters exit 3.
- Generate command syntax from the parser; keep tested semantic annotations for
  effects, idempotence, examples, and output fields. Preserve the canonical
  `commands` object and config metadata.
- Compact JSON whitespace. Consumers should parse JSON rather than compare its
  formatting or key order. The success envelope remains version `1`.
- Shorten embedded skills and build instructions. Put optional domain guidance
  in focused references; load it only when relevant.
- Add actual JSON Schema validation and discovery measurements to CI.

## Behavior changes to review

| Previous scaffold | Current scaffold | Consumer action |
| --- | --- | --- |
| Failing doctor could emit success on stdout and error on stderr | One error on stderr, full report in `error.details`, exit 2 | Read the report from the error when doctor fails |
| Read/write PID lock with a one-hour expiry | Kernel lock held by an open descriptor, no time-based expiry | Keep the guard in scope; do not delete its lock file |
| A forced or rejected caller could disturb the owner's lock | Bypass/rejection leaves the owner's lock intact | Treat `--force` as bypass, not takeover |
| Generic runtime errors encouraged blind retry | Recovery distinguishes safe retry from unknown write outcome | Reconcile uncertain writes before trying again |
| Standalone update could replace without the documented verification | Reference returns instructions with `latest_version: null`, `status: "not_checked"` | Implement and test the release policy before enabling updates |
| Unknown or managed source returned prose in `upgrade_command` | Field is null unless a concrete command exists | Use `release_url` or the installation owner's process |
| `_` split every environment field, including `install_source` | `__` separates nesting; `_` remains part of a field name | Use `GREETER_UPDATE__INSTALL_SOURCE`, not `GREETER_UPDATE_INSTALL_SOURCE` |
| Config exposed a `style` field that never affected greetings | Inert field removed; `hello --style` remains unchanged | Keep greeting choices in the command |

The envelope schema explicitly requires object/array `data` and accepts structured
`error.details`. `all_failed` remains allowed for existing consumers; new all-failed
operations should return a nonzero error with diagnostic details. See
[runtime contracts](contracts.md) before changing an established batch API.

The reference lock supports macOS/Linux. A tool shipping on Windows needs an
equivalent tested native implementation. Existing standalone updaters that verify
their artifacts can keep that behavior; the scaffold's instructions-only mode is
not a demand to remove working implementations.

## Adoption sequence

1. Capture your current manifest and run existing behavioral tests.
2. Update discovery and output helpers; preserve domain command names and defaults.
3. Port diagnostic and locking fixes where applicable. Audit callers of the
   now-fallible output helper and propagate failures.
4. Review update ownership and verification against [the standard](update-standard.md).
5. Run conformance, schema validation, and provider/domain tests. Publish
   consumer-visible behavior changes in your release notes.

Search, field selection, dry runs, idempotency, batches, and resumable jobs are
optional command-design patterns, not features automatically installed by this
revision. Add them when real tasks justify the complexity.
