#!/usr/bin/env python3
"""Measure actual locked Rust compilation; never substitute qualification tests."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def cargo_command(cargo: str, package: str | None) -> list[str]:
    command = [cargo, "build", "--locked", "--offline", "--quiet"]
    return command + (["-p", package, "--lib"] if package else ["--workspace", "--all-targets"])


def local_environment(parent: dict[str, str], target: Path, cache: Path, endpoint: Path) -> dict[str, str]:
    # A caller's GHA/S3/Redis backend or compiler wrapper must not contaminate A/B.
    env = {k: v for k, v in parent.items() if not k.startswith("SCCACHE_")}
    # Empty environment values also override a wrapper in Cargo's config file.
    env["RUSTC_WRAPPER"] = ""
    env["RUSTC_WORKSPACE_WRAPPER"] = ""
    env.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="0",
               SCCACHE_DIR=str(cache), SCCACHE_SERVER_UDS=str(endpoint),
               SCCACHE_CACHE_SIZE="2G", SCCACHE_IDLE_TIMEOUT="0",
               SCCACHE_CONF=str(cache.parent / "sccache.toml"))
    return env


def run(command: list[str], env: dict[str, str], log: Path, timeout: float) -> float:
    started = time.monotonic()
    with log.open("w") as stream:
        process = subprocess.Popen(command, cwd=ROOT, env=env, stdout=stream,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            returncode = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            # Cargo can leave rustc/linker children behind; terminate our whole group.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait()
            raise
    elapsed = time.monotonic() - started
    if returncode:
        raise RuntimeError(f"command failed ({returncode}); see {log}")
    return elapsed


def output(command: list[str], env: dict[str, str]) -> str:
    return subprocess.check_output(command, cwd=ROOT, env=env, text=True, timeout=60).strip()


def size(path: Path) -> int:
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file())


def summarize(rows: list[dict]) -> dict:
    groups: dict[str, list[float]] = {}
    for row in rows:
        groups.setdefault(row["case"], []).append(row["build_seconds"])
    return {name: {"samples": len(values), "median_seconds": statistics.median(values),
                   "min_seconds": min(values), "max_seconds": max(values)}
            for name, values in groups.items()}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sccache", required=True, type=Path)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--package", choices=["oteryn-game-server"])
    parser.add_argument("--repetitions", type=int, choices=range(2, 6), default=2)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--budget-seconds", type=int, choices=range(60, 4501), default=4500)
    parser.add_argument("--temporary-root", type=Path)
    args = parser.parse_args()
    deadline = time.monotonic() + args.budget_seconds
    args.output.mkdir(parents=True, exist_ok=False)
    wrapper = str(args.sccache.resolve(strict=True))
    report = {"schema_version": 1, "status": "running", "rows": [],
              "scope": "server library" if args.package else "workspace all targets",
              "command": cargo_command(args.cargo, args.package),
              "controller_sha": os.environ.get("BENCHMARK_CONTROLLER_SHA"),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "sccache_binary_sha256": hashlib.sha256(Path(wrapper).read_bytes()).hexdigest(),
              "remote_restore": "NOT_PERFORMED", "remote_upload": "NOT_PERFORMED",
              "source_sha": output(["git", "rev-parse", "HEAD"], os.environ.copy()),
              "lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
              "incremental": False, "cache_backend": "isolated local disk",
              "cache_capacity": "2G", "time_budget_seconds": args.budget_seconds,
              "rustflags": os.environ.get("RUSTFLAGS", ""),
              "encoded_rustflags": os.environ.get("CARGO_ENCODED_RUSTFLAGS", ""),
              "profile_overrides": {name: os.environ.get(name) for name in
                                    ("CARGO_PROFILE_DEV_DEBUG", "CARGO_BUILD_JOBS")},
              "limitations": ["Compilation only: no test execution, Clippy or CI queue wait.",
                              "No remote transfer or cross-agent reuse is measured.",
                              "Each case uses one stable checkout and the same target path."]}
    result_path = args.output / "results.json"
    def persist():
        report["summary"] = summarize(report["rows"])
        report["elapsed_seconds"] = time.monotonic() - (deadline - args.budget_seconds)
        result_path.write_text(json.dumps(report, indent=2) + "\n")

    with tempfile.TemporaryDirectory(prefix="oteryn-rust-cache-", dir=args.temporary_root) as directory:
        scratch = Path(directory)
        target, cache = scratch / "target", scratch / "cache"
        (scratch / "sccache.toml").write_text("# Isolated cache configuration.\n")
        env = local_environment(os.environ.copy(), target, cache, scratch / "sccache.sock")
        server_owned = False
        persist()
        try:
            report["toolchain"] = output([args.cargo, "--version"], env)
            report["sccache_version"] = output([wrapper, "--version"], env)
            report["rustc_verbose_version"] = output([os.environ.get("RUSTC") or "rustc", "-vV"], env)
            report["initial_free_bytes"] = shutil.disk_usage(scratch).free
            minimum_gib = 2 if args.package else 14
            if report["initial_free_bytes"] < minimum_gib * 1024**3:
                raise RuntimeError(f"pilot requires {minimum_gib} GiB free before compilation")

            def remaining_timeout():
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise RuntimeError("pilot time budget exhausted")
                return min(1800, remaining)

            report["active_phase"] = "dependency_fetch"
            persist()
            report["dependency_fetch_seconds"] = run(
                [args.cargo, "fetch", "--locked", "--quiet"], env, args.output / "fetch.log", remaining_timeout())

            def measure(case: str, iteration: int, cached: bool, clean: bool):
                if clean and target.exists():
                    shutil.rmtree(target)  # Only our TemporaryDirectory's target.
                free_bytes = shutil.disk_usage(scratch).free
                minimum = (minimum_gib if clean else 1) * 1024**3
                if free_bytes < minimum:
                    raise RuntimeError(f"insufficient free disk for {case}: {free_bytes} bytes")
                build_env = env.copy()
                if cached:
                    build_env["RUSTC_WRAPPER"] = wrapper
                    output([wrapper, "--zero-stats"], env)
                report["active_phase"] = f"{case}-{iteration}"
                report["active_log"] = f"{case}-{iteration}.log"
                persist()
                row = {"case": case, "iteration": iteration,
                       "build_seconds": run(report["command"], build_env,
                                            args.output / f"{case}-{iteration}.log", remaining_timeout()),
                       "target_bytes": size(target), "free_bytes_before": free_bytes}
                if cached:
                    row["sccache"] = json.loads(output([wrapper, "--show-stats", "--stats-format", "json"], env))
                    stats = row["sccache"]["stats"]
                    errors = sum(stats.get("cache_errors", {}).get("counts", {}).values())
                    errors += sum(stats.get(name, 0) for name in
                                  ("cache_read_errors", "cache_write_errors", "cache_timeouts"))
                    row["cache_healthy"] = errors == 0
                report["rows"].append(row)
                persist()
                print(f"{case} {iteration}: {row['build_seconds']:.3f}s", flush=True)
                if cached and not row["cache_healthy"]:
                    raise RuntimeError("cache errors invalidate this performance sample")

            for iteration in range(1, args.repetitions + 1):
                measure("plain_cold", iteration, False, True)
                measure("plain_warm_target", iteration, False, False)
                # Every cycle has a genuinely cold compiler-cache seed.
                if server_owned:
                    output([wrapper, "--stop-server"], env)
                    server_owned = False
                if cache.exists():
                    shutil.rmtree(cache)  # Only this experiment's private cache.
                output([wrapper, "--start-server"], env)
                server_owned = True
                initial = json.loads(output([wrapper, "--show-stats", "--stats-format", "json"], env))
                if initial.get("cache_location") != f'Local disk: "{cache}"':
                    raise RuntimeError("sccache server does not own the isolated cache directory")
                measure("sccache_cold", iteration, True, True)
                measure("sccache_warm_empty_target", iteration, True, True)
                measure("sccache_warm_target", iteration, True, False)
            report.pop("active_phase", None)
            report.pop("active_log", None)
            report["status"] = "success"
            return 0
        except Exception as error:
            report["status"] = "failure"
            report["error"] = str(error)
            report["timed_out"] = isinstance(error, subprocess.TimeoutExpired)
            report["failure_kind"] = "timeout" if report["timed_out"] else "error"
            raise
        finally:
            if server_owned:
                try:
                    subprocess.run([wrapper, "--stop-server"], env=env, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, check=False, timeout=30)
                except subprocess.TimeoutExpired:
                    report["shutdown_error"] = "owned sccache server did not stop within30s"
            persist()


if __name__ == "__main__":
    raise SystemExit(main())
