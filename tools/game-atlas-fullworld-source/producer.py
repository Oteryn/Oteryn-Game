#!/usr/bin/env python3
"""Game-owned offline full-world Atlas source projection API.

This module broadens only the spatial selection of the already qualified
DYN-ATLAS-001 producer.  It does not define an Atlas publication format and it
does not move legacy authority into Atlas.  Pinned OTBM/Tibia inputs remain
inside the Game/import boundary; callers receive only the semantic projection
records owned by Game.
"""
from __future__ import annotations

from dataclasses import dataclass
import importlib.util
from pathlib import Path
import sys
from typing import Any, Iterator

PRODUCER_API = "oteryn-game-atlas-fullworld-source-v0"
BOUNDED_EXPORT_REL = Path("tools/game-atlas-thais-fixture/export.py")


class ProducerError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class Runtime:
    legacy_root: Path
    map_path: Path
    asset_zip: Path
    assets_dir: Path
    bounded: Any
    legacy_semantic: Any
    appearances: dict[int, Any]
    sheets: list[Any]
    sheet_for_sprite: Any


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_bounded_module() -> Any:
    path = _repository_root() / BOUNDED_EXPORT_REL
    if not path.is_file():
        raise ProducerError(f"missing qualified DYN producer implementation: {path}")
    name = "oteryn_game_atlas_qualified_dyn_producer"
    existing = sys.modules.get(name)
    if existing is not None:
        return existing
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ProducerError(f"unable to load qualified DYN producer implementation: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def load_runtime(
    *,
    legacy_root: str | Path,
    map_path: str | Path,
    asset_zip: str | Path,
    assets_dir: str | Path,
) -> Runtime:
    """Validate the exact accepted inputs and prepare immutable projection state."""
    legacy_root = Path(legacy_root).resolve()
    map_path = Path(map_path).resolve()
    asset_zip = Path(asset_zip).resolve()
    assets_dir = Path(assets_dir).resolve()
    bounded = _load_bounded_module()
    appearance_path = bounded._validate_inputs(map_path, asset_zip, assets_dir)
    legacy_assets, legacy_semantic = bounded._load_legacy_modules(legacy_root)
    appearances = legacy_assets.load_object_appearances(appearance_path)
    sheets = legacy_assets.load_sprite_catalog(assets_dir)
    return Runtime(
        legacy_root=legacy_root,
        map_path=map_path,
        asset_zip=asset_zip,
        assets_dir=assets_dir,
        bounded=bounded,
        legacy_semantic=legacy_semantic,
        appearances=appearances,
        sheets=sheets,
        sheet_for_sprite=legacy_assets.sheet_for_sprite,
    )


def iter_records(runtime: Runtime, *, strict: bool = True) -> Iterator[Any]:
    """Yield Game/import-side semantic records from the pinned source exactly once."""
    yield from runtime.legacy_semantic.iter_map_records(runtime.map_path, strict=strict)


def is_map_header(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.MapHeader)


def is_tile(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Tile)


def is_town(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Town)


def is_waypoint(runtime: Runtime, record: Any) -> bool:
    return isinstance(record, runtime.legacy_semantic.Waypoint)


def native_floor(tile: Any) -> int:
    """Apply the accepted legacy-spatial import transform for a tile."""
    return -int(tile.position.z)


def project_tile(runtime: Runtime, tile: Any) -> tuple[dict[str, Any], dict[str, Any]]:
    """Resolve one tile while preserving explicitly unresolved appearances.

    For tiles whose visible server IDs all exist in the exact pinned appearance
    catalogue, this delegates byte-for-byte to the already-qualified DYN
    producer.  A visible server ID missing from that exact catalogue remains an
    explicit unresolved presentation with no resolved primitives.  No sprite or
    appearance is substituted or inferred.
    """
    if not is_tile(runtime, tile):
        raise ProducerError(f"project_tile requires a Tile, got {type(tile)!r}")

    visible: list[tuple[str, Any]] = []
    if tile.ground is not None:
        visible.append(("ground", tile.ground))
    visible.extend(("tile_item", item) for item in tile.items)
    missing_ids = {item.server_id for _role, item in visible if item.server_id not in runtime.appearances}
    if not missing_ids:
        return runtime.bounded._tile_record(
            tile,
            appearances=runtime.appearances,
            sheets=runtime.sheets,
            sheet_for_sprite=runtime.sheet_for_sprite,
        )

    x, y, z = tile.position.x, tile.position.y, tile.position.z
    floor = native_floor(tile)
    hook_south = False
    hook_east = False
    for _role, item in visible:
        appearance = runtime.appearances.get(item.server_id)
        if appearance is None:
            continue
        hook_south = hook_south or appearance.hook_direction == 1
        hook_east = hook_east or appearance.hook_direction == 2

    presentations: list[dict[str, Any]] = []
    appearance_ids: set[int] = set()
    sprite_ids: set[int] = set()
    primitive_count = 0
    unresolved_count = 0
    for order, (role, item) in enumerate(visible):
        appearance_ids.add(item.server_id)
        appearance = runtime.appearances.get(item.server_id)
        if appearance is None:
            unresolved_count += 1
            primitives: list[dict[str, Any]] = []
            resolution_state = "UNRESOLVED_APPEARANCE"
        else:
            primitives = runtime.bounded._resolved_primitives(
                item=item,
                appearance=appearance,
                x=x,
                y=y,
                z=z,
                hook_south=hook_south,
                hook_east=hook_east,
                sheet_for_sprite=runtime.sheet_for_sprite,
                sheets=runtime.sheets,
            )
            primitive_count += len(primitives)
            sprite_ids.update(primitive["sprite_source_id"] for primitive in primitives)
            resolution_state = "RESOLVED"
        presentation = {
            "canonical_entity_id": None,
            "entity_identity_state": "UNRESOLVED",
            "export_record_id": runtime.bounded._stable_id("presentation", x, y, floor, order, item.server_id),
            "appearance_source_id": item.server_id,
            "presentation_order": {"order": order, "plane": 0},
            "resolved_primitives": primitives,
            "source_role": role,
        }
        if resolution_state != "RESOLVED":
            presentation["presentation_resolution_state"] = resolution_state
        presentations.append(presentation)

    record = {
        "position": {"floor": floor, "x": x, "y": y},
        "presentation": presentations,
        "record_type": "tile",
        "source_position": {"legacy_x": x, "legacy_y": y, "legacy_z": z},
        "tile_record_id": runtime.bounded._stable_id("tile", x, y, floor),
    }
    return record, {
        "appearance_ids": appearance_ids,
        "primitive_count": primitive_count,
        "presentation_count": len(presentations),
        "sprite_ids": sprite_ids,
        "unresolved_appearance_ids": missing_ids,
        "unresolved_presentation_count": unresolved_count,
    }


def canonical_tile_bytes(runtime: Runtime, record: dict[str, Any]) -> bytes:
    """Encode one semantic tile identically to the qualified DYN producer."""
    data = runtime.bounded._canonical_json_bytes(record)
    if len(data) > int(runtime.bounded.MAX_TILE_LINE_BYTES):
        raise ProducerError(f"semantic tile record exceeds qualified per-tile cap: {len(data)} bytes")
    return data


def project_tile_bytes(runtime: Runtime, tile: Any) -> tuple[bytes, dict[str, Any]]:
    record, stats = project_tile(runtime, tile)
    return canonical_tile_bytes(runtime, record), stats
