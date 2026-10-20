#!/usr/bin/env python3
"""Keep full PostgreSQL/SIM qualification in MQ and only fast jobs on PRs.

The legacy entry-point name is retained for repository callers. MQ's existing
reviewed blob pin remains the qualification authority; this does not invent a
second list of PostgreSQL targets or simulation test commands.
"""
from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"
MERGE_GROUP_GATE = ROOT / ".github/workflows/merge-group-gate.yml"
CORE = Path(__file__).with_name("validate_repository_policy_core.py")
FAST_PR_JOBS = [
    "scope", "lanes", "governance", "dependency_review", "routing_contract",
    "world_bundle", "rust_policy", "rust_fast", "rust_supply_chain", "validate", "game_gate",
]


def validate() -> list[str]:
    spec = importlib.util.spec_from_file_location("pr_qualification_core", CORE)
    if spec is None or spec.loader is None:
        return ["unable to resolve canonical qualification policy"]
    core = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(core)
    if not MERGE_GATE.is_file() or not MERGE_GROUP_GATE.is_file():
        return ["missing canonical PR or Merge Queue gate"]
    pr = MERGE_GATE.read_text(encoding="utf-8")
    mq = MERGE_GROUP_GATE.read_text(encoding="utf-8")
    errors = []
    jobs = core.yaml_mapping_keys_at_indent(core.top_level_yaml_mapping_block(pr, "jobs") or "", 2)
    if jobs != FAST_PR_JOBS:
        errors.append("PR gate must contain only canonical fast jobs; full PG/SIM belongs to MQ")
    if "full_ci" in pr or "full-ci" in pr:
        errors.append("PR labels must not select full qualification")
    if core.git_blob_sha(mq.encode("utf-8")) != core.EXPECTED_MERGE_GROUP_GATE_BLOB:
        errors.append("MQ must retain the reviewed exact-candidate qualification contract")
    return errors


def main() -> int:
    errors = validate()
    if errors:
        for error in errors:
            print(f"- {error}")
        return 1
    print("PR fast-only / MQ PostgreSQL and simulation qualification contracts passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
