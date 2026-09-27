#!/usr/bin/env python3
from __future__ import annotations

import copy
import argparse
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
from unittest.mock import patch


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MODULE_PATH = HERE / "item_classification_crosswalk.py"
spec = importlib.util.spec_from_file_location("item_classification_crosswalk", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError(f"cannot load {MODULE_PATH}")
crosswalk = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = crosswalk
spec.loader.exec_module(crosswalk)


def inputs():
    return {name: crosswalk.read_pinned(ROOT, name) for name in crosswalk.PINNED_INPUTS}


def reject(action, marker: str) -> None:
    try:
        action()
    except crosswalk.CrosswalkError as exc:
        assert marker in str(exc), (marker, str(exc))
    else:
        raise AssertionError(f"expected rejection containing {marker}")


def test_crosswalk_outcome_partition() -> None:
    assert crosswalk.classify_candidates([])["outcome"] == "ZERO_MATCH"
    assert crosswalk.classify_candidates([
        {"native_key": "oteryn:item.a", "state": "ASSERTED"}
    ])["outcome"] == "EXACT_ONE"
    assert crosswalk.classify_candidates([
        {"native_key": "oteryn:item.a", "state": "CANDIDATE"},
        {"native_key": "oteryn:item.b", "state": "CANDIDATE"},
    ])["outcome"] == "AMBIGUOUS"
    assert crosswalk.classify_candidates([
        {"native_key": "oteryn:item.a", "state": "ASSERTED"},
        {"native_key": "oteryn:item.b", "state": "CANDIDATE"},
    ])["outcome"] == "CONFLICT"
    assert crosswalk.classify_candidates([
        {"native_key": "oteryn:item.a", "state": "ASSERTED"},
        {"native_key": "oteryn:item.a", "state": "ASSERTED"},
    ])["outcome"] == "CONFLICT"


def load_native_map(path: Path):
    value, payload = crosswalk.read_native_map(path)
    return value, crosswalk.digest(payload)


def test_full_census_is_closed_deterministic_and_non_promoting(native_map_path: Path) -> None:
    source = inputs()
    native_map, native_map_sha256 = load_native_map(native_map_path)
    first = crosswalk.compile_crosswalk(
        source["b1_catalog"], source["native_batch"], source["family_registry"], source["schema_readiness"], native_map, native_map_sha256
    )
    first_bytes = crosswalk.canonical_bytes(first)
    second = crosswalk.compile_crosswalk(
        source["b1_catalog"], source["native_batch"], source["family_registry"], source["schema_readiness"], native_map, native_map_sha256
    )
    assert first_bytes == crosswalk.canonical_bytes(second)
    counts = first["counts"]
    assert counts["records"] == 38_157
    assert counts["preserved_semantic_bindings"] == 64
    assert counts["opaque_registry_allocations"] == 38_093
    assert counts["crosswalk_outcomes"] == {
        "EXACT_ONE": 38_157, "ZERO_MATCH": 0, "AMBIGUOUS": 0, "CONFLICT": 0,
    }
    assert first["allocation_digest_sha256"] == crosswalk.ALLOCATION_DIGEST
    assert len({record["source_item_id"] for record in first["records"]}) == 38_157
    assert len({record["native_key"] for record in first["records"]}) == 38_157
    assert all(record["accepted_reference_classification"] == "UNKNOWN" for record in first["records"])
    assert all(record["current_source_status"] == "NOT_EVALUATED" for record in first["records"])
    assert any(profile["candidate_observations"] for profile in first["source_profiles"])
    assert any(not profile["candidate_observations"] for profile in first["source_profiles"])
    for profile in first["source_profiles"]:
        assert profile["accepted_reference_classification"] == "UNKNOWN"
        assert set(profile["capability_states"]) == set(crosswalk.CAPABILITIES)
        assert set(profile["classification_capability_states"]) == set(crosswalk.CLASSIFICATION_CAPABILITIES)
        assert all(value["accepted_reference_state"] == "UNKNOWN" for value in profile["capability_states"].values())
        assert all(value["accepted_reference_state"] == "UNKNOWN" for value in profile["classification_capability_states"].values())
        assert all(value["current_source_state"] == "NOT_EVALUATED" for value in profile["capability_states"].values())
    assert counts["classification_capability_present:weapon"] > 0
    assert counts["classification_capability_present:rune"] > 0
    assert counts["classification_capability_present:currency"] == 0


def test_native_map_bound(native_map_path: Path) -> None:
    payload = native_map_path.read_bytes()
    assert len(payload) == crosswalk.NATIVE_MAP_MAX_BYTES
    controlled_oversized = payload + (b" " * 16)
    with patch.object(Path, "stat", side_effect=AssertionError("stat must not be consulted")), patch.object(
        Path, "open", return_value=io.BytesIO(controlled_oversized)
    ):
        reject(
            lambda: crosswalk.read_native_map(Path("controlled-native-map.json")),
            "CANONICAL_NATIVE_MAP_MAX_PLUS_ONE",
        )
    with tempfile.TemporaryDirectory() as directory:
        oversized = Path(directory) / "native-map-max-plus-one.json"
        oversized.write_bytes(payload + b" ")
        reject(lambda: crosswalk.read_native_map(oversized), "CANONICAL_NATIVE_MAP_MAX_PLUS_ONE")


def test_duplicate_missing_and_remap_fail_closed(native_map_path: Path) -> None:
    source = inputs()
    rows = source["b1_catalog"]["semantic_catalog"]["identity_records"]
    bindings = source["native_batch"]["binding_map"]
    registry = source["family_registry"]
    native_map, _ = load_native_map(native_map_path)
    reject(lambda: crosswalk.validate_native_map(rows[:-1], bindings, registry, native_map), "IDENTITY_COUNT_MISMATCH")
    duplicate_rows = list(rows)
    duplicate_rows[-1] = duplicate_rows[-2]
    reject(lambda: crosswalk.validate_native_map(duplicate_rows, bindings, registry, native_map), "DUPLICATE_SOURCE_ITEM_ID")
    missing_map = copy.deepcopy(native_map)
    missing_map["records"].pop()
    reject(lambda: crosswalk.validate_native_map(rows, bindings, registry, missing_map), "CANONICAL_NATIVE_MAP_COUNT_MISMATCH")
    remapped = copy.deepcopy(native_map)
    remapped["records"][-1]["native_key"] = "oteryn:item.illegal_remap"
    reject(lambda: crosswalk.validate_native_map(rows, bindings, registry, remapped), "ALLOCATION_REMAP")
    duplicate_native = copy.deepcopy(native_map)
    duplicate_native["records"][1]["native_key"] = duplicate_native["records"][0]["native_key"]
    reject(lambda: crosswalk.validate_native_map(rows, bindings, registry, duplicate_native), "DUPLICATE_NATIVE_KEY")


def test_schema_coverage_is_exact() -> None:
    source = inputs()
    b1 = source["b1_catalog"]
    schema = copy.deepcopy(source["schema_readiness"])
    destinations = schema["b1_candidate_field_coverage"]["destinations"]
    assert len(destinations) == 90
    destinations.pop("item_type")
    reject(lambda: crosswalk.source_profiles(b1, schema), "SCHEMA_FIELD_COVERAGE_INVALID")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--native-map", type=Path, required=True)
    args = parser.parse_args()
    test_crosswalk_outcome_partition()
    test_native_map_bound(args.native_map)
    test_full_census_is_closed_deterministic_and_non_promoting(args.native_map)
    test_duplicate_missing_and_remap_fail_closed(args.native_map)
    test_schema_coverage_is_exact()
    print("item-classification-crosswalk self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
