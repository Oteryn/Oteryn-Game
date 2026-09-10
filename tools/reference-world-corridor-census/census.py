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


class CensusError(RuntimeError):
    pass


class SourceCorpusRequired(CensusError):
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
    def x_max_exclusive(self) -> int:
        return self.x_min + 32

    @property
    def y_max_exclusive(self) -> int:
        return self.y_min + 32


WINDOWS = (
    Window("newhaven", 32512, 32512, -7, "f-7-r1016-c1016", "fm000007_rxp000127_ryp000127"),
    Window("targuna", 31904, 31904, -7, "f-7-r997-c997", "fm000007_rxp000124_ryp000124"),
)


def checked_add(left: int, right: int, label: str) -> int:
    if left < 0 or right < 0 or left > U64_MAX - right:
        raise CensusError(f"u64 counter overflow: {label}")
    return left + right


def _source_required(label: str, path: Path) -> SourceCorpusRequired:
    return SourceCorpusRequired(f"SOURCE_CORPUS_REQUIRED: {label} unavailable: {path}")


def require_source_path(path: Path, label: str, *, directory: bool) -> Path:
    try:
        available = path.is_dir() if directory else path.is_file()
    except OSError as exc:
        raise _source_required(label, path) from exc
    if not available:
        raise _source_required(label, path)
    return path


def require_source_corpus_inputs(
    legacy_root: Path,
    map_path: Path,
    asset_zip: Path,
    assets_dir: Path,
) -> None:
    require_source_path(legacy_root, "legacy checkout", directory=True)
    require_source_path(map_path, "world.otbm", directory=False)
    require_source_path(asset_zip, "15.32.zip", directory=False)
    require_source_path(assets_dir, "extracted assets directory", directory=True)


def require_sha256(path: Path, expected: str, label: str) -> None:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(block)
    except OSError as exc:
        raise _source_required(label, path) from exc
    actual = digest.hexdigest()
    if actual != expected:
        raise CensusError(f"{label} digest mismatch: expected {expected}, got {actual}")


def canonical_summary_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def _git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ("git", "-C", str(repo), *args),
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()


def verify_legacy_checkout(legacy_root: Path) -> Path:
    root = legacy_root.resolve()
    try:
        top = Path(_git(root, "rev-parse", "--show-toplevel")).resolve()
        head = _git(root, "rev-parse", "HEAD")
        status = _git(root, "status", "--porcelain=v1", "--untracked-files=all")
    except (OSError, subprocess.CalledProcessError) as exc:
        raise CensusError("LEGACY_PARSER_REVISION_MISMATCH: legacy checkout cannot be verified") from exc
    if top != root:
        raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: legacy root {root} != git top-level {top}")
    if head != LEGACY_REVISION:
        raise CensusError("LEGACY_PARSER_REVISION_MISMATCH: legacy repository HEAD mismatch")
    if status:
        raise CensusError("LEGACY_PARSER_REVISION_MISMATCH: legacy worktree is not clean")
    return root


def require_fresh_legacy_import_context() -> None:
    contaminated = sorted(
        name for name in sys.modules
        if name == "tools" or name == "tools.otbm_atlas" or name.startswith("tools.otbm_atlas.")
    )
    if contaminated:
        raise CensusError(
            "LEGACY_PARSER_REVISION_MISMATCH: pre-existing legacy parser modules: "
            + ",".join(contaminated)
        )


