#!/usr/bin/env python3
from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
GAME_ROOT = HERE.parent.parent
MODULE_PATH = HERE / "ability_effect_formula_evidence_catalog.py"
EVIDENCE_PATH = (
    GAME_ROOT
    / "docs/agents/evidence/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json"
)

spec = importlib.util.spec_from_file_location("cw2_b4_catalog", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def test_protected_inputs_and_tracked_product_are_exact() -> None:
    value = catalog.build_catalog(GAME_ROOT)
    assert catalog.canonical_bytes(value) == EVIDENCE_PATH.read_bytes()
    assert value["counts"] == {
        "abilities": 2,
        "reference_cases": 4,
        "candidate_effect_bindings": 2,
        "resolved_native_ability_identities": 0,
        "resolved_exact_quantitative_formulas": 0,
        "executable_promotions": 0,
    }


def test_repeat_and_input_permutation_are_byte_deterministic() -> None:
    first = catalog.build_catalog(GAME_ROOT)
    second = catalog.build_catalog(GAME_ROOT)
    reversed_cases = catalog.build_catalog(GAME_ROOT, case_ids=reversed(catalog.CASE_IDS))
    assert catalog.canonical_bytes(first) == catalog.canonical_bytes(second)
    assert catalog.canonical_bytes(first) == catalog.canonical_bytes(reversed_cases)


def test_protected_blob_mismatch_fails_closed() -> None:
    altered = dict(catalog.PROTECTED_INPUTS)
    _, role = altered[catalog.MANIFEST_PATH]
    altered[catalog.MANIFEST_PATH] = ("0" * 40, role)
    try:
        catalog.build_catalog(GAME_ROOT, expected_inputs=altered)
    except catalog.CatalogError as exc:
        assert "GIT_BLOB_MISMATCH" in str(exc)
    else:
        raise AssertionError("protected input blob mismatch was accepted")


def test_protected_digest_mismatch_fails_closed() -> None:
    try:
        catalog.build_catalog(
            GAME_ROOT,
            expected_sha256={catalog.MANIFEST_PATH: "0" * 64},
        )
    except catalog.CatalogError as exc:
        assert "SHA256_MISMATCH" in str(exc)
    else:
        raise AssertionError("protected input digest mismatch was accepted")


def test_manifest_promotion_or_formula_invention_fails_closed() -> None:
    manifest = json.loads(
        (
            GAME_ROOT
            / "docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json"
        ).read_text(encoding="utf-8")
    )
    promoted = copy.deepcopy(manifest)
    promoted["cases"][0]["target"]["evidence_class"] = "PROVEN"
    try:
        catalog.validate_manifest_cases(promoted)
    except catalog.CatalogError as exc:
        assert "FAIL_CLOSED_STATE_MISMATCH" in str(exc)
    else:
        raise AssertionError("promoted Reference target case was accepted")


def test_no_native_identity_parity_or_formula_is_synthesized() -> None:
    value = catalog.build_catalog(GAME_ROOT)
    encoded = catalog.canonical_bytes(value).decode("utf-8")
    assert "oteryn:ability." not in encoded
    assert '"quantitative_formula":null' in encoded
    for ability in value["ability_effect_formula_candidates"]:
        assert ability["target_evidence"] == "UNKNOWN"
        assert ability["source_provenance"] == "PENDING"
        assert ability["legal_review"] == "PENDING"
        assert ability["parity"] == "PARITY_PENDING_EVIDENCE"
        assert ability["native_ability_identity"]["disposition"] == "UNRESOLVED"
        assert ability["effect_to_formula"]["formula_state"] == "UNKNOWN"
        assert ability["executable_promotion"]["disposition"] == "BLOCKED"


def main() -> int:
    test_protected_inputs_and_tracked_product_are_exact()
    test_repeat_and_input_permutation_are_byte_deterministic()
    test_protected_blob_mismatch_fails_closed()
    test_protected_digest_mismatch_fails_closed()
    test_manifest_promotion_or_formula_invention_fails_closed()
    test_no_native_identity_parity_or_formula_is_synthesized()
    print("ability-effect-formula-evidence-catalog self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
