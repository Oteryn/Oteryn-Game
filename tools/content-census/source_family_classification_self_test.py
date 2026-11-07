#!/usr/bin/env python3
"""Focused synthetic boundary tests for the G3 direct-signature classifier."""
from __future__ import annotations

import copy
import json
from pathlib import Path

from source_family_classification import (
    ARTIFACT,
    EXPECTED_COUNTS,
    EXPECTED_UNIVERSE_SHA256,
    classify_page,
    classify_universe,
    validate_pinned_universe,
)


def g1(**overrides):
    row = {
        "lane": "G1_LIVE_NON_ITEM",
        "artifact_id": "10798668295",
        "observed_title": "Synthetic",
        "source_shape": "STRUCTURED_PRIMARY",
        "redirect": False,
        "discovery_roots": ["creatures"],
        "source_surfaces": ["Stworzenia"],
        "templates": ["Predefinição:Infobox Criatura"],
        "categories": ["Categoria:Criaturas"],
        "candidate_families_evidence_only": ["Creature", "Encounter"],
        "family_classification_state_evidence_only": "MULTI_FAMILY_RELATION",
    }
    row.update(overrides)
    return row


def item(**overrides):
    row = {
        "lane": "PROTECTED_ITEM",
        "crosswalk_artifact_id": "10778892407",
        "observed_title": "Synthetic item",
        "source_shape": "INFOBOX_ITEM",
        "row_role": "PAGE_ID_AND_PROVENANCE_EVIDENCE_ONLY",
    }
    row.update(overrides)
    return row


def page(page_id, title, *provenance):
    return {
        "page_id": page_id,
        "title_observations": [{"lane": "G1_LIVE_NON_ITEM", "title": title}],
        "cross_lane_title_divergence": False,
        "provenance": list(provenance),
    }


def assert_unknown(record, basis="NO_STRICT_DIRECT_SIGNATURE"):
    result = classify_page(record)
    assert result["primary_definition_family"] == "UNKNOWN", result
    assert result["state"] == "UNKNOWN", result
    assert result["assignment_rule"] == basis, result
    return result


def test_mount_list_world_quest_stays_unknown():
    winterlight = page(45642, "Winterlight Solstice", {
        **g1(
            observed_title="Winterlight Solstice",
            discovery_roots=["mounts", "world-quests"],
            source_surfaces=["Mocowania", "Zadania światowe"],
            templates=["Predefinição:Infobox Mount", "Predefinição:Infobox Mount/List", "Predefinição:Infobox Mount/List/Header", "Predefinição:Infobox World Quest"],
            categories=["Categoria:World Quests", "Categoria:Spoilers de World Quests"],
            candidate_families_evidence_only=["Encounter", "Interaction", "Mount", "Quest"],
        )
    })
    assert_unknown(winterlight)


def test_infobox_without_root_or_surface_is_unknown():
    record = page(1, "Creature template outside source root", g1(discovery_roots=[], source_surfaces=[]))
    assert_unknown(record)


def test_root_and_surface_without_base_infobox_is_unknown():
    record = page(2, "Creature root without infobox", g1(templates=["Predefinição:Infobox Criatura/CélulaDano"]))
    assert_unknown(record)


def test_item_primary_keeps_g1_relationship_overlay_on_same_id():
    record = {
        "page_id": 6457,
        "title_observations": [
            {"lane": "G1_LIVE_NON_ITEM", "title": "Same page"},
            {"lane": "PROTECTED_ITEM", "title": "Same page"},
        ],
        "cross_lane_title_divergence": False,
        "provenance": [
            item(observed_title="Same page"),
            g1(
                observed_title="Same page",
                candidate_families_evidence_only=["Encounter"],
                discovery_roots=["tibiadrome"],
                source_surfaces=["Tibiadrom"],
                templates=["Predefinição:Infobox Tibiadrome"],
                categories=["Categoria:Tibiadrom"],
            ),
        ],
    }
    result = classify_page(record)
    assert result["primary_definition_family"] == "Item", result
    assert result["basis_signature_ids"] == ["PROTECTED_ITEM_INFOBOX_ITEM"], result
    evidence = result["candidate_relationships_evidence_only"]
    assert len(evidence) == 1 and evidence[0]["candidate_families_evidence_only"] == ["Encounter"], result
    assert result["candidate_relations_resolved"] is False


def test_conflicting_direct_families_fail_closed():
    creature = g1()
    item_row = item()
    # A source row cannot inherit the same lane's signature from candidate lists.
    record = page(3, "Conflicting direct assertions", creature, item_row)
    result = classify_page(record)
    assert result["primary_definition_family"] == "UNKNOWN", result
    assert result["assignment_rule"] == "MULTIPLE_STRICT_DIRECT_SIGNATURES", result


