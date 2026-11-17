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
HELD_SHA256 = "8a666c3d409edda5ac5996d6dec64cce24b30d5f43c75aa9a1175f1d07297c57"
BOSSES = Path(__file__).with_name("boss-spawns.json")
BOSS_CHANCES = Path(__file__).with_name("held-boss-chances.json")
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


def encounter_bosses() -> set[str]:
    """Keep quest-owned creatures and explicit encounter/room actors out of world clocks."""
    bindings = json.loads((ROOT / "imports/canary/bindings/creatures.json").read_text())
    room_paths = ("quests/ancient_tombs/", "quests/in_service_of_yalahar/",
                  "quests/isle_of_evil/", "quests/liquid_black/",
                  "quests/the_inquisition/", "quests/the_new_frontier/",
                  "quests/wrath_of_the_emperor/")
    keys = {b["target"]["key"] for b in bindings["bindings"]
            if b["external_id"].startswith(room_paths)}
    # These quest bosses have a generic bosses/ donor path. Their scripted entry/kill
    # conditions are not a world-reset scheduling policy.
    keys.update(f"oteryn:creature.{name}" for name in (
        "black_knight", "koshei_the_deathless", "energized_raging_mage",
        "the_flaming_orchid", "bibby_bloodbath", "renegade_orc",
        "thawing_dragon_lord", "dazed_leaf_golem",
    ))
    for path in CREATURES.glob("creatures-*.json"):
        for record in json.loads(path.read_text())["records"]:
            if record["authoring"]["profile"].get("encounters"):
                keys.add(record["definition"]["identity"]["key"])
    def collect_refs(value):
        if isinstance(value, dict):
            if value.get("family") == "Creature" and isinstance(value.get("key"), str):
                keys.add(value["key"])
            for child in value.values():
                collect_refs(child)
        elif isinstance(value, list):
            for child in value:
                collect_refs(child)
    for path in (ROOT / "content/encounters").rglob("*.json"):
        collect_refs(json.loads(path.read_text()))
    rooms = json.loads((ROOT / "content/quests/grizzly-boss-rooms.json").read_text())
    keys.update(f"oteryn:creature.{slug(room['name'])}" for room in rooms["rooms"])
    return keys


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
    held_bytes = HELD.read_bytes()
    if hashlib.sha256(held_bytes).hexdigest() != HELD_SHA256:
        sys.exit("held-groups.json is not the pinned file")
    held_records = json.loads(held_bytes)["records"]
    held = {r["source_group_ordinal"] for r in held_records}
    known, seen, records = creature_keys(), {}, []
    boss_keys = set()
    for path in sorted(CREATURES.glob("creatures-*.json")):
        for record in json.loads(path.read_text())["records"]:
            details = record["authoring"]["profile"]["details"]
            if details.get("bosstiary") is not None or details["system_eligibility"]["reward_boss"]:
                boss_keys.add(record["definition"]["identity"]["key"])
    held_bosses = []
    encounter_bound = encounter_bosses()
    chances_bytes = BOSS_CHANCES.read_bytes()
    chances = json.loads(chances_bytes)["bosses"]
    scheduled_bosses = []
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
        live_points = []
        for point in points:
            if point["creature"] in boss_keys:
                if point["creature"] in encounter_bound:
                    held_bosses.append({"source": key, "point": point,
                                        "reason": "encounter-bound"})
                else:
                    configured = chances[point["creature"]]
                    chance = configured["at_reset"]["chance"]
                    if not (0 <= chance["numerator"] <= chance["denominator"] <= 0x7FFFFFFFFFFFFFFF
                            and chance["denominator"] > 0):
                        sys.exit(f"{point['creature']}: invalid boss chance")
                    scheduled = {k: v for k, v in point.items() if k != "respawn_ms"}
                    scheduled["boss_policy"] = {"at_reset": configured["at_reset"]}
                    scheduled_bosses.append({"declaration": {
                        "centre": {"floor": cz, "x": cx, "y": cy},
                        "identity": {"key": f"{key}.boss.{slug(point['creature'])}", "revision": "definition-r1"},
                        "points": [scheduled],
                        "provenance": {"kind": configured["basis"],
                                       "derivation": configured["source"],
                                       "policy_source": "tools/world-bundle-compiler/held-boss-chances.json",
                                       "placement_source": SOURCE, "source_spawn": key},
                    }})
            else:
                live_points.append(point)
        points = live_points
        if not points:
            continue
        records.append(
            {
                "declaration": {
                    "centre": {"floor": cz, "x": cx, "y": cy},
                    "identity": {"key": key, "revision": "definition-r1"},
                    "points": points,
                }
            }
        )
    records.extend(scheduled_bosses)
    files, shards = {}, []
    for start in range(0, len(records), SHARD_SIZE):
        part = records[start : start + SHARD_SIZE]
        name = f"spawns-{start:05d}-{start + len(part) - 1:05d}.json"
        shards.append(f"content/world/spawns/{name}")
        files[name] = dump({"coordinate_frame": FRAME, "family": "Spawn.Source", "records": part})
    bosses_bytes = BOSSES.read_bytes()
    bosses = json.loads(bosses_bytes)
    if bosses["family"] != "Spawn.Source" or bosses["coordinate_frame"] != FRAME:
        sys.exit("boss spawn authoring family or frame mismatch")
    for record in bosses["records"]:
        for point in record["declaration"]["points"]:
            if point["creature"] not in known or "boss_policy" not in point or "respawn_ms" in point:
                sys.exit("boss spawn must bind an admitted creature and only a boss policy")
    held_name = "held-bosses.json"
    files[held_name] = dump({"family": "Held.Spawn.Point", "coordinate_frame": FRAME,
                             "records": held_bosses, "source": SOURCE})
    boss_name = "spawns-boss-spawn1.json"
    files[boss_name] = dump(bosses)
    shards.append(f"content/world/spawns/{boss_name}")
    all_records = records + bosses["records"]
    files["index.json"] = dump(
        {
            "coordinate_frame": FRAME,
            "family": "Spawn.Source",
            "generator": "tools/world-bundle-compiler/convert_spawns.py",
            "held_groups": {
                "group_count": len(held_records),
                "path": "imports/crystalserver/summer-update/map-content-linking/spawns/held-groups.json",
                "point_count": sum(r["point_count"] for r in held_records),
                "sha256": HELD_SHA256,
            },
            "held_bosses": {"path": f"content/world/spawns/{held_name}",
                            "point_count": len(held_bosses),
                            "sha256": hashlib.sha256((files[held_name]+"\n").encode()).hexdigest()},
            "population_state": "POPULATED",
            "point_count": sum(len(r["declaration"]["points"]) for r in all_records),
            "record_count": len(all_records),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shards,
            "source": SOURCE,
            "additional_sources": [{"path": "tools/world-bundle-compiler/boss-spawns.json",
                                    "sha256": hashlib.sha256(bosses_bytes).hexdigest(),
                                    "record_count": len(bosses["records"])},
                                   {"path": "tools/world-bundle-compiler/held-boss-chances.json",
                                    "sha256": hashlib.sha256(chances_bytes).hexdigest(),
                                    "record_count": len(chances)}],
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
