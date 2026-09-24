#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "item_identity_catalog.py"

spec = importlib.util.spec_from_file_location("cw2_b1_item_identity_catalog", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)

def xml(items: str) -> bytes:
    return f'<?xml version="1.0"?><items>{items}</items>'.encode("utf-8")

def item(item_id: int, name: str, attributes: str = "") -> str:
    return f'<item id="{item_id}" name="{name}">{attributes}</item>'

def attr(key: str, value: str, children: str = "") -> str:
    return f'<attribute key="{key}" value="{value}">{children}</attribute>'

def test_no_auto_mint_and_supported_field_mapping() -> None:
    source = xml(
        item(
            100,
            "Example Blade",
            attr("weaponType", "sword")
            + attr("attack", "42")
            + attr("imbuementslot", "1", '<attribute key="critical hit" value="3"/>'),
        )
    )
    result = catalog.build_semantic_catalog(source)
    assert result["counts"]["total_emitted_source_identity_records"] == 1
    record = result["identity_records"][0]
    assert record["source_item_id"] == 100
    assert record["native_mapping"]["disposition"] == "UNRESOLVED"
    assert record["native_mapping"]["content_key"] is None
    fields = {entry["normalized_source_key"]: entry for entry in result["field_disposition_records"]}
    assert fields["weapontype"]["disposition"] == "GAME_ITEM_CANDIDATE"
    assert fields["attack"]["native_field"] == "attack"
    assert fields["imbuementslot"]["semantic_family"] == "imbuement"
    semantic = result["semantic_candidate_node_records"]
    assert len(semantic) == 1
    candidates = {entry["native_field"]: entry for entry in semantic[0]["candidate_fields"]}
    assert candidates["weapon_type"]["source_value"] == "sword"
    assert candidates["attack"]["source_value"] == "42"
    assert candidates["slot_and_allowed_family_tier"]["source_value"] == "1"
    assert candidates["slot_and_allowed_family_tier"]["nested_values"] == [
        {"key": "critical hit", "value": "3"}
    ]

def test_duplicate_id_and_overlapping_range_fail_closed() -> None:
    duplicate = xml(item(100, "A") + item(100, "B"))
    try:
        catalog.build_semantic_catalog(duplicate)
    except catalog.CatalogError as exc:
        assert "DUPLICATE_SOURCE_ID_AFTER_RANGE_EXPANSION" in str(exc)
    else:
        raise AssertionError("duplicate source ID was accepted")

    overlap = xml(
        '<item fromid="100" toid="102" name="range"/>'
        '<item fromid="102" toid="104" name="overlap"/>'
    )
    try:
        catalog.build_semantic_catalog(overlap)
    except catalog.CatalogError as exc:
        assert "DUPLICATE_SOURCE_ID_AFTER_RANGE_EXPANSION" in str(exc)
    else:
        raise AssertionError("overlapping ranges were accepted")

def test_reversed_range_is_explicitly_excluded() -> None:
    source = xml(
        '<item fromid="105" toid="103" name="reversed"/>'
        + item(200, "normal")
    )
    result = catalog.build_semantic_catalog(source)
    assert result["counts"]["source_reversed_range_nodes_excluded"] == 1
    assert result["counts"]["expanded_source_item_identities"] == 1
    assert result["range_exclusion_records"][0]["kind"] == "REVERSED_RANGE_EXCLUDED"
    assert result["count_invariants"]["range_exclusions_are_explicit"] is True

def test_name_collision_never_resolves_identity() -> None:
    source = xml(item(100, "Same Name") + item(101, "same name"))
    result = catalog.build_semantic_catalog(source)
    assert result["counts"]["name_collision_groups"] == 1
    assert [r["native_mapping"]["disposition"] for r in result["identity_records"]] == [
        "UNRESOLVED",
        "UNRESOLVED",
    ]

