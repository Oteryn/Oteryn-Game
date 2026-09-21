#!/usr/bin/env python3
"""Project the verified Thais Z7 fixture into a renderer-friendly dense scene.

The output contains semantic IDs and layout data only. It never contains source
or decoded proprietary pixels and is intended to feed prepare-real-content.py.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
from typing import Any, Iterable

THAIS_FIXTURE_ARTIFACT = (
    "sha256:4b340053f72b3522a9fe644c9afdf08d9c7b9b686aec0b57cef764a7cb7dc468"
)
SOURCE_ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
EXPECTED_TILES = 24_311
EXPECTED_PRESENTATIONS = 39_282
EXPECTED_PRIMITIVES = 39_282
EXPECTED_UNIQUE_SPRITES = 990
DENSE_X_MIN = 32_400
DENSE_X_MAX = 32_439
DENSE_Y_MIN = 32_239
DENSE_Y_MAX = 32_268
DENSE_FLOOR = -7
EXPECTED_DENSE_TILES = (DENSE_X_MAX - DENSE_X_MIN + 1) * (DENSE_Y_MAX - DENSE_Y_MIN + 1)
EXPECTED_DENSE_PRIMITIVES = 2_215
MAX_MANIFEST_BYTES = 2 * 1024 * 1024
MAX_TILES_BYTES = 128 * 1024 * 1024
MAX_TILE_LINE_BYTES = 2 * 1024 * 1024
MAX_DIAGNOSTICS_BYTES = 8 * 1024 * 1024
ARTIFACT_DOMAIN = b"OTERYN-DYN-ATLAS-THAIS-Z7-JSONL-V0\0"
SCENE_SCHEMA = "oteryn-world-vfx-real-content-thais-scene-v1"


class SceneError(RuntimeError):
    pass


def canonical_json_bytes(value: Any) -> bytes:
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_bounded(path: Path, limit: int, label: str) -> bytes:
    try:
        size = path.stat().st_size
    except OSError as exc:
        raise SceneError(f"cannot stat {label} {path}: {exc}") from exc
    if size > limit:
        raise SceneError(f"{label} exceeds bounded size: {size} > {limit}")
    try:
        return path.read_bytes()
    except OSError as exc:
        raise SceneError(f"cannot read {label} {path}: {exc}") from exc


def compute_artifact_digest(
    manifest_core: dict[str, Any], tiles_bytes: bytes, diagnostics_bytes: bytes
) -> str:
    digest = hashlib.sha256()
    digest.update(ARTIFACT_DOMAIN)
    digest.update(canonical_json_bytes(manifest_core))
    digest.update(tiles_bytes)
    digest.update(diagnostics_bytes)
    return f"sha256:{digest.hexdigest()}"


def require_path(root: dict[str, Any], path: tuple[str, ...]) -> Any:
    value: Any = root
    for key in path:
        if not isinstance(value, dict) or key not in value:
            raise SceneError(f"missing fixture field {'.'.join(path)}")
        value = value[key]
    return value


def verify_fixture_payload(
    manifest: dict[str, Any],
    tiles_bytes: bytes,
    diagnostics_bytes: bytes,
    *,
    expected_artifact: str = THAIS_FIXTURE_ARTIFACT,
) -> None:
    if manifest.get("artifact_digest") != expected_artifact:
        raise SceneError(
            f"fixture artifact mismatch: expected {expected_artifact}, got {manifest.get('artifact_digest')}"
        )
    if require_path(manifest, ("asset_catalog_revision", "zip_sha256")) != SOURCE_ZIP_SHA256:
        raise SceneError("fixture source ZIP identity mismatch")
    expected_counts = {
        "tiles": EXPECTED_TILES,
        "presentation_records": EXPECTED_PRESENTATIONS,
        "resolved_primitives": EXPECTED_PRIMITIVES,
        "unique_sprite_source_ids": EXPECTED_UNIQUE_SPRITES,
    }
    for key, expected in expected_counts.items():
        actual = require_path(manifest, ("counts", key))
        if actual != expected:
            raise SceneError(f"fixture count {key} mismatch: expected {expected}, got {actual}")

    for name, payload in (("tiles.jsonl", tiles_bytes), ("diagnostics.json", diagnostics_bytes)):
        expected_hash = require_path(manifest, ("files", name, "sha256"))
        expected_bytes = require_path(manifest, ("files", name, "bytes"))
        actual_hash = sha256_bytes(payload)
        if expected_hash != actual_hash or expected_bytes != len(payload):
            raise SceneError(
                f"fixture {name} identity mismatch: expected {expected_hash}/{expected_bytes}, "
                f"got {actual_hash}/{len(payload)}"
            )

    manifest_core = dict(manifest)
    manifest_core.pop("artifact_digest", None)
    actual_artifact = compute_artifact_digest(manifest_core, tiles_bytes, diagnostics_bytes)
    if actual_artifact != expected_artifact:
        raise SceneError(
            f"fixture artifact recomputation mismatch: expected {expected_artifact}, got {actual_artifact}"
        )


def load_verified_fixture(fixture_dir: Path) -> tuple[dict[str, Any], bytes]:
    manifest_bytes = read_bounded(
        fixture_dir / "manifest.json", MAX_MANIFEST_BYTES, "Thais fixture manifest"
    )
    tiles_bytes = read_bounded(fixture_dir / "tiles.jsonl", MAX_TILES_BYTES, "Thais tiles")
    diagnostics_bytes = read_bounded(
        fixture_dir / "diagnostics.json", MAX_DIAGNOSTICS_BYTES, "Thais diagnostics"
    )
    try:
        manifest = json.loads(manifest_bytes)
    except json.JSONDecodeError as exc:
        raise SceneError(f"invalid Thais fixture manifest JSON: {exc}") from exc
    if not isinstance(manifest, dict):
        raise SceneError("Thais fixture manifest root must be an object")
    verify_fixture_payload(manifest, tiles_bytes, diagnostics_bytes)
    return manifest, tiles_bytes


def dense_contains(x: int, y: int, floor: int) -> bool:
    return (
        DENSE_X_MIN <= x <= DENSE_X_MAX
        and DENSE_Y_MIN <= y <= DENSE_Y_MAX
        and floor == DENSE_FLOOR
    )


def int_field(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise SceneError(f"{label} must be an integer")
    return value


def normalize_primitive(
    *,
    x: int,
    y: int,
    presentation: dict[str, Any],
    primitive: dict[str, Any],
    primitive_index: int,
) -> dict[str, Any]:
    order_record = presentation.get("presentation_order")
    if not isinstance(order_record, dict):
        raise SceneError(f"tile {x},{y} presentation has no presentation_order")
    displacement = primitive.get("displacement")
    if not isinstance(displacement, dict):
        raise SceneError(f"tile {x},{y} primitive has no displacement")
    coverage = primitive.get("visual_coverage_offsets")
    if not isinstance(coverage, list) or not coverage:
        raise SceneError(f"tile {x},{y} primitive has no visual coverage")

    normalized_coverage = []
    for offset in coverage:
        if not isinstance(offset, dict):
            raise SceneError(f"tile {x},{y} primitive has malformed visual coverage")
        normalized_coverage.append(
            {
                "dx_tiles": int_field(offset.get("dx_tiles"), "coverage.dx_tiles"),
                "dy_tiles": int_field(offset.get("dy_tiles"), "coverage.dy_tiles"),
            }
        )

    sprite_id = int_field(primitive.get("sprite_source_id"), "sprite_source_id")
    if sprite_id <= 0:
        raise SceneError(f"tile {x},{y} primitive has invalid sprite_source_id {sprite_id}")
    width = int_field(primitive.get("width_units"), "width_units")
    height = int_field(primitive.get("height_units"), "height_units")
    if (width, height) not in ((32, 32), (32, 64), (64, 32), (64, 64)):
        raise SceneError(f"tile {x},{y} primitive has unsupported geometry {width}x{height}")

    appearance_id = int_field(
        presentation.get("appearance_source_id"), "appearance_source_id"
    )
    source_role = presentation.get("source_role")
    if source_role not in ("ground", "tile_item"):
        raise SceneError(f"tile {x},{y} presentation has unsupported source_role {source_role!r}")

    return {
        "tile": {
            "world_x": x,
            "world_y": y,
            "scene_x": x - DENSE_X_MIN,
            "scene_y": y - DENSE_Y_MIN,
        },
        "appearance_source_id": appearance_id,
        "source_role": source_role,
        "presentation_order": {
            "order": int_field(order_record.get("order"), "presentation_order.order"),
            "plane": int_field(order_record.get("plane"), "presentation_order.plane"),
        },
        "primitive_index": primitive_index,
        "sprite_source_id": sprite_id,
        "source_geometry": [width, height],
        "displacement": {
            "dx_units": int_field(displacement.get("dx_units"), "displacement.dx_units"),
            "dy_units": int_field(displacement.get("dy_units"), "displacement.dy_units"),
        },
        "visual_coverage_offsets": normalized_coverage,
        "layer_index": int_field(primitive.get("layer_index"), "layer_index"),
        "phase": int_field(primitive.get("phase"), "phase"),
        "frame_group_id": int_field(primitive.get("frame_group_id"), "frame_group_id"),
        "frame_group_type": int_field(primitive.get("frame_group_type"), "frame_group_type"),
    }


def select_dense_scene(rows: Iterable[dict[str, Any]], *, strict: bool) -> dict[str, Any]:
    selected_tiles = 0
    primitives: list[dict[str, Any]] = []
    for row in rows:
        position = row.get("position")
        if not isinstance(position, dict):
            raise SceneError("tile record has no position")
        x = int_field(position.get("x"), "position.x")
        y = int_field(position.get("y"), "position.y")
        floor = int_field(position.get("floor"), "position.floor")
        if not dense_contains(x, y, floor):
            continue
        selected_tiles += 1
        presentations = row.get("presentation")
        if not isinstance(presentations, list):
            raise SceneError(f"tile {x},{y},{floor} has no presentation list")
        for presentation in presentations:
            if not isinstance(presentation, dict):
                raise SceneError(f"tile {x},{y},{floor} has malformed presentation")
            resolved = presentation.get("resolved_primitives")
            if not isinstance(resolved, list):
                raise SceneError(f"tile {x},{y},{floor} has malformed resolved_primitives")
            for index, primitive in enumerate(resolved):
                if not isinstance(primitive, dict):
                    raise SceneError(f"tile {x},{y},{floor} has malformed primitive")
                primitives.append(
                    normalize_primitive(
                        x=x,
                        y=y,
                        presentation=presentation,
                        primitive=primitive,
                        primitive_index=index,
                    )
                )

    primitives.sort(
        key=lambda primitive: (
            primitive["tile"]["scene_y"],
            primitive["tile"]["scene_x"],
            primitive["presentation_order"]["plane"],
            primitive["presentation_order"]["order"],
            primitive["layer_index"],
            primitive["primitive_index"],
        )
    )
    if strict and selected_tiles != EXPECTED_DENSE_TILES:
        raise SceneError(
            f"dense Thais viewport must contain {EXPECTED_DENSE_TILES} tile records, got {selected_tiles}"
        )
    if strict and len(primitives) != EXPECTED_DENSE_PRIMITIVES:
        raise SceneError(
            f"dense Thais viewport must contain {EXPECTED_DENSE_PRIMITIVES} primitives, got {len(primitives)}"
        )

    unique_sprites = sorted({primitive["sprite_source_id"] for primitive in primitives})
    return {
        "schema": SCENE_SCHEMA,
        "fixture_artifact": THAIS_FIXTURE_ARTIFACT,
        "viewport": {
            "floor": DENSE_FLOOR,
            "x_min": DENSE_X_MIN,
            "x_max_inclusive": DENSE_X_MAX,
            "y_min": DENSE_Y_MIN,
            "y_max_inclusive": DENSE_Y_MAX,
            "width_tiles": DENSE_X_MAX - DENSE_X_MIN + 1,
            "height_tiles": DENSE_Y_MAX - DENSE_Y_MIN + 1,
        },
        "counts": {
            "tile_records": selected_tiles,
            "primitives": len(primitives),
            "unique_sprite_source_ids": len(unique_sprites),
        },
        "sprite_source_ids": unique_sprites,
        "primitives": primitives,
        "proprietary_pixels_embedded": False,
    }


def parse_tile_rows(tiles_bytes: bytes) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(tiles_bytes.splitlines(), start=1):
        if len(line) > MAX_TILE_LINE_BYTES:
            raise SceneError(f"tiles.jsonl line {line_number} exceeds bounded size")
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise SceneError(f"invalid tiles.jsonl line {line_number}: {exc}") from exc
        if not isinstance(row, dict):
            raise SceneError(f"tiles.jsonl line {line_number} is not an object")
        rows.append(row)
    if len(rows) != EXPECTED_TILES:
        raise SceneError(f"expected {EXPECTED_TILES} fixture tile records, got {len(rows)}")
    return rows


def prepare_scene(fixture_dir: Path) -> dict[str, Any]:
    _manifest, tiles_bytes = load_verified_fixture(fixture_dir)
    return select_dense_scene(parse_tile_rows(tiles_bytes), strict=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", required=True, type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    experiment_root = Path(__file__).resolve().parent
    output = args.output or experiment_root / "target" / "thais-dense-scene.json"
    try:
        scene = prepare_scene(args.fixture_dir)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(canonical_json_bytes(scene))
    except (SceneError, OSError) as exc:
        print(f"prepare-thais-scene-error: {exc}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "output": str(output),
                "primitives": scene["counts"]["primitives"],
                "tile_records": scene["counts"]["tile_records"],
                "unique_sprite_source_ids": scene["counts"]["unique_sprite_source_ids"],
                "proprietary_pixels_embedded": False,
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
