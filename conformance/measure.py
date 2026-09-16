#!/usr/bin/env python3
import argparse
import base64
import json
import math
import os
import platform
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Callable

TIMEOUT_SECONDS = 10
Validator = Callable[[str, dict[str, Any]], None] | None
RESOURCE_HELPER = r"""
import base64, json, resource, subprocess, sys, time
started = time.perf_counter_ns()
try:
    result = subprocess.run(sys.argv[1:], stdin=subprocess.DEVNULL, capture_output=True, timeout=10, check=False)
except subprocess.TimeoutExpired:
    print(json.dumps({"error": "target exceeded 10s timeout"})); raise SystemExit(0)
elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
usage = resource.getrusage(resource.RUSAGE_CHILDREN); rss_scale = 1 if sys.platform == "darwin" else 1024
print(json.dumps({"returncode": result.returncode, "stdout": base64.b64encode(result.stdout).decode("ascii"),
    "stderr": base64.b64encode(result.stderr).decode("ascii"), "elapsed_ms": elapsed_ms,
    "cpu_ms": (usage.ru_utime + usage.ru_stime) * 1000, "peak_rss_bytes": usage.ru_maxrss * rss_scale}))
"""


class MeasurementError(Exception):
    pass


def positive_int(value: str) -> int:
    try:
        parsed = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be an integer") from exc
    if parsed <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return parsed


def non_empty(value: str) -> str:
    if not value.strip():
        raise argparse.ArgumentTypeError("must not be empty")
    return value


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Measure a conforming agent CLI safely."
    )
    parser.add_argument("binary", help="path to the CLI executable")
    parser.add_argument(
        "--samples",
        type=positive_int,
        default=10,
        help="timed runs per case (default: 10)",
    )
    parser.add_argument(
        "--command", type=non_empty, help="path for 'agent-info --command PATH'"
    )
    parser.add_argument("--baseline", type=Path, help="prior measure.py JSON report")
    parser.add_argument(
        "--resources", action="store_true", help="measure child CPU and peak RSS"
    )
    return parser.parse_args()


