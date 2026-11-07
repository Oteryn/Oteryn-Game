#!/usr/bin/env python3
"""Deterministic synthetic tests for item_field_verification.py."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("item_field_verification.py")
spec = importlib.util.spec_from_file_location("item_field_verification", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("unable to load item_field_verification.py")
field = importlib.util.module_from_spec(spec)
spec.loader.exec_module(field)


def coverage() -> dict[str, str]:
    result = {
        "attack": "weapon.attack",
        "defense": "weapon.defense",
        "weight": "physical.weight",
        "duration": "temporal.duration_ms+consumption_mode",
        "modifier.criticalhitchance": "skill_modifiers.modifiers",
        "modifier.lifeleechchance": "skill_modifiers.modifiers",
    }
    for index in range(84):
        result[f"synthetic_{index:02d}"] = (
            f"EXPLICIT_UNSUPPORTED_SYNTHETIC_{index:02d}"
        )
    assert len(result) == 90
    return result


def profile(profile_id: str, observations: list[dict]) -> dict:
    return {
        "profile_id": profile_id,
        "candidate_observations": observations,
    }


def obs(native_field: str, value: object, nested: list | None = None) -> dict:
    return {
        "native_field": native_field,
        "source_key": native_field,
        "source_value": value,
        "nested_values": [] if nested is None else nested,
        "authority": "OTS_HYPOTHESIS_ONLY",
    }


def inputs() -> tuple[dict, dict, dict]:
    crosswalk = {
        "schema": field.CROSSWALK_SCHEMA,
        "source_profiles": [
            profile(
                "p1",
                [
                    obs("attack", "42"),
                    obs("weight", "100"),
                    obs("modifier.criticalhitchance", "10"),
                    obs("modifier.lifeleechchance", "20"),
                ],
            ),
            profile("p2", [obs("attack", "10"), obs("defense", "5")]),
            profile("p3", []),
        ],
        "records": [
            {
                "source_item_id": 1,
                "native_key": "oteryn:item.1",
                "source_profile_id": "p1",
                "identity_origin": "PRESERVED_SEMANTIC_BINDING",
            },
            {
                "source_item_id": 2,
                "native_key": "oteryn:item.2",
                "source_profile_id": "p2",
                "identity_origin": "OPAQUE_REGISTRY_ALLOCATION",
            },
            {
                "source_item_id": 3,
                "native_key": "oteryn:item.3",
                "source_profile_id": "p3",
                "identity_origin": "OPAQUE_REGISTRY_ALLOCATION",
            },
        ],
    }
    current = {
        "schema": field.CURRENT_SOURCE_SCHEMA,
        "collector_profile": field.CURRENT_SOURCE_PROFILE,
        "target_cut": field.TARGET_CUT,
        "source": {
            "id": field.CURRENT_SOURCE_ID,
            "role": "STRUCTURED_REFERENCE_DATA",
        },
        "authority": {"semantic_promotion": "FORBIDDEN"},
        "records": [
            {
                "source_item_id": 1,
                "native_key": "oteryn:item.1",
                "source_profile_id": "p1",
                "discovery_names": ["Synthetic Sword"],
                "current_source": {
                    "disposition": "WIKI_MATCHED",
                    "reason": "NAME_DISCOVERY_PLUS_NON_NAME_CORROBORATION",
                    "candidate_page_ids": [101],
                    "matched_non_name_signals": ["attack"],
                    "contradicted_non_name_signals": [],
                },
            },
            {
                "source_item_id": 2,
                "native_key": "oteryn:item.2",
                "source_profile_id": "p2",
                "discovery_names": ["Conflict Sword"],
                "current_source": {
                    "disposition": "WIKI_CONFLICT",
                    "reason": "STABLE_SIGNAL_CONTRADICTION",
                    "candidate_page_ids": [102],
                    "matched_non_name_signals": ["defense"],
                    "contradicted_non_name_signals": ["attack"],
                },
            },
            {
                "source_item_id": 3,
                "native_key": "oteryn:item.3",
                "source_profile_id": "p3",
                "discovery_names": ["Name Only"],
                "current_source": {
                    "disposition": "WIKI_AMBIGUOUS",
                    "reason": "NAME_ONLY_INSUFFICIENT",
                    "candidate_page_ids": [103],
                    "matched_non_name_signals": [],
                    "contradicted_non_name_signals": [],
                },
            },
        ],
        "pages": [
            {
                "page_id": 101,
                "revision_id": 1001,
                "revision_timestamp": "2026-09-22T00:00:00Z",
                "target_continuity": "UNKNOWN",
                "normalized_fields": {
                    "name": {"state": "VALUE", "value": "Synthetic Sword"},
                    "attack": {"state": "VALUE", "value": 42},
                    "stackable": {"state": "VALUE", "value": False},
                },
            },
            {
                "page_id": 102,
                "revision_id": 1002,
                "revision_timestamp": "2026-09-22T00:00:00Z",
                "target_continuity": "UNKNOWN",
                "normalized_fields": {
                    "name": {"state": "VALUE", "value": "Conflict Sword"},
                    "attack": {"state": "VALUE", "value": 11},
                    "defense": {"state": "VALUE", "value": 5},
                },
            },
            {
                "page_id": 103,
                "revision_id": 1003,
                "revision_timestamp": "2026-09-22T00:00:00Z",
                "target_continuity": "UNKNOWN",
                "normalized_fields": {
                    "name": {"state": "VALUE", "value": "Name Only"},
                    "attack": {"state": "VALUE", "value": 99},
                },
            },
        ],
    }
    schema = {"b1_candidate_field_coverage": {"destinations": coverage()}}
    return crosswalk, current, schema


def record(full: dict, native_key: str) -> dict:
    return next(
        row for row in full["records"] if row["native_key"] == native_key
    )


def main() -> int:
    # Synthetic cardinality only; production path fixes this at 38,157.
    field.TARGET_COUNT = 3
    crosswalk, current, schema = inputs()
    first = field.compile_verification(crosswalk, current, schema)
    second = field.compile_verification(
        copy.deepcopy(crosswalk),
        copy.deepcopy(current),
        copy.deepcopy(schema),
    )
    assert field.canonical_bytes(first) == field.canonical_bytes(second)

    one = record(first, "oteryn:item.1")
    assert (
        one["field_overrides"]["weapon.attack"]["field_state"]
        == "CORROBORATED_CURRENT"
    )
    assert (
        one["field_overrides"]["weapon.attack"]["continuity_to_target"]
        == "UNKNOWN"
    )
    assert one["field_overrides"]["weapon.attack"]["promotion"] == "BLOCKED"
    assert (
        one["field_overrides"]["presentation.name"]["field_state"]
        == "CORROBORATED_CURRENT"
    )
    assert one["field_overrides"]["stack.stackable"]["field_state"] == "UNKNOWN"
    assert (
        one["field_overrides"]["stack.stackable"]["observations"][0]["value"]
        is False
    )

    p1 = next(
        item
        for item in first["profile_field_rules"]
        if item["profile_id"] == "p1"
    )
    assert p1["fields"]["physical.weight"]["field_state"] == "OTS_ONLY"
    critical_path = (
        "skill_modifiers.modifiers[modifier.criticalhitchance]"
    )
    leech_path = "skill_modifiers.modifiers[modifier.lifeleechchance]"
    assert p1["fields"][critical_path]["field_state"] == "OTS_ONLY"
    assert p1["fields"][leech_path]["field_state"] == "OTS_ONLY"
    assert critical_path != leech_path

    two = record(first, "oteryn:item.2")
    assert two["field_overrides"]["weapon.attack"]["field_state"] == "CONFLICT"
    assert (
        two["field_overrides"]["weapon.attack"]["continuity_to_target"]
        == "CONFLICT"
    )
    assert (
        two["field_overrides"]["weapon.defense"]["field_state"]
        == "CORROBORATED_CURRENT"
    )
    assert (
        two["field_overrides"]["weapon.defense"]["promotion"] == "BLOCKED"
    )

    three = record(first, "oteryn:item.3")
    assert three["field_overrides"] == {}, (
        "name-only candidate must never bind fields"
    )
    assert first["counts"]["field_states"]["UNKNOWN"] > 0
    assert first["invariants"]["missing_coerced_to_false_or_zero"] is False
    assert first["counts"]["promotion"].get("ELIGIBLE", 0) == 0

    # A separately proven/derived target bridge is required before an otherwise corroborated
    # field can become promotion-eligible.
    derived_current = copy.deepcopy(current)
    derived_current["pages"][0]["target_continuity"] = "DERIVED"
    derived = field.compile_verification(crosswalk, derived_current, schema)
    derived_one = record(derived, "oteryn:item.1")
    assert derived_one["field_overrides"]["weapon.attack"]["promotion"] == (
        "ELIGIBLE"
    )
    assert derived_one["field_overrides"]["presentation.name"]["promotion"] == (
        "ELIGIBLE"
    )

    # Conflicting OTS observations stay explicit and cannot be voted away.
    conflicting_crosswalk = copy.deepcopy(crosswalk)
    conflicting_crosswalk["source_profiles"][0]["candidate_observations"].append(
        obs("attack", "43")
    )
    conflicted = field.compile_verification(
        conflicting_crosswalk, current, schema
    )
    p1_conflict = next(
        item
        for item in conflicted["profile_field_rules"]
        if item["profile_id"] == "p1"
    )
    assert (
        p1_conflict["fields"]["weapon.attack"]["field_state"] == "CONFLICT"
    )

    # Exact identity join is mandatory.
    bad_current = copy.deepcopy(current)
    bad_current["records"][0]["source_item_id"] = 999
    try:
        field.compile_verification(crosswalk, bad_current, schema)
    except field.VerificationError as exc:
        assert "IDENTITY" in str(exc)
    else:
        raise AssertionError("identity mismatch was accepted")

    print("item-field-verification-self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
