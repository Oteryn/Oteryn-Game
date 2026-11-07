"""SRV7 one source-qualified Summer weighted point; no guessed cells or extra actors."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INPUT = Path(__file__).with_name("summer-weighted-source.json")
FRAME = "global-target-2026-09-27"
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
KEY = "oteryn:spawn.x33866_y32698_z9"
SOURCE_XML_SHA256 = "f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e"


def dump(value):
    return (
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        + "\n"
    )


def record(value):
    if (
        value["revision"] != PIN
        or value["key"] != KEY
        or value["schema"] != 1
        or value["sha256"] != SOURCE_XML_SHA256
        or value["path"] != "data-global/world/world-monster.xml"
    ):
        raise ValueError("SUMMER_WEIGHTED_SOURCE_PIN")
    if value["centre"] != {"x": 33866, "y": 32698, "floor": 9}:
        raise ValueError("SUMMER_WEIGHTED_SOURCE_CELL")
    # Crystal spawn_monster.cpp:461-492 merges by cell, and the final same-cell
    # child owns direction/interval. :558-587 selects by positive integer weight.
    expected = [
        {
            "name": "Silverfrost Sentinel",
            "x": "0",
            "y": "0",
            "z": "9",
            "spawntime": "60",
            "weight": "25",
        },
        {
            "name": "True Frost Flower Asura",
            "x": "0",
            "y": "0",
            "z": "9",
            "spawntime": "47104",
            "weight": "25",
        },
        {
            "name": "True Frost Flower Asura",
            "x": "0",
            "y": "1",
            "z": "9",
            "spawntime": "60",
        },
    ]
    if value["points"] != expected:
        raise ValueError("SUMMER_WEIGHTED_SOURCE_POINTS")
    sentinel = "oteryn:creature.silverfrost_sentinel"
    asura = "oteryn:creature.true_frost_flower_asura"
    points = [
        {
            "cell": value["centre"],
            "creature": asura,
            "direction": "north",
            "respawn_ms": 47_104_000,
            "weighted": [
                {"creature": sentinel, "weight": 25},
                {"creature": asura, "weight": 25},
            ],
        },
        {
            "cell": {"x": 33866, "y": 32699, "floor": 9},
            "creature": asura,
            "direction": "north",
            "respawn_ms": 60_000,
        },
    ]
    return {
        "declaration": {
            "identity": {"key": KEY, "revision": "definition-r1"},
            "centre": value["centre"],
            "points": points,
            "provenance": value,
        }
    }


def augment(files):
    value = json.loads(INPUT.read_bytes())
    row = record(value)
    index = json.loads(files["index.json"])
    name = "spawns-srv7-summer-weighted.json"
    relative = "content/world/spawns/" + name
    if relative in index["shards"]:
        if json.loads(files[name])["records"] != [row]:
            raise ValueError("SUMMER_WEIGHTED_EXISTING_SOURCE_CONFLICT")
        return files
    for shard in index["shards"]:
        for existing in json.loads(files[Path(shard).name])["records"]:
            if existing["declaration"]["identity"]["key"] == KEY:
                raise ValueError("SUMMER_WEIGHTED_DUPLICATE_SOURCE")
    files[name] = dump(
        {"coordinate_frame": FRAME, "family": "Spawn.Source", "records": [row]}
    )
    index["shards"].append(relative)
    index["record_count"] += 1
    index["point_count"] += 2
    index.setdefault("additional_sources", []).append(
        {
            "path": str(INPUT.relative_to(ROOT)),
            "record_count": 1,
            "sha256": hashlib.sha256(INPUT.read_bytes()).hexdigest(),
        }
    )
    # Original raw held import evidence remains immutable. This exact proved
    # group's qualification supersedes its importer hold, not other weighted groups.
    index["qualified_held_groups"] = [
        {
            "key": KEY,
            "source_point_count": 3,
            "physical_point_count": 2,
            "weighted_choices": 2,
            "source_revision": PIN,
        }
    ]
    files["index.json"] = dump(index)
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    import convert_spawns

    files = convert_spawns.convert(convert_spawns.XML.read_bytes())
    for name, text in files.items():
        path = convert_spawns.OUT / name
        if args.check:
            if path.read_text() != text:
                raise ValueError("SUMMER_WEIGHTED_SPAWN_DRIFT:" + name)
        else:
            path.write_text(text)


if __name__ == "__main__":
    main()