def _under(path: Path, parent: Path) -> bool:
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def verify_loaded_legacy_modules(legacy_root: Path) -> list[dict[str, str]]:
    root = legacy_root.resolve()
    tools_root = root / "tools"
    required = {"tools", "tools.otbm_atlas", "tools.otbm_atlas.assets", "tools.otbm_atlas.semantic"}
    names = sorted(
        name for name in sys.modules
        if name == "tools" or name == "tools.otbm_atlas" or name.startswith("tools.otbm_atlas.")
    )
    missing = sorted(required.difference(names))
    if missing:
        raise CensusError("LEGACY_PARSER_REVISION_MISMATCH: missing loaded parser modules: " + ",".join(missing))

    verified: list[dict[str, str]] = []
    for name in names:
        module = sys.modules[name]
        file_name = getattr(module, "__file__", None)
        search_locations = list(getattr(getattr(module, "__spec__", None), "submodule_search_locations", None) or [])
        if file_name is None and not search_locations:
            raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: unverifiable origin for {name}")
        for location in search_locations:
            if not _under(Path(location), tools_root):
                raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: package origin outside pinned tree for {name}")
        if file_name is None:
            verified.append({"module": name, "origin": "namespace-under-pinned-tools"})
            continue

        file_path = Path(file_name).resolve()
        if not _under(file_path, tools_root):
            raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: module origin outside pinned tree for {name}")
        relative = file_path.relative_to(root).as_posix()
        try:
            tracked = _git(root, "ls-files", "--error-unmatch", "--", relative)
            working_blob = _git(root, "hash-object", "--", relative)
            pinned_blob = _git(root, "rev-parse", f"{LEGACY_REVISION}:{relative}")
        except subprocess.CalledProcessError as exc:
            raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: untracked/unverifiable module {name}") from exc
        if tracked != relative or working_blob != pinned_blob:
            raise CensusError(f"LEGACY_PARSER_REVISION_MISMATCH: loaded blob mismatch for {name}")
        verified.append({"module": name, "path": relative, "blob": pinned_blob})
    return verified


def _inside_source_position(position: Any, window: Window) -> bool:
    """Filter source-only metadata; this is not a second tile transform authority."""
    return (
        window.x_min <= position.x < window.x_max_exclusive
        and window.y_min <= position.y < window.y_max_exclusive
        and -int(position.z) == window.floor
    )


def _source_position_metadata(position: Any) -> dict[str, int]:
    """Serialize non-tile source metadata for which the producer exposes no API."""
    return {"floor": -int(position.z), "x": position.x, "y": position.y}


def _inside_tile(producer: Any, tile: Any, window: Window) -> bool:
    return (
        window.x_min <= tile.position.x < window.x_max_exclusive
        and window.y_min <= tile.position.y < window.y_max_exclusive
        and producer.native_floor(tile) == window.floor
    )


def _walk_items(items: Iterable[Any]) -> Iterable[Any]:
    for item in items:
        yield item
        yield from _walk_items(getattr(item, "children", ()))


def _structural_observations(producer: Any, tile: Any) -> list[dict[str, Any]]:
    visible = ([tile.ground] if tile.ground is not None else []) + list(tile.items)
    result = []
    for item_order, item in enumerate(_walk_items(visible)):
        base = {
            "item_order": item_order,
            "position": {
                "floor": producer.native_floor(tile),
                "x": tile.position.x,
                "y": tile.position.y,
            },
            "source_item_id": item.server_id,
        }
        fields = (
            ("ACTION_ID", "action_id"),
            ("HOUSE_DOOR_ID", "house_door_id"),
            ("TELEPORT_DESTINATION", "teleport_destination"),
            ("UNIQUE_ID", "unique_id"),
        )
        for kind, field in fields:
            value = getattr(item, field, None)
            if value is None:
                continue
            encoded = (
                _source_position_metadata(value)
                if field == "teleport_destination" else value
            )
            result.append({**base, "structural_kind": kind, "source_value": encoded})
    return sorted(
        result,
        key=lambda x: (
            x["position"]["floor"], x["position"]["y"], x["position"]["x"],
            x["structural_kind"], x["item_order"],
        ),
    )


def semantic_shard_for(x: int, y: int, floor: int) -> str:
    return f"f{floor}-r{y // 32}-c{x // 32}"


def expansion_requirement(window: Window, *, x: int, y: int, floor: int, reason: str) -> dict[str, Any]:
    if window.x_min <= x < window.x_max_exclusive and window.y_min <= y < window.y_max_exclusive and floor == window.floor:
        raise CensusError("expansion requirement must point outside the authorized start shard")
    return {
        "classification": "WINDOW_EXPANSION_REQUIRED",
        "reason": reason,
        "proposed_floor": floor,
        "proposed_semantic_shard": semantic_shard_for(x, y, floor),
    }


