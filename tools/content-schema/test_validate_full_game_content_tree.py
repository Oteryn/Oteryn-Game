#!/usr/bin/env python3
"""Focused checks for Full Game Content & Ruleset Tree v1."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = ROOT / "tools/content-schema/validate_full_game_content_tree.py"
EVIDENCE_PATH = ROOT / "docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json"

spec = importlib.util.spec_from_file_location("full_content_tree_validator", VALIDATOR_PATH)
if spec is None or spec.loader is None:
    raise SystemExit("validator import failed")
validator = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validator)


def expect_failure(value: dict, expected: str) -> None:
    try:
        validator.validate(value)
    except validator.ValidationError as exc:
        if expected not in str(exc):
            raise AssertionError(f"expected {expected}, got {exc}") from exc
    else:
        raise AssertionError(f"expected validation failure containing {expected}")


def main() -> int:
    value = json.loads(EVIDENCE_PATH.read_text(encoding="utf-8"))
    validator.validate(value)

    missing_domain = copy.deepcopy(value)
    missing_domain["protected_domain_groups"].remove("economy")
    expect_failure(missing_domain, "DOMAIN_SET_MISMATCH")

    missing_system = copy.deepcopy(value)
    missing_system["selected_system_surfaces"].remove("Prey")
    expect_failure(missing_system, "SYSTEM_SET_MISMATCH")

    store_leak = copy.deepcopy(value)
    for assignment in store_leak["system_assignments"]:
        if assignment["surface"] == "Store":
            assignment["static_path"] = "content/store/"
    expect_failure(store_leak, "STORE_GAME_PATH_LEAK")

    duplicate_path = copy.deepcopy(value)
    duplicate_path["target_tree_nodes"].append(copy.deepcopy(duplicate_path["target_tree_nodes"][0]))
    expect_failure(duplicate_path, "DUPLICATE_TREE_PATH")

    missing_state = copy.deepcopy(value)
    missing_state["durable_state_ownership"] = [
        row for row in missing_state["durable_state_ownership"] if row["scope"] != "ItemInstance"
    ]
    expect_failure(missing_state, "STATE_SCOPE_COVERAGE")

    unassigned = copy.deepcopy(value)
    unassigned["summary"]["unassigned_systems"] = 1
    expect_failure(unassigned, "UNASSIGNED_SYSTEMS_NONZERO")

    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "tree.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        validator.validate(validator.load(path))

    print("PASS positive=2 negative=6")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
