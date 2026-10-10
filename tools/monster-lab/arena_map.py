#!/usr/bin/env python3
"""Export the existing native entry room without editing or inventing map cells."""
import argparse
import hashlib
import json
from pathlib import Path

SOURCE = "apps/game-server/src/content/project/native_entry_room.json"
SOURCE_SHA256 = "b0d5d0348a59d7ef3ff31e9062f5b207634c1a2c3a29807a97ed0222a1308940"


def export_map(source_path, expected_sha256, maximum_cells=256):
    source_path = Path(source_path).resolve()
    raw = source_path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != expected_sha256:
        raise ValueError("Map source SHA256 does not match the configured source")
    source = json.loads(raw)
    if source.get("schema") != "OTERYN_NATIVE_ENTRY_ROOM_SOURCE/v1":
        raise ValueError("Unsupported map source schema")
    world, entry = source["world"], source["native_first_entry"]
    bounds = world["bounds"]
    if not (bounds["min_x"] < bounds["max_x_exclusive"] and
            bounds["min_y"] < bounds["max_y_exclusive"]):
        raise ValueError("Invalid map bounds")
    placements = source["placements"]
    if not 1 <= len(placements) <= maximum_cells:
        raise ValueError("Map exceeds bounded arena cell limit")
    static = {cell["placement_key"]: cell for cell in entry["cells"]}
    doors = {door["cell"]["placement_key"]: door for door in entry.get("doors", [])}
    keys, positions, cells = set(), set(), []
    for placement in placements:
        key = placement["key"]
        position = (placement["x"], placement["y"], placement["floor"])
        x, y, floor = position
        if key in keys or position in positions:
            raise ValueError("Duplicate map cell")
        keys.add(key)
        positions.add(position)
        if not (bounds["min_x"] <= x < bounds["max_x_exclusive"] and
                bounds["min_y"] <= y < bounds["max_y_exclusive"] and
                floor in world["floors"]):
            raise ValueError("Map placement outside declared bounds or floors")
        if (placement["world"] != world["key"] or
                placement["coordinate_frame"] != world["coordinate_frame"]):
            raise ValueError("Map placement uses another world or coordinate frame")
        declaration = static.get(key)
        collision = declaration["collision"] if declaration else "Unknown"
        if collision not in {"Walkable", "Blocked", "Unknown"}:
            raise ValueError("Unsupported collision declaration")
        cell = {"key": key, "x": x, "y": y, "floor": floor,
                "terrain": placement["definition"],
                "walkability": collision, "spawn_eligible": collision == "Walkable"}
        if key in doors:
            cell.update(walkability="DoorStateDependent", spawn_eligible=False,
                        terrain_collision=doors[key]["cell"]["collision"])
        cells.append(cell)
    if (set(static) | set(doors)) - keys:
        raise ValueError("Collision declaration references absent placement")
    player_key = "oteryn:cell/entry-start"
    # Room revision 2 (SPAWN-1a): one monster slot per authored den cell, in authored order.
    monster_keys = entry["spawn"]["cell_keys"]
    by_key = {cell["key"]: cell for cell in cells}
    roles = [("player", player_key)] + [("monster", key) for key in monster_keys]
    if not monster_keys or len({key for _, key in roles}) != len(roles) or any(
            not by_key.get(key, {}).get("spawn_eligible") for _, key in roles):
        raise ValueError("Arena player and monster require distinct explicit walkable cells")
    slots = [{"role": role, **by_key[key]} for role, key in roles]
    return {"schema": "oteryn-monster-lab-arena-map-v1", "map": {
        "source_path": str(source_path), "source_sha256": digest,
        "world_key": world["key"], "coordinate_frame": world["coordinate_frame"],
        "map_revision": entry["revisions"]["map"], "bounds": bounds,
        "floors": world["floors"], "cells": cells, "spawn_slots": slots,
        "player_slot": slots[0], "monster_slot": slots[1],
        "simultaneous_monster_capacity": len(monster_keys),
        "physical_walkability": "Explicit native source declarations; dynamic door excluded",
        "source_modified": False,
        "scope": "Existing native room; one creature per den cell, no large-area spell coverage"
    }}


def build_arena_map(source_path, expected_sha256=SOURCE_SHA256):
    """Return the map object accepted by the real native test harness."""
    return export_map(source_path, expected_sha256)["map"]


def prepare_arena(config, output_path=None):
    if isinstance(config, (str, Path)):
        config = json.loads(Path(config).read_text())
    settings = config.get("arena", {})
    source_path = settings.get("map_source", str(Path(config["repository"]) / SOURCE))
    manifest = export_map(source_path, settings.get("map_source_sha256", SOURCE_SHA256),
                          settings.get("maximum_cells", 256))
    if output_path:
        output_path = Path(output_path)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    manifest = prepare_arena(args.config, args.output)
    print(json.dumps({"status": "PASS", "cells": len(manifest["map"]["cells"]),
                      "spawn_slots": len(manifest["map"]["spawn_slots"])}))


if __name__ == "__main__":
    main()
