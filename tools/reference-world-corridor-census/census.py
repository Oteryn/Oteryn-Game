#!/usr/bin/env python3
"""Bounded Phase-A census over the qualified full-world producer stream."""
from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Iterable

U64_MAX = (1 << 64) - 1
LEGACY_REVISION = "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
MAP_SHA256 = "3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034"
ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
CATALOG_SHA256 = "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85"
APPEARANCE_SHA256 = "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075"


class CensusError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class Window:
    name: str
    x_min: int
    y_min: int
    floor: int
    semantic_shard: str
    fullworld_region: str

    @property
    def x_max_exclusive(self) -> int: return self.x_min + 32
    @property
    def y_max_exclusive(self) -> int: return self.y_min + 32


WINDOWS = (
    Window("newhaven", 32512, 32512, -7, "f-7-r1016-c1016", "fm000007_rxp000127_ryp000127"),
    Window("targuna", 31904, 31904, -7, "f-7-r997-c997", "fm000007_rxp000124_ryp000124"),
)


def checked_add(left: int, right: int, label: str) -> int:
    if left < 0 or right < 0 or left > U64_MAX - right:
        raise CensusError(f"u64 counter overflow: {label}")
    return left + right


def require_sha256(path: Path, expected: str, label: str) -> None:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    actual = digest.hexdigest()
    if actual != expected:
        raise CensusError(f"{label} digest mismatch: expected {expected}, got {actual}")


def canonical_summary_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _inside(position: Any, window: Window) -> bool:
    return (window.x_min <= position.x < window.x_max_exclusive
            and window.y_min <= position.y < window.y_max_exclusive
            and -int(position.z) == window.floor)


def _walk_items(items: Iterable[Any]) -> Iterable[Any]:
    for item in items:
        yield item
        yield from _walk_items(getattr(item, "children", ()))


def _structural_observations(tile: Any) -> list[dict[str, Any]]:
    visible = ([tile.ground] if tile.ground is not None else []) + list(tile.items)
    result = []
    for item_order, item in enumerate(_walk_items(visible)):
        base = {"item_order": item_order, "position": {"floor": -tile.position.z, "x": tile.position.x, "y": tile.position.y}, "source_item_id": item.server_id}
        fields = (("ACTION_ID", "action_id"), ("HOUSE_DOOR_ID", "house_door_id"), ("TELEPORT_DESTINATION", "teleport_destination"), ("UNIQUE_ID", "unique_id"))
        for kind, field in fields:
            value = getattr(item, field, None)
            if value is None: continue
            encoded = ({"floor": -value.z, "x": value.x, "y": value.y} if field == "teleport_destination" else value)
            result.append({**base, "structural_kind": kind, "source_value": encoded})
    return sorted(result, key=lambda x: (x["position"]["floor"], x["position"]["y"], x["position"]["x"], x["structural_kind"], x["item_order"]))


def measure_window(producer: Any, runtime: Any, window: Window, records: Iterable[Any]) -> dict[str, Any]:
    tile_records = non_empty = placements = unresolved = encoded_bytes = 0
    max_per_cell = 0
    appearances: set[int] = set()
    sprites: set[int] = set()
    unresolved_ids: set[int] = set()
    resolved_presentation_ids: set[str] = set()
    landmarks: list[dict[str, Any]] = []
    transitions: list[dict[str, Any]] = []
    composites: list[dict[str, Any]] = []
    occupied_edges: set[str] = set()
    for source_order, record in enumerate(records):
        if producer.is_tile(runtime, record):
            if not _inside(record.position, window): continue
            tile_records = checked_add(tile_records, 1, "tile_records")
            data, stats = producer.project_tile_bytes(runtime, record)
            count = int(stats["presentation_count"])
            placements = checked_add(placements, count, "ordered_presentations")
            encoded_bytes = checked_add(encoded_bytes, len(data), "encoded_semantic_bytes")
            unresolved = checked_add(unresolved, int(stats.get("unresolved_presentation_count", 0)), "unresolved_presentations")
            non_empty = checked_add(non_empty, int(count > 0), "non_empty_tile_records")
            max_per_cell = max(max_per_cell, count)
            appearances.update(int(x) for x in stats["appearance_ids"])
            sprites.update(int(x) for x in stats["sprite_ids"])
            unresolved_ids.update(int(x) for x in stats.get("unresolved_appearance_ids", set()))
            decoded = json.loads(data)
            if len(decoded["presentation"]) != count:
                raise CensusError("producer presentation count does not match ordered semantic record")
            for presentation in decoded["presentation"]:
                coverage = [point for primitive in presentation["resolved_primitives"] for point in primitive.get("visual_coverage_offsets", [])]
                if "presentation_resolution_state" not in presentation:
                    resolved_presentation_ids.add(str(presentation["export_record_id"]))
                if any(point.get("dx_tiles") != 0 or point.get("dy_tiles") != 0 for point in coverage):
                    composites.append({"appearance_source_id": presentation["appearance_source_id"], "export_record_id": presentation["export_record_id"], "position": decoded["position"], "source_order": source_order})
            transitions.extend(_structural_observations(record))
            if record.position.x == window.x_min: occupied_edges.add("west")
            if record.position.x == window.x_max_exclusive - 1: occupied_edges.add("east")
            if record.position.y == window.y_min: occupied_edges.add("north")
            if record.position.y == window.y_max_exclusive - 1: occupied_edges.add("south")
        elif producer.is_town(runtime, record) and _inside(record.temple, window):
            landmarks.append({"kind": "town", "name": record.name, "position": {"floor": -record.temple.z, "x": record.temple.x, "y": record.temple.y}, "town_id": record.town_id})
        elif producer.is_waypoint(runtime, record) and _inside(record.position, window):
            landmarks.append({"kind": "waypoint", "name": record.name, "position": {"floor": -record.position.z, "x": record.position.x, "y": record.position.y}})
    clipping = []
    if occupied_edges:
        clipping.append({"boundary": "+".join(x for x in ("north", "east", "south", "west") if x in occupied_edges), "classification": "AMBIGUOUS_EXPANSION", "reason": "source tile records occupy the bounded edge, but Phase-A source structure alone does not prove which adjacent shard is semantically required"})
    return {
        "bounds": {"x_min": window.x_min, "x_max_exclusive": window.x_max_exclusive, "y_min": window.y_min, "y_max_exclusive": window.y_max_exclusive},
        "candidate_composite_diagnostics": sorted(composites, key=lambda x: (x["position"]["floor"], x["position"]["y"], x["position"]["x"], x["export_record_id"])),
        "cell_capacity": 32 * 32, "clipping": clipping, "encoded_semantic_bytes": encoded_bytes,
        "expansion_order": [], "final_semantic_shards": [window.semantic_shard], "floors": [window.floor],
        "fullworld_source_regions": [window.fullworld_region], "max_presentations_per_cell": max_per_cell,
        "name": window.name, "non_empty_tile_records": non_empty, "ordered_presentations": placements,
        "resolved_presentation_count": placements - unresolved, "source_landmarks": sorted(landmarks, key=lambda x: (x["kind"], x["name"])),
        "start_semantic_shard": window.semantic_shard, "tile_records": tile_records,
        "transition_like_records": transitions, "unique_appearance_source_ids": sorted(appearances),
        "unique_resolved_presentation_ids": sorted(resolved_presentation_ids),
        "unique_resolved_sprite_ids": sorted(sprites), "unresolved_appearance_ids": sorted(unresolved_ids),
        "unresolved_presentation_count": unresolved,
    }