def test_unsupported_and_excluded_attributes_are_explicit() -> None:
    source = xml(
        item(
            100,
            "Door",
            attr("levelDoor", "10")
            + attr("script", "moveevent")
            + attr("mysteryField", "x"),
        )
    )
    result = catalog.build_semantic_catalog(source)
    fields = {entry["normalized_source_key"]: entry for entry in result["field_disposition_records"]}
    assert fields["leveldoor"]["disposition"] == "UNSUPPORTED"
    assert fields["script"]["disposition"] == "EXCLUDED_BY_POLICY"
    assert fields["mysteryfield"]["disposition"] == "UNKNOWN"
    assert result["counts"]["unsupported_field_observations"] == 1
    assert result["counts"]["unknown_field_observations"] == 1
    assert result["counts"]["excluded_by_policy_field_observations"] == 1
    assert result["semantic_candidate_node_records"] == []

def test_binding_resolution_ambiguous_and_conflict_classes() -> None:
    resolved = catalog.resolve_native_mapping(
        100,
        [
            {
                "source_item_id": 100,
                "content_key": "oteryn:item.example",
                "evidence_ref": "protected-binding-a",
                "state": "ASSERTED",
            }
        ],
    )
    assert resolved["disposition"] == "RESOLVED"
    assert resolved["content_key"] == "oteryn:item.example"

    ambiguous = catalog.resolve_native_mapping(
        100,
        [
            {
                "source_item_id": 100,
                "content_key": "oteryn:item.a",
                "evidence_ref": "candidate-a",
                "state": "CANDIDATE",
            },
            {
                "source_item_id": 100,
                "content_key": "oteryn:item.b",
                "evidence_ref": "candidate-b",
                "state": "CANDIDATE",
            },
        ],
    )
    assert ambiguous["disposition"] == "AMBIGUOUS"

    conflict = catalog.resolve_native_mapping(
        100,
        [
            {
                "source_item_id": 100,
                "content_key": "oteryn:item.a",
                "evidence_ref": "asserted-a",
                "state": "ASSERTED",
            },
            {
                "source_item_id": 100,
                "content_key": "oteryn:item.b",
                "evidence_ref": "asserted-b",
                "state": "ASSERTED",
            },
        ],
    )
    assert conflict["disposition"] == "CONFLICT"

def test_distinct_observations_mapping_to_same_native_field_conflict() -> None:
    source = xml(
        item(
            100,
            "Conflict",
            attr("attack", "40") + attr("attack", "41"),
        )
    )
    result = catalog.build_semantic_catalog(source)
    assert result["counts"]["field_conflict_records"] == 1
    assert result["field_conflict_records"][0]["native_field"] == "attack"

def test_source_enumeration_reorder_is_semantically_identical() -> None:
    a = item(101, "Beta", attr("weight", "100"))
    b = item(100, "Alpha", attr("charges", "5"))
    forward = catalog.build_semantic_catalog(xml(a + b))
    reverse = catalog.build_semantic_catalog(xml(b + a))
    assert catalog.canonical_bytes(forward) == catalog.canonical_bytes(reverse)

def test_clean_repeat_is_byte_identical() -> None:
    source = xml(
        '<item fromid="100" toid="102" name="Range">'
        + attr("containerSize", "20")
        + "</item>"
        + item(200, "Single", attr("allowpickUpAble", "true"))
    )
    first = catalog.build_semantic_catalog(source)
    second = catalog.build_semantic_catalog(source)
    assert catalog.canonical_bytes(first) == catalog.canonical_bytes(second)

def main() -> int:
    test_no_auto_mint_and_supported_field_mapping()
    test_duplicate_id_and_overlapping_range_fail_closed()
    test_reversed_range_is_explicitly_excluded()
    test_name_collision_never_resolves_identity()
    test_unsupported_and_excluded_attributes_are_explicit()
    test_binding_resolution_ambiguous_and_conflict_classes()
    test_distinct_observations_mapping_to_same_native_field_conflict()
    test_source_enumeration_reorder_is_semantically_identical()
    test_clean_repeat_is_byte_identical()
    print("item-identity-catalog self-test: PASS")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
