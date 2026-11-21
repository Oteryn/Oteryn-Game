#!/usr/bin/env python3
"""Bound the four existing Cargo cache roots before protected-main publication.

The budget counts apparent file bytes (including sparse files), not allocated
disk blocks or a prediction of zstd size. Registry roots are measured only.
Workspace outputs and final executables are disposable; retained dependency
libraries, build-script outputs and fingerprints are kept as package groups.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
from dataclasses import dataclass
import errno
import json
import os
from pathlib import Path
import re
import stat
import subprocess


DEFAULT_BUDGET = 4 << 30
# normalize_cargo_mtimes --record runs after trimming. Its tracked-path snapshot
# needs a little room; the subsequent --check-only measures its actual size.
SNAPSHOT_RESERVE = 16 << 20
MARKERS = {".oteryn-cargo-source-snapshot-v1.json", ".rustc_info.json", "CACHEDIR.TAG"}
LIBRARY_SUFFIXES = {".rlib", ".rmeta", ".so", ".dylib", ".dll", ".a", ".lib", ".d"}
HASHED = re.compile(r"(.+)-([0-9a-f]{16})")
DIRECTORY_FLAGS = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW


@dataclass(frozen=True)
class Entry:
    parts: tuple[str, ...]
    size: int
    device: int
    inode: int
    mode: int


def absolute(path: Path) -> Path:
    return Path(os.path.abspath(path))


def open_directory(path: Path) -> int:
    """Open every component without following directory symlinks."""
    descriptor = os.open(path.anchor, DIRECTORY_FLAGS)
    try:
        for part in path.parts[1:]:
            child = os.open(part, DIRECTORY_FLAGS, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = child
        return descriptor
    except BaseException:
        os.close(descriptor)
        raise


def scan(root: Path) -> list[Entry]:
    try:
        descriptor = open_directory(root)
    except FileNotFoundError:
        return []
    entries = []

    def visit(directory: int, prefix: tuple[str, ...]) -> None:
        with os.scandir(directory) as iterator:
            names = sorted(entry.name for entry in iterator)
        for name in names:
            info = os.stat(name, dir_fd=directory, follow_symlinks=False)
            parts = (*prefix, name)
            if stat.S_ISDIR(info.st_mode):
                child = os.open(name, DIRECTORY_FLAGS, dir_fd=directory)
                try:
                    visit(child, parts)
                finally:
                    os.close(child)
            elif stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode):
                entries.append(Entry(parts, info.st_size, info.st_dev, info.st_ino, info.st_mode))
            else:
                raise ValueError("unsupported special file in Cargo cache")

    try:
        visit(descriptor, ())
    finally:
        os.close(descriptor)
    return entries


def workspace_packages(workspace: Path, target: Path) -> set[str]:
    command = ["cargo", "+1.100.0", "metadata", "--offline", "--locked", "--no-deps", "--format-version", "1"]
    result = subprocess.run(command, cwd=workspace, capture_output=True, text=True, timeout=60)
    if result.returncode:
        raise ValueError("cannot read locked offline Cargo workspace metadata")
    document = json.loads(result.stdout)
    if absolute(Path(document["workspace_root"])) != workspace or absolute(Path(document["target_directory"])) != target:
        raise ValueError("Cargo metadata does not name the managed workspace/target")
    members = set(document["workspace_members"])
    packages = {package["name"] for package in document["packages"] if package["id"] in members}
    if not packages or any(not isinstance(name, str) or not re.fullmatch(r"[A-Za-z0-9_-]+", name) for name in packages):
        raise ValueError("invalid Cargo workspace package names")
    return packages


def package_location(parts: tuple[str, ...]):
    for index, part in enumerate(parts[:-1]):
        # Cargo 1.100 also places both outputs and fingerprints under
        # build/PACKAGE/HASH/{out,fingerprint}/ instead of PACKAGE-HASH.
        if (part == "build" and index + 2 < len(parts)
                and re.fullmatch(r"[A-Za-z0-9_-]+", parts[index + 1])
                and re.fullmatch(r"[0-9a-f]{16}", parts[index + 2])):
            return parts[:index], parts[index + 1], parts[index + 2]
        if part in {".fingerprint", "build"} and index + 1 < len(parts):
            match = HASHED.fullmatch(parts[index + 1])
            if match:
                return parts[:index], match[1], match[2]
    return None


def deletion_plan(entries: list[Entry], packages: set[str], available: int):
    # The unit hash associates deps/libNAME-HASH.* with its fingerprint, even
    # when [lib].name differs from the Cargo package name. Build-script hashes
    # differ, so all units of one package are evicted together.
    units = {}
    for entry in entries:
        location = package_location(entry.parts)
        if location:
            profile, package, unit = location
            key = profile, unit
            if key in units and units[key] != package:
                raise ValueError("ambiguous Cargo dependency unit")
            units[key] = package
    groups, mandatory, protected = defaultdict(list), [], []
    evict = set(packages)
    for entry in entries:
        parts = entry.parts
        if len(parts) == 1 and parts[0] in MARKERS and stat.S_ISREG(entry.mode):
            protected.append(entry)
            continue
        location = package_location(parts)
        package = location[1] if location else None
        if package is None and "deps" in parts[:-1]:
            index = parts.index("deps")
            stem = Path(parts[-1]).stem if Path(parts[-1]).suffix else parts[-1]
            match = HASHED.fullmatch(stem)
            if match:
                package = units.get((parts[:index], match[2]))
        disposable = ("incremental" in parts[:-1] or "examples" in parts[:-1]
                      or stat.S_ISLNK(entry.mode)
                      or ("deps" in parts[:-1] and Path(parts[-1]).suffix not in LIBRARY_SUFFIXES))
        if package is not None:
            groups[package].append(entry)
            if disposable:
                evict.add(package)
        else:
            mandatory.append(entry)
    removed = list(mandatory)
    kept = {}
    for package, files in groups.items():
        if package in evict:
            removed.extend(files)
        else:
            kept[package] = files
    remaining = sum(entry.size for entry in protected) + sum(entry.size for files in kept.values() for entry in files)
    for package in sorted(kept, key=lambda name: (-sum(entry.size for entry in kept[name]), name)):
        if remaining <= available:
            break
        files = kept[package]
        remaining -= sum(entry.size for entry in files)
        removed.extend(files)
        evict.add(package)
    if remaining > available:
        raise ValueError("Cargo cache budget cannot retain its source snapshot")
    return sorted(removed, key=lambda entry: entry.parts), sorted(evict & groups.keys())


def unlink_entry(target: Path, entry: Entry) -> None:
    descriptor = open_directory(target.joinpath(*entry.parts[:-1]))
    try:
        info = os.stat(entry.parts[-1], dir_fd=descriptor, follow_symlinks=False)
        if (info.st_dev, info.st_ino, info.st_mode, info.st_size) != (entry.device, entry.inode, entry.mode, entry.size):
            raise ValueError("Cargo cache changed during bounded preparation")
        os.unlink(entry.parts[-1], dir_fd=descriptor)
    finally:
        os.close(descriptor)


def remove_empty_directories(target: Path) -> None:
    try:
        descriptor = open_directory(target)
    except FileNotFoundError:
        return

    def prune(directory: int) -> None:
        with os.scandir(directory) as iterator:
            names = sorted(entry.name for entry in iterator)
        for name in names:
            if not stat.S_ISDIR(os.stat(name, dir_fd=directory, follow_symlinks=False).st_mode):
                continue
            child = os.open(name, DIRECTORY_FLAGS, dir_fd=directory)
            try:
                prune(child)
            finally:
                os.close(child)
            try:
                os.rmdir(name, dir_fd=directory)
            except OSError as error:
                if error.errno not in (errno.ENOTEMPTY, errno.EEXIST):
                    raise

    try:
        prune(descriptor)
    finally:
        os.close(descriptor)


def prepare(workspace: Path, target: Path, cargo_home: Path, *, max_bytes: int = DEFAULT_BUDGET,
            reserve_bytes: int = SNAPSHOT_RESERVE, check_only: bool = False,
            packages: set[str] | None = None) -> dict:
    workspace, target, cargo_home = map(absolute, (workspace, target, cargo_home))
    if target != workspace / "target":
        raise ValueError("only the workspace target directory is managed")
    roots = [cargo_home / path for path in ("registry/index", "registry/cache", "git/db")]
    if any(root.is_relative_to(target) or target.is_relative_to(root) for root in roots):
        raise ValueError("Cargo registry and managed target must be disjoint")
    if type(max_bytes) is not int or max_bytes <= 0 or type(reserve_bytes) is not int or reserve_bytes < 0:
        raise ValueError("invalid Cargo cache byte budget")
    reserve = 0 if check_only else reserve_bytes
    if reserve >= max_bytes:
        raise ValueError("snapshot reserve must be smaller than the cache budget")
    registry = {str(root): sum(entry.size for entry in scan(root)) for root in roots}
    registry_bytes = sum(registry.values())
    if registry_bytes > max_bytes - reserve:
        raise ValueError("Cargo registry alone exceeds the cache budget; nothing was deleted")
    entries = scan(target)
    before_target = sum(entry.size for entry in entries)
    removed, evicted = [], []
    if not check_only:
        packages = workspace_packages(workspace, target) if packages is None else packages
        removed, evicted = deletion_plan(entries, packages, max_bytes - reserve - registry_bytes)
        for entry in removed:
            unlink_entry(target, entry)
        remove_empty_directories(target)
    after_target = sum(entry.size for entry in scan(target))
    observed_registry = {str(root): sum(entry.size for entry in scan(root)) for root in roots}
    if registry != observed_registry:
        raise ValueError("Cargo registry changed during bounded preparation")
    if after_target + registry_bytes > max_bytes - reserve:
        raise ValueError("Cargo cache exceeds its byte budget; publication is blocked")
    return {"schema": "cargo-cache-budget-v1", "check_only": check_only, "max_bytes": max_bytes,
            "snapshot_reserve_bytes": reserve, "registry_roots_bytes": registry,
            "before": {"target_bytes": before_target, "registry_bytes": registry_bytes,
                       "total_bytes": before_target + registry_bytes},
            "after": {"target_bytes": after_target, "registry_bytes": registry_bytes,
                      "total_bytes": after_target + registry_bytes},
            "removed_bytes": before_target - after_target, "removed_files": len(removed),
            "evicted_packages": evicted}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, default=Path.cwd())
    parser.add_argument("--target", type=Path, default=Path("target"))
    parser.add_argument("--cargo-home", type=Path, required=True)
    parser.add_argument("--max-bytes", type=int, default=DEFAULT_BUDGET)
    parser.add_argument("--snapshot-reserve-bytes", type=int, default=SNAPSHOT_RESERVE)
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--metrics", type=Path, required=True)
    args = parser.parse_args()
    workspace = absolute(args.workspace)
    target = args.target if args.target.is_absolute() else workspace / args.target
    metrics = absolute(args.metrics)
    try:
        cache_roots = [absolute(target), *(absolute(args.cargo_home) / path
                        for path in ("registry/index", "registry/cache", "git/db"))]
        if metrics.is_relative_to(workspace) or any(metrics.is_relative_to(root) for root in cache_roots):
            raise ValueError("metrics must be outside the source tree and all cache roots")
        # Refuse an existing/symlink output before mutating any cached products.
        descriptor = open_directory(metrics.parent)
        try:
            output = os.open(metrics.name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=descriptor)
        finally:
            os.close(descriptor)
        with os.fdopen(output, "w", encoding="utf-8") as stream:
            report = prepare(workspace, target, args.cargo_home, max_bytes=args.max_bytes,
                             reserve_bytes=args.snapshot_reserve_bytes, check_only=args.check_only)
            stream.write(json.dumps(report, indent=2, sort_keys=True) + "\n")
        print(json.dumps(report, sort_keys=True))
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"Cargo cache publication blocked: {error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
