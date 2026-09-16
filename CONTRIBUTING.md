# Contributing

Bring a reproducible agent failure, a useful simplification, or a measured
improvement. Keep changes focused and preserve public contracts unless the
benefit warrants a documented migration.

## Where changes belong

| File | Purpose |
| --- | --- |
| `README.md` | Short introduction and navigation |
| `AGENTS.md` | Build instructions agents can act on |
| `docs/` | Design, implementation, release, and evaluation guidance |
| `schemas/` | Published JSON shapes |
| `example/src/` | Runnable reference implementation |
| `example/tests/` | Observable contract tests |
| `conformance/` | Portable probe, full schema validation, and measurements |

Update relevant instructions, implementation, and tests together. Keep code in
the example rather than duplicating helper implementations in documentation.
The greeter is a small scaffold; domain-specific features belong in real tools.

## Validate a change

From the repository root:

```bash
cargo fmt --manifest-path example/Cargo.toml --check
cargo clippy --manifest-path example/Cargo.toml --all-targets --all-features --locked -- -D warnings
cargo test --manifest-path example/Cargo.toml --locked
cargo build --manifest-path example/Cargo.toml --release --locked
conformance/conformance.sh example/target/release/greeter
python3 -m venv /tmp/acf-validation
/tmp/acf-validation/bin/pip install -r conformance/requirements.txt
/tmp/acf-validation/bin/python conformance/validate.py example/target/release/greeter --example
python3 conformance/measure.py example/target/release/greeter --command hello
```

The Bash probe needs `jq`. Python schema validation needs `jsonschema`; the
measurement script uses only the standard library. CI runs the same contract
checks on macOS and Linux. A domain CLI should run schema validation without
`--example`, then add its own controlled fixtures.

Test outcomes, stream discipline, defaults, aliases, recovery, and relevant state
changes. Never execute arbitrary manifest examples against a live account as a
generic conformance check. For timing changes, report the measured setup and
paired results; see [evaluation](docs/evaluation.md).

Open a pull request describing the problem, resulting behavior, validation, and
compatibility implications. Cite sources for technical claims. Write short,
plain sentences and give examples that actually work.

Contributions are licensed under the [MIT License](LICENSE).