def _minimal_expansion_point(window: Window, *, x: int, y: int, floor: int) -> tuple[int, int, int]:
    proposed_x = window.x_min - 1 if x < window.x_min else window.x_max_exclusive if x >= window.x_max_exclusive else x
    proposed_y = window.y_min - 1 if y < window.y_min else window.y_max_exclusive if y >= window.y_max_exclusive else y
    return proposed_x, proposed_y, floor


def _expansion_from_structural_observation(window: Window, observation: dict[str, Any]) -> dict[str, Any] | None:
    if observation.get("structural_kind") != "TELEPORT_DESTINATION":
        return None
    target = observation.get("source_value")
    if not isinstance(target, dict) or not {"floor", "x", "y"}.issubset(target):
        return None
    target_floor = int(target["floor"])
    target_x = int(target["x"])
    target_y = int(target["y"])
    # The pinned corpus contains (0,0,0) source values whose gameplay meaning is
    # intentionally unproven. They are not usable evidence for expansion.
    if (target_x, target_y, target_floor) == (0, 0, 0):
        return None
    if (
        window.x_min <= target_x < window.x_max_exclusive
        and window.y_min <= target_y < window.y_max_exclusive
        and target_floor == window.floor
    ):
        return None
    proposed_x, proposed_y, proposed_floor = _minimal_expansion_point(
        window, x=target_x, y=target_y, floor=target_floor
    )
    origin = observation["position"]
    reason = (
        "source TELEPORT_DESTINATION from "
        f"({origin['x']},{origin['y']},{origin['floor']}) reaches outside authorized start shard at "
        f"({target_x},{target_y},{target_floor})"
    )
    return expansion_requirement(
        window,
        x=proposed_x,
        y=proposed_y,
        floor=proposed_floor,
        reason=reason,
    )


def _append_expansion_request(requests: list[dict[str, Any]], request: dict[str, Any]) -> None:
    key = (request["proposed_floor"], request["proposed_semantic_shard"])
    if any((item["proposed_floor"], item["proposed_semantic_shard"]) == key for item in requests):
        return
    requests.append(request)


