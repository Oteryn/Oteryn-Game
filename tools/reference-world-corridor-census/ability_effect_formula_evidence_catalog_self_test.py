#!/usr/bin/env python3
from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import sys

sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
GAME_ROOT = HERE.parent.parent
MODULE_PATH = HERE / "ability_effect_formula_evidence_catalog.py"
EVIDENCE_PATH = (
    GAME_ROOT
    / "docs/agents/evidence/OTV2-20260921-content-world-cw2-ability-family-source-catalogue.json"
)

spec = importlib.util.spec_from_file_location("cw2_ability_source_catalog", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def test_tracked_product_is_exact(source_repo: Path) -> None:
    value = catalog.build_catalog(source_repo, GAME_ROOT)
    assert catalog.canonical_bytes(value) == EVIDENCE_PATH.read_bytes()
    assert value["counts"] == {
        "player_spells": 199,
        "player_spell_subfamilies": {
            "attack": 69,
            "conjuring": 49,
            "familiar": 5,
            "healing": 28,
            "house": 4,
            "party": 5,
            "support": 39,
        },
        "runes": 36,
        "monster_spells": 590,
        "abilities_total": 825,
        "source_catalogued": 825,
        "source_remaining": 0,
        "historical_b4_overlay_joined": 2,
    }
    assert value["loss_report"]["silently_dropped_records"] == 0


def test_repeat_and_input_order_permutation_are_deterministic(source_repo: Path) -> None:
    enumeration = catalog.enumerate_source_paths(source_repo)
    first = catalog.build_catalog(source_repo, GAME_ROOT)
    second = catalog.build_catalog(source_repo, GAME_ROOT)
    reversed_input = catalog.build_catalog(
        source_repo,
        GAME_ROOT,
        source_paths=reversed(enumeration["selected"]),
    )
    assert catalog.canonical_bytes(first) == catalog.canonical_bytes(second)
    assert catalog.canonical_bytes(first) == catalog.canonical_bytes(reversed_input)


def test_duplicate_and_incomplete_selection_fail_closed(source_repo: Path) -> None:
    enumeration = catalog.enumerate_source_paths(source_repo)
    selected = list(enumeration["selected"])
    try:
        catalog.validate_selected_source_paths(selected, selected + [selected[0]])
    except catalog.CatalogError as exc:
        assert "DUPLICATE_SELECTED_SOURCE_PATH" in str(exc)
    else:
        raise AssertionError("duplicate source path was accepted")

    try:
        catalog.validate_selected_source_paths(selected, selected[:-1])
    except catalog.CatalogError as exc:
        assert "SOURCE_SELECTION_SET_MISMATCH" in str(exc)
    else:
        raise AssertionError("incomplete source selection was accepted")


def test_pinned_source_blob_drift_fails_closed(source_repo: Path) -> None:
    try:
        catalog._source_object(
            source_repo,
            "data/scripts/spells/attack/ice_strike.lua",
            expected_blob="0" * 40,
        )
    except catalog.CatalogError as exc:
        assert "SOURCE_BLOB_MISMATCH" in str(exc)
    else:
        raise AssertionError("source blob drift was accepted")


def test_exclusions_are_exact(source_repo: Path) -> None:
    enumeration = catalog.enumerate_source_paths(source_repo)
    exclusions = {row["path"]: row for row in enumeration["exclusions"]}
    assert set(exclusions) == {
        catalog.EXAMPLE_PATH,
        catalog.MONSTER_HELPER_PATH,
    }
    selected = set(enumeration["selected"])
    assert catalog.EXAMPLE_PATH not in selected
    assert catalog.MONSTER_HELPER_PATH not in selected
    for path, (expected_blob, reason) in catalog.EXPECTED_EXCLUSIONS.items():
        assert exclusions[path]["blob"] == expected_blob
        assert exclusions[path]["reason"] == reason
        assert exclusions[path]["byte_size"] > 0


def test_historical_b4_overlay_is_exact_and_nonpromoting(source_repo: Path) -> None:
    value = catalog.build_catalog(source_repo, GAME_ROOT)
    assert value["historical_b4_overlay"]["join_count"] == 2
    overlays = [
        row["historical_b4_overlay"]
        for row in value["source_records"]
        if "historical_b4_overlay" in row
    ]
    assert len(overlays) == 2
    assert {row["source_candidate_id"] for row in overlays} == set(
        catalog.OVERLAY_SOURCES
    )
    encoded = catalog.canonical_bytes(value).decode("utf-8")
    assert "oteryn:ability." not in encoded
    for overlay in overlays:
        assert overlay["target_evidence"] == "UNKNOWN"
        assert overlay["source_provenance"] == "PENDING"
        assert overlay["legal_review"] == "PENDING"
        assert overlay["parity"] == "PARITY_PENDING_EVIDENCE"
        assert overlay["native_ability_identity"]["content_key"] is None
        assert overlay["effect_to_formula"]["quantitative_formula"] is None
        assert overlay["executable_promotion"]["disposition"] == "BLOCKED"


def test_mapper_revision_binds_current_tool() -> None:
    recorded = catalog.verify_mapper_revision(GAME_ROOT)
    mapper_payload = (GAME_ROOT / catalog.MAPPER_PATH).read_bytes()
    canonical = catalog.canonical_repository_text_bytes(mapper_payload)
    assert recorded["canonical_size"] == len(canonical)
    assert recorded["canonical_sha256"] == catalog.sha256_bytes(canonical)
    assert recorded["git_blob"] == catalog._git(
        GAME_ROOT, "rev-parse", f"HEAD:{catalog.MAPPER_PATH}"
    )


def test_historical_mapper_is_pinned_not_rewritten() -> None:
    historical = catalog.verify_historical_b4_overlay(GAME_ROOT)
    mapper = historical["historical_mapper"]
    assert mapper["blob"] == catalog.HISTORICAL_B4_MAPPER_BLOB
    assert mapper["canonical_size"] == catalog.HISTORICAL_B4_MAPPER_CANONICAL_SIZE
    assert mapper["canonical_sha256"] == catalog.HISTORICAL_B4_MAPPER_CANONICAL_SHA256


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-repo", type=Path, required=True)
    args = parser.parse_args()
    source_repo = args.source_repo.resolve()

    test_tracked_product_is_exact(source_repo)
    test_repeat_and_input_order_permutation_are_deterministic(source_repo)
    test_duplicate_and_incomplete_selection_fail_closed(source_repo)
    test_pinned_source_blob_drift_fails_closed(source_repo)
    test_exclusions_are_exact(source_repo)
    test_historical_b4_overlay_is_exact_and_nonpromoting(source_repo)
    test_mapper_revision_binds_current_tool()
    test_historical_mapper_is_pinned_not_rewritten()
    print("ability-source-catalogue self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