def test_same_title_distinct_page_ids_are_distinct():
    first = page(7, "Duplicate title", g1())
    second = page(8, "Duplicate title", g1())
    assert first["page_id"] != second["page_id"]
    assert classify_page(first)["primary_definition_family"] == "Creature"
    assert classify_page(second)["primary_definition_family"] == "Creature"


def test_parse_error_redirect_and_no_infobox_item_stay_unknown():
    for source_shape in ("INFOBOX_ITEM_PARSE_ERROR", "NO_INFOBOX_ITEM"):
        row = item(source_shape=source_shape)
        assert_unknown(page(20 + len(source_shape), "Protected item", row))
    assert_unknown(page(29, "Protected item redirect", item(redirect=True)))
    assert_unknown(page(30, "Creature redirect", g1(redirect=True)))
    assert_unknown(page(31, "Alternate structure", g1(source_shape="STRUCTURED_ALTERNATE")))
    assert_unknown(page(32, "Unresolved shape", g1(source_shape="SOURCE_CLASSIFICATION_UNRESOLVED")))


def test_actual_mount_and_creature_signatures():
    mount = g1(
        discovery_roots=["mounts"],
        source_surfaces=["Mocowania"],
        templates=["Predefinição:Infobox Mount", "Predefinição:Infobox Mount/Template/Montarias"],
        categories=["Categoria:Montarias"],
        candidate_families_evidence_only=["Mount"],
    )
    assert classify_page(page(40, "Mount", mount))["primary_definition_family"] == "Mount"
    creature = g1(templates=["Predefinição:Infobox Criatura", "Predefinição:Infobox Item"])
    assert classify_page(page(41, "Creature with incidental item template", creature))["primary_definition_family"] == "Creature"


def test_pinned_input_corruption_fails_closed():
    universe = {
        "schema": "OTERYN_GLOBAL_SOURCE_ID_UNIVERSE/v1",
        "identity_basis": "EXACT_MEDIAWIKI_PAGE_ID",
        "authority": {
            "identity_resolution": "NOT_PERFORMED",
            "canonical_identity_selection": "NOT_PERFORMED",
            "semantic_promotion": "NOT_PERFORMED",
            "gameplay_truth": "NONE",
        },
        "counts": EXPECTED_COUNTS,
        "pages": [],
    }
    manifest = {
        "schema": "OTERYN_GLOBAL_SOURCE_OVERLAP_MANIFEST/v1",
        "global_universe_sha256": EXPECTED_UNIVERSE_SHA256,
        "counts": EXPECTED_COUNTS,
        "invariants": {"semantic_promotion_performed": False, "canonical_identity_selection_performed": False},
        "input_digests": {
            "g1_artifact_id": "10798668295", "g1_run_id": "35977349690",
            "protected_item_crosswalk_artifact_id": "10778892407", "protected_item_crosswalk_run_id": "35925860576",
            "g1_archive_sha256": "bf08a2715891d138b5db4ed4be0a69034770315e8d67f3e83867f22b9afc9863",
            "protected_item_crosswalk_archive_sha256": "834c10d3dfb24b8857ffd666046743c96ee4093e7e09ef170dcd477ffea91b43",
            "g1_head_sha": "f1d7dbd6577b033c53545d8650ffba05aafc9480",
            "protected_item_crosswalk_head_sha": "61d051a13329c51ae04d8a011655e279664c334c",
            "g1_stable_without_retrieval_timestamp_sha256": "17f72a8f63861244b3193e639c3e33a530b641b77a7539653cc066a70c093c6c",
            "protected_item_stable_digest": "389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a",
        },
    }
    try:
        validate_pinned_universe(universe, manifest, ARTIFACT["digest"].removeprefix("sha256:"))
    except ValueError as error:
        assert "G2 universe content digest mismatch" in str(error)
    else:
        raise AssertionError("corrupt G2 universe passed digest validation")


def run():
    tests = [
        test_mount_list_world_quest_stays_unknown,
        test_infobox_without_root_or_surface_is_unknown,
        test_root_and_surface_without_base_infobox_is_unknown,
        test_item_primary_keeps_g1_relationship_overlay_on_same_id,
        test_conflicting_direct_families_fail_closed,
        test_same_title_distinct_page_ids_are_distinct,
        test_parse_error_redirect_and_no_infobox_item_stay_unknown,
        test_actual_mount_and_creature_signatures,
        test_pinned_input_corruption_fails_closed,
    ]
    for test in tests:
        test()
    print(f"PASS: {len(tests)} focused G3 source-family tests")


if __name__ == "__main__":
    run()
