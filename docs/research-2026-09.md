# Research behind the September 2026 revision

Reviewed 11 September 2026. These sources inform design choices; they are not
benchmarks of this framework or claims about any particular current model.

## Findings and decisions

| Evidence | What we adopt | Boundary |
| --- | --- | --- |
| [Anthropic: writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) recommends task-oriented tools, relevant responses, and evaluations on held-out tasks. | Search and filter inside the CLI; return identifiers and the information needed for the next action. | A shorter response is useful only if the task still succeeds. |
| [Anthropic: advanced tool use](https://www.anthropic.com/engineering/advanced-tool-use) describes on-demand discovery, programmatic orchestration, and input examples. | Discover one command at a time; keep examples as executable argument arrays. | Its reported improvements concern its own models and test setup. |
| [GitHub: fewer Copilot tools](https://github.blog/ai-and-ml/github-copilot/how-were-making-github-copilot-smarter-with-fewer-tools/) reports improved outcomes after reducing the core toolset and expanding other tools on demand. | A short entry point, focused references, and a small command surface. | Thirteen is not a universal optimal tool count. |
| [Google Workspace CLI](https://github.com/googleworkspace/cli) provides method schema discovery, request previews, JSON output, pagination controls, and workflow helpers. | Scoped discovery and domain-specific optional profiles for reads, writes, batches, and jobs. | We retain our existing flag meanings and envelope; we do not copy its whole API surface. |
| [RTK](https://github.com/rtk-ai/rtk) compresses command output before an agent reads it and distinguishes output savings from total billing savings. | Measure output bytes; provide bounded summaries with a path to complete results. | We do not equate bytes with model tokens or copy the project's advertised savings. |
| [SWE-agent, arXiv:2405.15793](https://arxiv.org/abs/2405.15793) studies the effect of the agent-computer interface on software tasks. | Test task completion and recovery, not just whether a command parses. | Historical benchmark results do not establish performance on today's models. |
| [GUI vs. CLI, arXiv:2606.24551](https://arxiv.org/abs/2606.24551) compares matched tasks and reports that improving CLI skill coverage changes outcomes. | Keep instructions executable and check the resulting state. | This is a desktop benchmark, not a general ranking of interfaces. |
| [The Scaffolding Matters More Than the Interface, arXiv:2608.08654](https://arxiv.org/abs/2608.08654) reports highly variable paired CLI/MCP costs across scaffoldings on one fixed software task. | Remove the universal CLI-versus-MCP cost claim; evaluate the actual harness, task, and model together. | The study covers one task; its ratios should not be generalized. |

## X search

Searched X directly through `xmaster search` for CLI/agent discussions about
schemas, tokens, dry runs, and progressive discovery, using recent and top
searches. Search results included unrelated posts, so they were treated as leads.

- [@Nozelcode, 10 September 2026](https://x.com/Nozelcode/status/2098092983019000279)
  called out structured output, dry runs, meaningful exit codes, actionable errors,
  and checking documented commands. These are practitioner suggestions, not
  measured evidence; the corresponding framework contracts are tested locally.
- [@HNRY234, 10 September 2026](https://x.com/HNRY234/status/2098121937385816104)
  suggested compact discovery and deterministic recovery. We adopt the idea as a
  hypothesis to measure, without treating the post as a performance result.

## What changed as a result

1. `--help` provides the entry point; `agent-info --command "config show"`
   provides detail in the same manifest format. Full discovery remains available.
2. Command syntax comes from Clap, with semantic metadata kept separately and
   covered by contract tests. JSON is compact by default.
3. Concurrent runs use an operating-system lock; a timeout never proves that a
   remote write failed. Locking and idempotency have separate contracts.
4. The build instructions contain the essentials. Optional command patterns and
   implementation details are loaded only when the task needs them.
5. A local measurement tool reports bytes, process latency, CPU time, and peak RSS.
   Rust remains a preferred reference; performance contracts apply to every language.
   Real agent
   improvements require paired task evaluations, described in [evaluation.md](evaluation.md).

## Keeping the framework current

Revisit these decisions when a reproducible failure or paired evaluation warrants
it. Record the exact model identifier, harness version, framework revision,
fixtures, and observed final state. A new model name alone is not evidence that a
contract should change. Keep incompatible changes explicit and migrate consumers.
