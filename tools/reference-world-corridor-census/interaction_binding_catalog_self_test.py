#!/usr/bin/env python3
from __future__ import annotations

import hashlib
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


def relative_evidence_path(tracked_path: str) -> str:
    prefix = "docs/agents/evidence/"
    assert tracked_path.startswith(prefix), tracked_path
    return tracked_path[len(prefix):]


def verify_storage(index, family_docs, payloads):
    family_counts = {}
    for entry in index["storage"]["family_files"]:
        family = entry["family"]
        relative = relative_evidence_path(entry["tracked_path"])
        payload = payloads[relative]
        assert len(payload) == entry["bytes"]
        assert hashlib.sha256(payload).hexdigest() == entry["sha256"]
        document = json.loads(payload.decode("utf-8"))
        assert document["family"] == family
        assert document["logical_product_digest_sha256"] == index["logical_product_digest_sha256"]
        assert document["records_digest_sha256"] == entry["records_digest_sha256"]
        assert document["counts"]["occurrences"] == entry["records"]

        if entry["storage_mode"] == "INLINE":
            assert entry["shard_count"] == 0
            records = document["records"]
            assert len(records) == entry["records"]
            assert catalog.sha256_bytes(catalog.canonical_bytes(records)) == entry["records_digest_sha256"]
            family_counts[family] = len(records)
            continue

        assert entry["storage_mode"] == "SHARDED"
        storage = document["storage"]
        assert storage["mode"] == "SHARDED"
        assert storage["max_shard_bytes"] == catalog.MAX_SHARD_BYTES
        assert storage["shard_count"] == entry["shard_count"]

        reconstructed = []
        for ordinal, shard_meta in enumerate(storage["shards"], start=1):
            assert shard_meta["ordinal"] == ordinal
            shard_relative = relative_evidence_path(shard_meta["tracked_path"])
            shard_payload = payloads[shard_relative]
            assert len(shard_payload) == shard_meta["bytes"] <= catalog.MAX_SHARD_BYTES
            assert hashlib.sha256(shard_payload).hexdigest() == shard_meta["sha256"]
            shard = json.loads(shard_payload.decode("utf-8"))
            assert shard["schema"] == catalog.SHARD_SCHEMA
            assert shard["family"] == family
            assert shard["ordinal"] == ordinal
            assert shard["shard_count"] == entry["shard_count"]
            assert shard["logical_product_digest_sha256"] == index["logical_product_digest_sha256"]
            assert shard["family_records_digest_sha256"] == entry["records_digest_sha256"]
            assert shard["record_count"] == shard_meta["record_count"] == len(shard["records"])
            assert catalog.sha256_bytes(catalog.canonical_bytes(shard["records"])) == shard["records_digest_sha256"]
            assert shard["records_digest_sha256"] == shard_meta["records_digest_sha256"]
            reconstructed.extend(shard["records"])

        assert len(reconstructed) == entry["records"]
        assert catalog.sha256_bytes(catalog.canonical_bytes(reconstructed)) == entry["records_digest_sha256"]
        assert catalog.sha256_bytes(catalog.canonical_bytes(family_docs[family]["records"])) == entry["records_digest_sha256"]
        family_counts[family] = len(reconstructed)

    assert set(family_counts) == set(catalog.FAMILIES)
    return family_counts


def build(records, counts):
    index, families = catalog.build_catalog(records, counts)
    payloads = catalog.storage_payloads(index, families)
    return index, families, payloads


def main() -> int:
    records = [
        observation("UNIQUE_ID", 10, 20, -7, 0, 100, 500),
        observation("ACTION_ID", 10, 20, -7, 1, 101, 600),
        observation("TELEPORT_DESTINATION", 11, 20, -7, 0, 102, {"x": 1, "y": 2, "floor": -8}),
        observation("HOUSE_DOOR_ID", 12, 20, -7, 0, 103, 7),
        observation("ACTION_ID", 13, 20, -7, 0, 104, 600),
    ]
    counts = {"map_header": 1, "tile": 3, "town": 0, "waypoint": 0}

    first_index, first_families, first_payloads = build(records, counts)
    second_index, second_families, second_payloads = build(list(reversed(records)), counts)

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

    entries = {entry["family"]: entry for entry in first_index["storage"]["family_files"]}
    assert entries["ACTION_ID"]["storage_mode"] == "INLINE"
    assert entries["UNIQUE_ID"]["storage_mode"] == "INLINE"
    assert entries["TELEPORT_DESTINATION"]["storage_mode"] == "SHARDED"
    assert entries["HOUSE_DOOR_ID"]["storage_mode"] == "SHARDED"
    assert entries["TELEPORT_DESTINATION"]["shard_count"] == 1
    assert entries["HOUSE_DOOR_ID"]["shard_count"] == 1
    assert verify_storage(first_index, first_families, first_payloads) == {
        "ACTION_ID": 2,
        "UNIQUE_ID": 1,
        "TELEPORT_DESTINATION": 1,
        "HOUSE_DOOR_ID": 1,
    }

    bulk = []
    for i in range(1_600):
        bulk.append(
            observation(
                "TELEPORT_DESTINATION",
                20_000 + i,
                30_000 + (i // 1000),
                -7,
                i % 7,
                500 + (i % 300),
                {"x": 40_000 + i, "y": 50_000 + (i % 2000), "floor": -8},
            )
        )
    for i in range(2_500):
        bulk.append(
            observation(
                "HOUSE_DOOR_ID",
                40_000 + i,
                41_000 + (i // 1000),
                -7,
                i % 5,
                900 + (i % 200),
                i % 255,
            )
        )

    bulk_index, bulk_families, bulk_payloads = build(
        bulk,
        {"map_header": 1, "tile": 4_100, "town": 0, "waypoint": 0},
    )
    bulk_entries = {entry["family"]: entry for entry in bulk_index["storage"]["family_files"]}
    assert bulk_entries["TELEPORT_DESTINATION"]["shard_count"] == 3
    assert bulk_entries["HOUSE_DOOR_ID"]["shard_count"] == 5
    assert all(
        shard_meta["bytes"] <= catalog.MAX_SHARD_BYTES
        for family in ("TELEPORT_DESTINATION", "HOUSE_DOOR_ID")
        for shard_meta in json.loads(
            bulk_payloads[
                relative_evidence_path(bulk_entries[family]["tracked_path"])
            ].decode("utf-8")
        )["storage"]["shards"]
    )
    verified_bulk = verify_storage(bulk_index, bulk_families, bulk_payloads)
    assert verified_bulk["TELEPORT_DESTINATION"] == 1_600
    assert verified_bulk["HOUSE_DOOR_ID"] == 2_500

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        catalog.write_catalog(root, first_payloads)
        assert (root / catalog.INDEX_NAME).is_file()
        assert (root / catalog.FAMILY_DIR_NAME / "action-id.json").is_file()
        assert (root / catalog.FAMILY_DIR_NAME / "teleport-destination-0001.json").is_file()

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
