#!/usr/bin/env python3
"""Input key and success marker for the pull-request merge-gate job-result cache.

A cached job is skipped only when an earlier run of the same job on the same pull
request already succeeded on identical inputs. The key is a SHA-256 over the job
identity, the git blob ids of every file the job can read (`git ls-files -s`, the
same squash-stable pattern as the World bundle `inputs_digest`) and a few literal
facts. When in doubt a mode includes more files, never fewer: a false hit would
skip a test, a false miss only costs a re-run.

Modes
  rust-fast       changed-crate closure (workspace path dependencies of every
                  selected crate, all kinds) plus Cargo/toolchain inputs. A crate
                  whose closure sources can read outside their own directory
                  (include_str!, CARGO_MANIFEST_DIR, `../`, processes, ...) is
                  not hermetic: the whole tree minus provably unread prefixes
                  is keyed instead.
  rust-workspace  every Rust source, manifest, lock, toolchain/config and
                  Cargo-adjacent file (Rust policy, supply chain).
  tree            every tracked file (governance and policy validators).

Failures are never cached and any error here yields an empty key, which means
"no caching, run everything". Only the standard library is used.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath

KEY_VERSION = "jobresult-v1"
DOMAIN = "oteryn:merge-gate/job-result/v1"
MARKER_NAME = "marker.json"

# Always part of every key, in every mode.
COMMON_FILES = (
    ".github/workflows/merge-gate.yml",
    "tools/repository/classify_pr_test_lanes.py",
    "tools/repository/job_result_key.py",
)
# Cargo and toolchain inputs, matched on the base name anywhere in the tree.
CARGO_BASENAMES = frozenset({
    "Cargo.toml", "Cargo.lock", "rust-toolchain", "rust-toolchain.toml",
    "rustfmt.toml", ".rustfmt.toml", "clippy.toml", ".clippy.toml", "deny.toml",
    "workspace-boundaries.toml", ".cargo-checksum.json", "build.rs",
})
CARGO_DIR_SEGMENT = ".cargo"
WORKSPACE_PREFIXES = ("vendor/", "docs/migration/")
WORKSPACE_SUFFIXES = (".rs", ".toml")

# Directories that no Rust source reads (guarded by test_job_result_key.py). They
# only relax the whole-tree fallback of non-hermetic crates.
UNREAD_PREFIXES = (
    ".claude/", ".agents/", "docs/superpowers/", "docs/agents/programs/",
    "docs/agents/prompts/", "docs/agents/tasks/",
)

# Anything that lets Rust code, a build script or a test look outside the files of
# its own crate directory.
ESCAPE = re.compile(
    r"CARGO_MANIFEST_DIR|OUT_DIR|include_str!|include_bytes!|\binclude!|#\s*\[\s*path\b"
    r"|current_dir|env::var|option_env!|\benv!\s*\(|Command::new|std::process|read_dir"
    r"|walkdir|WalkDir|canonicalize|\.parent\(\)|ancestors\(\)|\bglob\s*\(|git2"
    r'|"[^"\n]*\.\.[/"]'
)


class KeyError_(Exception):
    """The inputs cannot be enumerated completely; the caller must not cache."""


def git(root: Path, *args: str) -> bytes:
    try:
        return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.DEVNULL)
    except (OSError, subprocess.SubprocessError) as error:
        raise KeyError_(f"git {' '.join(args)} failed") from error


def tracked(root: Path) -> dict[str, tuple[str, str, str]]:
    """Tracked path -> (mode, blob, stage), from the index."""
    listing = git(root, "ls-files", "-s", "-z").decode("utf-8", "surrogateescape")
    files: dict[str, tuple[str, str, str]] = {}
    for row in listing.split("\0"):
        if not row:
            continue
        meta, _, path = row.partition("\t")
        parts = meta.split(" ")
        if len(parts) != 3 or not path:
            raise KeyError_("malformed git ls-files row")
        files[path] = (parts[0], parts[1], parts[2])
    if not files:
        raise KeyError_("no tracked files")
    return files


def is_cargo_input(path: str) -> bool:
    pure = PurePosixPath(path)
    return pure.name in CARGO_BASENAMES or CARGO_DIR_SEGMENT in pure.parts[:-1] or pure.name == ".cargo"


def is_workspace_input(path: str) -> bool:
    return (
        is_cargo_input(path)
        or path.startswith(WORKSPACE_PREFIXES)
        or path.endswith(WORKSPACE_SUFFIXES)
        or path.endswith(".lock")
    )


def under(path: str, directory: str) -> bool:
    return directory in ("", ".") or path == directory or path.startswith(directory.rstrip("/") + "/")


def read_packages(tsv: Path) -> list[tuple[str, str]]:
    rows = []
    for line in tsv.read_text(encoding="utf-8").splitlines():
        if line:
            name, _, flags = line.partition("\t")
            rows.append((name, flags))
    if not rows:
        raise KeyError_("empty selected package list")
    return rows


def closure_directories(root: Path, metadata: dict, selected: list[str]) -> tuple[set[str], bool]:
    """Directories of the selected crates and every path dependency (all kinds).

    The flag is true when a path dependency is not a workspace member, whose own
    dependencies are then unknown (the caller falls back to the whole tree).
    """
    workspace_root = Path(metadata["workspace_root"]).resolve(strict=True)
    if workspace_root != root.resolve(strict=True):
        raise KeyError_("metadata workspace root differs from the checkout")
    members = set(metadata["workspace_members"])
    by_name: dict[str, dict] = {}
    by_dir: dict[str, dict] = {}
    for package in metadata["packages"]:
        if package["id"] not in members:
            continue
        directory = Path(package["manifest_path"]).resolve(strict=True).parent.relative_to(root.resolve(strict=True)).as_posix()
        by_name[package["name"]] = package
        by_dir[directory] = package
    directories: set[str] = set()
    external = False
    pending = list(selected)
    seen: set[str] = set()
    while pending:
        name = pending.pop()
        if name in seen:
            continue
        seen.add(name)
        package = by_name.get(name)
        if package is None:
            raise KeyError_(f"selected package {name} is not a workspace member")
        directory = Path(package["manifest_path"]).resolve(strict=True).parent.relative_to(root.resolve(strict=True)).as_posix()
        directories.add("" if directory == "." else directory)
        for dependency in package.get("dependencies", []):
            path = dependency.get("path")
            if not path:
                continue
            try:
                relative = Path(path).resolve(strict=True).relative_to(root.resolve(strict=True)).as_posix()
            except (OSError, ValueError) as error:
                raise KeyError_("path dependency outside the checkout") from error
            relative = "" if relative == "." else relative
            owner = by_dir.get(relative)
            if owner is None:
                directories.add(relative)
                external = True
            else:
                pending.append(owner["name"])
    return directories, external


def escapes_crate(root: Path, files: dict[str, tuple[str, str, str]], directories: set[str]) -> bool:
    for path in files:
        if not path.endswith(".rs") or not any(under(path, directory) for directory in directories):
            continue
        try:
            text = (root / path).read_text(encoding="utf-8", errors="replace")
        except OSError as error:
            raise KeyError_(f"cannot read {path}") from error
        if ESCAPE.search(text):
            return True
    return False


def select_paths(mode: str, root: Path, files: dict[str, tuple[str, str, str]], metadata: Path | None, packages: Path | None) -> tuple[set[str], list[str]]:
    """Return (keyed tracked paths, human notes)."""
    selected = {path for path in files if path in COMMON_FILES or is_cargo_input(path)}
    notes: list[str] = []
    if mode == "tree":
        return set(files), ["whole tree"]
    if mode == "rust-workspace":
        selected |= {path for path in files if is_workspace_input(path)}
        return selected, ["rust workspace inputs"]
    if mode != "rust-fast":
        raise KeyError_(f"unknown mode {mode}")
    if metadata is None or packages is None:
        raise KeyError_("rust-fast needs --metadata and --packages")
    rows = read_packages(packages)
    directories, external = closure_directories(root, json.loads(metadata.read_text(encoding="utf-8")), [name for name, _ in rows])
    notes.append(f"closure directories={len(directories)}")
    if external or escapes_crate(root, files, directories):
        notes.append("non-hermetic closure: whole tree minus unread prefixes")
        selected |= {path for path in files if not path.startswith(UNREAD_PREFIXES)}
    else:
        selected |= {path for path in files if any(under(path, directory) for directory in directories)}
    return selected, notes


def compute_key(job: str, mode: str, root: Path, extras: list[str], metadata: Path | None = None, packages: Path | None = None) -> tuple[str, list[str]]:
    label = re.sub(r"[^A-Za-z0-9._-]+", "-", job).strip("-")
    if not label:
        raise KeyError_("empty job label")
    files = tracked(root)
    for path in COMMON_FILES:
        if path not in files:
            raise KeyError_(f"{path} is not tracked")
    keyed, notes = select_paths(mode, root, files, metadata, packages)
    digest = hashlib.sha256()
    digest.update(DOMAIN.encode() + b"\n")
    digest.update(f"job {label}\nmode {mode}\n".encode())
    for extra in sorted(extras):
        digest.update(f"extra {extra}\n".encode())
    if packages is not None:
        digest.update(b"packages\n" + packages.read_bytes() + b"\n")
    for path in sorted(keyed):
        mode_bits, blob, stage = files[path]
        digest.update(f"{mode_bits} {blob} {stage} {path}\n".encode("utf-8", "surrogateescape"))
    notes.append(f"keyed files={len(keyed)}/{len(files)}")
    return f"{KEY_VERSION}-{label}-{digest.hexdigest()}", notes


def write_output(name: str, value: str) -> None:
    target = os.environ.get("GITHUB_OUTPUT")
    if target:
        with open(target, "a", encoding="utf-8") as handle:
            handle.write(f"{name}={value}\n")


def command_key(args: argparse.Namespace) -> int:
    try:
        key, notes = compute_key(
            args.job, args.mode, Path(args.root), args.extra,
            Path(args.metadata) if args.metadata else None,
            Path(args.packages) if args.packages else None,
        )
    except (KeyError_, OSError, ValueError, KeyError) as error:
        print(f"::warning title=job-result cache disabled::{type(error).__name__}: {error}")
        write_output("key", "")
        return 0
    print(f"job-result key {key} ({'; '.join(notes)})")
    write_output("key", key)
    return 0


def command_marker(args: argparse.Namespace) -> int:
    directory = Path(args.dir)
    directory.mkdir(parents=True, exist_ok=True)
    server = os.environ.get("GITHUB_SERVER_URL", "")
    repository = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    marker = {
        "version": 1,
        "key": args.key,
        "run_id": run_id,
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "run_url": f"{server}/{repository}/actions/runs/{run_id}",
        "sha": os.environ.get("GITHUB_SHA", ""),
    }
    (directory / MARKER_NAME).write_text(json.dumps(marker, sort_keys=True) + "\n", encoding="utf-8")
    return 0


def command_verify(args: argparse.Namespace) -> int:
    hit = False
    if args.key:
        try:
            marker = json.loads((Path(args.dir) / MARKER_NAME).read_text(encoding="utf-8"))
            hit = marker.get("version") == 1 and marker.get("key") == args.key
        except (OSError, ValueError, AttributeError):
            hit = False
        if hit:
            print(f"::notice title=job-result cache hit::{args.key} succeeded earlier in {marker.get('run_url')} (sha {marker.get('sha')}); steps skipped")
        else:
            print("job-result cache miss: running the job")
    else:
        print("job-result cache disabled for this run: running the job")
    write_output("hit", "true" if hit else "false")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    key = sub.add_parser("key")
    key.add_argument("--job", required=True)
    key.add_argument("--mode", required=True, choices=("rust-fast", "rust-workspace", "tree"))
    key.add_argument("--root", default=".")
    key.add_argument("--metadata")
    key.add_argument("--packages")
    key.add_argument("--extra", action="append", default=[])
    key.set_defaults(handler=command_key)
    marker = sub.add_parser("marker")
    marker.add_argument("--key", required=True)
    marker.add_argument("--dir", required=True)
    marker.set_defaults(handler=command_marker)
    verify = sub.add_parser("verify")
    verify.add_argument("--key", default="")
    verify.add_argument("--dir", required=True)
    verify.set_defaults(handler=command_verify)
    args = parser.parse_args(argv)
    return args.handler(args)


if __name__ == "__main__":
    sys.exit(main())
