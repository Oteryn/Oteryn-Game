#!/usr/bin/env python3
"""Reuse Cargo outputs with the source mtimes observed by the cache producer.

An inherited target directory can contain variants that the producer did not
build. Preserve each changed source's newer mtime through subsequent saves so
those variants remain stale until Cargo builds them. Parent directory mtimes
also matter to build scripts that watch directories or removed entries.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import stat
import subprocess
import time
from pathlib import Path

TARGET = Path("target")
SNAPSHOT = TARGET / ".oteryn-cargo-source-snapshot-v1.json"
LEGACY_MARKER = TARGET / ".oteryn-cache-source-sha"
SHA = re.compile(r"[0-9a-f]{40}")


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root)


def head(root: Path) -> str:
    source = git(root, "rev-parse", "HEAD").decode("ascii").strip()
    if SHA.fullmatch(source) is None:
        raise ValueError("invalid checkout identity")
    if git(root, "diff", "--name-only", "HEAD", "--"):
        raise ValueError("tracked worktree differs from the checkout identity")
    return source


def parents(path: str) -> list[str]:
    return [parent.as_posix() for parent in Path(path).parents]


def paths(output: bytes) -> set[str]:
    return {os.fsdecode(path) for path in output.split(b"\0") if path}


def tracked_mtimes(root: Path) -> dict[str, int]:
    files = paths(git(root, "ls-files", "-z"))
    directories = {parent for file in files for parent in parents(file)}
    mtimes = {}
    for path in sorted(files | directories):
        info = (root / path).lstat()
        expected_type = stat.S_ISDIR if path in directories else stat.S_ISREG
        if not expected_type(info.st_mode):
            raise ValueError(f"unsupported tracked path type: {path}")
        mtimes[path] = info.st_mtime_ns
    return mtimes


def parse_snapshot(path: Path, now: int) -> tuple[str, dict[str, int]]:
    snapshot = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(snapshot, dict) or snapshot.get("schema") != "cargo-source-mtimes-v1":
        raise ValueError("invalid cache source snapshot schema")
    source = snapshot.get("source")
    recorded_at = snapshot.get("recorded_at_ns")
    mtimes = snapshot.get("mtimes")
    if not isinstance(source, str) or SHA.fullmatch(source) is None:
        raise ValueError("invalid cache source snapshot identity")
    if type(recorded_at) is not int or not 0 < recorded_at < now:
        raise ValueError("invalid cache source snapshot capture time")
    if not isinstance(mtimes, dict) or "." not in mtimes or any(
        type(mtime) is not int or not 0 < mtime <= recorded_at for mtime in mtimes.values()
    ):
        raise ValueError("invalid cache source snapshot mtimes")
    return source, mtimes


def restore_snapshot(root: Path) -> str:
    fresh = time.time_ns()
    if (root / TARGET).is_symlink():
        raise ValueError("Cargo target is a symlink")
    source, previous = parse_snapshot(root / SNAPSHOT, fresh)
    current = head(root)
    try:
        subprocess.check_call(
            ["git", "cat-file", "-e", f"{source}^{{commit}}"], cwd=root, stderr=subprocess.DEVNULL
        )
    except subprocess.CalledProcessError:
        # Git compares trees, so the cached commit need not bring its blobs.
        subprocess.check_call(
            ["git", "fetch", "--no-tags", "--depth=1", "--filter=blob:none", "origin", source],
            cwd=root,
        )
    changed = paths(git(root, "diff", "--no-renames", "--name-only", "-z", source, current, "--"))
    dirty = changed | {parent for path in changed for parent in parents(path)}
    tracked = tracked_mtimes(root)
    unchanged = tracked.keys() - dirty
    if not unchanged <= previous.keys():
        raise ValueError("missing mtimes for unchanged tracked paths")
    for path in sorted(tracked):
        mtime = previous[path] if path in unchanged else fresh
        os.utime(root / path, ns=(mtime, mtime), follow_symlinks=False)
    return f"cache source {source[:12]}: restored {len(unchanged)} mtimes, refreshed {len(tracked) - len(unchanged)}"


def normalize(root: Path, *, discard_invalid: bool = False) -> str:
    try:
        return restore_snapshot(root)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        # A restored target with no reliable provenance cannot attest the checkout.
        # In CI it is disposable; downloaded Cargo registry files are outside it.
        target = root / TARGET
        if discard_invalid and (target.exists() or target.is_symlink()):
            if target.is_symlink() or not target.is_dir():
                target.unlink()
            else:
                shutil.rmtree(target)
            return f"discarded restored Cargo target: {error}"
        return f"Cargo target snapshot unavailable: {error}"


def record_snapshot(root: Path) -> str:
    source = head(root)
    mtimes = tracked_mtimes(root)
    recorded_at = time.time_ns()
    if any(not 0 < mtime <= recorded_at for mtime in mtimes.values()):
        raise ValueError("cannot record invalid tracked path mtimes")
    snapshot = {
        "schema": "cargo-source-mtimes-v1",
        "source": source,
        "recorded_at_ns": recorded_at,
        "mtimes": mtimes,
    }
    temporary = (root / SNAPSHOT).with_suffix(".tmp")
    temporary.write_text(json.dumps(snapshot, sort_keys=True), encoding="utf-8")
    temporary.replace(root / SNAPSHOT)
    (root / LEGACY_MARKER).unlink(missing_ok=True)
    return f"recorded Cargo source mtimes for {source[:12]} ({len(mtimes)} paths)"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--record", action="store_true", help="record the source after a successful build")
    args = parser.parse_args()
    root = Path.cwd()
    if args.record:
        print(record_snapshot(root))
    else:
        print(normalize(root, discard_invalid=os.environ.get("GITHUB_ACTIONS") == "true"))


if __name__ == "__main__":
    main()