def json_object(case: str, stdout: bytes) -> dict[str, Any]:
    try:
        value = json.loads(stdout)
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise MeasurementError(f"{case}: stdout is not valid JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise MeasurementError(f"{case}: JSON output must be an object")
    return value


def success_envelope(case: str, value: dict[str, Any]) -> None:
    if (
        value.get("version") != "1"
        or value.get("status") != "success"
        or not isinstance(value.get("data"), dict)
    ):
        raise MeasurementError(f"{case}: expected a version 1 success envelope")


def full_manifest(case: str, value: dict[str, Any]) -> None:
    if (
        not isinstance(value.get("name"), str)
        or not value["name"]
        or not isinstance(value.get("commands"), dict)
    ):
        raise MeasurementError(
            f"{case}: raw manifest requires string 'name' and object 'commands'"
        )
    if value.get("status") == "success" and "data" in value:
        raise MeasurementError(f"{case}: agent-info must be raw, not envelope-wrapped")


def percentile(values: list[float], fraction: float) -> float:
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    return ordered[lower] + (
        ordered[min(lower + 1, len(ordered) - 1)] - ordered[lower]
    ) * (position - lower)


def invoke(
    case: str,
    argv: list[str],
    env: dict[str, str],
    cwd: str,
    validate: Validator,
    resources: bool,
) -> tuple[int, int, float, float | None, int | None]:
    started = time.perf_counter_ns()
    try:
        run_argv = [sys.executable, "-c", RESOURCE_HELPER, *argv] if resources else argv
        result = subprocess.run(
            run_argv,
            cwd=cwd,
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            timeout=15 if resources else TIMEOUT_SECONDS,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise MeasurementError(
            f"{case}: exceeded {15 if resources else TIMEOUT_SECONDS}s timeout"
        ) from exc
    except OSError as exc:
        raise MeasurementError(f"{case}: could not execute binary: {exc}") from exc
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    cpu_ms: float | None = None
    peak_rss: int | None = None
    if resources:
        if result.returncode != 0 or result.stderr:
            raise MeasurementError(f"{case}: resource helper failed")
        helper = json_object(f"{case} resource helper", result.stdout)
        if isinstance(helper.get("error"), str):
            raise MeasurementError(f"{case}: {helper['error']}")
        if not all(
            field in helper
            for field in (
                "returncode",
                "stdout",
                "stderr",
                "elapsed_ms",
                "cpu_ms",
                "peak_rss_bytes",
            )
        ):
            raise MeasurementError(f"{case}: resource helper returned invalid data")
        try:
            stdout = base64.b64decode(helper["stdout"], validate=True)
            stderr = base64.b64decode(helper["stderr"], validate=True)
            returncode = int(helper["returncode"])
            elapsed_ms = float(helper["elapsed_ms"])
            cpu_ms = float(helper["cpu_ms"])
            peak_rss = int(helper["peak_rss_bytes"])
        except (TypeError, ValueError) as exc:
            raise MeasurementError(
                f"{case}: resource helper returned invalid data"
            ) from exc
        if elapsed_ms < 0 or cpu_ms < 0 or peak_rss < 0:
            raise MeasurementError(f"{case}: resource helper returned negative metrics")
    else:
        stdout, stderr, returncode = result.stdout, result.stderr, result.returncode
    if returncode != 0:
        raise MeasurementError(f"{case}: unexpected exit code {returncode}")
    if stderr:
        raise MeasurementError(
            f"{case}: expected empty stderr, got {len(stderr)} bytes"
        )
    value = json_object(case, stdout)
    if validate is not None:
        validate(case, value)
    return len(stdout), len(stderr), elapsed_ms, cpu_ms, peak_rss


def measure_case(
    case: str,
    argv: list[str],
    samples: int,
    env: dict[str, str],
    cwd: str,
    validate: Validator,
    resources: bool,
) -> dict[str, Any]:
    invoke(case, argv, env, cwd, validate, resources)  # Untimed warmup.
    stdout_sizes: list[int] = []
    latencies: list[float] = []
    cpu_times: list[float] = []
    peak_rss_values: list[float] = []
    for _ in range(samples):
        stdout_size, _, elapsed_ms, cpu_ms, peak_rss = invoke(
            case, argv, env, cwd, validate, resources
        )
        stdout_sizes.append(stdout_size)
        latencies.append(elapsed_ms)
        if cpu_ms is not None and peak_rss is not None:
            cpu_times.append(cpu_ms)
            peak_rss_values.append(peak_rss)
    measured = {
        "argv": argv,
        "stdout_bytes": max(stdout_sizes),
        "stderr_bytes": 0,
        "p50_ms": round(percentile(latencies, 0.50), 3),
        "p95_ms": round(percentile(latencies, 0.95), 3),
    }
    if resources:
        measured.update(
            {
                "p50_cpu_ms": round(percentile(cpu_times, 0.50), 3),
                "p95_cpu_ms": round(percentile(cpu_times, 0.95), 3),
                "p50_peak_rss_bytes": round(percentile(peak_rss_values, 0.50)),
                "p95_peak_rss_bytes": round(percentile(peak_rss_values, 0.95)),
            }
        )
    return measured


def load_baseline(path: Path) -> tuple[Path, dict[str, Any]]:
    resolved = path.expanduser().resolve()
    try:
        value = json.loads(resolved.read_bytes())
    except OSError as exc:
        raise MeasurementError(f"baseline: cannot read {resolved}: {exc}") from exc
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise MeasurementError(f"baseline: invalid JSON in {resolved}: {exc}") from exc
    if not isinstance(value, dict) or value.get("version") != "1":
        raise MeasurementError("baseline: expected a version 1 report object")
    if not all(
        isinstance(value.get(field), str)
        for field in ("binary", "platform", "python_version", "latency_scope")
    ):
        raise MeasurementError("baseline: report metadata fields must be strings")
    if type(value.get("samples")) is not int or value["samples"] <= 0:
        raise MeasurementError("baseline: 'samples' must be a positive integer")
    cases = value.get("cases")
    if not isinstance(cases, dict):
        raise MeasurementError("baseline: 'cases' must be an object")
    for name, case in cases.items():
        if not isinstance(name, str) or not isinstance(case, dict):
            raise MeasurementError("baseline: every case must be a named object")
        argv = case.get("argv")
        if (
            not isinstance(argv, list)
            or not argv
            or not all(isinstance(arg, str) for arg in argv)
        ):
            raise MeasurementError(f"baseline: case {name!r} has invalid 'argv'")
        for field in ("stdout_bytes", "stderr_bytes"):
            if type(case.get(field)) is not int or case[field] < 0:
                raise MeasurementError(f"baseline: case {name!r} has invalid '{field}'")
        for field in ("p50_ms", "p95_ms"):
            metric = case.get(field)
            if (
                isinstance(metric, bool)
                or not isinstance(metric, (int, float))
                or not math.isfinite(metric)
                or metric < 0
            ):
                raise MeasurementError(f"baseline: case {name!r} has invalid '{field}'")
    return resolved, cases


def add_comparison(report: dict[str, Any], baseline_path: Path) -> None:
    resolved, baseline_cases = load_baseline(baseline_path)
    comparisons: dict[str, Any] = {}
    for name, current in report["cases"].items():
        before_case = baseline_cases.get(name)
        if before_case is None or before_case["argv"][1:] != current["argv"][1:]:
            continue
        before, after = before_case["stdout_bytes"], current["stdout_bytes"]
        delta = None if before == 0 else round((after - before) * 100 / before, 3)
        comparisons[name] = {
            "stdout_bytes": {"before": before, "after": after, "delta_percent": delta}
        }
    report["comparison"] = {"baseline": str(resolved), "cases": comparisons}


def main() -> int:
    args = parse_args()
    if args.resources and sys.platform not in ("darwin", "linux"):
        raise MeasurementError("--resources is supported only on macOS and Linux")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        raise MeasurementError(f"binary is not an executable file: {binary}")
    with tempfile.TemporaryDirectory(prefix="agent-cli-measure-") as temp_dir:
        env = os.environ.copy()
        isolated = (
            "HOME",
            "USERPROFILE",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "TMPDIR",
            "TMP",
            "TEMP",
        )
        for name in isolated:
            env[name] = temp_dir
        executable = str(binary)
        specs: list[tuple[str, list[str], Validator]] = [
            ("help", [executable, "--help"], success_envelope),
            ("version", [executable, "--version"], success_envelope),
            ("agent_info", [executable, "agent-info"], full_manifest),
        ]
        if args.command is not None:
            specs.append(
                (
                    "agent_info_filtered",
                    [executable, "agent-info", "--command", args.command],
                    full_manifest,
                )
            )
        cases = {
            name: measure_case(
                name, argv, args.samples, env, temp_dir, validate, args.resources
            )
            for name, argv, validate in specs
        }
    report: dict[str, Any] = {
        "version": "1",
        "binary": str(binary),
        "platform": platform.platform(),
        "python_version": platform.python_version(),
        "samples": args.samples,
        "latency_scope": (
            "Child wall time including process startup, measured inside a fresh helper after one untimed warmup; "
            "warm OS caches, not a true cold start; helper overhead excluded."
            if args.resources
            else "Process startup with warm OS caches after one untimed warmup; not a true cold start."
        ),
        "cases": cases,
    }
    if args.resources:
        report["resource_scope"] = (
            "Fresh-child user plus system CPU time and peak RSS per invocation; peak RSS uses ru_maxrss "
            "platform units converted to bytes. These metrics do not measure power or cost."
        )
    if args.baseline is not None:
        add_comparison(report, args.baseline)
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except MeasurementError as exc:
        print(f"measure.py: error: {exc}", file=sys.stderr)
        raise SystemExit(1)
