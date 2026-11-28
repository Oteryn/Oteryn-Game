"""Admit provisional ground where Crystal and the pinned client agree."""

import argparse
import hashlib
import json
import sys
from pathlib import Path

from summer_ground import APPEARANCES_SHA256, ROOT, ground_record, read_appearances

sys.path.insert(0, str(ROOT / "tools/content-schema/native-gameplay"))
from build_crystal_item_collision import load_source
from build_source_world import fields, policies

SHARD = ROOT / "content/world/terrain/terrain-09000-09492.json"
PLACEMENTS = ROOT / "content/world/placements/index.json"


def exclusive_records(records):
    owned = {}
    for shard in sorted(SHARD.parent.glob("terrain-*.json")):
        if shard != SHARD:
            for record in json.loads(shard.read_bytes())["records"]:
                key = record["identity"]["key"]
                if key in owned:
                    raise ValueError(f"Terrain key already has multiple owners: {key}")
                owned[key] = record
    result = []
    for record in records:
        key = record["identity"]["key"]
        if key not in owned:
            result.append(record)
        elif record != owned[key]:
            raise ValueError(f"Conflicting Crystal ground facts for {key}")
    if len(result) != 477 or not any(r["provenance"]["source_item_id"] == 53066 for r in result):
        raise ValueError(f"Crystal ground catalogue coverage changed: {len(result)}")
    return result


def write_output(path, document, check):
    raw = (json.dumps(document, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()
    if check:
        if path.read_bytes() != raw:
            raise ValueError(f"stale Crystal ground: {path}")
    else:
        path.write_bytes(raw)


def reconcile_overlays(check):
    # Keep the authored owner only after verifying that the facts are identical.
    document = json.loads(SHARD.read_bytes())
    document["records"] = exclusive_records(document["records"])
    write_output(SHARD, document, check)


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
    index = json.loads(PLACEMENTS.read_bytes())
    # The scoped overlay owns these IDs and restricts their placements.
    scoped = {
        row["source_item_id"] for row in index["palette"] if row.get("scoped_spawn_ground")
    }
    records = []
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
        record = ground_record(ROOT, artifact, appearance, identifier)
        row.update(key=record["identity"]["key"], provisional=False)
        records.append(record)
    for path, document in (
        (SHARD, {"family": "Terrain", "records": exclusive_records(records)}),
        (PLACEMENTS, index),
    ):
        write_output(path, document, check)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", nargs="?", type=Path, help="pinned Crystal data/items directory")
    parser.add_argument("--reconcile-overlays", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.reconcile_overlays and args.source is None:
        reconcile_overlays(args.check)
    elif args.source is not None and not args.reconcile_overlays:
        regenerate(args.source, args.check)
    else:
        parser.error("provide a source directory or --reconcile-overlays")