def measure_window(producer: Any, runtime: Any, window: Window, records: Iterable[Any]) -> dict[str, Any]:
    tile_records = non_empty = placements = unresolved = aggregate_bytes = 0
    max_per_cell = max_record_bytes = 0
    stream_digest = hashlib.sha256()
    appearances: set[int] = set()
    sprites: set[int] = set()
    unresolved_ids: set[int] = set()
    resolved_presentation_ids: set[str] = set()
    landmarks: list[dict[str, Any]] = []
    transitions: list[dict[str, Any]] = []
    composites: list[dict[str, Any]] = []
    occupied_edges: set[str] = set()
    expansion_requests: list[dict[str, Any]] = []

    for source_order, record in enumerate(records):
        if producer.is_tile(runtime, record):
            if not _inside_tile(producer, record, window):
                continue
            tile_records = checked_add(tile_records, 1, "tile_records")
            record_bytes, record_stats = producer.project_tile_bytes(runtime, record)
            count = int(record_stats["presentation_count"])
            placements = checked_add(placements, count, "ordered_presentations")
            aggregate_bytes = checked_add(aggregate_bytes, len(record_bytes), "aggregate_encoded_byte_count")
            max_record_bytes = max(max_record_bytes, len(record_bytes))
            stream_digest.update(record_bytes)
            unresolved = checked_add(
                unresolved,
                int(record_stats.get("unresolved_presentation_count", 0)),
                "unresolved_presentations",
            )
            non_empty = checked_add(non_empty, int(count > 0), "non_empty_tile_records")
            max_per_cell = max(max_per_cell, count)
            appearances.update(int(x) for x in record_stats["appearance_ids"])
            sprites.update(int(x) for x in record_stats["sprite_ids"])
            unresolved_ids.update(int(x) for x in record_stats.get("unresolved_appearance_ids", set()))

            decoded = json.loads(record_bytes)
            if len(decoded["presentation"]) != count:
                raise CensusError("producer presentation count does not match ordered semantic record")
            for presentation in decoded["presentation"]:
                coverage = [
                    point
                    for primitive in presentation["resolved_primitives"]
                    for point in primitive.get("visual_coverage_offsets", [])
                ]
                if "presentation_resolution_state" not in presentation:
                    resolved_presentation_ids.add(str(presentation["export_record_id"]))
                if any(point.get("dx_tiles") != 0 or point.get("dy_tiles") != 0 for point in coverage):
                    composites.append({
                        "appearance_source_id": presentation["appearance_source_id"],
                        "export_record_id": presentation["export_record_id"],
                        "position": decoded["position"],
                        "source_order": source_order,
                    })
            tile_transitions = _structural_observations(producer, record)
            transitions.extend(tile_transitions)
            for observation in tile_transitions:
                request = _expansion_from_structural_observation(window, observation)
                if request is not None:
                    _append_expansion_request(expansion_requests, request)
            if record.position.x == window.x_min:
                occupied_edges.add("west")
            if record.position.x == window.x_max_exclusive - 1:
                occupied_edges.add("east")
            if record.position.y == window.y_min:
                occupied_edges.add("north")
            if record.position.y == window.y_max_exclusive - 1:
                occupied_edges.add("south")
        elif producer.is_town(runtime, record) and _inside_source_position(record.temple, window):
            landmarks.append({
                "kind": "town", "name": record.name,
                "position": _source_position_metadata(record.temple),
                "town_id": record.town_id,
            })
        elif producer.is_waypoint(runtime, record) and _inside_source_position(record.position, window):
            landmarks.append({
                "kind": "waypoint", "name": record.name,
                "position": _source_position_metadata(record.position),
            })

    return {
        "aggregate_encoded_byte_count": aggregate_bytes,
        "boundary_occupancy": sorted(occupied_edges),
        "boundary_occupancy_classification": "EDGE_OCCUPANCY_ONLY_NOT_CLIPPING_PROOF",
        "bounds": {
            "x_min": window.x_min, "x_max_exclusive": window.x_max_exclusive,
            "y_min": window.y_min, "y_max_exclusive": window.y_max_exclusive,
        },
        "candidate_composite_diagnostics": sorted(
            composites,
            key=lambda x: (x["position"]["floor"], x["position"]["y"], x["position"]["x"], x["export_record_id"]),
        ),
        "cell_capacity": 32 * 32,
        "floors": [window.floor],
        "fullworld_source_region_lookup_envelope": window.fullworld_region,
        "max_presentations_per_cell": max_per_cell,
        "max_record_encoded_bytes": max_record_bytes,
        "name": window.name,
        "non_empty_tile_records": non_empty,
        "ordered_presentations": placements,
        "ordered_stream_sha256": stream_digest.hexdigest(),
        "resolved_presentation_count": placements - unresolved,
        "source_landmarks": sorted(landmarks, key=lambda x: (x["kind"], x["name"])),
        "start_semantic_shard": window.semantic_shard,
        "tile_records": tile_records,
        "transition_like_records": transitions,
        "unique_appearance_source_ids": sorted(appearances),
        "unique_resolved_presentation_ids": sorted(resolved_presentation_ids),
        "unique_resolved_sprite_ids": sorted(sprites),
        "unresolved_appearance_ids": sorted(unresolved_ids),
        "unresolved_presentation_count": unresolved,
        "window_expansion_required": expansion_requests,
        "window_result": "WINDOW_EXPANSION_REQUIRED" if expansion_requests else "PASS",
    }


