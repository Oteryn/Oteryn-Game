#!/usr/bin/env python3
"""Fail-closed verifier for the proof-only DYN-ATLAS-001 Thais fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any, Iterator

from public_policy import validate_manifest_shape, validate_tile_shape

CONTRACT_ID = "oteryn-game-atlas-export-v1"
PHYSICAL_PROFILE = "dyn-atlas-thais-z7-jsonl-v0"
FIXTURE_ID = "dyn-atlas-001-semantic-thais-z7-v0"
COORDINATE_PROFILE = "oteryn-world-spatial-v1"
LEGACY_IMPORT_PROFILE = "oteryn-crystalserver-legacy-spatial-import-v1"
APPEARANCE_PROFILE = "oteryn-atlas-15-32-appearance-spatial-v1"

X_MIN = 32280
X_MAX_EXCLUSIVE = 32441
Y_MIN = 32155
Y_MAX_EXCLUSIVE = 32306
FLOOR = -7
EXPECTED_TILE_COUNT = (X_MAX_EXCLUSIVE - X_MIN) * (Y_MAX_EXCLUSIVE - Y_MIN)

MAX_PRESENTATIONS_PER_TILE = 512
MAX_PRIMITIVES_PER_PRESENTATION = 2048
MAX_TILE_LINE_BYTES = 1_048_576
MAX_CANONICAL_BYTES = 128 * 1024 * 1024


class VerifyError(RuntimeError):
    pass


def _canonical_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _coverage(width_units: int, height_units: int) -> list[dict[str, int]]:
    if width_units <= 0 or height_units <= 0 or width_units % 32 or height_units % 32:
        raise VerifyError(f"unsupported sprite dimensions {width_units}x{height_units}")
    width_tiles = width_units // 32
    height_tiles = height_units // 32
    return [
        {"dx_tiles": dx, "dy_tiles": dy}
        for dy in range(-(height_tiles - 1), 1)
        for dx in range(-(width_tiles - 1), 1)
    ]


def _load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise VerifyError(f"unable to decode {path.name}: {exc}") from exc


def _iter_jsonl(path: Path) -> Iterator[tuple[int, bytes, Any]]:
    with path.open("rb") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.endswith(b"\n"):
                raise VerifyError(f"tiles.jsonl line {line_number} lacks LF terminator")
            if len(line) > MAX_TILE_LINE_BYTES:
                raise VerifyError(f"tiles.jsonl line {line_number} exceeds proof cap")
            try:
                decoded = line.decode("utf-8")
                value = json.loads(decoded)
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise VerifyError(f"invalid tiles.jsonl line {line_number}: {exc}") from exc
            if _canonical_json_bytes(value) != line:
                raise VerifyError(f"tiles.jsonl line {line_number} is not canonical JSON")
            yield line_number, line, value


def verify(root: Path) -> dict[str, Any]:
    manifest_path = root / "manifest.json"
    tiles_path = root / "tiles.jsonl"
    diagnostics_path = root / "diagnostics.json"
    artifact_sha_path = root / "artifact.sha256"
    for path in (manifest_path, tiles_path, diagnostics_path, artifact_sha_path):
        if not path.is_file():
            raise VerifyError(f"missing required fixture file {path.name}")

    manifest_bytes = manifest_path.read_bytes()
    tiles_bytes = tiles_path.read_bytes()
    diagnostics_bytes = diagnostics_path.read_bytes()
    if len(manifest_bytes) + len(tiles_bytes) + len(diagnostics_bytes) > MAX_CANONICAL_BYTES:
        raise VerifyError("fixture exceeds proof-only canonical byte cap")

    manifest = _load_json(manifest_path)
    diagnostics = _load_json(diagnostics_path)
    if _canonical_json_bytes(manifest) != manifest_bytes:
        raise VerifyError("manifest.json is not canonical JSON")
    if _canonical_json_bytes(diagnostics) != diagnostics_bytes:
        raise VerifyError("diagnostics.json is not canonical JSON")
    if diagnostics != {"diagnostics": []}:
        raise VerifyError("proof fixture must contain zero diagnostics")

    # The proof projection is default-deny: any extra canonical field is a policy
    # violation, even if it is otherwise valid JSON or ignored by the consumer.
    validate_manifest_shape(manifest)

    expected_manifest = {
        "contract_id": CONTRACT_ID,
        "physical_profile": PHYSICAL_PROFILE,
        "fixture_id": FIXTURE_ID,
        "coordinate_profile": COORDINATE_PROFILE,
        "legacy_import_profile": LEGACY_IMPORT_PROFILE,
        "appearance_profile": APPEARANCE_PROFILE,
        "semantic_revision": 1,
    }
    for key, expected in expected_manifest.items():
        if manifest.get(key) != expected:
            raise VerifyError(f"manifest {key} mismatch: {manifest.get(key)!r} != {expected!r}")

    producer_sha = manifest.get("producer_repository_sha")
    if not isinstance(producer_sha, str) or re.fullmatch(r"[0-9a-f]{40}", producer_sha) is None:
        raise VerifyError("producer_repository_sha is not a full lowercase SHA")
    if manifest.get("exporter_revision") != producer_sha:
        raise VerifyError("exporter_revision must equal exact producer_repository_sha for this proof")

    required = manifest.get("required_consumer_capabilities")
    capabilities = manifest.get("capabilities")
    expected_caps = ["resolved-appearance-primitives-v0", "semantic-tiles-v0"]
    if capabilities != expected_caps or required != expected_caps:
        raise VerifyError("proof capability declaration mismatch")

    selection = manifest.get("selection", {})
    if selection.get("native") != {
        "floor": FLOOR,
        "x_max_exclusive": X_MAX_EXCLUSIVE,
        "x_min": X_MIN,
        "y_max_exclusive": Y_MAX_EXCLUSIVE,
        "y_min": Y_MIN,
    }:
        raise VerifyError("native selection mismatch")
    if selection.get("legacy") != {
        "x_max_inclusive": X_MAX_EXCLUSIVE - 1,
        "x_min": X_MIN,
        "y_max_inclusive": Y_MAX_EXCLUSIVE - 1,
        "y_min": Y_MIN,
        "z": 7,
    }:
        raise VerifyError("legacy selection mismatch")

    files = manifest.get("files")
    if not isinstance(files, dict):
        raise VerifyError("manifest files section is missing")
    expected_file_facts = {
        "tiles.jsonl": {"bytes": len(tiles_bytes), "sha256": _sha256(tiles_bytes)},
        "diagnostics.json": {"bytes": len(diagnostics_bytes), "sha256": _sha256(diagnostics_bytes)},
    }
    if files != expected_file_facts:
        raise VerifyError("manifest file digest/size facts mismatch")

    artifact_digest = manifest.get("artifact_digest")
    if not isinstance(artifact_digest, str) or re.fullmatch(r"sha256:[0-9a-f]{64}", artifact_digest) is None:
        raise VerifyError("artifact_digest must be sha256:<64 hex>")
    manifest_core = dict(manifest)
    manifest_core.pop("artifact_digest")
    artifact_hash = hashlib.sha256()
    artifact_hash.update(b"OTERYN-DYN-ATLAS-THAIS-Z7-JSONL-V0\0")
    artifact_hash.update(_canonical_json_bytes(manifest_core))
    artifact_hash.update(tiles_bytes)
    artifact_hash.update(diagnostics_bytes)
    expected_artifact_digest = artifact_hash.hexdigest()
    if artifact_digest != f"sha256:{expected_artifact_digest}":
        raise VerifyError("artifact digest mismatch")
    if artifact_sha_path.read_text(encoding="ascii").strip() != expected_artifact_digest:
        raise VerifyError("artifact.sha256 mismatch")

    forbidden_nondeterministic_keys = {"generated_at", "timestamp", "runner_id", "workspace_path", "hostname"}
    if forbidden_nondeterministic_keys.intersection(manifest):
        raise VerifyError("canonical manifest contains nondeterministic operational metadata")

    tile_ids: set[str] = set()
    presentation_ids: set[str] = set()
    appearance_ids: set[int] = set()
    sprite_ids: set[int] = set()
    presentation_count = 0
    primitive_count = 0
    ground_count = 0
    top_level_item_count = 0
    expected_positions = ((x, y) for y in range(Y_MIN, Y_MAX_EXCLUSIVE) for x in range(X_MIN, X_MAX_EXCLUSIVE))

    line_count = 0
    for (expected_x, expected_y), (line_number, _line, tile) in zip(expected_positions, _iter_jsonl(tiles_path), strict=True):
        line_count += 1
        validate_tile_shape(tile, path=f"tiles.jsonl line {line_number}")
        if not isinstance(tile, dict) or tile.get("record_type") != "tile":
            raise VerifyError(f"line {line_number}: expected tile record")
        if tile.get("position") != {"floor": FLOOR, "x": expected_x, "y": expected_y}:
            raise VerifyError(f"line {line_number}: unexpected tile ordering/position")
        if tile.get("source_position") != {"legacy_x": expected_x, "legacy_y": expected_y, "legacy_z": 7}:
            raise VerifyError(f"line {line_number}: source/native position mapping mismatch")

        tile_id = tile.get("tile_record_id")
        if not isinstance(tile_id, str) or not tile_id.startswith("tile:") or tile_id in tile_ids:
            raise VerifyError(f"line {line_number}: invalid/duplicate tile_record_id")
        tile_ids.add(tile_id)

        presentations = tile.get("presentation")
        if not isinstance(presentations, list) or len(presentations) > MAX_PRESENTATIONS_PER_TILE:
            raise VerifyError(f"line {line_number}: invalid presentation list")

        ground_seen = False
        for order, presentation in enumerate(presentations):
            if not isinstance(presentation, dict):
                raise VerifyError(f"line {line_number}: presentation must be an object")
            if presentation.get("presentation_order") != {"order": order, "plane": 0}:
                raise VerifyError(f"line {line_number}: presentation order is not explicit/source-stable")
            record_id = presentation.get("export_record_id")
            if not isinstance(record_id, str) or not record_id.startswith("presentation:") or record_id in presentation_ids:
                raise VerifyError(f"line {line_number}: invalid/duplicate presentation record id")
            presentation_ids.add(record_id)
            if presentation.get("canonical_entity_id") is not None or presentation.get("entity_identity_state") != "UNRESOLVED":
                raise VerifyError(f"line {line_number}: proof must not invent canonical Game entity identity")

            appearance_id = presentation.get("appearance_source_id")
            if not isinstance(appearance_id, int) or appearance_id < 0:
                raise VerifyError(f"line {line_number}: invalid appearance_source_id")
            appearance_ids.add(appearance_id)

            role = presentation.get("source_role")
            if role == "ground":
                if order != 0 or ground_seen:
                    raise VerifyError(f"line {line_number}: ground presentation ordering invalid")
                ground_seen = True
                ground_count += 1
            elif role == "tile_item":
                top_level_item_count += 1
            else:
                raise VerifyError(f"line {line_number}: unknown source_role {role!r}")

            primitives = presentation.get("resolved_primitives")
            if not isinstance(primitives, list) or not primitives or len(primitives) > MAX_PRIMITIVES_PER_PRESENTATION:
                raise VerifyError(f"line {line_number}: invalid resolved_primitives")
            observed_layers: list[int] = []
            for primitive in primitives:
                if not isinstance(primitive, dict):
                    raise VerifyError(f"line {line_number}: primitive must be an object")
                if primitive.get("source_profile_id") != APPEARANCE_PROFILE:
                    raise VerifyError(f"line {line_number}: primitive source profile mismatch")
                layer_index = primitive.get("layer_index")
                if not isinstance(layer_index, int) or layer_index < 0:
                    raise VerifyError(f"line {line_number}: invalid layer index")
                observed_layers.append(layer_index)
                sprite_id = primitive.get("sprite_source_id")
                if not isinstance(sprite_id, int) or sprite_id < 0:
                    raise VerifyError(f"line {line_number}: invalid sprite_source_id")
                sprite_ids.add(sprite_id)
                width_units = primitive.get("width_units")
                height_units = primitive.get("height_units")
                if not isinstance(width_units, int) or not isinstance(height_units, int):
                    raise VerifyError(f"line {line_number}: primitive dimensions must be integers")
                if primitive.get("visual_coverage_offsets") != _coverage(width_units, height_units):
                    raise VerifyError(f"line {line_number}: visual coverage does not match decoded dimensions")
                displacement = primitive.get("displacement")
                if not isinstance(displacement, dict) or set(displacement) != {"dx_units", "dy_units"}:
                    raise VerifyError(f"line {line_number}: invalid displacement")
                if not all(isinstance(displacement[key], int) for key in ("dx_units", "dy_units")):
                    raise VerifyError(f"line {line_number}: displacement values must be integers")
                pattern = primitive.get("pattern")
                if not isinstance(pattern, dict) or set(pattern) != {"x", "y", "z"}:
                    raise VerifyError(f"line {line_number}: explicit pattern selection missing")
                if not all(isinstance(pattern[key], int) and pattern[key] >= 0 for key in ("x", "y", "z")):
                    raise VerifyError(f"line {line_number}: invalid pattern selection")
                if not isinstance(primitive.get("phase"), int) or primitive["phase"] < 0:
                    raise VerifyError(f"line {line_number}: invalid phase")
                primitive_count += 1
            if observed_layers != list(range(len(observed_layers))):
                raise VerifyError(f"line {line_number}: appearance layers are not explicit deterministic order")
            presentation_count += 1

    if line_count != EXPECTED_TILE_COUNT:
        raise VerifyError(f"expected {EXPECTED_TILE_COUNT} tile lines, got {line_count}")

    counts = manifest.get("counts")
    if not isinstance(counts, dict):
        raise VerifyError("manifest counts section missing")
    expected_counts = {
        "ground_items": ground_count,
        "presentation_records": presentation_count,
        "resolved_primitives": primitive_count,
        "tiles": line_count,
        "top_level_tile_items": top_level_item_count,
        "unique_appearance_source_ids": len(appearance_ids),
        "unique_sprite_source_ids": len(sprite_ids),
    }
    for key, value in expected_counts.items():
        if counts.get(key) != value:
            raise VerifyError(f"manifest count {key} mismatch: {counts.get(key)!r} != {value!r}")
    if not isinstance(counts.get("source_item_tree_without_ground"), int) or counts["source_item_tree_without_ground"] < top_level_item_count:
        raise VerifyError("source item tree count is invalid")

    return {
        "artifact_digest": artifact_digest,
        "canonical_bytes": len(manifest_bytes) + len(tiles_bytes) + len(diagnostics_bytes),
        "presentation_records": presentation_count,
        "resolved_primitives": primitive_count,
        "tiles": line_count,
        "unique_appearance_source_ids": len(appearance_ids),
        "unique_sprite_source_ids": len(sprite_ids),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("fixture", type=Path)
    args = parser.parse_args()
    try:
        result = verify(args.fixture)
    except (VerifyError, OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
