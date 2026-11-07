#!/usr/bin/env python3
"""Print the `cargo test` target arguments for one Merge Queue integration-test shard.

The Merge Queue `rust_linux` matrix runs the workspace integration-test targets in two legs.
`integration-heavy` takes the named long-running targets; `integration-rest` takes every other
workspace integration-test target that `cargo test --workspace` runs (kind `test` with
`test = true`), read from `cargo metadata`. The two shards are disjoint and their union is that
whole set by construction, so a new test target always lands in a shard. The planner fails
closed when a heavy name is not such a target, when a shard would be empty, or when an example
or bench target opts into `cargo test` (no shard runs those).

Usage: plan_rust_test_shards.py <cargo-metadata.json> <shard>
Prints one argument per line (`--test`, `<name>`, ...).
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Integration targets that take most of the run time; measured on Merge Queue run 38154056379.
HEAVY_TARGETS = (
    "content_npc_catalogue",
    "content_npc_service_model",
    "content_reference_artifact",
    "content_world_cw2_b1_import",
    "content_world_project_repository",
    "current_spell_sources",
    "world_bundle_artifact_boot",
)
SHARDS = ("integration-heavy", "integration-rest")


class PlanError(Exception):
    pass


def tested_integration_targets(metadata: dict) -> set[str]:
    members = set(metadata.get("workspace_members") or [])
    if not members:
        raise PlanError("cargo metadata lists no workspace members")
    names: set[str] = set()
    for package in metadata.get("packages") or []:
        if package.get("id") not in members:
            continue
        for target in package.get("targets") or []:
            kinds = target.get("kind") or []
            if not isinstance(target.get("test"), bool):
                raise PlanError(f"target {target.get('name')!r} has no boolean test flag")
            if kinds == ["test"] and target["test"]:
                names.add(target["name"])
            elif target["test"] and kinds and kinds[0] in ("example", "bench"):
                raise PlanError(f"{kinds[0]} target {target['name']!r} sets test = true; no shard runs it")
    return names


def plan(metadata: dict, shard: str) -> list[str]:
    if shard not in SHARDS:
        raise PlanError(f"unknown shard {shard!r}; expected one of {', '.join(SHARDS)}")
    targets = tested_integration_targets(metadata)
    missing = sorted(set(HEAVY_TARGETS) - targets)
    if missing:
        raise PlanError(f"heavy targets are not tested workspace integration targets: {', '.join(missing)}")
    selected = sorted(HEAVY_TARGETS) if shard == "integration-heavy" else sorted(targets - set(HEAVY_TARGETS))
    if not selected:
        raise PlanError(f"shard {shard} selects no targets")
    return [argument for name in selected for argument in ("--test", name)]


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: plan_rust_test_shards.py <cargo-metadata.json> <shard>", file=sys.stderr)
        return 2
    try:
        metadata = json.loads(Path(argv[1]).read_text(encoding="utf-8"))
        arguments = plan(metadata, argv[2])
    except (PlanError, OSError, ValueError) as error:
        print(f"rust test shard plan: FAIL: {error}", file=sys.stderr)
        return 1
    print("\n".join(arguments))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
