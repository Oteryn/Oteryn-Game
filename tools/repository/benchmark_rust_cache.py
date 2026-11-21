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
    env.update(CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target), CARGO_INCREMENTAL="0",
               SCCACHE_DIR=str(cache), SCCACHE_SERVER_UDS=str(endpoint),
               SCCACHE_CACHE_SIZE="2G", SCCACHE_IDLE_TIMEOUT="0",
               SCCACHE_CONF=str(cache.parent / "sccache.toml"))
    return env


def run(command: list[str], env: dict[str, str], log: Path, timeout: float, cwd: Path = ROOT) -> float:
    started = time.monotonic()
    with log.open("w") as stream:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=stream,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            returncode = process.wait(timeout=timeout)
        except (subprocess.TimeoutExpired, KeyboardInterrupt):
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


def output(command: list[str], env: dict[str, str], cwd: Path, timeout: float) -> str:
    return subprocess.check_output(command, cwd=cwd, env=env, text=True, timeout=timeout).strip()


def size(path: Path) -> int:
    return sum(p.stat().st_size for p in path.rglob("*") if p.is_file())


def summarize(rows: list[dict]) -> dict:
    groups: dict[str, list[float]] = {}
    for row in rows:
        groups.setdefault(row["case"], []).append(row["build_seconds"])
    return {name: {"samples": len(values), "median_seconds": statistics.median(values),
                   "min_seconds": min(values), "max_seconds": max(values)}
            for name, values in groups.items()}


def rust_edit_paths(raw_diff: str) -> list[str]:
    """Require changed blobs in existing, regular Rust files with unchanged modes."""
    changes = raw_diff.split("\0")[:-1]
    paths = []
    for details, path in zip(changes[::2], changes[1::2], strict=True):
        old_mode, new_mode, old_blob, new_blob, status = details.split()
        if (new_mode not in {"100644", "100755"} or old_mode != f":{new_mode}" or old_blob == new_blob
                or status != "M" or not path.endswith(".rs")):
            raise RuntimeError("edit patch must change only existing tracked Rust source files")
        paths.append(path)
    if not paths:
        raise RuntimeError("edit patch did not change any tracked Rust source")
    return paths


