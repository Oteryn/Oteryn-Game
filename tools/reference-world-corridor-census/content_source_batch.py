#!/usr/bin/env python3
"""Bounded CW2 typed source batch over existing full-world producer records.

This module does not parse OTBM and does not promote migration geometry to the
Reference target.  It combines exact Phase-A source provenance with explicit
source-occurrence identity bindings so CW3 can consume typed candidates without
inventing canonical identity from legacy numeric IDs, coordinates or source
order.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

BATCH_SCHEMA = "OTERYN_REFERENCE_CONTENT_SOURCE_BATCH/v1"
SOURCE_CLASSIFICATION = "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY"
SOURCE_COORDINATE_FRAME = "LEGACY_OTS_MIGRATION_FRAME/v1"
IDENTITY_ADAPTER_PROFILE = "OTERYN_TYPED_SOURCE_IDENTITY_ADAPTER/v1"

DEFERRED_REQUIRES_PHASE_B = (
    "canonical_target_spatial_address",
    "canonical_target_floor_mapping",
    "target_ordered_placement_sequence",
    "target_collision_walkability",
    "target_presentation_footprint",
    "target_collision_footprint",
    "target_geometry_transition_endpoints",
)


class SourceBatchError(RuntimeError):
    pass


def _require_nonempty(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise SourceBatchError(f"{label} must be a non-empty string")
    return value


def validate_phase_a_summary(summary: dict[str, Any]) -> None:
    if summary.get("classification") != SOURCE_CLASSIFICATION:
        raise SourceBatchError("Phase-A source classification mismatch")
    if summary.get("phase_a_result") != "PASS":
        raise SourceBatchError("Phase-A source batch is not PASS")
    if summary.get("phase_b_target_parity") != "NOT_PERFORMED":
        raise SourceBatchError("CW2 requires retained Phase-A source evidence only")
    producer = summary.get("producer")
    source = summary.get("source")
    if not isinstance(producer, dict) or not isinstance(source, dict):
        raise SourceBatchError("Phase-A summary lacks producer/source provenance")
    for key in ("api", "code_commit", "repository"):
        _require_nonempty(producer.get(key), f"Phase-A producer.{key}")
    for key in (
        "appearance_sha256",
        "asset_zip_sha256",
        "catalog_sha256",
        "legacy_revision",
        "world_otbm_sha256",
    ):
        _require_nonempty(source.get(key), f"Phase-A source.{key}")


def load_phase_a_summary(path: str | Path) -> dict[str, Any]:
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SourceBatchError(f"cannot load Phase-A summary: {path}") from exc
    if not isinstance(value, dict):
        raise SourceBatchError("Phase-A summary root must be an object")
    validate_phase_a_summary(value)
    return value


def _field_binding_map() -> list[dict[str, str]]:
    return [
        {
            "source_field": "appearance_source_id",
            "d1_binding": "source provenance only; never canonical identity by inference",
        },
        {
            "source_field": "explicit SourceIdentityBinding.definition_*",
            "d1_binding": "TypedDefinitionRef",
        },
        {
            "source_field": "explicit SourceIdentityBinding.placement_key",
            "d1_binding": "PlacementKey independent of coordinates/order/shard",
        },
        {
            "source_field": "source_position / position",
            "d1_binding": "source coordinate provenance only; target SpatialAddress deferred",
        },
        {
            "source_field": "presentation_order",
            "d1_binding": "source ordering evidence only; target ordered placements deferred",
        },
    ]


def _phase_a_source_snapshot(summary: dict[str, Any]) -> dict[str, Any]:
    source = summary["source"]
    producer = summary["producer"]
    return {
        "phase_a_classification": summary["classification"],
        "phase_a_producer": {
            "api": producer["api"],
            "code_commit": producer["code_commit"],
            "repository": producer["repository"],
        },
        "legacy_revision": source["legacy_revision"],
        "world_otbm_sha256": source["world_otbm_sha256"],
        "appearance_sha256": source["appearance_sha256"],
        "asset_zip_sha256": source["asset_zip_sha256"],
        "catalog_sha256": source["catalog_sha256"],
    }


def build_content_source_batch(
    *,
    producer: Any,
    tile_records: list[dict[str, Any]],
    bindings: dict[str, Any],
    phase_a_summary: dict[str, Any],
    adapter_revision: str,
) -> dict[str, Any]:
    """Build a deterministic typed source batch without target promotion."""

    validate_phase_a_summary(phase_a_summary)
    _require_nonempty(adapter_revision, "adapter_revision")

    definitions: dict[tuple[str, str, str], dict[str, str]] = {}
    placements: dict[str, dict[str, Any]] = {}
    unresolved: dict[str, dict[str, Any]] = {}
    seen_occurrences: set[str] = set()
    consumed_bindings: set[str] = set()

    for tile in tile_records:
        if not isinstance(tile, dict) or tile.get("record_type") != "tile":
            raise SourceBatchError("batch input must contain projected tile records")
        presentations = tile.get("presentation")
        if not isinstance(presentations, list):
            raise SourceBatchError("projected tile lacks presentation list")

        occurrence_ids: set[str] = set()
        for presentation in presentations:
            if not isinstance(presentation, dict):
                raise SourceBatchError("projected presentation must be an object")
            record_id = presentation.get("export_record_id")
            if not isinstance(record_id, str) or not record_id:
                raise SourceBatchError("projected presentation lacks export_record_id")
            if record_id in seen_occurrences:
                raise SourceBatchError(f"duplicate source occurrence across batch: {record_id}")
            seen_occurrences.add(record_id)
            occurrence_ids.add(record_id)

        local_bindings = {
            record_id: bindings[record_id]
            for record_id in occurrence_ids
            if record_id in bindings
        }
        try:
            adapted = producer.adapt_tile_source_identities(tile, local_bindings)
        except Exception as exc:
            raise SourceBatchError(f"source identity adapter rejected tile: {exc}") from exc

        by_id = {
            presentation["export_record_id"]: presentation
            for presentation in presentations
        }
        for candidate in adapted:
            occurrence = candidate["source_occurrence_ref"]
            presentation = by_id[occurrence]
            source_context = {
                "source_occurrence_ref": occurrence,
                "appearance_source_id": candidate["appearance_source_id"],
                "source_role": candidate["source_role"],
                "source_coordinate_frame": SOURCE_COORDINATE_FRAME,
                "source_position": dict(tile.get("source_position") or {}),
                "source_native_position": dict(tile.get("position") or {}),
                "source_presentation_order": candidate["source_presentation_order"],
                "source_evidence_classification": SOURCE_CLASSIFICATION,
            }

            if candidate["identity_disposition"] != "EXPLICITLY_BOUND":
                unresolved[occurrence] = {
                    **source_context,
                    "identity_disposition": "UNRESOLVED_SOURCE_IDENTITY",
                    "typed_definition_ref": None,
                    "placement_key": None,
                }
                continue

            consumed_bindings.add(occurrence)
            typed_ref = candidate["typed_definition_ref"]
            assert isinstance(typed_ref, dict)
            identity = (
                typed_ref["definition_family"],
                typed_ref["production_key"],
                typed_ref["definition_revision"],
            )
            definitions.setdefault(
                identity,
                {
                    "definition_family": identity[0],
                    "production_key": identity[1],
                    "definition_revision": identity[2],
                },
            )

            placement_key = candidate["placement_key"]
            assert isinstance(placement_key, str)
            previous = placements.get(placement_key)
            placement = {
                "placement_key": placement_key,
                "typed_definition_ref": dict(typed_ref),
                "source_provenance": source_context,
                "target_sensitive_fields": {
                    field: "DEFERRED_REQUIRES_PHASE_B"
                    for field in DEFERRED_REQUIRES_PHASE_B
                },
            }
            if previous is not None and previous["source_provenance"]["source_occurrence_ref"] != occurrence:
                raise SourceBatchError(
                    f"duplicate PlacementKey across source occurrences: {placement_key}"
                )
            placements[placement_key] = placement

    stale_bindings = sorted(set(bindings) - consumed_bindings)
    if stale_bindings:
        raise SourceBatchError(
            f"identity binding does not match any batch occurrence: {stale_bindings[0]}"
        )

    ordered_definitions = [definitions[key] for key in sorted(definitions)]
    ordered_placements = [placements[key] for key in sorted(placements)]
    ordered_unresolved = [unresolved[key] for key in sorted(unresolved)]

    return {
        "schema": BATCH_SCHEMA,
        "identity_adapter_profile": IDENTITY_ADAPTER_PROFILE,
        "identity_adapter_revision": adapter_revision,
        "selected_batch": "bounded projected Phase-A tile/presentation source occurrences",
        "source_snapshot": _phase_a_source_snapshot(phase_a_summary),
        "field_binding_map": _field_binding_map(),
        "definitions": ordered_definitions,
        "placements": ordered_placements,
        "unresolved_source_occurrences": ordered_unresolved,
        "counts": {
            "source_tile_records": len(tile_records),
            "source_occurrences": len(seen_occurrences),
            "bound_definitions": len(ordered_definitions),
            "bound_placements": len(ordered_placements),
            "unresolved_source_occurrences": len(ordered_unresolved),
        },
        "loss_conflict_unknown_summary": {
            "unresolved_source_identity_count": len(ordered_unresolved),
            "target_geometry_promoted": False,
            "target_collision_promoted": False,
            "target_order_promoted": False,
        },
        "deferred_requires_phase_b": list(DEFERRED_REQUIRES_PHASE_B),
        "production_authority": "NONE",
        "reference_parity_claim": "NONE",
    }


def canonical_batch_bytes(batch: dict[str, Any]) -> bytes:
    return (
        json.dumps(batch, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")
