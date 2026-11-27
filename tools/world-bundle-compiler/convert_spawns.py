#!/usr/bin/env python3
"""Convert the pinned CrystalServer monster spawn file into the `Spawn.Source` family.

Writes content/world/spawns/ (index.json plus spawns-*.json shards): one source per XML
`<monster centerx centery centerz radius>` element, in file order, each point bound to the
creature definition `oteryn:creature.<slug(name)>` and placed at the centre plus its offset.
The reconciliation packet supplies the reviewed same-floor corrections against the pinned
Crystal map. Reconciliation keeps every source key, centre, creature and respawn policy.
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
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "content/world/spawns"
CREATURES = ROOT / "content/creatures/definitions"
IMPORT = ROOT / "imports/crystalserver/summer-update"
XML = IMPORT / "raw/data-global/world/world-monster.xml"
HELD = IMPORT / "map-content-linking/spawns/held-groups.json"
HELD_SHA256 = "8a666c3d409edda5ac5996d6dec64cce24b30d5f43c75aa9a1175f1d07297c57"
DELTA = IMPORT.parent / "spawn-delta-201189f8.json"
DELTA_REMOVALS = DELTA.with_name("spawn-removals-201189f8.json")
DELTA_SHARD = "spawns-crystal-201189f8.json"
BOSSES = Path(__file__).with_name("boss-spawns.json")
BOSS_CHANCES = Path(__file__).with_name("held-boss-chances.json")
FIRST_RECONCILIATION = Path(__file__).with_name("spawn-reconciliation-1.json")
SCOPED_RECONCILIATION = ROOT / "imports/crystalserver/spawn-reconciliation/slice-2.json"
MAP_RECONCILIATION = Path(__file__).with_name("spawn-map-slice-3.json")
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


# Public Ancient Tombs bosses use the fixed respawn in Crystal 201189f8
# data-global/world/world-monster.xml:186588–199168, also retained by the
# pinned Summer map. Match each exact source point before admitting it.
PHARAOHS = {
    "oteryn:creature.dipthrah": (33097, 32589, 15),
    "oteryn:creature.mahrdis": (33191, 32957, 15),
    "oteryn:creature.morguthis": (33172, 32694, 14),
    "oteryn:creature.omruc": (33201, 33001, 14),
    "oteryn:creature.rahemos": (33076, 32780, 14),
    "oteryn:creature.thalas": (33396, 32839, 14),
    "oteryn:creature.vashresamun": (33121, 32655, 15),
}

# Crystal 201189f8 world-monster.xml:190614–190616. This mine leader is a
# public respawn with death credit, not an encounter-owned actor.
FRONTIER_MINE_LEADER = "oteryn:creature.shard_of_corruption"


def encounter_bosses() -> set[str]:
    """Keep quest-owned creatures and explicit encounter/room actors out of world clocks."""
    bindings = json.loads((ROOT / "imports/canary/bindings/creatures.json").read_text())
    room_paths = (
        "quests/ancient_tombs/",
        "quests/in_service_of_yalahar/",
        "quests/isle_of_evil/",
        "quests/liquid_black/",
        "quests/the_inquisition/",
        "quests/the_new_frontier/",
        "quests/wrath_of_the_emperor/",
    )
    keys = {
        b["target"]["key"]
        for b in bindings["bindings"]
        if b["external_id"].startswith(room_paths)
    }
    # These quest bosses have a generic bosses/ donor path. Their scripted entry/kill
    # conditions are not a world-reset scheduling policy.
    keys.update(
        f"oteryn:creature.{name}"
        for name in (
            "black_knight",
            "koshei_the_deathless",
            "energized_raging_mage",
            "the_flaming_orchid",
            "bibby_bloodbath",
            "renegade_orc",
            "thawing_dragon_lord",
            "dazed_leaf_golem",
        )
    )
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


# These points require the donor's shared Cobra Flask spawn effect or faction
# targeting between world actors. Keep their exact positions in the capture.
DELTA_HELD = {
    "cobra assassin",
    "cobra scout",
    "quara kindler",
    "rootthing conservator",
}
DELTA_FIXED_BOSSES = {"adlerauge", "clavius", "gorga", "michael the stalwart"}


def compare_delta_capture(baseline_xml, donor_xml):
    """Verify the additions and derive the removals from both pinned XML files."""
    capture = json.loads(DELTA.read_bytes())
    for raw, pin in [
        (baseline_xml, capture["baseline"]),
        (donor_xml, capture["source"]),
    ]:
        if hashlib.sha256(raw).hexdigest() != pin["sha256"]:
            sys.exit(
                "Crystal spawn delta verification requires the pinned XML revisions"
            )

    def groups(raw):
        for ordinal, group in enumerate(ET.fromstring(raw)):
            cx, cy, cz = (
                int(group.get(key)) for key in ("centerx", "centery", "centerz")
            )
            points = []
            for point_id, point in enumerate(group):
                positions = [
                    cx + int(point.get("x")),
                    cy + int(point.get("y")),
                    int(point.get("z", cz)),
                ]
                points.append(
                    {
                        "point": point_id,
                        "name": point.get("name").lower(),
                        "cell": positions,
                        "respawn_s": int(point.get("spawntime")),
                        "direction": int(point.get("direction", 0)),
                    }
                )
            yield {
                "group": ordinal,
                "centre": [cx, cy, cz],
                "radius": int(group.get("radius")),
                "points": points,
            }

    def identity(point):
        return point["name"], tuple(point["cell"])

    remaining = Counter(
        identity(point) for group in groups(baseline_xml) for point in group["points"]
    )
    difference = []
    for group in groups(donor_xml):
        added = []
        for point in group["points"]:
            key = identity(point)
            if remaining[key]:
                remaining[key] -= 1
            else:
                added.append(point)
        if added:
            difference.append({**group, "points": added})
    if difference != capture["groups"]:
        sys.exit("retained Crystal spawn delta differs from the pinned XML comparison")
    return [
        {"name": name, "cell": list(cell)}
        for name, cell in sorted(remaining.elements())
    ]


def crystal_capture():
    capture = json.loads(DELTA.read_bytes())
    raid_capture = json.loads(
        DELTA.with_name("world-monster-delta-201189f.json").read_bytes()
    )
    source = capture["source"]
    if (
        raid_capture["repository"],
        raid_capture["revision"],
        raid_capture["path"],
        raid_capture["sha256"],
        raid_capture["predecessor_sha256"],
        raid_capture["git_blob"],
    ) != (
        source["repository"],
        source["revision"],
        source["path"],
        source["sha256"],
        capture["baseline"]["sha256"],
        "7949251d25d8db8f0b2ca65b67df76ac577a5caf",
    ):
        raise ValueError("Crystal spawn delta source mismatch")

    compact = {
        (group["group"], row["point"]): (
            group["centre"],
            row["name"],
            row["cell"],
            row["direction"],
            row["respawn_s"],
        )
        for group in capture["groups"]
        for row in group["points"]
    }
    reviewed = {
        (group["source_group_ordinal"], row["source_point_ordinal"]): (
            group["center"],
            row["name"].lower(),
            row["cell"],
            row["direction"],
            row["respawn_seconds"],
        )
        for group in raid_capture["groups"]
        for row in group["points"]
    }
    for packet in (capture, raid_capture):
        groups = packet["groups"]
        if len(groups) != 412 or sum(len(group["points"]) for group in groups) != 700:
            raise ValueError("Crystal spawn delta capture is incomplete")
    if len(compact) != 700 or compact != reviewed:
        raise ValueError("Crystal spawn delta captures disagree")
    return capture


def crystal_delta(known):
    raw = DELTA.read_bytes()
    capture = crystal_capture()
    records, unsupported = [], []
    for group in capture["groups"]:
        cx, cy, cz = group["centre"]
        key = f"oteryn:spawn.crystal_201189f8.g{group['group']:05d}"
        points = []
        for row in group["points"]:
            name = row["name"]
            creature = f"oteryn:creature.{slug(name)}"
            x, y, z = row["cell"]
            if z != cz or not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF):
                sys.exit(f"{key}: point ({x}, {y}, {z}) is off the source")
            seconds = row["respawn_s"]
            point = {
                "cell": {"floor": z, "x": x, "y": y},
                "creature": creature,
                "direction": DIRECTIONS[str(row["direction"])],
            }
            # The donor XML owns world points with a respawn. Script ownership
            # applies only to script-created points outside this capture.
            if name in DELTA_HELD:
                unsupported.append(
                    {
                        "source": key,
                        "source_point_ordinal": row["point"],
                        "point": {**point, "respawn_ms": seconds * 1000},
                        "reason": "unsupported-source-behavior",
                    }
                )
                continue
            if creature not in known:
                sys.exit(f"{key}: `{name}` is not an admitted creature definition")
            if name in DELTA_FIXED_BOSSES:
                point["boss_policy"] = {"respawn": {"min_s": seconds, "max_s": seconds}}
            else:
                point["respawn_ms"] = seconds * 1000
            points.append(point)
        if points:
            records.append(
                {
                    "declaration": {
                        "centre": {"floor": cz, "x": cx, "y": cy},
                        "identity": {"key": key, "revision": "definition-r1"},
                        "points": points,
                    }
                }
            )
    removal_bytes = DELTA_REMOVALS.read_bytes()
    removed = json.loads(removal_bytes)
    if (
        removed["source"] != capture["source"]
        or removed["baseline"] != capture["baseline"]
    ):
        raise ValueError("Crystal spawn removal source mismatch")
    provenance = {
        "path": DELTA.relative_to(ROOT).as_posix(),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "record_count": len(records),
        "source": capture["source"],
        "removals": {
            "path": DELTA_REMOVALS.relative_to(ROOT).as_posix(),
            "sha256": hashlib.sha256(removal_bytes).hexdigest(),
            "point_count": len(removed["points"]),
        },
    }
    removals = Counter(
        (f"oteryn:creature.{slug(row['name'])}", tuple(row["cell"]))
        for row in removed["points"]
    )
    return records, unsupported, provenance, removals
def reconcile_cells(records, evidence):
    """Keep each original spawn and apply only the reviewed coordinate corrections."""
    moves = {}
    for row in evidence["points"]:
        if row["from"] != row["to"]:
            source = row["source"]
            x, y, z = row["from"]
            nx, ny, nz = row["to"]
            key = (source, row["creature"], x, y, z)
            if (
                key in moves
                or source > evidence["last_source_key"]
                or z != nz
                or abs(x - nx) + abs(y - ny) not in (1, 2)
                or not (0 <= nx <= 0xFFFF and 0 <= ny <= 0xFFFF)
            ):
                raise ValueError(f"invalid spawn reconciliation: {key}")
            moves[key] = {"x": nx, "y": ny, "floor": nz}
    for record in records:
        declaration = record["declaration"]
        source = declaration["identity"]["key"]
        for point in declaration["points"]:
            cell = point["cell"]
            key = (source, point["creature"], cell["x"], cell["y"], cell["floor"])
            if key in moves:
                point["cell"] = moves.pop(key)
    if moves:
        raise ValueError(f"unmatched spawn reconciliation: {next(iter(moves))}")


def convert(xml: bytes) -> dict[str, str]:
    if hashlib.sha256(xml).hexdigest() != SOURCE["files"][0]["sha256"]:
        sys.exit("the spawn XML is not the pinned file")
    held_bytes = HELD.read_bytes()
    if hashlib.sha256(held_bytes).hexdigest() != HELD_SHA256:
        sys.exit("held-groups.json is not the pinned file")
    held_records = json.loads(held_bytes)["records"]
    held = {r["source_group_ordinal"] for r in held_records}
    known, seen, records = creature_keys(), {}, []
    delta, unsupported, delta_source, removals = crystal_delta(known)
    boss_keys = set()
    for path in sorted(CREATURES.glob("creatures-*.json")):
        for record in json.loads(path.read_text())["records"]:
            details = record["authoring"]["profile"]["details"]
            if (
                details.get("bosstiary") is not None
                or details["system_eligibility"]["reward_boss"]
            ):
                boss_keys.add(record["definition"]["identity"]["key"])
    held_bosses = []
    encounter_bound = encounter_bosses()
    chances_bytes = BOSS_CHANCES.read_bytes()
    chances = json.loads(chances_bytes)["bosses"]
    scheduled_bosses = []
    koshei_spawn = None
    pharaohs = {}
    scoped_bytes = SCOPED_RECONCILIATION.read_bytes()
    scoped_anchors = json.loads(scoped_bytes)["points"]
    corrections = {}
    for row in scoped_anchors:
        if "destination" in row:
            key = (row["source"], row["creature"], tuple(row["cell"]))
            if key in corrections:
                raise ValueError(f"duplicate scoped spawn reconciliation: {key}")
            corrections[key] = tuple(row["destination"])
    mine_leader = None
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
            x, y, z = (
                cx + int(point.get("x")),
                cy + int(point.get("y")),
                int(point.get("z", cz)),
            )
            if z != cz or not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF):
                sys.exit(f"{key}: point ({x}, {y}, {z}) is off the source")
            destination = corrections.pop((key, creature, (x, y, z)), None)
            if destination is not None:
                x, y, z = destination
                if z != cz or not (0 <= x <= 0xFFFF and 0 <= y <= 0xFFFF):
                    sys.exit(
                        f"{key}: corrected position is off the source floor or plane"
                    )
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
            # Crystal 201189f8 world-monster.xml:155927–155929: 15 seconds.
            # Keep the alternate example-map spawn held.
            if (point["creature"] == "oteryn:creature.koshei_the_deathless"
                    and point["cell"] == {"x": 33271, "y": 32406, "floor": 12}):
                if koshei_spawn is not None or point["respawn_ms"] != 15000:
                    sys.exit("Koshei mainland spawn differs from the reviewed source")
                koshei_spawn = {"declaration": {
                    "centre": {"floor": cz, "x": cx, "y": cy},
                    "identity": {"key": key, "revision": "definition-r1"},
                    "points": [point],
                }}
                continue
            elif point["creature"] in PHARAOHS:
                creature = point["creature"]
                x, y, z = PHARAOHS[creature]
                if (
                    creature in pharaohs
                    or point["cell"] != {"floor": z, "x": x, "y": y}
                    or point["respawn_ms"] != 90_000
                ):
                    sys.exit(f"{key}: Ancient Tombs spawn differs from its source")
                scheduled = {
                    key: value for key, value in point.items() if key != "respawn_ms"
                }
                scheduled["boss_policy"] = {"respawn": {"min_s": 90, "max_s": 90}}
                pharaohs[creature] = {"declaration": {
                    "centre": {"floor": cz, "x": cx, "y": cy},
                    "identity": {"key": f"{key}.pharaoh", "revision": "definition-r1"},
                    "points": [scheduled],
                }}
            elif point["creature"] == FRONTIER_MINE_LEADER:
                if (mine_leader is not None or key != "oteryn:spawn.x33071_y31433_z15"
                        or point["cell"] != {"floor": 15, "x": 33072, "y": 31434}
                        or point["respawn_ms"] != 90_000):
                    sys.exit("Farmine mine leader placement differs from its source")
                scheduled = {k: v for k, v in point.items() if k != "respawn_ms"}
                scheduled["boss_policy"] = {"respawn": {"min_s": 90, "max_s": 90}}
                mine_leader = {"declaration": {
                    "centre": {"floor": cz, "x": cx, "y": cy},
                    "identity": {"key": f"{key}.mine_leader", "revision": "definition-r1"},
                    "points": [scheduled],
                }}
            elif point["creature"] in boss_keys:
                if point["creature"] in encounter_bound:
                    held_bosses.append(
                        {"source": key, "point": point, "reason": "encounter-bound"}
                    )
                else:
                    configured = chances[point["creature"]]
                    chance = configured["at_reset"]["chance"]
                    if not (
                        0
                        <= chance["numerator"]
                        <= chance["denominator"]
                        <= 0x7FFFFFFFFFFFFFFF
                        and chance["denominator"] > 0
                    ):
                        sys.exit(f"{point['creature']}: invalid boss chance")
                    scheduled = {k: v for k, v in point.items() if k != "respawn_ms"}
                    scheduled["boss_policy"] = {"at_reset": configured["at_reset"]}
                    scheduled_bosses.append(
                        {
                            "declaration": {
                                "centre": {"floor": cz, "x": cx, "y": cy},
                                "identity": {
                                    "key": f"{key}.boss.{slug(point['creature'])}",
                                    "revision": "definition-r1",
                                },
                                "points": [scheduled],
                                "provenance": {
                                    "kind": configured["basis"],
                                    "derivation": configured["source"],
                                    "policy_source": "tools/world-bundle-compiler/held-boss-chances.json",
                                    "placement_source": SOURCE,
                                    "source_spawn": key,
                                },
                            }
                        }
                    )
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
    if corrections:
        sys.exit(f"spawn reconciliation matches no donor point: {sorted(corrections)}")
    if pharaohs.keys() != PHARAOHS.keys():
        sys.exit("Ancient Tombs source spawns are incomplete")
    records.extend(scheduled_bosses)
    if koshei_spawn is None:
        sys.exit("Koshei mainland spawn missing from the pinned source")
    records.append(koshei_spawn)
    records.extend(pharaohs[key] for key in sorted(pharaohs))
    reconciliation_bytes = FIRST_RECONCILIATION.read_bytes()
    reconciliation = json.loads(reconciliation_bytes)
    reconcile_cells(records, reconciliation)
    if mine_leader is None:
        sys.exit("Farmine mine leader placement is missing")
    records.append(mine_leader)
    files, shards, retained_records = {}, [], []
    for start in range(0, len(records), SHARD_SIZE):
        part = records[start : start + SHARD_SIZE]
        name = f"spawns-{start:05d}-{start + len(part) - 1:05d}.json"
        # Preserve the pinned Summer shard boundaries used by other authors.
        kept = []
        for record in part:
            declaration = record["declaration"]
            points = []
            for point in declaration["points"]:
                cell = point["cell"]
                position = (cell["x"], cell["y"], cell["floor"])
                identity = (point["creature"], position)
                if removals[identity]:
                    removals[identity] -= 1
                    continue
                points.append(point)
            if points:
                declaration["points"] = points
                kept.append(record)
        retained_records.extend(kept)
        shards.append(f"content/world/spawns/{name}")
        files[name] = dump(
            {"coordinate_frame": FRAME, "family": "Spawn.Source", "records": kept}
        )
    if +removals:
        sys.exit("Crystal removals did not match their pinned baseline spawn cells")
    bosses_bytes = BOSSES.read_bytes()
    bosses = json.loads(bosses_bytes)
    if bosses["family"] != "Spawn.Source" or bosses["coordinate_frame"] != FRAME:
        sys.exit("boss spawn authoring family or frame mismatch")
    for record in bosses["records"]:
        for point in record["declaration"]["points"]:
            if (
                point["creature"] not in known
                or "boss_policy" not in point
                or "respawn_ms" in point
            ):
                sys.exit(
                    "boss spawn must bind an admitted creature and only a boss policy"
                )
    held_name = "held-bosses.json"
    files[held_name] = dump(
        {
            "family": "Held.Spawn.Point",
            "coordinate_frame": FRAME,
            "records": held_bosses,
            "source": SOURCE,
        }
    )
    boss_name = "spawns-boss-spawn1.json"
    files[boss_name] = dump(bosses)
    shards.append(f"content/world/spawns/{boss_name}")
    files[DELTA_SHARD] = dump(
        {"family": "Spawn.Source", "coordinate_frame": FRAME, "records": delta}
    )
    shards.append(f"content/world/spawns/{DELTA_SHARD}")
    unsupported_name = "unsupported-crystal-201189f8.json"
    files[unsupported_name] = dump(
        {
            "coordinate_frame": FRAME,
            "family": "Unsupported.Spawn.Point",
            "records": unsupported,
        }
    )
    all_records = retained_records + bosses["records"] + delta
    files["index.json"] = dump(
        {
            "coordinate_frame": FRAME,
            "family": "Spawn.Source",
            "generator": "tools/world-bundle-compiler/convert_spawns.py",
            "map_reconciliation": {
                "path": MAP_RECONCILIATION.relative_to(ROOT).as_posix(),
                "sha256": hashlib.sha256(MAP_RECONCILIATION.read_bytes()).hexdigest(),
            },
            "held_groups": {
                "group_count": len(held_records),
                "path": "imports/crystalserver/summer-update/map-content-linking/spawns/held-groups.json",
                "point_count": sum(r["point_count"] for r in held_records),
                "sha256": HELD_SHA256,
            },
            "held_bosses": {
                "path": f"content/world/spawns/{held_name}",
                "point_count": len(held_bosses),
                "sha256": hashlib.sha256(
                    (files[held_name] + "\n").encode()
                ).hexdigest(),
            },
            "population_state": "POPULATED",
            "point_count": sum(len(r["declaration"]["points"]) for r in all_records),
            "record_count": len(all_records),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shards,
            "source": SOURCE,
            "unsupported_crystal_delta": {
                "path": f"content/world/spawns/{unsupported_name}",
                "point_count": len(unsupported),
                "sha256": hashlib.sha256(
                    (files[unsupported_name] + "\n").encode()
                ).hexdigest(),
            },
            "additional_sources": [
                {
                    "path": "tools/world-bundle-compiler/boss-spawns.json",
                    "sha256": hashlib.sha256(bosses_bytes).hexdigest(),
                    "record_count": len(bosses["records"]),
                },
                {
                    "path": "tools/world-bundle-compiler/held-boss-chances.json",
                    "sha256": hashlib.sha256(chances_bytes).hexdigest(),
                    "record_count": len(chances),
                },
                {
                    "path": "tools/world-bundle-compiler/spawn-reconciliation-1.json",
                    "sha256": hashlib.sha256(reconciliation_bytes).hexdigest(),
                    "record_count": len(reconciliation["points"]),
                },
                {
                    "path": SCOPED_RECONCILIATION.relative_to(ROOT).as_posix(),
                    "sha256": hashlib.sha256(scoped_bytes).hexdigest(),
                    "record_count": len(scoped_anchors),
                },
                delta_source,
            ],
        }
    )
    return {name: text + "\n" for name, text in files.items()}


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--xml", default=XML, type=Path)
    parser.add_argument("--check", action="store_true")
    delta_mode = parser.add_mutually_exclusive_group()
    delta_mode.add_argument("--verify-delta-xml", type=Path)
    delta_mode.add_argument("--capture-delta-removals", type=Path, metavar="DONOR_XML")
    args = parser.parse_args()
    baseline = args.xml.read_bytes()
    if donor := args.verify_delta_xml or args.capture_delta_removals:
        removed = compare_delta_capture(baseline, donor.read_bytes())
        capture = json.loads(DELTA.read_bytes())
        packet = {
            "source": capture["source"],
            "baseline": capture["baseline"],
            "points": removed,
        }
        if args.capture_delta_removals:
            DELTA_REMOVALS.write_text(json.dumps(packet, indent=2) + "\n")
        elif packet != json.loads(DELTA_REMOVALS.read_bytes()):
            sys.exit(
                "retained Crystal spawn removals differ from the pinned XML comparison"
            )
    files = convert(baseline)
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
