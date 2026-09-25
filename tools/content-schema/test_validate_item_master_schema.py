#!/usr/bin/env python3
"""Focused negative/positive checks for the Item master schema census validator."""

from __future__ import annotations

import copy
import importlib.util
import json
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
VALIDATOR_PATH = ROOT / "tools/content-schema/validate_item_master_schema.py"
EVIDENCE_PATH = ROOT / "docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json"

spec = importlib.util.spec_from_file_location("item_master_validator", VALIDATOR_PATH)
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

    missing_field = copy.deepcopy(value)
    missing_field["field_mappings"] = [
        row for row in missing_field["field_mappings"] if row["source_parameter"] != "sounds"
    ]
    expect_failure(missing_field, "MISSING_TEMPLATE_PARAMETERS:sounds")

    unassigned = copy.deepcopy(value)
    unassigned["summary"]["unassigned_fields"] = 1
    expect_failure(unassigned, "UNASSIGNED_FIELDS_NONZERO")

    missing_family = copy.deepcopy(value)
    missing_family["navigation_families"].remove("Fontes de Luz")
    expect_failure(missing_family, "MISSING_NAV_FAMILIES:Fontes de Luz")

    duplicate_assignment = copy.deepcopy(value)
    duplicate_assignment["family_profiles"][0]["navigation_families"].append("Fontes de Luz")
    expect_failure(duplicate_assignment, "FAMILY_ASSIGNED_TWICE:Fontes de Luz")

    missing_light = copy.deepcopy(value)
    missing_light["attrib_promotion_candidates"] = [
        row for row in missing_light["attrib_promotion_candidates"] if row.get("concept") != "light emission"
    ]
    expect_failure(missing_light, "ATTRIB_PROMOTION_CONCEPT_MISSING:light emission")

    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "census.json"
        path.write_text(json.dumps(value), encoding="utf-8")
        validator.validate(validator.load(path))

    print("PASS positive=2 negative=5")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