def _load_producer(root: Path) -> Any:
    path = root / "tools/game-atlas-fullworld-source/producer.py"
    spec = importlib.util.spec_from_file_location("corridor_qualified_producer", path)
    if spec is None or spec.loader is None:
        raise CensusError(f"cannot load producer: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--legacy-root", type=Path, required=True)
    parser.add_argument("--map", type=Path, required=True)
    parser.add_argument("--asset-zip", type=Path, required=True)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    require_source_corpus_inputs(args.legacy_root, args.map, args.asset_zip, args.assets)
    legacy_root = verify_legacy_checkout(args.legacy_root)
    require_fresh_legacy_import_context()
    require_sha256(args.map, MAP_SHA256, "world.otbm")
    require_sha256(args.asset_zip, ZIP_SHA256, "15.32.zip")
    producer = _load_producer(root)
    try:
        runtime = producer.load_runtime(
            legacy_root=legacy_root,
            map_path=args.map,
            asset_zip=args.asset_zip,
            assets_dir=args.assets,
        )
    except OSError as exc:
        raise SourceCorpusRequired(
            "SOURCE_CORPUS_REQUIRED: producer source inputs unavailable during load_runtime"
        ) from exc
    parser_modules = verify_loaded_legacy_modules(legacy_root)

    retained = {window.name: [] for window in WINDOWS}
    try:
        for record in producer.iter_records(runtime, strict=True):
            for window in WINDOWS:
                if (
                    (producer.is_tile(runtime, record) and _inside_tile(producer, record, window))
                    or (producer.is_town(runtime, record) and _inside_source_position(record.temple, window))
                    or (producer.is_waypoint(runtime, record) and _inside_source_position(record.position, window))
                ):
                    retained[window.name].append(record)
    except OSError as exc:
        raise SourceCorpusRequired("SOURCE_CORPUS_REQUIRED: source record stream unavailable") from exc

    windows = [measure_window(producer, runtime, window, retained[window.name]) for window in WINDOWS]
    expansion = [
        {"name": window["name"], "requests": window["window_expansion_required"]}
        for window in windows if window["window_expansion_required"]
    ]
    phase_result = "WINDOW_EXPANSION_REQUIRED" if expansion else "PASS"
    summary = {
        "classification": "MIGRATION_EVIDENCE / OTS_HYPOTHESIS_ONLY",
        "phase_a_result": phase_result,
        "phase_b_target_parity": "NOT_PERFORMED",
        "production_authority": "NONE",
        "registry_maxima_selected": False,
        "producer": {
            "api": producer.PRODUCER_API,
            "code_commit": _git(root, "log", "-1", "--format=%H", "--", "tools/game-atlas-fullworld-source/producer.py"),
            "repository": "Oteryn/Oteryn-Game",
        },
        "source": {
            "appearance_sha256": runtime.bounded.ASSET_APPEARANCE_SHA256,
            "asset_drive_file_id": "1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv",
            "asset_zip_sha256": runtime.bounded.ASSET_ZIP_SHA256,
            "catalog_sha256": runtime.bounded.ASSET_CATALOG_SHA256,
            "legacy_parser_modules": parser_modules,
            "legacy_repository": "blakinio/Otheryn",
            "legacy_revision": LEGACY_REVISION,
            "legacy_worktree_clean": True,
            "world_otbm_sha256": runtime.bounded.MAP_SHA256,
        },
        "source_streams": 1,
        "window_expansion_required": expansion,
        "windows": windows,
    }
    output = canonical_summary_bytes(summary)
    args.output.write_bytes(output)
    print(json.dumps({
        "output": str(args.output),
        "phase_a_result": phase_result,
        "sha256": hashlib.sha256(output).hexdigest(),
        "windows": [
            {
                "name": item["name"],
                "tile_records": item["tile_records"],
                "ordered_presentations": item["ordered_presentations"],
                "ordered_stream_sha256": item["ordered_stream_sha256"],
            }
            for item in windows
        ],
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SourceCorpusRequired as exc:
        raise SystemExit(str(exc))
    except (CensusError, OSError, subprocess.CalledProcessError) as exc:
        raise SystemExit(f"ERROR: {exc}")
