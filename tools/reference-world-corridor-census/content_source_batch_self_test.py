#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[2]


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


producer = _load(
    "cw2_fullworld_producer",
    ROOT / "tools/game-atlas-fullworld-source/producer.py",
)
batch = _load(
    "cw2_content_source_batch",
    Path(__file__).resolve().parent / "content_source_batch.py",
)


def phase_a_summary():
    return {
        "classification": "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY",
        "phase_a_result": "PASS",
        "phase_b_target_parity": "NOT_PERFORMED",
        "producer": {
            "api": "oteryn-game-atlas-fullworld-source-v0",
            "code_commit": "d7c207876920dfe7c7026a0d1316a2ee603ebf0d",
            "repository": "Oteryn/Oteryn-Game",
        },
        "source": {
            "appearance_sha256": "a" * 64,
            "asset_zip_sha256": "b" * 64,
            "catalog_sha256": "c" * 64,
            "legacy_revision": "e417c5e7c22986bf4acef0495eb47f7b72c97cce",
            "world_otbm_sha256": "d" * 64,
        },
    }


def tile(record_id: str, appearance_source_id: int, x: int):
    return {
        "record_type": "tile",
        "position": {"floor": -7, "x": x, "y": 32513},
        "source_position": {"legacy_x": x, "legacy_y": 32513, "legacy_z": 7},
        "presentation": [
            {
                "export_record_id": record_id,
                "appearance_source_id": appearance_source_id,
                "source_role": "tile_item",
                "presentation_order": {"plane": 0, "order": 0},
                "canonical_entity_id": None,
                "entity_identity_state": "UNRESOLVED",
            }
        ],
    }


def binding(record_id: str, appearance_source_id: int, placement_key: str):
    return producer.SourceIdentityBinding(
        export_record_id=record_id,
        appearance_source_id=appearance_source_id,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.local-door",
        definition_revision="definition-r1",
        placement_key=placement_key,
    )


def test_bound_and_unbound_occurrences_are_explicit() -> None:
    first = tile("presentation:aaa", 2031, 32538)
    second = tile("presentation:bbb", 3687, 32542)
    result = batch.build_content_source_batch(
        producer=producer,
        tile_records=[first, second],
        bindings={
            "presentation:aaa": binding(
                "presentation:aaa",
                2031,
                "oteryn:reference.placement.local-door",
            )
        },
        phase_a_summary=phase_a_summary(),
        adapter_revision="3985c07fbb1556559c7672087cbef7cdc20db16e",
    )

    assert result["schema"] == "OTERYN_REFERENCE_CONTENT_SOURCE_BATCH/v1"
    assert result["counts"] == {
        "source_tile_records": 2,
        "source_occurrences": 2,
        "bound_definitions": 1,
        "bound_placements": 1,
        "unresolved_source_occurrences": 1,
    }
    placement = result["placements"][0]
    assert placement["placement_key"] == "oteryn:reference.placement.local-door"
    assert placement["typed_definition_ref"]["definition_family"] == "LOCAL_OBJECT"
    assert placement["source_provenance"]["appearance_source_id"] == 2031
    assert (
        placement["source_provenance"]["source_evidence_classification"]
        == "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY"
    )
    assert all(
        value == "DEFERRED_REQUIRES_PHASE_B"
        for value in placement["target_sensitive_fields"].values()
    )
    assert result["unresolved_source_occurrences"][0]["source_occurrence_ref"] == "presentation:bbb"
    assert result["loss_conflict_unknown_summary"]["target_geometry_promoted"] is False
    assert result["production_authority"] == "NONE"
    assert result["reference_parity_claim"] == "NONE"


def test_batch_is_deterministic_under_source_enumeration_reorder() -> None:
    first = tile("presentation:aaa", 2031, 32538)
    second = tile("presentation:bbb", 3687, 32542)
    bindings = {
        "presentation:aaa": binding(
            "presentation:aaa",
            2031,
            "oteryn:reference.placement.local-door.a",
        ),
        "presentation:bbb": binding(
            "presentation:bbb",
            3687,
            "oteryn:reference.placement.local-door.b",
        ),
    }
    forward = batch.build_content_source_batch(
        producer=producer,
        tile_records=[first, second],
        bindings=bindings,
        phase_a_summary=phase_a_summary(),
        adapter_revision="revision-1",
    )
    reverse = batch.build_content_source_batch(
        producer=producer,
        tile_records=[second, first],
        bindings=bindings,
        phase_a_summary=phase_a_summary(),
        adapter_revision="revision-1",
    )
    assert batch.canonical_batch_bytes(forward) == batch.canonical_batch_bytes(reverse)


def test_duplicate_placement_identity_fails_closed() -> None:
    first = tile("presentation:aaa", 2031, 32538)
    second = tile("presentation:bbb", 3687, 32542)
    duplicate_key = "oteryn:reference.placement.duplicate"
    try:
        batch.build_content_source_batch(
            producer=producer,
            tile_records=[first, second],
            bindings={
                "presentation:aaa": binding("presentation:aaa", 2031, duplicate_key),
                "presentation:bbb": binding("presentation:bbb", 3687, duplicate_key),
            },
            phase_a_summary=phase_a_summary(),
            adapter_revision="revision-1",
        )
    except batch.SourceBatchError as exc:
        assert "duplicate PlacementKey" in str(exc)
    else:
        raise AssertionError("duplicate PlacementKey was accepted")


def test_stale_binding_and_wrong_phase_a_classification_fail_closed() -> None:
    first = tile("presentation:aaa", 2031, 32538)
    try:
        batch.build_content_source_batch(
            producer=producer,
            tile_records=[first],
            bindings={
                "presentation:stale": binding(
                    "presentation:stale",
                    2031,
                    "oteryn:reference.placement.stale",
                )
            },
            phase_a_summary=phase_a_summary(),
            adapter_revision="revision-1",
        )
    except batch.SourceBatchError as exc:
        assert "does not match any batch occurrence" in str(exc)
    else:
        raise AssertionError("stale binding was accepted")

    wrong = phase_a_summary()
    wrong["classification"] = "PROVEN"
    try:
        batch.build_content_source_batch(
            producer=producer,
            tile_records=[first],
            bindings={},
            phase_a_summary=wrong,
            adapter_revision="revision-1",
        )
    except batch.SourceBatchError as exc:
        assert "classification mismatch" in str(exc)
    else:
        raise AssertionError("promoted Phase-A classification was accepted")


def main() -> int:
    test_bound_and_unbound_occurrences_are_explicit()
    test_batch_is_deterministic_under_source_enumeration_reorder()
    test_duplicate_placement_identity_fails_closed()
    test_stale_binding_and_wrong_phase_a_classification_fail_closed()
    print("content-source-batch self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
