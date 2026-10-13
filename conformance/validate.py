#!/usr/bin/env python3
"""Validate live, read-only CLI responses against the published JSON Schemas.
Requires jsonschema (see requirements.txt). Never execute advertised examples:
they may mutate provider state. --example adds only known greeter fixtures.
"""

import argparse
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from jsonschema import Draft202012Validator


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--example", action="store_true")
    args = parser.parse_args()
    binary = str(args.binary.resolve(strict=True))
    schemas = Path(__file__).resolve().parents[1] / "schemas"
    validators = {}
    for name in ("agent-info", "envelope"):
        schema = json.loads((schemas / f"{name}.schema.json").read_text())
        Draft202012Validator.check_schema(schema)
        validators[name] = Draft202012Validator(schema)
    # Check each discovery response against its own shape, not either one.
    branch = {k: v for k, v in validators["agent-info"].schema.items() if k != "anyOf"}
    for shape in ("manifest", "scoped"):
        validators[shape] = Draft202012Validator({**branch, "$ref": f"#/$defs/{shape}"})
    count = 0
    with tempfile.TemporaryDirectory(prefix="acf-schema-") as home:
        env = dict(
            os.environ,
            HOME=home,
            USERPROFILE=home,
            XDG_CONFIG_HOME=f"{home}/.config",
            XDG_DATA_HOME=f"{home}/.local/share",
            XDG_CACHE_HOME=f"{home}/.cache",
        )
        # Keep the known example independent of caller configuration.
        if args.example:
            env = {k: v for k, v in env.items() if not k.startswith("GREETER_")}

        def run(argv, code=0, shape="envelope"):
            nonlocal count
            result = subprocess.run(
                [binary, *argv],
                capture_output=True,
                env=env,
                cwd=home,
                stdin=subprocess.DEVNULL,
                timeout=10,
            )
            if result.returncode != code:
                raise ValueError(
                    f"{argv}: expected exit {code}, got {result.returncode}"
                )
            payload, other = (
                (result.stdout, result.stderr)
                if code == 0
                else (result.stderr, result.stdout)
            )
            if other:
                raise ValueError(
                    f"{argv}: unexpected output on {'stderr' if code == 0 else 'stdout'}"
                )
            value = json.loads(payload)
            validators[shape].validate(value)
            if shape == "envelope" and value["status"] != (
                "success" if code == 0 else "error"
            ):
                raise ValueError(f"{argv}: status disagrees with exit code")
            count += 1
            return value

        full = run(["agent-info"], shape="manifest")
        run(["--help", "--json"])
        run(["--version", "--json"])
        run(["definitely-not-a-command-xyz", "--json"], code=3)
        info = full["commands"].get("agent-info", "")
        scoped = (
            "--command" in info
            if isinstance(info, str)
            else any(o["name"] == "--command" for o in info.get("options", []))
        )
        for path, command in full["commands"].items():
            parts = path.split()
            run([*parts, "--help", "--json"])
            selected = {path: command} if isinstance(command, dict) else {}
            if scoped:
                filtered = run(["agent-info", "--command", path], shape="scoped")
                if (filtered["name"], filtered["version"]) != (
                    full["name"],
                    full["version"],
                ):
                    raise ValueError(f"{path}: scoped name/version differ from the manifest")
                selected = filtered["commands"]
                if any(k != path and not k.startswith(f"{path} ") for k in selected):
                    raise ValueError(f"{path}: scoped discovery returned other commands")
                if isinstance(command, dict) and selected.get(path) != command:
                    raise ValueError(f"{path}: scoped object differs from the manifest")
            for key, entry in selected.items():
                for alias in entry.get("aliases", []):
                    run([*key.split()[:-1], alias, "--help", "--json"])
        if scoped:
            run(["agent-info", "--command", "definitely-not-a-command-xyz"], code=3)
        # The hidden contract hook is harmless by design; probe every documented code.
        hook = subprocess.run(
            [binary, "contract", "0"], capture_output=True, env=env, cwd=home,
            stdin=subprocess.DEVNULL, timeout=10,
        )
        if hook.returncode == 0:
            for code in sorted(int(c) for c in full["exit_codes"]):
                run(["contract", str(code)], code=code)
        run(["config", "path"])
        run(["skill", "status"])
        if args.example:
            run(["hello", "Ada", "--style", "pirate"])
            run(["hello", "Ada", "--style", "invalid"], code=3)
            run(["doctor"])
            config = Path(run(["config", "path"])["data"]["path"])
            config.parent.mkdir(parents=True, exist_ok=True)
            config.write_text("style = [invalid toml")
            run(["doctor"], code=2)
            run(["agent-info", "--command", "hello"], shape="scoped")
    print(
        json.dumps(
            {
                "status": "success",
                "validated_responses": count,
                "scoped_discovery": scoped,
                "contract_hook": hook.returncode == 0,
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"Schema validation failed: {error}", file=sys.stderr)
        sys.exit(1)