def build_activity(log: Path) -> dict[str, int]:
    """Count Cargo artifacts, including artifacts reconstructed from sccache hits."""
    counts = {"fresh_artifacts": 0, "rebuilt_artifacts": 0}
    with log.open(errors="replace") as stream:
        for line in stream:
            if not line.startswith("{"):
                continue
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue  # Non-Cargo output from a tool sharing the log.
            if message.get("reason") == "compiler-artifact":
                counts["fresh_artifacts" if message["fresh"] else "rebuilt_artifacts"] += 1
    return counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sccache", required=True, type=Path)
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--package", choices=["oteryn-game-server"])
    parser.add_argument("--repetitions", type=int, choices=range(2, 6), default=2)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--budget-seconds", type=int, choices=range(60, 4501), default=4500,
                        metavar="60..4500")
    parser.add_argument("--temporary-root", type=Path)
    parser.add_argument("--edit-patch", type=Path,
                        help="Git patch against HEAD that changes existing Rust source used by the build")
    args = parser.parse_args()
    started = time.monotonic()
    deadline = started + args.budget_seconds
    args.output.mkdir(parents=True, exist_ok=False)
    command = cargo_command(args.cargo, args.package)
    if args.edit_patch:
        command.append("--message-format=json")
    report = {"schema_version": 2 if args.edit_patch else 1, "status": "running", "rows": [],
              "scope": "server library" if args.package else "workspace all targets",
              "command": command,
              "controller_sha": os.environ.get("BENCHMARK_CONTROLLER_SHA"),
              "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "remote_restore": "NOT_PERFORMED", "remote_upload": "NOT_PERFORMED",
              "cache_backend": "isolated local disk",
              "cache_capacity": "2G", "time_budget_seconds": args.budget_seconds,
              "rustflags": os.environ.get("RUSTFLAGS", ""),
              "encoded_rustflags": os.environ.get("CARGO_ENCODED_RUSTFLAGS", ""),
              "profile_overrides": {name: os.environ.get(name) for name in
                                    ("CARGO_PROFILE_DEV_DEBUG", "CARGO_BUILD_JOBS")},
              "cleanup": {"scratch": "not_created", "sccache_server": "not_started"},
              "limitations": ["Compilation only: no test execution, Clippy or CI queue wait.",
                              "No remote transfer or cross-agent reuse is measured.",
                              "All cases use the same checkout and target paths.",
                              "Build times are wall time for Cargo and its work, including any sccache waits. "
                              "They exclude setup, patching, target deletion and statistics queries.",
                              "No per-process CPU, aggregate peak memory or OS page-cache isolation is measured. "
                              "Cargo's fetched dependency sources may be shared."]}
    if not args.edit_patch:
        report["incremental"] = False
    else:
        report["limitations"].append("Artifact counts report Cargo freshness. An artifact may be rebuilt "
                                     "by Cargo using a compiler-cache hit.")
    result_path = args.output / "results.json"

    def persist():
        report["summary"] = summarize(report["rows"])
        report["elapsed_seconds"] = time.monotonic() - started
        result_path.write_text(json.dumps(report, indent=2) + "\n")

    def remaining_timeout(limit: float = 1800) -> float:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired(report.get("active_phase", "setup"), args.budget_seconds)
        return min(limit, remaining)

    def phase(name: str) -> Path:
        report["active_phase"] = name
        report["active_log"] = f"{name}.log"
        persist()
        return args.output / report["active_log"]

    directory = None
    server_owned = False
    cwd = ROOT
    persist()
    try:
        wrapper = str(args.sccache.resolve(strict=True))
        report["sccache_binary_sha256"] = hashlib.sha256(Path(wrapper).read_bytes()).hexdigest()
        directory = tempfile.TemporaryDirectory(prefix="oteryn-rust-cache-", dir=args.temporary_root)
        scratch = Path(directory.name).resolve()
        report["cleanup"]["scratch"] = "created"
        target, cache = scratch / "target", scratch / "cache"
        (scratch / "sccache.toml").write_text("# Isolated cache configuration.\n")
        parent = os.environ.copy()
        if args.edit_patch:
            # Git routing overrides could otherwise redirect private index/object writes.
            parent = {name: value for name, value in parent.items() if not name.startswith("GIT_")}
        env = local_environment(parent, target, cache, scratch / "sccache.sock")

        def query(*args: str) -> str:
            return output(list(args), env, cwd, remaining_timeout(60))

        report["initial_free_bytes"] = shutil.disk_usage(scratch).free
        minimum_gib = 2 if args.package else 14
        if report["initial_free_bytes"] < minimum_gib * 1024**3:
            raise RuntimeError(f"pilot requires {minimum_gib} GiB free before compilation")
        report["source_sha"] = query("git", "rev-parse", "HEAD")
        source_tree = None
        if args.edit_patch:
            # A shared clone borrows immutable Git objects and owns its working tree, index and ref state.
            checkout = scratch / "source"
            run(["git", "clone", "--quiet", "--shared", "--no-checkout", "--", str(ROOT), str(checkout)],
                env, phase("source_clone"), remaining_timeout())
            cwd = checkout
            run(["git", "-c", "core.hooksPath=/dev/null", "checkout", "--quiet", "--detach", report["source_sha"]],
                env, phase("source_checkout"), remaining_timeout(), cwd)
            base_tree = query("git", "rev-parse", "HEAD^{tree}")
            source_tree = base_tree
            patch_bytes = args.edit_patch.read_bytes()
            patch = scratch / "source-edit.patch"
            patch.write_bytes(patch_bytes)

            def apply_edit(label: str, reverse: bool = False) -> str:
                apply = ["git", "apply", "--index", "--whitespace=nowarn"]
                if reverse:
                    apply.append("--reverse")
                run([*apply, str(patch)], env, phase(label), remaining_timeout(), cwd)
                return query("git", "write-tree")

            edited_tree = apply_edit("source_edit_validate")
            paths = rust_edit_paths(query("git", "diff", "--cached", "--raw", "--full-index", "--abbrev=64",
                                         "--no-renames", "-z", "HEAD"))
            if not query("git", "diff", "--cached", "--no-ext-diff", "--no-textconv", "-w", "--ignore-blank-lines",
                         "-I", r"^[[:space:]]*//", "HEAD"):
                raise RuntimeError("edit patch must contain a Rust code change beyond whitespace or line comments")
            (args.output / patch.name).write_bytes(patch_bytes)
            report["source_edit"] = {"base_tree": base_tree, "edited_tree": edited_tree, "paths": paths,
                                     "patch": patch.name, "patch_sha256": hashlib.sha256(patch_bytes).hexdigest()}

            def set_edit(label: str, reverse: bool = False) -> str:
                tree = apply_edit(label, reverse)
                if tree != (base_tree if reverse else edited_tree):
                    raise RuntimeError("private checkout tree differs from the validated edit or base")
                return tree

            source_tree = set_edit("source_edit_restore", reverse=True)

        report["lock_sha256"] = hashlib.sha256((cwd / "Cargo.lock").read_bytes()).hexdigest()
        report["toolchain"] = query(args.cargo, "--version")
        report["sccache_version"] = query(wrapper, "--version")
        report["rustc_verbose_version"] = query(os.environ.get("RUSTC") or "rustc", "-vV")
        report["dependency_fetch_seconds"] = run(
            [args.cargo, "fetch", "--locked", "--quiet"], env, phase("fetch"), remaining_timeout(), cwd)

        def measure(case: str, iteration: int, cached: bool, clean: bool, *, incremental: bool = False):
            log = phase(f"{case}-{iteration}")
            if args.edit_patch and query("git", "diff", "--no-ext-diff", "--name-only", source_tree, "--"):
                raise RuntimeError(f"private checkout has unexpected tracked changes before {case}")
            if clean and target.exists():
                shutil.rmtree(target)  # Only our TemporaryDirectory's target.
            free_bytes = shutil.disk_usage(scratch).free
            minimum = (minimum_gib if clean else 1) * 1024**3
            if free_bytes < minimum:
                raise RuntimeError(f"insufficient free disk for {case}: {free_bytes} bytes")
            build_env = env.copy()
            build_env["CARGO_INCREMENTAL"] = "1" if incremental else "0"
            if cached:
                build_env["RUSTC_WRAPPER"] = wrapper
                query(wrapper, "--zero-stats")
            row = {"case": case, "iteration": iteration,
                   "build_seconds": run(command, build_env, log, remaining_timeout(), cwd),
                   "target_bytes": size(target), "free_bytes_before": free_bytes}
            if args.edit_patch:
                row.update(source_tree=source_tree, incremental=incremental,
                           target_state="empty" if clean else "retained", **build_activity(log))
                if row["fresh_artifacts"] + row["rebuilt_artifacts"] == 0:
                    raise RuntimeError(f"Cargo reported no artifacts for {case}; see {log}")
                if case.endswith("_noop") and row["rebuilt_artifacts"]:
                    raise RuntimeError(f"unchanged build rebuilt artifacts for {case}; see {log}")
                if source_tree == edited_tree and not row["rebuilt_artifacts"]:
                    raise RuntimeError(f"edited build did not rebuild artifacts for {case}; see {log}")
            if cached:
                row["sccache"] = json.loads(query(wrapper, "--show-stats", "--stats-format", "json"))
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
            # Every cycle starts with a genuinely cold private compiler cache.
            if cache.exists():
                shutil.rmtree(cache)
            if args.edit_patch:
                measure("cargo_incremental_cold", iteration, False, True, incremental=True)
                measure("cargo_incremental_noop", iteration, False, False, incremental=True)
                source_tree = set_edit(f"cargo_incremental_apply_edit-{iteration}")
                measure("cargo_incremental_edit_retained_target", iteration, False, False, incremental=True)
                source_tree = set_edit(f"cargo_incremental_reverse_edit-{iteration}", reverse=True)
            else:
                measure("plain_cold", iteration, False, True)
                measure("plain_warm_target", iteration, False, False)
            server_owned = True  # A timed-out start may already have launched the daemon.
            run([wrapper, "--start-server"], env, phase(f"sccache_start-{iteration}"), remaining_timeout(), cwd)
            report["cleanup"]["sccache_server"] = "started"
            initial = json.loads(query(wrapper, "--show-stats", "--stats-format", "json"))
            if initial.get("cache_location") != f'Local disk: "{cache}"':
                raise RuntimeError("sccache server does not own the isolated cache directory")
            measure("sccache_cold", iteration, True, True)
            if args.edit_patch:
                measure("sccache_noop", iteration, True, False)
                source_tree = set_edit(f"sccache_apply_edit-{iteration}")
                measure("sccache_edit_empty_target", iteration, True, True)
                source_tree = set_edit(f"sccache_reverse_edit-{iteration}", reverse=True)
            else:
                measure("sccache_warm_empty_target", iteration, True, True)
                measure("sccache_warm_target", iteration, True, False)
            run([wrapper, "--stop-server"], env, phase(f"sccache_stop-{iteration}"), remaining_timeout(30), cwd)
            report["cleanup"]["sccache_server"] = "stopped"
            server_owned = False
        report.pop("active_phase", None)
        report.pop("active_log", None)
        report["status"] = "success"
    except (Exception, KeyboardInterrupt) as error:
        report["status"] = "failure"
        report["error"] = str(error) or type(error).__name__
        report["timed_out"] = isinstance(error, subprocess.TimeoutExpired)
        report["failure_kind"] = "timeout" if report["timed_out"] else "error"
        raise
    finally:
        if server_owned:
            try:
                run([wrapper, "--stop-server"], env, args.output / "sccache-cleanup.log", 30, cwd)
                report["cleanup"]["sccache_server"] = "stopped"
            except (Exception, KeyboardInterrupt) as error:
                report["cleanup"]["sccache_server"] = "failed"
                report["shutdown_error"] = str(error) or type(error).__name__
        if directory is not None:
            try:
                directory.cleanup()
                report["cleanup"]["scratch"] = "removed"
            except (Exception, KeyboardInterrupt) as error:
                report["cleanup"]["scratch"] = "failed"
                report["scratch_cleanup_error"] = str(error) or type(error).__name__
                if report["status"] == "success":
                    report.update(status="failure", error=report["scratch_cleanup_error"], failure_kind="cleanup")
                    raise
            finally:
                persist()
        else:
            persist()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
