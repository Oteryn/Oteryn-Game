#!/usr/bin/env python3
"""Convert the pinned CrystalServer monster spawn file into the `Spawn.Source` family.

Writes content/world/spawns/ (index.json plus spawns-*.json shards): one source per XML
`<monster centerx centery centerz radius>` element, in file order, each point bound to the
creature definition `oteryn:creature.<slug(name)>` and placed at the centre plus its offset.
The source is OTS_HYPOTHESIS_ONLY migration evidence: only normalized facts are written. The
groups listed in the import's `spawns/held-groups.json` stay out (they wait for their owning
contract). Fails closed on a hash other than the pinned one, an unbound name, a point off its
source's floor, or a position outside the u16 plane.

    python convert_spawns.py [--xml world-monster.xml] [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "content/world/spawns"
CREATURES = ROOT / "content/creatures/definitions"
IMPORT = ROOT / "imports/crystalserver/summer-update"
XML = IMPORT / "raw/data-global/world/world-monster.xml"
HELD = IMPORT / "map-content-linking/spawns/held-groups.json"
FRAME = "global-target-2026-09-27"
SHARD_SIZE = 2000
SOURCE = {
    "evidence": "OtsHypothesisOnly",
    "files": [
        {
            "path": "data-global/world/world-monster.xml",
            "sha256": "a3188bc1275fbf5bac1ff5c06cc26b1d2999e51c088a7ffa464c40a1aff81570",
        }
    ],
    "ref": "summer-update",
    "repository": "zimbadev/crystalserver",
    "revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
    "source_key": "oteryn:source.crystalserver",
}
DIRECTIONS = {None: "north", "0": "north", "1": "east", "2": "south", "3": "west"}


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def creature_keys() -> set[str]:
    keys = set()
    for path in sorted(CREATURES.glob("creatures-*.json")):
        for record in json.loads(path.read_text())["records"]:
            keys.add(record["definition"]["identity"]["key"])
    return keys


def dump(value) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def convert(xml: bytes) -> dict[str, str]:
    if hashlib.sha256(xml).hexdigest() != SOURCE["files"][0]["sha256"]:
        sys.exit("the spawn XML is not the pinned file")
    held = {r["source_group_ordinal"] for r in json.loads(HELD.read_text())["records"]}
    known, seen, records = creature_keys(), {}, []
    for ordinal, element in enumerate(ET.fromstring(xml)):
        cx, cy, cz = (int(element.get(k)) for k in ("centerx", "centery", "centerz"))
        base = f"oteryn:spawn.x{cx}_y{cy}_z{cz}"
        seen[base] = seen.get(base, 0) + 1
        key = base if seen[base] == 1 else f"{base}_{seen[base]}"
        if ordinal in held:
            continue
        points = []
        for point in element:
            name = point.get("name")
            creature = f"oteryn:creature.{slug(name)}"
            if creature not in known:
                sys.exit(f"{key}: `{name}` is not an admitted creature definition")
            x, y, z = cx + int(point.get("x")), cy + int(point.get("y")), int(point.get("z", cz))
            if z != cz or not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF):
                sys.exit(f"{key}: point ({x}, {y}, {z}) is off the source")
            points.append(
                {
                    "cell": {"floor": z, "x": x, "y": y},
                    "creature": creature,
                    "direction": DIRECTIONS[point.get("direction")],
                    "respawn_ms": int(point.get("spawntime")) * 1000,
                }
            )
        records.append(
            {
                "declaration": {
                    "centre": {"floor": cz, "x": cx, "y": cy},
                    "identity": {"key": key, "revision": "definition-r1"},
                    "points": points,
                }
            }
        )
    files, shards = {}, []
    for start in range(0, len(records), SHARD_SIZE):
        part = records[start : start + SHARD_SIZE]
        name = f"spawns-{start:05d}-{start + len(part) - 1:05d}.json"
        shards.append(f"content/world/spawns/{name}")
        files[name] = dump({"coordinate_frame": FRAME, "family": "Spawn.Source", "records": part})
    files["index.json"] = dump(
        {
            "coordinate_frame": FRAME,
            "family": "Spawn.Source",
            "generator": "tools/world-bundle-compiler/convert_spawns.py",
            "population_state": "POPULATED",
            "point_count": sum(len(r["declaration"]["points"]) for r in records),
            "record_count": len(records),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shards,
            "source": SOURCE,
        }
    )
    return {name: text + "\n" for name, text in files.items()}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--xml", default=XML, type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = convert(args.xml.read_bytes())
    if args.check:
        have = {p.name: p.read_text() for p in OUT.glob("*.json")}
        if have != files:
            sys.exit("content/world/spawns is not the conversion of the pinned file")
        return
    OUT.mkdir(parents=True, exist_ok=True)
    for stale in OUT.glob("*.json"):
        stale.unlink()
    for name, text in files.items():
        (OUT / name).write_text(text)


if __name__ == "__main__":
    main()
