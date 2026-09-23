#!/usr/bin/env python3
"""Synthetic tests for item_semantic_promotion.py."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("item_semantic_promotion.py")
spec = importlib.util.spec_from_file_location("item_semantic_promotion", MODULE_PATH)
if spec is None or spec.loader is None:
    raise RuntimeError("unable to load item_semantic_promotion.py")
promotion = importlib.util.module_from_spec(spec)
spec.loader.exec_module(promotion)


def protected_manifest():
    return {
        "schema": promotion.PROTECTED_MANIFEST_SCHEMA,
        "target_date": promotion.TARGET_DATE,
        "compiler": {"sha256": promotion.PROTECTED_CONTINUITY_COMPILER_SHA256},
        "counts": {
            "candidate_fields": promotion.EXPECTED_PROMOTED_FIELDS,
            "candidate_pages": promotion.EXPECTED_CANDIDATE_PAGES,
            "continuity": {
                "CONFLICT": 0,
                "DERIVED": promotion.EXPECTED_PROMOTED_FIELDS,
                "UNKNOWN": 0,
            },
            "per_field": {
                f"{field}:DERIVED": count
                for field, count in sorted(promotion.EXPECTED_PER_FIELD.items())
            },
        },
        "invariants": {
            "proven_continuity_emitted": False,
            "semantic_promotion_performed": False,
        },
    }


def source_value(field: str, ordinal: int):
    if field == "presentation.name":
        return f"Synthetic Item {ordinal}"
    if field == "weapon.hit_chance":
        return [5, 7][ordinal % 2]
    if field == "weapon.range_cells":
        return ordinal + 1
    if field == "charges.count":
        return 9
    if field == "container.capacity":
        return 20
    return ordinal + 10


def continuity():
    records = []
    source_id = 1_000
    page_id = 20_000
    for field, count in sorted(promotion.EXPECTED_PER_FIELD.items()):
        for ordinal in range(count):
            source_id += 1
            page_id = 20_000 + (len(records) % promotion.EXPECTED_CANDIDATE_PAGES)
            records.append(
                {
                    "source_item_id": source_id,
                    "native_key": f"oteryn:item.synthetic.i{source_id:08d}",
                    "field_path": field,
                    "page_id": page_id,
                    "source_field": field.rsplit(".", 1)[-1],
                    "current_value": source_value(field, ordinal),
                    "continuity_to_target": "DERIVED",
                    "promotion_bridge": "ELIGIBLE_FOR_SEMANTIC_PROMOTION_GENERATION",
                    "evidence": [],
                }
            )
    assert len(records) == promotion.EXPECTED_PROMOTED_FIELDS
    return {
        "schema": promotion.CONTINUITY_SCHEMA,
        "profile": promotion.CONTINUITY_PROFILE,
        "target_date": promotion.TARGET_DATE,
        "counts": {
            "candidate_fields": promotion.EXPECTED_PROMOTED_FIELDS,
            "candidate_pages": promotion.EXPECTED_CANDIDATE_PAGES,
            "continuity": {
                "DERIVED": promotion.EXPECTED_PROMOTED_FIELDS,
                "UNKNOWN": 0,
                "CONFLICT": 0,
            },
        },
        "records": records,
        "invariants": {
            "only_corrob_current_examined": True,
            "name_only_identity_resolution": False,
            "proven_continuity_emitted": False,
            "semantic_promotion_performed": False,
        },
    }


def compile_packet():
    return promotion.compile_promotion(
        continuity(),
        protected_manifest(),
        protected_manifest_sha256="a" * 64,
        compiler_sha256="b" * 64,
    )


def expect_error(value, fragment):
    try:
        value()
    except promotion.PromotionError as exc:
        assert fragment in str(exc), (fragment, str(exc))
    else:
        raise AssertionError(f"expected PromotionError containing {fragment!r}")


def test_exact_partition_and_determinism():
    first = compile_packet()
    second = compile_packet()
    assert promotion.canonical_bytes(first) == promotion.canonical_bytes(second)
    assert first["counts"]["promoted_fields"] == 69
    assert first["counts"]["promoted_items"] == 69
    assert first["counts"]["per_field"] == promotion.EXPECTED_PER_FIELD
    assert first["invariants"]["identity_reminted"] is False
    assert first["invariants"]["whole_item_promotion"] is False
    assert first["invariants"]["mutable_wiki_revision_metadata_retained"] is False


def test_percent_points_are_canonical_rationals():
    assert promotion.typed_value("weapon.hit_chance", 5) == {
        "kind": "RATIONAL_PERCENT",
        "numerator": 1,
        "denominator": 20,
        "source_unit": "PERCENT_POINTS",
    }
    assert promotion.typed_value("weapon.hit_chance", 7) == {
        "kind": "RATIONAL_PERCENT",
        "numerator": 7,
        "denominator": 100,
        "source_unit": "PERCENT_POINTS",
    }
    assert promotion.typed_value("weapon.hit_chance", -25) == {
        "kind": "RATIONAL_PERCENT",
        "numerator": -1,
        "denominator": 4,
        "source_unit": "PERCENT_POINTS",
    }


def test_non_derived_record_fails_closed():
    value = continuity()
    value["records"][0]["continuity_to_target"] = "UNKNOWN"
    expect_error(
        lambda: promotion.compile_promotion(
            value,
            protected_manifest(),
            protected_manifest_sha256="a" * 64,
            compiler_sha256="b" * 64,
        ),
        "NON_DERIVED_RECORD_NOT_PROMOTABLE",
    )


def test_duplicate_atomic_field_fails_closed():
    value = continuity()
    value["records"][1]["native_key"] = value["records"][0]["native_key"]
    value["records"][1]["source_item_id"] = value["records"][0]["source_item_id"]
    value["records"][1]["field_path"] = value["records"][0]["field_path"]
    expect_error(
        lambda: promotion.compile_promotion(
            value,
            protected_manifest(),
            protected_manifest_sha256="a" * 64,
            compiler_sha256="b" * 64,
        ),
        "PROMOTION_FIELD_DUPLICATE",
    )


def test_native_source_identity_conflict_fails_closed():
    value = continuity()
    value["records"][1]["native_key"] = value["records"][0]["native_key"]
    expect_error(
        lambda: promotion.compile_promotion(
            value,
            protected_manifest(),
            protected_manifest_sha256="a" * 64,
            compiler_sha256="b" * 64,
        ),
        "PROMOTION_IDENTITY_BINDING_CONFLICT",
    )


def test_wrong_protected_partition_fails_closed():
    manifest = protected_manifest()
    manifest["counts"]["candidate_fields"] = 68
    expect_error(
        lambda: promotion.compile_promotion(
            continuity(),
            manifest,
            protected_manifest_sha256="a" * 64,
            compiler_sha256="b" * 64,
        ),
        "PROTECTED_CANDIDATE_FIELD_COUNT_MISMATCH",
    )


def test_typed_bounds_fail_closed():
    expect_error(
        lambda: promotion.typed_value("weapon.range_cells", 65_536),
        "WEAPON_RANGE_CELLS_OUT_OF_RANGE",
    )
    expect_error(
        lambda: promotion.typed_value("container.capacity", -1),
        "CONTAINER_CAPACITY_OUT_OF_RANGE",
    )
    expect_error(
        lambda: promotion.typed_value("presentation.name", ""),
        "PRESENTATION_NAME_INVALID",
    )


def test_mutable_revision_metadata_is_not_emitted():
    value = continuity()
    for row in value["records"]:
        row["current_revision_id"] = 999999
        row["current_revision_timestamp"] = "2099-01-01T00:00:00Z"
        row["evidence"] = [{"revision_id": 1, "source_digest": "c" * 64}]
    packet = promotion.compile_promotion(
        value,
        protected_manifest(),
        protected_manifest_sha256="a" * 64,
        compiler_sha256="b" * 64,
    )
    serialized = promotion.canonical_bytes(packet)
    assert b"current_revision_id" not in serialized
    assert b"current_revision_timestamp" not in serialized
    assert b"source_digest" not in serialized


def main() -> int:
    tests = [
        value
        for name, value in sorted(globals().items())
        if name.startswith("test_") and callable(value)
    ]
    for test in tests:
        test()
    print(f"item-semantic-promotion self-test: PASS tests={len(tests)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
