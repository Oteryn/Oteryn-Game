#!/usr/bin/env python3
"""Select changed workspace packages and reverse dependents; uncertainty runs all."""

import argparse
import json
import os
from pathlib import Path
import subprocess


def command(*args):
    return subprocess.check_output(args, text=True).strip()


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
    merge_base = command("git", "merge-base", base, head)
    paths = subprocess.check_output([
        "git", "diff", "--name-only", "--no-renames", "-z", f"{merge_base}...{head}"
    ]).decode().split("\0")
    selected = set()
    for name in filter(None, paths):
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
