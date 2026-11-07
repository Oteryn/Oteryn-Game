#!/usr/bin/env python3
"""Produce the bounded Game-owned DYN-ATLAS-001 Thais Z7 semantic fixture.

This tool is a proof/migration producer. OTBM and Tibia asset files are accepted
only on the producer/import side. The output is an explicit semantic projection;
Atlas must never reopen the legacy inputs to reinterpret missing semantics.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
from typing import Any, Iterable

CONTRACT_ID = "oteryn-game-atlas-export-v1"
SEMANTIC_REVISION = 1
PHYSICAL_PROFILE = "dyn-atlas-thais-z7-jsonl-v0"
FIXTURE_ID = "dyn-atlas-001-semantic-thais-z7-v0"
COORDINATE_PROFILE = "oteryn-world-spatial-v1"
LEGACY_IMPORT_PROFILE = "oteryn-crystalserver-legacy-spatial-import-v1"
APPEARANCE_PROFILE = "oteryn-atlas-15-32-appearance-spatial-v1"

LEGACY_REPOSITORY = "blakinio/Otheryn"
LEGACY_REPOSITORY_SHA = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
CRYSTALSERVER_SHA = "5e89bf8329ea406cb4ea8f4a18f32954f13e5418"
MAP_SHA256 = "3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034"

ASSET_DRIVE_FILE_ID = "1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv"
ASSET_ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
ASSET_CATALOG_SHA256 = "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85"
ASSET_APPEARANCE_SHA256 = "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075"

LEGACY_X1 = 32280
LEGACY_X2 = 32440
LEGACY_Y1 = 32155
LEGACY_Y2 = 32305
LEGACY_Z = 7
NATIVE_FLOOR = -7
EXPECTED_TILE_COUNT = (LEGACY_X2 - LEGACY_X1 + 1) * (LEGACY_Y2 - LEGACY_Y1 + 1)

# Proof-harness limits only. These are deliberately not production resource limits.
MAX_PRESENTATIONS_PER_TILE = 512
MAX_PRIMITIVES_PER_PRESENTATION = 2048
MAX_TILE_LINE_BYTES = 1_048_576
MAX_CANONICAL_BYTES = 128 * 1024 * 1024

_MODERN_FLUID_COLORS = (0, 1, 7, 3, 3, 2, 4, 3, 5, 6, 7, 2, 5, 3, 5, 6, 3, 3, 8, 10, 9)


class ExportError(RuntimeError):
    pass


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _canonical_json_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _stable_id(domain: str, *parts: object) -> str:
    payload = "\0".join([domain, *(str(part) for part in parts)]).encode("utf-8")
    return f"{domain}:{hashlib.sha256(payload).hexdigest()[:32]}"


def _proof_export_id(producer_sha: str) -> str:
    return _stable_id(
        "dyn-atlas-export",
        FIXTURE_ID,
        producer_sha,
        MAP_SHA256,
        ASSET_ZIP_SHA256,
        LEGACY_X1,
        LEGACY_X2,
        LEGACY_Y1,
        LEGACY_Y2,
        LEGACY_Z,
    )


def _item_patterns(
    appearance: Any,
    frame: Any,
    item: Any,
    x: int,
    y: int,
    z: int,
    hook_south: bool,
    hook_east: bool,
) -> tuple[int, int, int]:
    """Resolve the pinned migration-source item pattern under Game ownership."""
    if frame.pattern_width <= 0 or frame.pattern_height <= 0 or frame.pattern_depth <= 0:
        raise ExportError(f"invalid pattern dimensions for appearance {appearance.appearance_id}")

    px = x % frame.pattern_width
    py = y % frame.pattern_height
    pz = z % frame.pattern_depth

    if appearance.stackable and frame.pattern_width == 4 and frame.pattern_height == 2:
        count = item.subtype or 0
        if count <= 0:
            px, py = 0, 0
        elif count < 5:
            px, py = count - 1, 0
        elif count < 10:
            px, py = 0, 1
        elif count < 25:
            px, py = 1, 1
        elif count < 50:
            px, py = 2, 1
        else:
            px, py = 3, 1
        pz = 0
    elif appearance.hangable:
        px = 1 if hook_south and frame.pattern_width >= 2 else 2 if hook_east and frame.pattern_width >= 3 else 0
        py = 0
        pz = 0
    elif appearance.splash or appearance.fluid_container:
        subtype = item.subtype or 0
        color = _MODERN_FLUID_COLORS[subtype] if 0 <= subtype < len(_MODERN_FLUID_COLORS) else 0
        px = (color % 4) % frame.pattern_width
        py = (color // 4) % frame.pattern_height
        pz = 0

    return px, py, pz


def _coverage(width_units: int, height_units: int) -> list[dict[str, int]]:
    if width_units <= 0 or height_units <= 0 or width_units % 32 or height_units % 32:
        raise ExportError(f"unsupported sprite dimensions {width_units}x{height_units}")
    width_tiles = width_units // 32
    height_tiles = height_units // 32
    return [
        {"dx_tiles": dx, "dy_tiles": dy}
        for dy in range(-(height_tiles - 1), 1)
        for dx in range(-(width_tiles - 1), 1)
    ]


def _load_legacy_modules(legacy_root: Path) -> tuple[Any, Any]:
    resolved = str(legacy_root.resolve())
    if resolved not in sys.path:
        sys.path.insert(0, resolved)
    try:
        from tools.otbm_atlas import assets as legacy_assets  # type: ignore
        from tools.otbm_atlas import semantic as legacy_semantic  # type: ignore
    except Exception as exc:  # pragma: no cover - integration path
        raise ExportError(f"unable to import pinned migration parser from {legacy_root}: {exc}") from exc
    return legacy_assets, legacy_semantic


def _validate_inputs(map_path: Path, asset_zip: Path, assets_dir: Path) -> Path:
    if _sha256_file(map_path) != MAP_SHA256:
        raise ExportError("canonical world.otbm SHA-256 mismatch")
    if _sha256_file(asset_zip) != ASSET_ZIP_SHA256:
        raise ExportError("15.32 asset ZIP SHA-256 mismatch")

    catalog = assets_dir / "catalog-content.json"
    if _sha256_file(catalog) != ASSET_CATALOG_SHA256:
        raise ExportError("15.32 asset catalog SHA-256 mismatch")

    appearances = sorted(assets_dir.glob("appearances-*.dat"))
    if len(appearances) != 1:
        raise ExportError(f"expected exactly one appearances-*.dat, got {len(appearances)}")
    if _sha256_file(appearances[0]) != ASSET_APPEARANCE_SHA256:
        raise ExportError("15.32 appearance SHA-256 mismatch")
    return appearances[0]


def _resolved_primitives(
    *,
    item: Any,
    appearance: Any,
    x: int,
    y: int,
    z: int,
    hook_south: bool,
    hook_east: bool,
    sheet_for_sprite: Any,
    sheets: list[Any],
) -> list[dict[str, Any]]:
    if not appearance.frames:
        raise ExportError(f"appearance {appearance.appearance_id} has no frame records")

    frame = appearance.frames[0]
    px, py, pz = _item_patterns(appearance, frame, item, x, y, z, hook_south, hook_east)
    phase = frame.default_start_phase % frame.animation_phases
    shift_x, shift_y = appearance.shift or (0, 0)
    height = appearance.height or 0
    displacement = {"dx_units": -(shift_x + height), "dy_units": -(shift_y + height)}

    primitives: list[dict[str, Any]] = []
    for layer in range(frame.layers):
        index = ((((phase * frame.pattern_depth + pz) * frame.pattern_height + py) * frame.pattern_width + px) * frame.layers + layer)
        if index >= len(frame.sprite_ids):
            raise ExportError(
                f"appearance {appearance.appearance_id} layer selection index {index} exceeds {len(frame.sprite_ids)} sprite refs"
            )
        sprite_id = frame.sprite_ids[index]
        sheet = sheet_for_sprite(sheets, sprite_id)
        if sheet is None:
            raise ExportError(f"appearance {appearance.appearance_id} references unknown sprite {sprite_id}")
        width_units, height_units = sheet.sprite_size
        primitives.append(
            {
                "displacement": displacement,
                "frame_group_id": frame.frame_group_id,
                "frame_group_type": frame.frame_group_type,
                "height_units": height_units,
                "layer_index": layer,
                "pattern": {"x": px, "y": py, "z": pz},
                "phase": phase,
                "source_profile_id": APPEARANCE_PROFILE,
                "sprite_source_id": sprite_id,
                "visual_coverage_offsets": _coverage(width_units, height_units),
                "width_units": width_units,
            }
        )

    if len(primitives) > MAX_PRIMITIVES_PER_PRESENTATION:
        raise ExportError(f"appearance {appearance.appearance_id} exceeds proof primitive cap")
    return primitives


def _tile_record(
    tile: Any,
    *,
    appearances: dict[int, Any],
    sheets: list[Any],
    sheet_for_sprite: Any,
) -> tuple[dict[str, Any], dict[str, int | set[int]]]:
    x, y, z = tile.position.x, tile.position.y, tile.position.z
    floor = -z

    visible: list[tuple[str, Any]] = []
    if tile.ground is not None:
        visible.append(("ground", tile.ground))
    visible.extend(("tile_item", item) for item in tile.items)

    if len(visible) > MAX_PRESENTATIONS_PER_TILE:
        raise ExportError(f"tile {x},{y},{z} exceeds proof presentation cap")

    hook_south = False
    hook_east = False
    for _role, item in visible:
        appearance = appearances.get(item.server_id)
        if appearance is None:
            raise ExportError(f"missing appearance {item.server_id} at {x},{y},{z}")
        hook_south = hook_south or appearance.hook_direction == 1
        hook_east = hook_east or appearance.hook_direction == 2

    presentations: list[dict[str, Any]] = []
    appearance_ids: set[int] = set()
    sprite_ids: set[int] = set()
    primitive_count = 0

    for order, (role, item) in enumerate(visible):
        appearance = appearances[item.server_id]
        appearance_ids.add(item.server_id)
        primitives = _resolved_primitives(
            item=item,
            appearance=appearance,
            x=x,
            y=y,
            z=z,
            hook_south=hook_south,
            hook_east=hook_east,
            sheet_for_sprite=sheet_for_sprite,
            sheets=sheets,
        )
        primitive_count += len(primitives)
        sprite_ids.update(primitive["sprite_source_id"] for primitive in primitives)

        presentations.append(
            {
                "canonical_entity_id": None,
                "entity_identity_state": "UNRESOLVED",
                "export_record_id": _stable_id("presentation", x, y, floor, order, item.server_id),
                "appearance_source_id": item.server_id,
                "presentation_order": {"order": order, "plane": 0},
                "resolved_primitives": primitives,
                "source_role": role,
            }
        )

    record = {
        "position": {"floor": floor, "x": x, "y": y},
        "presentation": presentations,
        "record_type": "tile",
        "source_position": {"legacy_x": x, "legacy_y": y, "legacy_z": z},
        "tile_record_id": _stable_id("tile", x, y, floor),
    }
    return record, {
        "appearance_ids": appearance_ids,
        "primitive_count": primitive_count,
        "presentation_count": len(presentations),
        "sprite_ids": sprite_ids,
    }


def produce(
    *,
    legacy_root: Path,
    map_path: Path,
    asset_zip: Path,
    assets_dir: Path,
    producer_sha: str,
    output_dir: Path,
) -> dict[str, Any]:
    if re.fullmatch(r"[0-9a-f]{40}", producer_sha) is None:
        raise ExportError("producer SHA must be a full lowercase 40-character commit SHA")

    appearance_path = _validate_inputs(map_path, asset_zip, assets_dir)
    legacy_assets, legacy_semantic = _load_legacy_modules(legacy_root)
    appearances = legacy_assets.load_object_appearances(appearance_path)
    sheets = legacy_assets.load_sprite_catalog(assets_dir)

    selected_tiles: list[Any] = []
    source_tree_items_without_ground = 0
    for record in legacy_semantic.iter_map_records(map_path, strict=True):
        if not isinstance(record, legacy_semantic.Tile):
            continue
        position = record.position
        if position.z != LEGACY_Z:
            continue
        if not (LEGACY_X1 <= position.x <= LEGACY_X2 and LEGACY_Y1 <= position.y <= LEGACY_Y2):
            continue
        selected_tiles.append(record)
        source_tree_items_without_ground += sum(1 for _ in legacy_semantic.walk_items(record.items))

    selected_tiles.sort(key=lambda tile: (tile.position.y, tile.position.x))
    if len(selected_tiles) != EXPECTED_TILE_COUNT:
        raise ExportError(f"expected {EXPECTED_TILE_COUNT} selected tiles, got {len(selected_tiles)}")

    tile_lines: list[bytes] = []
    appearance_ids: set[int] = set()
    sprite_ids: set[int] = set()
    presentation_count = 0
    primitive_count = 0
    ground_count = 0
    top_level_item_count = 0

    for tile in selected_tiles:
        if tile.ground is not None:
            ground_count += 1
        top_level_item_count += len(tile.items)
        record, stats = _tile_record(
            tile,
            appearances=appearances,
            sheets=sheets,
            sheet_for_sprite=legacy_assets.sheet_for_sprite,
        )
        line = _canonical_json_bytes(record)
        if len(line) > MAX_TILE_LINE_BYTES:
            raise ExportError(f"tile line exceeds proof cap at {tile.position.x},{tile.position.y},{tile.position.z}")
        tile_lines.append(line)
        presentation_count += int(stats["presentation_count"])
        primitive_count += int(stats["primitive_count"])
        appearance_ids.update(stats["appearance_ids"])  # type: ignore[arg-type]
        sprite_ids.update(stats["sprite_ids"])  # type: ignore[arg-type]

    tiles_bytes = b"".join(tile_lines)
    diagnostics_bytes = _canonical_json_bytes({"diagnostics": []})
    if len(tiles_bytes) + len(diagnostics_bytes) > MAX_CANONICAL_BYTES:
        raise ExportError("canonical proof artifact exceeds proof-only size cap")

    tiles_sha = hashlib.sha256(tiles_bytes).hexdigest()
    diagnostics_sha = hashlib.sha256(diagnostics_bytes).hexdigest()

    manifest_core: dict[str, Any] = {
        "appearance_profile": APPEARANCE_PROFILE,
        "asset_catalog_revision": {
            "appearance_sha256": ASSET_APPEARANCE_SHA256,
            "catalog_sha256": ASSET_CATALOG_SHA256,
            "drive_file_id": ASSET_DRIVE_FILE_ID,
            "source_name": "15.32.zip",
            "zip_sha256": ASSET_ZIP_SHA256,
        },
        "capabilities": ["resolved-appearance-primitives-v0", "semantic-tiles-v0"],
        "content_revision": {
            "crystalserver_repository_sha": CRYSTALSERVER_SHA,
            "world_otbm_sha256": MAP_SHA256,
        },
        "contract_id": CONTRACT_ID,
        "coordinate_profile": COORDINATE_PROFILE,
        "counts": {
            "ground_items": ground_count,
            "presentation_records": presentation_count,
            "resolved_primitives": primitive_count,
            "source_item_tree_without_ground": source_tree_items_without_ground,
            "tiles": len(selected_tiles),
            "top_level_tile_items": top_level_item_count,
            "unique_appearance_source_ids": len(appearance_ids),
            "unique_sprite_source_ids": len(sprite_ids),
        },
        "export_id": _proof_export_id(producer_sha),
        "export_policy_revision": "dyn-atlas-001-static-public-presentation-v0",
        "exporter_revision": producer_sha,
        "files": {
            "diagnostics.json": {"bytes": len(diagnostics_bytes), "sha256": diagnostics_sha},
            "tiles.jsonl": {"bytes": len(tiles_bytes), "sha256": tiles_sha},
        },
        "fixture_id": FIXTURE_ID,
        "legacy_import_profile": LEGACY_IMPORT_PROFILE,
        "physical_profile": PHYSICAL_PROFILE,
        "producer_repository_sha": producer_sha,
        "required_consumer_capabilities": ["resolved-appearance-primitives-v0", "semantic-tiles-v0"],
        "selection": {
            "legacy": {"x_max_inclusive": LEGACY_X2, "x_min": LEGACY_X1, "y_max_inclusive": LEGACY_Y2, "y_min": LEGACY_Y1, "z": LEGACY_Z},
            "native": {"floor": NATIVE_FLOOR, "x_max_exclusive": LEGACY_X2 + 1, "x_min": LEGACY_X1, "y_max_exclusive": LEGACY_Y2 + 1, "y_min": LEGACY_Y1},
        },
        "semantic_revision": SEMANTIC_REVISION,
        "source_provenance": {
            "legacy_repository": LEGACY_REPOSITORY,
            "legacy_repository_sha": LEGACY_REPOSITORY_SHA,
            "source_kind": "bounded-legacy-import-proof",
        },
        "world_id": "oteryn-proof:crystalserver-thais-z7",
        "world_schema_revision": "dyn-atlas-migration-proof-v0",
    }

    manifest_core_bytes = _canonical_json_bytes(manifest_core)
    artifact_hash = hashlib.sha256()
    artifact_hash.update(b"OTERYN-DYN-ATLAS-THAIS-Z7-JSONL-V0\0")
    artifact_hash.update(manifest_core_bytes)
    artifact_hash.update(tiles_bytes)
    artifact_hash.update(diagnostics_bytes)
    artifact_digest = artifact_hash.hexdigest()

    manifest = dict(manifest_core)
    manifest["artifact_digest"] = f"sha256:{artifact_digest}"
    manifest_bytes = _canonical_json_bytes(manifest)

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "manifest.json").write_bytes(manifest_bytes)
    (output_dir / "tiles.jsonl").write_bytes(tiles_bytes)
    (output_dir / "diagnostics.json").write_bytes(diagnostics_bytes)
    (output_dir / "artifact.sha256").write_text(f"{artifact_digest}\n", encoding="ascii", newline="\n")

    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--legacy-root", type=Path, required=True)
    parser.add_argument("--map", dest="map_path", type=Path, required=True)
    parser.add_argument("--asset-zip", type=Path, required=True)
    parser.add_argument("--assets", dest="assets_dir", type=Path, required=True)
    parser.add_argument("--producer-sha", required=True)
    parser.add_argument("--output", dest="output_dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        manifest = produce(
            legacy_root=args.legacy_root,
            map_path=args.map_path,
            asset_zip=args.asset_zip,
            assets_dir=args.assets_dir,
            producer_sha=args.producer_sha,
            output_dir=args.output_dir,
        )
    except (ExportError, OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
