#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "g4_item_bulk_population.py"
spec = importlib.util.spec_from_file_location("g4_item_bulk_population", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load bulk population compiler")
bulk = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = bulk
spec.loader.exec_module(bulk)


def make_inputs(page_rows: list[dict]) -> tuple[dict, dict, dict, dict]:
    census = {
        "schema": bulk.CENSUS_SCHEMA,
        "collector_profile": "test-profile",
        "source": {"id": bulk.SOURCE_ID, "role": "STRUCTURED_REFERENCE_DATA", "api": "test", "discovery": {"kind": "MEDIAWIKI_CATEGORYMEMBERS", "namespace": 0}},
        "retrieval_timestamp": "2026-09-25T00:00:00Z",
        "pages": page_rows,
        "counts": {"discovered_pages": len(page_rows), "fetched_pages": len(page_rows)},
    }
    stable = dict(census)
    stable.pop("retrieval_timestamp")
    census_manifest = {
        "schema": bulk.CENSUS_MANIFEST_SCHEMA,
        "status": "WIKI_FIRST_SOURCE_EVIDENCE_ONLY_NO_IDENTITY_PROMOTION",
        "full_output": {"sha256": bulk.digest(bulk.canonical_bytes(census)), "stable_without_retrieval_timestamp_sha256": bulk.digest(bulk.canonical_bytes(stable))},
    }
    targets = [
        {"source_item_id": 101, "native_key": "oteryn:item.registry.a", "native_revision": "definition-r1", "source_profile_id": "profile-a", "crosswalk_outcome": "EXACT_ONE"},
        {"source_item_id": 102, "native_key": "oteryn:item.registry.b", "native_revision": "definition-r2", "source_profile_id": "profile-b", "crosswalk_outcome": "EXACT_ONE"},
    ]
    profiles = [
        {"profile_id": "profile-a", "candidate_observations": [{"native_field": "attack", "source_value": "10", "nested_values": []}, {"native_field": "defense", "source_value": "5", "nested_values": []}, {"native_field": "range", "source_value": "2", "nested_values": []}]},
        {"profile_id": "profile-b", "candidate_observations": [{"native_field": "attack", "source_value": "11", "nested_values": []}, {"native_field": "defense", "source_value": "4", "nested_values": []}, {"native_field": "range", "source_value": "3", "nested_values": []}]},
    ]
    crosswalk = {"schema": bulk.CROSSWALK_SCHEMA, "records": targets, "source_profiles": profiles, "canonical_native_map": {"sha256": "a" * 64}, "full_record_output": {"records_sha256": "b" * 64}, "allocation_digest_sha256": "c" * 64, "counts": {"opaque_registry_allocations": 1}}
    native_map = {"schema": bulk.NATIVE_MAP_SCHEMA, "item_count": 2, "records": [{key: row[key] for key in ("source_item_id", "native_key", "native_revision")} for row in targets]}
    return census, census_manifest, crosswalk, native_map


def page(page_id: int, attack: int, defense: int, *, name: str = "Example") -> dict:
    return {
        "page_id": page_id,
        "title": name,
        "revision_id": page_id + 10,
        "revision_timestamp": "2026-09-24T00:00:00Z",
        "source_digest": f"{page_id:064x}",
        "source_shape": "INFOBOX_ITEM",
        "normalized_fields": {
            "attack": {"state": "VALUE", "value": attack},
            "defense": {"state": "VALUE", "value": defense},
            "range": {"state": "VALUE", "value": 2},
            "name": {"state": "VALUE", "value": name},
        },
    }


def test_unique_exact_binding_and_field_candidates() -> None:
    inputs = make_inputs([page(1, 10, 5)])
    with patch.object(bulk, "EXPECTED_CANONICAL_ITEMS", 2), patch.object(bulk, "EXPECTED_DISCOVERY_PAGES", 1):
        evidence, manifest = bulk.compile_bulk(*inputs)
    assert manifest["typed_binding_candidate_count"] == 1
    assert evidence["rows"][0]["status"] == "EXACT"
    assert evidence["rows"][0]["title"] == "Example"
    binding = evidence["typed_binding_candidates"][0]
    assert set(binding) == {"source_key", "source_revision", "identity_namespace", "external_id", "target", "disposition"}
    assert binding["identity_namespace"] == "mediawiki/page_id"
    assert binding["target"] == {"family": "Item", "key": "oteryn:item.registry.a", "revision": "definition-r1"}
    promotions = evidence["semantic_field_candidates"]
    assert {row["field_path"] for row in promotions} == {"presentation.name", "weapon.attack", "weapon.defense", "weapon.range_cells"}
    assert evidence["invariants"]["reference_item_semantics_applied"] is False


def test_duplicate_page_claims_block_binding_and_collision_conflicts() -> None:
    duplicate = make_inputs([page(1, 10, 5), page(2, 10, 5)])
    with patch.object(bulk, "EXPECTED_CANONICAL_ITEMS", 2), patch.object(bulk, "EXPECTED_DISCOVERY_PAGES", 2):
        evidence, manifest = bulk.compile_bulk(*duplicate)
    assert manifest["typed_binding_candidate_count"] == 0
    assert all(row["status"] == "AMBIGUOUS" for row in evidence["rows"])
    conflict = make_inputs([page(1, 10, 99)])
    with patch.object(bulk, "EXPECTED_CANONICAL_ITEMS", 2), patch.object(bulk, "EXPECTED_DISCOVERY_PAGES", 1):
        evidence, manifest = bulk.compile_bulk(*conflict)
    assert manifest["typed_binding_candidate_count"] == 0
    assert evidence["rows"][0]["status"] == "CONFLICT"


def test_names_and_numeric_ids_do_not_create_identity() -> None:
    row = page(101, 0, 0, name="oteryn:item.registry.a")
    row["normalized_fields"]["range"]["value"] = 9
    inputs = make_inputs([row])
    with patch.object(bulk, "EXPECTED_CANONICAL_ITEMS", 2), patch.object(bulk, "EXPECTED_DISCOVERY_PAGES", 1):
        evidence, manifest = bulk.compile_bulk(*inputs)
    assert manifest["typed_binding_candidate_count"] == 0
    assert evidence["rows"][0]["status"] == "NO_MATCH"


def test_field_type_bounds_are_fail_closed() -> None:
    assert bulk.typed_value("weapon.hit_chance", 2) == {"kind": "RATIONAL_PERCENT", "numerator": 1, "denominator": 50, "source_unit": "PERCENT_POINTS"}
    try:
        bulk.typed_value("weapon.range_cells", -1)
    except bulk.BulkError as exc:
        assert "CELLS_INVALID" in str(exc)
    else:
        raise AssertionError("negative range must be rejected")


def main() -> None:
    test_unique_exact_binding_and_field_candidates()
    test_duplicate_page_claims_block_binding_and_collision_conflicts()
    test_names_and_numeric_ids_do_not_create_identity()
    test_field_type_bounds_are_fail_closed()
    print("g4-item-bulk-population-self-test: PASS")


if __name__ == "__main__":
    main()
