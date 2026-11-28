"""Admit provisional ground where Crystal and the pinned client agree."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

from summer_ground import APPEARANCES_SHA256, ROOT, ground_records, read_appearances

sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
from build_crystal_item_collision import load_source
from build_source_world import fields, policies

SHARD = ROOT / "content/world/terrain/terrain-09000-09492.json"
PLACEMENTS = ROOT / "content/world/placements/index.json"
GROUND = "oteryn:terrain.tibia.i53066"
SHARED_SHARDS = ("terrain-08751-08765.json", "terrain-08886-08887.json")


def other_records():
    return {
        record["identity"]["key"]: record
        for shard in SHARD.parent.glob("terrain-*.json")
        if shard != SHARD
        for record in json.loads(shard.read_bytes())["records"]
    }


def unique_records(records, existing):
    unique = []
    for record in records:
        key = record["identity"]["key"]
        if key not in existing:
            unique.append(record)
        elif record != existing[key]:
            raise ValueError(f"Crystal ground disagrees with existing terrain: {key}")
    if not any(record["identity"]["key"] == GROUND for record in unique):
        raise ValueError(f"Crystal ground missing {GROUND}")
    return unique


def write_document(path, document, check):
    raw = (json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
    if check:
        if path.read_bytes() != raw:
            raise ValueError(f"stale Crystal ground: {path}")
    else:
        path.write_bytes(raw)


def reconcile(check):
    """Retain source-qualified records whose exact identity is still owned here."""
    document = json.loads(SHARD.read_bytes())
    document["records"] = unique_records(document["records"], other_records())
    write_document(SHARD, document, check)


def regenerate(source, check):
    appearance_bytes = load_source(source, "appearances.dat")
    xml = load_source(source, "items.xml")
    identifiers = {fields(record)[1][0] for record in fields(appearance_bytes)[1]}
    donor = {
        item["server_item_id"]: item
        for item in policies(appearance_bytes, xml, identifiers)
    }
    artifact = f"content/assets/files/appearances-{APPEARANCES_SHA256}.dat"
    official_bytes = (ROOT / artifact).read_bytes()
    if hashlib.sha256(official_bytes).hexdigest() != APPEARANCES_SHA256:
        raise ValueError("official appearances digest")
    official = read_appearances(official_bytes)
    previous = {
        record["provenance"]["source_item_id"]
        for record in json.loads(SHARD.read_bytes())["records"]
    } if SHARD.exists() else set()
    for name in SHARED_SHARDS:
        shard = SHARD.with_name(name)
        if shard.is_file():
            previous.update(
                record["provenance"]["source_item_id"]
                for record in json.loads(shard.read_bytes())["records"]
            )
    index = json.loads(PLACEMENTS.read_bytes())
    # The scoped overlay owns these IDs and restricts their placements.
    scoped = {
        row["source_item_id"] for row in index["palette"] if row.get("scoped_spawn_ground")
    }
    admitted = []
    for row in index["palette"]:
        identifier = row["source_item_id"]
        if identifier in scoped:
            continue
        if not row["provisional"] and identifier not in previous:
            continue
        source_item = donor.get(identifier)
        appearance = official.get(identifier)
        if (
            source_item is None or appearance is None
            or not source_item["ground"] or source_item["floor_change"]
            or "bank" not in appearance.flags or not 0 < appearance.speed <= 1000
            or source_item["ground_speed"] != appearance.speed
            or source_item["block_solid"] != ("unpass" in appearance.flags)
        ):
            continue
        admitted.append(identifier)
        row.update(key=f"oteryn:terrain.tibia.i{identifier}", provisional=False)
    if len(admitted) != 493:
        raise ValueError(f"Crystal ground catalogue coverage changed: {len(admitted)}")
    records = unique_records(ground_records(admitted), other_records())
    write_document(SHARD, {"family": "Terrain", "records": records}, check)
    write_document(PLACEMENTS, index, check)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", nargs="?", type=Path, help="pinned Crystal data/items directory")
    parser.add_argument("--reconcile", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.reconcile and args.source is None:
        reconcile(args.check)
    elif not args.reconcile and args.source is not None:
        regenerate(args.source, args.check)
    else:
        parser.error("provide either a Crystal source directory or --reconcile")
