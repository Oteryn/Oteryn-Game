#!/usr/bin/env python3
"""Game-owned full-world projection enriched with explicit animation refs."""
from __future__ import annotations

from dataclasses import dataclass
import importlib.util
from pathlib import Path
import sys
from typing import Any, Iterator

HERE = Path(__file__).resolve().parent
BASE_PATH = HERE / "producer.py"
APPEARANCE_PATH = HERE.parent / "game-atlas-appearances" / "export.py"
PRODUCER_API = "oteryn-game-atlas-fullworld-animated-source-v1"


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"unable to load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _mods():
    return _load(BASE_PATH, "oteryn_game_atlas_fullworld_base"), _load(APPEARANCE_PATH, "oteryn_game_atlas_appearance_product_fullworld")


@dataclass(frozen=True, slots=True)
class AnimatedRuntime:
    base: Any
    appearance_product: Path
    base_module: Any
    appearance_module: Any


def load_runtime(*, legacy_root, map_path, asset_zip, assets_dir, appearance_product) -> AnimatedRuntime:
    base, appearances = _mods()
    product = Path(appearance_product).resolve()
    appearances.load_program_indexes(product)
    runtime = base.load_runtime(legacy_root=legacy_root, map_path=map_path, asset_zip=asset_zip, assets_dir=assets_dir)
    return AnimatedRuntime(runtime, product, base, appearances)


def iter_records(runtime: AnimatedRuntime, *, strict: bool = True) -> Iterator[Any]:
    yield from runtime.base_module.iter_records(runtime.base, strict=strict)


def is_map_header(runtime: AnimatedRuntime, record: Any) -> bool:
    return runtime.base_module.is_map_header(runtime.base, record)


def is_tile(runtime: AnimatedRuntime, record: Any) -> bool:
    return runtime.base_module.is_tile(runtime.base, record)


def is_town(runtime: AnimatedRuntime, record: Any) -> bool:
    return runtime.base_module.is_town(runtime.base, record)


def is_waypoint(runtime: AnimatedRuntime, record: Any) -> bool:
    return runtime.base_module.is_waypoint(runtime.base, record)


def native_floor(tile: Any) -> int:
    base, _ = _mods()
    return base.native_floor(tile)


def enrich_tile_record(record: dict[str, Any], appearance_product: Path, *, appearance_module=None) -> tuple[dict[str, Any], dict[str, int]]:
    appearances = appearance_module or _load(APPEARANCE_PATH, "oteryn_game_atlas_appearance_product_enrich")
    animated = 0
    static = 0
    unresolved = 0
    for presentation in record.get("presentation", []):
        if presentation.get("presentation_resolution_state") not in (None, "RESOLVED"):
            presentation["animation_resolution_state"] = "UNRESOLVED_PRESENTATION"
            unresolved += 1
            continue
        primitives = presentation.get("resolved_primitives")
        if not isinstance(primitives, list) or not primitives:
            presentation["animation_resolution_state"] = "UNRESOLVED_PRESENTATION"
            unresolved += 1
            continue
        patterns = {
            (int(p["pattern"]["x"]), int(p["pattern"]["y"]), int(p["pattern"]["z"]))
            for p in primitives if isinstance(p, dict) and isinstance(p.get("pattern"), dict)
        }
        if len(patterns) != 1:
            raise appearances.ProductError("resolved appearance layers disagree on concrete pattern")
        x, y, z = next(iter(patterns))
        ref = appearances.resolve_object_animation_ref(appearance_product, int(presentation["appearance_source_id"]), {"x": x, "y": y, "z": z})
        if ref is None:
            presentation["animation_resolution_state"] = "STATIC"
            static += 1
            continue
        presentation["animation_resolution_state"] = "RESOLVED"
        presentation["animation_ref"] = ref
        animated += 1
    return record, {"animated_presentations": animated, "static_presentations": static, "unresolved_animation_presentations": unresolved}


def project_tile(runtime: AnimatedRuntime, tile: Any):
    record, stats = runtime.base_module.project_tile(runtime.base, tile)
    record, animation_stats = enrich_tile_record(record, runtime.appearance_product, appearance_module=runtime.appearance_module)
    return record, {**stats, **animation_stats}


def canonical_tile_bytes(runtime: AnimatedRuntime, record: dict[str, Any]) -> bytes:
    data = runtime.base.bounded._canonical_json_bytes(record)
    if len(data) > int(runtime.base.bounded.MAX_TILE_LINE_BYTES):
        raise runtime.base_module.ProducerError(f"semantic tile record exceeds qualified per-tile cap: {len(data)} bytes")
    return data


def project_tile_bytes(runtime: AnimatedRuntime, tile: Any):
    record, stats = project_tile(runtime, tile)
    return canonical_tile_bytes(runtime, record), stats