def _load_producer(root: Path) -> Any:
    path = root / "tools/game-atlas-fullworld-source/producer.py"
    spec = importlib.util.spec_from_file_location("corridor_qualified_producer", path)
    if spec is None or spec.loader is None: raise CensusError(f"cannot load producer: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _git(repo: Path, *args: str) -> str:
    return subprocess.run(("git", "-C", str(repo), *args), check=True, text=True, stdout=subprocess.PIPE).stdout.strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--legacy-root", type=Path, required=True); parser.add_argument("--map", type=Path, required=True)
    parser.add_argument("--asset-zip", type=Path, required=True); parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if _git(args.legacy_root, "rev-parse", "HEAD") != LEGACY_REVISION: raise CensusError("legacy repository revision mismatch")
    require_sha256(args.map, MAP_SHA256, "world.otbm"); require_sha256(args.asset_zip, ZIP_SHA256, "15.32.zip")
    require_sha256(args.assets / "catalog-content.json", CATALOG_SHA256, "catalog-content.json")
    appearances = sorted(args.assets.glob("appearances-*.dat"))
    if len(appearances) != 1: raise CensusError(f"expected one appearance DAT, got {len(appearances)}")
    require_sha256(appearances[0], APPEARANCE_SHA256, "appearance DAT")
    producer = _load_producer(root)
    runtime = producer.load_runtime(legacy_root=args.legacy_root, map_path=args.map, asset_zip=args.asset_zip, assets_dir=args.assets)
    retained = {window.name: [] for window in WINDOWS}
    for record in producer.iter_records(runtime, strict=True):
        for window in WINDOWS:
            if ((producer.is_tile(runtime, record) and _inside(record.position, window))
                or (producer.is_town(runtime, record) and _inside(record.temple, window))
                or (producer.is_waypoint(runtime, record) and _inside(record.position, window))):
                retained[window.name].append(record)
    summary = {
        "classification": "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY", "phase_a_result": "PASS",
        "phase_b_target_parity": "NOT_PERFORMED", "production_authority": "NONE", "registry_maxima_selected": False,
        "producer": {"api": producer.PRODUCER_API, "code_commit": _git(root, "log", "-1", "--format=%H", "--", "tools/game-atlas-fullworld-source/producer.py"), "repository": "Oteryn/Oteryn-Game"},
        "source": {"appearance_sha256": APPEARANCE_SHA256, "asset_drive_file_id": "1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv", "asset_zip_sha256": ZIP_SHA256, "catalog_sha256": CATALOG_SHA256, "legacy_repository": "blakinio/Otheryn", "legacy_revision": LEGACY_REVISION, "world_otbm_sha256": MAP_SHA256},
        "source_streams": 1, "windows": [measure_window(producer, runtime, window, retained[window.name]) for window in WINDOWS],
    }
    args.output.write_bytes(canonical_summary_bytes(summary))
    print(json.dumps({"output": str(args.output), "sha256": hashlib.sha256(canonical_summary_bytes(summary)).hexdigest(), "windows": [{"name": x["name"], "tile_records": x["tile_records"], "ordered_presentations": x["ordered_presentations"]} for x in summary["windows"]]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    try: raise SystemExit(main())
    except (CensusError, OSError, subprocess.CalledProcessError) as exc: raise SystemExit(f"ERROR: {exc}")
