#!/usr/bin/env python3
"""Build a bounded local replay slice from the pinned Atlas FullWorld publication.

The generated replay contains semantic records only: no proprietary pixel bytes.
It is intended for the non-production World + VFX physical evidence gate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from urllib.request import Request, urlopen

EXPECTED = {
    "publicationRoot": "sha256:9d0d2f3bb16a5a90f9b51a21366e4ed42963f5cb12366c404a20d9502ec4857f",
    "semanticRoot": "sha256:27d7a83a7d9f498ea614b440ab4216cae5e6d11ea0527482410e40948cade5a9",
    "pixelRoot": "sha256:8b8228fcc4574903e547cb7d65b96f3d45e5a9e67045091c1bceb6e54d3690ad",
    "runtimeIndexRoot": "sha256:fa30ae5fc47f0ca8a6d482ed87b5db2cd74f32f7f523df16187ca719b8e04f08",
}
CREATURE_DIGEST = "sha256:7dc951874c95424279737eaaf51cf2d50940162ef4799daea39a187a581ef0e8"
CREATURE_CAPABILITY = "animated-creatures-v1"

def fetch_bytes(url: str, *, byte_range: tuple[int, int] | None = None) -> bytes:
    headers = {"Cache-Control": "no-store"}
    if byte_range is not None:
        headers["Range"] = f"bytes={byte_range[0]}-{byte_range[1]}"
    with urlopen(Request(url, headers=headers), timeout=30) as response:
        data = response.read()
        if byte_range is not None and response.status != 206:
            raise RuntimeError(f"range request did not return 206: {url} status={response.status}")
        return data


def fetch_json(url: str) -> dict:
    return json.loads(fetch_bytes(url).decode("utf-8"))


def content_id(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def overlaps(bounds: dict, x0: int, x1: int, y0: int, y1: int) -> bool:
    return not (
        x1 <= bounds["x_min"] or x0 >= bounds["x_max_exclusive"]
        or y1 <= bounds["y_min"] or y0 >= bounds["y_max_exclusive"]
    )

def reduced_tile(raw: dict) -> dict:
    presentations = []
    for entry in raw.get("presentation", []):
        primitives = []
        for primitive in entry.get("resolved_primitives", []):
            displacement = primitive.get("displacement", {})
            primitives.append({
                "sprite_id": int(primitive["sprite_source_id"]),
                "width_units": int(primitive["width_units"]),
                "height_units": int(primitive["height_units"]),
                "dx_units": int(displacement.get("dx_units", 0)),
                "dy_units": int(displacement.get("dy_units", 0)),
                "layer_index": int(primitive.get("layer_index", 0)),
            })
        presentations.append({
            "order": int(entry["presentation_order"]["order"]),
            "role": str(entry["source_role"]),
            "appearance_source_id": int(entry["appearance_source_id"]),
            "primitives": primitives,
        })
    position = raw["position"]
    return {"x": int(position["x"]), "y": int(position["y"]),
            "floor": int(position["floor"]), "presentations": presentations}

def reduced_creature(record: dict) -> dict:
    presentation = record.get("outfit_presentation") or {}
    moving = next((group for group in presentation.get("groups", [])
                   if group.get("frame_group", {}).get("semantic") == "outfit-moving"), None)
    static = presentation.get("static_projection") or {}
    animation = (moving or {}).get("animation") or {}
    appearance = record.get("appearance") or {}
    position = record["position"]
    return {
        "record_id": str(record["record_id"]),
        "kind": str(record["kind"]),
        "name": str(record["name"]),
        "x": int(position["x"]), "y": int(position["y"]), "floor": int(position["floor"]),
        "look_type": int(appearance.get("look_type", 0)),
        "presentation_resolved": record.get("presentation_resolution_state") == "RESOLVED",
        "dx_units": int((static.get("displacement") or {}).get("x", 0)),
        "dy_units": int((static.get("displacement") or {}).get("y", 0)),
        "phase_count": int((moving or static).get("phase_count", 1)),
        "phase_durations_ms": [int(value) for value in animation.get("presentation_durations_ms", [])],
        "synchronized": bool(animation.get("synchronized", False)),
        "loop_type": str(animation.get("loop_type", "infinite")),
    }

def load_floor_tiles(origin: str, runtime_world: dict, floor: int, bounds: dict) -> list[dict]:
    floor_entry = next(entry for entry in runtime_world["floors"] if int(entry["floor"]) == floor)
    runtime_floor = fetch_json(f"{origin}/fullworld/runtime-index/{floor_entry['path']}")
    tiles = []
    for chunk in runtime_floor["chunks"]:
        x0 = int(chunk["logicalAddress"]["region_x"]) * int(runtime_floor["regionSpan"])
        x1 = x0 + int(runtime_floor["regionSpan"])
        if x1 <= bounds["x_min"] or x0 >= bounds["x_max_exclusive"]:
            continue
        for group in chunk["groups"]:
            y0, y1 = int(group["yMin"]), int(group["yMaxExclusive"])
            if not overlaps(bounds, x0, x1, y0, y1):
                continue
            start = int(group["offset"]); end = start + int(group["bytes"]) - 1
            url = f"{origin}/fullworld/publication/semantic/{chunk['path']}"
            payload = fetch_bytes(url, byte_range=(start, end))
            if len(payload) != int(group["bytes"]) or content_id(payload) != group["contentId"]:
                raise RuntimeError(f"semantic group integrity mismatch: {floor}:{x0}:{y0}")
            for line in payload.decode("utf-8").splitlines():
                raw = json.loads(line); position = raw["position"]
                if bounds["x_min"] <= position["x"] < bounds["x_max_exclusive"] and bounds["y_min"] <= position["y"] < bounds["y_max_exclusive"]:
                    tiles.append(reduced_tile(raw))
    return tiles

def load_creatures(origin: str, floors: list[int], bounds: dict) -> list[dict]:
    index = fetch_json(f"{origin}/data/creatures/index.json")
    source = index.get("source", {})
    if source.get("capability") != CREATURE_CAPABILITY or source.get("semantic_digest") != CREATURE_DIGEST:
        raise RuntimeError("creature publication identity mismatch")
    creatures = []
    for chunk in index["chunks"]:
        floor = int(chunk["floor"])
        if floor not in floors:
            continue
        x0, y0 = int(chunk["chunk_x"]) * 64, int(chunk["chunk_y"]) * 64
        if not overlaps(bounds, x0, x0 + 64, y0, y0 + 64):
            continue
        payload = fetch_bytes(f"{origin}/data/creatures/{chunk['path']}")
        if len(payload) != int(chunk["bytes"]) or content_id(payload) != chunk["digest"]:
            raise RuntimeError(f"creature shard integrity mismatch: {chunk['path']}")
        for record in json.loads(payload.decode("utf-8"))["records"]:
            pos = record["position"]
            if bounds["x_min"] <= pos["x"] < bounds["x_max_exclusive"] and bounds["y_min"] <= pos["y"] < bounds["y_max_exclusive"]:
                creatures.append(reduced_creature(record))
    return creatures

def build(origin: str, output: Path, manifest_output: Path | None, bounds: dict, floors: list[int]) -> dict:
    origin = origin.rstrip("/")
    publication = fetch_json(f"{origin}/fullworld/publication/publication.json")
    runtime_world = fetch_json(f"{origin}/fullworld/runtime-index/world.json")
    observed = {
        "publicationRoot": publication.get("rootContentId"),
        "semanticRoot": publication.get("semantic", {}).get("rootContentId"),
        "pixelRoot": publication.get("pixels", {}).get("rootContentId"),
        "runtimeIndexRoot": runtime_world.get("rootContentId"),
    }
    if observed != EXPECTED:
        raise RuntimeError(f"FullWorld identity mismatch: expected={EXPECTED} observed={observed}")
    tiles = []
    for floor in floors:
        tiles.extend(load_floor_tiles(origin, runtime_world, floor, bounds))
    creatures = load_creatures(origin, floors, bounds)
    unique_sprites = sorted({primitive["sprite_id"] for tile in tiles
                             for presentation in tile["presentations"]
                             for primitive in presentation["primitives"]})
    resolved_primitives = sum(len(p["primitives"]) for tile in tiles for p in tile["presentations"])
    product = {
        "schema": "oteryn-world-vfx-atlas-slice-v1",
        "source": {**observed, "gameSha": publication["source"]["gameSha"],
                   "sourceFingerprint": publication["source"]["sourceFingerprint"],
                   "creatureSemanticDigest": CREATURE_DIGEST},
        "bounds": {**bounds, "floors": floors},
        "tiles": sorted(tiles, key=lambda value: (value["floor"], value["y"], value["x"])),
        "creatures": sorted(creatures, key=lambda value: (value["floor"], value["y"], value["x"], value["record_id"])),
    }
    encoded = json.dumps(product, sort_keys=True, separators=(",", ":")).encode("utf-8") + b"\n"
    output.parent.mkdir(parents=True, exist_ok=True); output.write_bytes(encoded)
    manifest = {
        "schema": "oteryn-world-vfx-atlas-slice-evidence-v1",
        "source": product["source"], "bounds": product["bounds"],
        "slice_sha256": hashlib.sha256(encoded).hexdigest(), "slice_bytes": len(encoded),
        "counts": {"tiles": len(tiles), "resolved_primitives": resolved_primitives,
                   "creatures": len(creatures), "unique_sprite_ids": len(unique_sprites)},
        "pixels_committed": False,
        "workload_policy": "real-fullworld-static+real-creature-templates+synthetic-dynamic-vfx",
    }
    if manifest_output is not None:
        manifest_output.parent.mkdir(parents=True, exist_ok=True)
        manifest_output.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return manifest

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--origin", default="http://192.168.1.2:8097")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path)
    parser.add_argument("--x-min", type=int, default=32288)
    parser.add_argument("--y-min", type=int, default=32128)
    parser.add_argument("--width", type=int, default=96)
    parser.add_argument("--height", type=int, default=96)
    parser.add_argument("--center-floor", type=int, default=-7)
    args = parser.parse_args()
    if args.width <= 0 or args.height <= 0:
        raise SystemExit("width/height must be positive")
    bounds = {"x_min": args.x_min, "x_max_exclusive": args.x_min + args.width,
              "y_min": args.y_min, "y_max_exclusive": args.y_min + args.height}
    floors = [args.center_floor - 1, args.center_floor, args.center_floor + 1]
    manifest = build(args.origin, args.output, args.manifest_output, bounds, floors)
    print(json.dumps(manifest, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
