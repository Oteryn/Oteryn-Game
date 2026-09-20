#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile

HERE = Path(__file__).resolve().parent
PATH = HERE / "interaction_binding_catalog.py"

spec = importlib.util.spec_from_file_location("cw2_b6_catalog", PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("cannot load B6 generator")
catalog = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = catalog
spec.loader.exec_module(catalog)


def observation(family, x, y, floor, item_order, source_item_id, source_value):
    return {
        "structural_kind": family,
        "position": {"x": x, "y": y, "floor": floor},
        "item_order": item_order,
        "source_item_id": source_item_id,
        "source_value": source_value,
    }


def expect_error(fragment, fn):
    try:
        fn()
    except catalog.CatalogError as exc:
        assert fragment in str(exc), exc
    else:
        raise AssertionError(f"expected CatalogError containing {fragment!r}")


def verify_storage_round_trip(records, counts):
    index, families, payloads = catalog.build_catalog(records, counts)
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        catalog.write_catalog(root, index, payloads)
        parsed = json.loads((root / catalog.INDEX_NAME).read_text(encoding="utf-8"))
        verification = catalog.verify_written_catalog(root, parsed)
        assert verification["family_counts"] == {
            family: families[family]["counts"]["occurrences"] for family in catalog.FAMILIES
        }
        return parsed, verification


def main() -> int:
    records = [
        observation("UNIQUE_ID", 10, 20, -7, 0, 100, 500),
        observation("ACTION_ID", 10, 20, -7, 1, 101, 600),
        observation("TELEPORT_DESTINATION", 11, 20, -7, 0, 102, {"x": 1, "y": 2, "floor": -8}),
        observation("HOUSE_DOOR_ID", 12, 20, -7, 0, 103, 7),
        observation("ACTION_ID", 13, 20, -7, 0, 104, 600),
    ]
    counts = {"map_header": 1, "tile": 3, "town": 0, "waypoint": 0}

    first_index, first_families, first_payloads = catalog.build_catalog(records, counts)
    second_index, second_families, second_payloads = catalog.build_catalog(list(reversed(records)), counts)

    assert catalog.canonical_bytes(first_index) == catalog.canonical_bytes(second_index)
    assert first_payloads == second_payloads
    for family in catalog.FAMILIES:
        assert catalog.canonical_bytes(first_families[family]) == catalog.canonical_bytes(second_families[family])

    assert first_index["total_occurrences"] == 5
    assert first_index["dispositions"] == {
        "UNKNOWN": 5, "UNSUPPORTED": 0, "AMBIGUOUS": 0, "CONFLICT": 0, "LOSS": 0
    }
    assert first_index["silent_drop"] == 0
    assert first_index["unclassified"] == 0
    assert first_families["ACTION_ID"]["counts"]["occurrences"] == 2
    assert first_families["ACTION_ID"]["counts"]["unique_source_values"] == 1
    assert first_families["ACTION_ID"]["counts"]["source_value_reuse_groups"] == 1
    assert first_families["ACTION_ID"]["counts"]["max_source_value_reuse"] == 2
    assert first_families["TELEPORT_DESTINATION"]["records"][0]["semantic_disposition"] == "UNKNOWN"
    assert all(
        not record["executable_eligible"]
        for family in catalog.FAMILIES
        for record in first_families[family]["records"]
    )

    direct = {
        entry["family"]: entry for entry in first_index["storage"]["family_files"]
    }
    assert direct["ACTION_ID"]["storage_mode"] == "DIRECT"
    assert direct["UNIQUE_ID"]["storage_mode"] == "DIRECT"
    assert direct["TELEPORT_DESTINATION"]["storage_mode"] == "SHARDED"
    assert direct["HOUSE_DOOR_ID"]["storage_mode"] == "SHARDED"

    parsed, verification = verify_storage_round_trip(records, counts)
    assert parsed["logical_product_digest_sha256"] == first_index["logical_product_digest_sha256"]
    assert verification["family_counts"]["ACTION_ID"] == 2
    assert verification["family_counts"]["UNIQUE_ID"] == 1
    assert verification["family_counts"]["TELEPORT_DESTINATION"] == 1
    assert verification["family_counts"]["HOUSE_DOOR_ID"] == 1

    bulk = [
        observation(
            "TELEPORT_DESTINATION",
            20_000 + i,
            30_000 + (i // 1000),
            -7,
            i % 7,
            500 + (i % 300),
            {"x": 40_000 + i, "y": 50_000 + (i % 2000), "floor": -8},
        )
        for i in range(1_600)
    ]
    bulk_index, bulk_families, bulk_payloads = catalog.build_catalog(
        bulk,
        {"map_header": 1, "tile": 1_600, "town": 0, "waypoint": 0},
    )
    teleport_entry = next(
        entry for entry in bulk_index["storage"]["family_files"]
        if entry["family"] == "TELEPORT_DESTINATION"
    )
    assert teleport_entry["storage_mode"] == "SHARDED"
    assert teleport_entry["shard_count"] >= 2
    assert sum(shard["record_count"] for shard in teleport_entry["shards"]) == 1_600
    assert all(shard["bytes"] <= catalog.MAX_SHARD_BYTES for shard in teleport_entry["shards"])
    assert all(path.endswith(".json") for path in bulk_payloads)
    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        catalog.write_catalog(root, bulk_index, bulk_payloads)
        verified = catalog.verify_written_catalog(root, bulk_index)
        assert verified["family_counts"]["TELEPORT_DESTINATION"] == 1_600
        assert verified["family_files"]["TELEPORT_DESTINATION"]["shard_count"] >= 2
        reconstructed_digest = verified["family_files"]["TELEPORT_DESTINATION"]["records_digest_sha256"]
        assert reconstructed_digest == bulk_families["TELEPORT_DESTINATION"]["records_digest_sha256"]

    expect_error(
        "UNSUPPORTED_STRUCTURAL_FAMILY",
        lambda: catalog.build_catalog([observation("OTHER", 1, 2, -7, 0, 1, 2)], counts),
    )
    expect_error(
        "MALFORMED_TELEPORT_DESTINATION",
        lambda: catalog.build_catalog(
            [observation("TELEPORT_DESTINATION", 1, 2, -7, 0, 1, {"x": 1, "y": 2})], counts
        ),
    )
    duplicate = observation("ACTION_ID", 1, 2, -7, 0, 1, 100)
    expect_error(
        "DUPLICATE_RECORD_ID",
        lambda: catalog.build_catalog([duplicate, dict(duplicate)], counts),
    )

    try:
        catalog.validate_real_source_counts(counts)
    except catalog.CatalogError as exc:
        assert "SOURCE_STREAM_COUNT_MISMATCH" in str(exc)
    else:
        raise AssertionError("synthetic counts must not pass real-source validation")

    print("interaction_binding_catalog_self_test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
