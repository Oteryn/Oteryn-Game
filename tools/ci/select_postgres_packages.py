#!/usr/bin/env python3
"""Select changed workspace packages and reverse dependents; uncertainty runs all."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess


DISPATCH = "crates/gameplay/dispatch"
TRANSPORT = f"{DISPATCH}/src/gameplay_transport"
POSTGRES_TESTS = f"{TRANSPORT}/postgres_tests"
SUPPORT = f"{POSTGRES_TESTS}/support"
DISPATCH_PACKAGE = "oteryn-gameplay-dispatch"
FIXTURE_PATH = re.compile(re.escape(SUPPORT) + r"/[a-z][a-z0-9_]*_fixtures\.rs")
MOUNT_FILES = {
    f"{DISPATCH}/build.rs": '''build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests/support",
        "postgres_test_support_mounts.rs",
        "",
    )?;''',
    f"{TRANSPORT}/monk_save.rs": '''#[cfg(test)]
#[path = "monk_save_postgres_tests.rs"]
pub(super) mod postgres_tests;''',
    f"{TRANSPORT}/monk_save_postgres_tests.rs":
        'include!(concat!(env!("OUT_DIR"), "/postgres_test_support_mounts.rs"));',
}


def command(*args):
    return subprocess.check_output(args, text=True).strip()


def git_changes(base, head):
    raw = subprocess.check_output([
        "git", "diff", "--raw", "--no-ext-diff", "--no-textconv", "--no-renames",
        "-z", base, head, "--",
    ]).decode()
    fields = raw.split("\0")
    if fields[-1] or len(fields) % 2 != 1:
        raise ValueError("incomplete Git diff")
    changes = {}
    for header, path in zip(fields[::2], fields[1::2]):
        old_mode, new_mode, _, _, status = header.removeprefix(":").split()
        if not header.startswith(":") or not path or path in changes:
            raise ValueError("unexpected Git diff entry")
        changes[path] = (old_mode, new_mode, status)
    return changes


def external_fixture_consumer(head, paths):
    for path, witness in MOUNT_FILES.items():
        if command("git", "show", f"{head}:{path}").count(witness) != 1:
            return f"unverified Dispatch test mount: {path}"

    # Check exact candidate source for other mounts of the fixtures, their
    # directory or the generated module. Cargo dependencies alone miss source
    # shared through attributes, includes, and build or workflow scripts.
    patterns = (
        "postgres_tests/support", 'postgres_tests"', "postgres_tests'", "postgres_tests/*",
        "postgres_test_support_mounts.rs", "monk_save_postgres_tests.rs",
        *(Path(path).stem for path in paths),
    )
    args = ["git", "grep", "-I", "-l", "-z", "-F"]
    for pattern in patterns:
        args.extend(("-e", pattern))
    # qlane-meta JSON files are historical reports of Rust symbol locations.
    args.extend((head, "--", "*.rs", "*.toml", "*.py", "*.sh", "*.bash", "*.yml", "*.yaml",
                 "*.json", "*.nix", "Makefile", "Dockerfile", "Dockerfile.*",
                 ":(exclude,glob)qlane-meta/**/*.json"))
    scan = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if scan.returncode not in (0, 1):
        return "Dispatch fixture consumer scan failed"

    recognized = set(MOUNT_FILES) | {
        f"{DISPATCH}/build_pg_shards.rs",
        "tools/architecture-check/src/logging.rs",
        "tools/ci/select_postgres_packages.py",
        "tools/repository/check_dispatch_pg_shards.py",
        "tools/repository/test_check_dispatch_pg_shards.py",
        "tools/repository/test_select_postgres_packages.py",
    }
    for record in scan.stdout.decode().split("\0"):
        if not record:
            continue
        prefix, _, path = record.partition(":")
        if prefix != head or not path:
            return "unverified Dispatch fixture consumer scan output"
        if path in recognized or path.startswith(POSTGRES_TESTS + "/"):
            continue
        if path == f"{TRANSPORT}/bundle_gates_tests.rs" and command(
            "git", "show", f"{head}:{TRANSPORT}/mod.rs"
        ).count("#[cfg(test)]\nmod bundle_gates_tests;") == 1:
            continue
        return f"other Dispatch fixture consumer: {path}"
    return None


def select(base, head):
    root = Path(command("git", "rev-parse", "--show-toplevel"))
    metadata = json.loads(command(
        "cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"
    ))
    members = set(metadata["workspace_members"])
    packages = [p for p in metadata["packages"] if p["id"] in members]
    owners = {Path(p["manifest_path"]).parent.resolve(): p["name"] for p in packages}
    if not packages or len(owners) != len(members):
        raise ValueError("incomplete workspace metadata")
    reverse = {p["name"]: set() for p in packages}
    # no-deps retains dependency declarations, including optional, build, dev,
    # renamed and target-specific path dependencies. Include all of them.
    for package in packages:
        for dependency in package["dependencies"]:
            if dependency.get("path"):
                owner = owners.get(Path(dependency["path"]).resolve())
                if owner:
                    reverse[owner].add(package["name"])
    candidate = command("git", "rev-parse", "--verify", f"{head}^{{commit}}")
    merge_base = command("git", "merge-base", base, candidate)
    changes = git_changes(merge_base, candidate)
    if changes and all(FIXTURE_PATH.fullmatch(path) for path in changes):
        if (owners.get(root / DISPATCH) != DISPATCH_PACKAGE
                or any(change != ("100644", "100644", "M") for change in changes.values())):
            return None, "unverified Dispatch fixture paths or modes"
        for package in packages:
            for target in package["targets"]:
                source = Path(target["src_path"])
                if source.is_relative_to(root / POSTGRES_TESTS):
                    return None, f"independent Cargo target in Dispatch fixtures: {target['src_path']}"
        if issue := external_fixture_consumer(candidate, changes):
            return None, issue
        return [DISPATCH_PACKAGE], f"Dispatch cfg(test) fixtures only at merge base {merge_base}"
    selected = set()
    for name in changes:
        path = Path(name)
        if (name in {"Cargo.toml", "Cargo.lock", ".github/workflows/pr-postgres-harness.yml"}
                or name.startswith(("rust-toolchain", ".cargo/", "vendor/", "tools/ci/"))
                or "migrations" in path.parts):
            return None, f"build input changed: {name}"
        matches = [(len(directory.parts), owner) for directory, owner in owners.items()
                   if (root / path).is_relative_to(directory)]
        if matches:
            selected.add(max(matches)[1])
        elif path.suffix == ".md" and (name.startswith("docs/") or len(path.parts) == 1):
            continue
        else:
            return None, f"unowned build input: {name}"
    pending = list(selected)
    while pending:
        for dependent in reverse[pending.pop()] - selected:
            selected.add(dependent)
            pending.append(dependent)
    return sorted(selected), f"merge base {merge_base}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/main")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--event", default=os.environ.get("GITHUB_EVENT_NAME", ""))
    args = parser.parse_args()
    packages, reason = None, "event is not pull_request"
    if args.event == "pull_request":
        try:
            packages, reason = select(args.base, args.head)
        except Exception as error:
            reason = f"selection failed: {error}"
    result = {"mode": "full" if packages is None else "selected",
              "packages": packages, "reason": reason}
    print(json.dumps(result, sort_keys=True))
    pg_owners = {"oteryn-game-server", "oteryn-gameplay-dispatch", "oteryn-gameplay-durability"}
    libraries = sorted(pg_owners if packages is None else pg_owners.intersection(packages))
    if output := os.environ.get("GITHUB_OUTPUT"):
        with open(output, "a") as stream:
            stream.write(f"packages={json.dumps(packages)}\n")
            stream.write(f"lib_packages={json.dumps(libraries)}\n")
            stream.write(f"empty={'false' if libraries else 'true'}\n")
    if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(summary, "a") as stream:
            stream.write("### PostgreSQL package selection\n\n")
            stream.write("All workspace packages" if packages is None else
                         ", ".join(f"`{p}`" for p in packages) or "Empty (docs only)")
            stream.write(f"\n\nReason: {reason}\n")


if __name__ == "__main__":
    main()
