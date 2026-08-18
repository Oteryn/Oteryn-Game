#!/usr/bin/env python3
"""Default-deny public shape policy for the DYN-ATLAS-001 proof fixture.

The proof physical profile is intentionally closed: every canonical field must be
explicitly allowlisted here. Adding a field requires an intentional policy
revision rather than silently serializing more migration/source state.
"""

from __future__ import annotations

from typing import Any

PUBLIC_POLICY_REVISION = "dyn-atlas-001-static-public-presentation-v0"


class PublicPolicyError(ValueError):
    pass


def _mapping(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise PublicPolicyError(f"{path} must be an object")
    return value


def _exact_keys(value: Any, expected: set[str], path: str) -> dict[str, Any]:
    mapping = _mapping(value, path)
    actual = set(mapping)
    if actual != expected:
        extra = sorted(actual - expected)
        missing = sorted(expected - actual)
        raise PublicPolicyError(f"{path} public-field policy mismatch; extra={extra}, missing={missing}")
    return mapping


def validate_manifest_shape(manifest: Any) -> None:
    root = _exact_keys(
        manifest,
        {
            "appearance_profile",
            "artifact_digest",
            "asset_catalog_revision",
            "capabilities",
            "content_revision",
            "contract_id",
            "coordinate_profile",
            "counts",
            "export_id",
            "export_policy_revision",
            "exporter_revision",
            "files",
            "fixture_id",
            "legacy_import_profile",
            "physical_profile",
            "producer_repository_sha",
            "required_consumer_capabilities",
            "selection",
            "semantic_revision",
            "source_provenance",
            "world_id",
            "world_schema_revision",
        },
        "manifest",
    )
    if root["export_policy_revision"] != PUBLIC_POLICY_REVISION:
        raise PublicPolicyError("manifest export_policy_revision does not match the default-deny proof policy")

    _exact_keys(
        root["asset_catalog_revision"],
        {"appearance_sha256", "catalog_sha256", "drive_file_id", "source_name", "zip_sha256"},
        "manifest.asset_catalog_revision",
    )
    _exact_keys(
        root["content_revision"],
        {"crystalserver_repository_sha", "world_otbm_sha256"},
        "manifest.content_revision",
    )
    _exact_keys(
        root["counts"],
        {
            "ground_items",
            "presentation_records",
            "resolved_primitives",
            "source_item_tree_without_ground",
            "tiles",
            "top_level_tile_items",
            "unique_appearance_source_ids",
            "unique_sprite_source_ids",
        },
        "manifest.counts",
    )
    files = _exact_keys(root["files"], {"diagnostics.json", "tiles.jsonl"}, "manifest.files")
    for file_name in ("diagnostics.json", "tiles.jsonl"):
        _exact_keys(files[file_name], {"bytes", "sha256"}, f"manifest.files.{file_name}")
    selection = _exact_keys(root["selection"], {"legacy", "native"}, "manifest.selection")
    _exact_keys(
        selection["legacy"],
        {"x_max_inclusive", "x_min", "y_max_inclusive", "y_min", "z"},
        "manifest.selection.legacy",
    )
    _exact_keys(
        selection["native"],
        {"floor", "x_max_exclusive", "x_min", "y_max_exclusive", "y_min"},
        "manifest.selection.native",
    )
    _exact_keys(
        root["source_provenance"],
        {"legacy_repository", "legacy_repository_sha", "source_kind"},
        "manifest.source_provenance",
    )


def validate_tile_shape(tile: Any, *, path: str = "tile") -> None:
    root = _exact_keys(
        tile,
        {"position", "presentation", "record_type", "source_position", "tile_record_id"},
        path,
    )
    _exact_keys(root["position"], {"floor", "x", "y"}, f"{path}.position")
    _exact_keys(
        root["source_position"],
        {"legacy_x", "legacy_y", "legacy_z"},
        f"{path}.source_position",
    )

    presentations = root["presentation"]
    if not isinstance(presentations, list):
        raise PublicPolicyError(f"{path}.presentation must be an array")
    for presentation_index, presentation in enumerate(presentations):
        presentation_path = f"{path}.presentation[{presentation_index}]"
        item = _exact_keys(
            presentation,
            {
                "appearance_source_id",
                "canonical_entity_id",
                "entity_identity_state",
                "export_record_id",
                "presentation_order",
                "resolved_primitives",
                "source_role",
            },
            presentation_path,
        )
        _exact_keys(item["presentation_order"], {"order", "plane"}, f"{presentation_path}.presentation_order")
        primitives = item["resolved_primitives"]
        if not isinstance(primitives, list):
            raise PublicPolicyError(f"{presentation_path}.resolved_primitives must be an array")
        for primitive_index, primitive in enumerate(primitives):
            primitive_path = f"{presentation_path}.resolved_primitives[{primitive_index}]"
            resolved = _exact_keys(
                primitive,
                {
                    "displacement",
                    "frame_group_id",
                    "frame_group_type",
                    "height_units",
                    "layer_index",
                    "pattern",
                    "phase",
                    "source_profile_id",
                    "sprite_source_id",
                    "visual_coverage_offsets",
                    "width_units",
                },
                primitive_path,
            )
            _exact_keys(resolved["displacement"], {"dx_units", "dy_units"}, f"{primitive_path}.displacement")
            _exact_keys(resolved["pattern"], {"x", "y", "z"}, f"{primitive_path}.pattern")
            coverage = resolved["visual_coverage_offsets"]
            if not isinstance(coverage, list):
                raise PublicPolicyError(f"{primitive_path}.visual_coverage_offsets must be an array")
            for coverage_index, offset in enumerate(coverage):
                _exact_keys(
                    offset,
                    {"dx_tiles", "dy_tiles"},
                    f"{primitive_path}.visual_coverage_offsets[{coverage_index}]",
                )
