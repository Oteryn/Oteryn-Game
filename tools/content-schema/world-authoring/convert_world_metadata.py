#!/usr/bin/env python3
"""Convert pinned CrystalServer world metadata into Oteryn content families.

Writes City Areas (content/world/areas/cities/), Houses (content/houses/) and teleport
Transitions (content/world/transitions/) plus the committed capture summary. The source
is OTS_HYPOTHESIS_ONLY migration evidence: only normalized facts are written, never map
bytes. Terrain, objects and placements are out of scope.

    python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import otbm_reader

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "samples/source-capture-v1.json"
ITEM_BINDINGS = ROOT / "imports/crystalserver/bindings/items.json"

SOURCE = {
    "evidence": "OtsHypothesisOnly",
    "files": [
        {
            "path": "data-global/world/world-house.xml",
            "sha256": "36044bf9636c5a84cac7dda6582965fba05378822fdce4a5c1f84d0dd7e6e90b",
        },
        {
            "path": "data-global/world/world.otbm",
            "sha256": "dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7",
        },
    ],
    "ref": "summer-update",
    "repository": "zimbadev/crystalserver",
    "revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
    "source_key": "oteryn:source.crystalserver",
}
COORDINATE_FRAME = "global-target-2026-09-27"
REVISION = "definition-r1"
SHARD_SIZE = 500
MAX_FLOOR = 15
GENERATOR = "tools/content-schema/world-authoring/convert_world_metadata.py"

FAMILIES = {
    "Area.City": {
        "dir": "content/world/areas/cities",
        "stem": "cities",
        "schema": "OTERYN_AREA_AUTHORING_SHARD/v1",
    },
    "Area.HuntingPlace": {
        "dir": "content/world/areas/hunting-places",
        "stem": "hunting-places",
        "schema": "OTERYN_AREA_AUTHORING_SHARD/v1",
    },
    "House": {
        "dir": "content/houses",
        "stem": "houses",
        "schema": "OTERYN_HOUSE_AUTHORING_SHARD/v1",
    },
    "Transition.Teleport": {
        "dir": "content/world/transitions",
        "stem": "teleports",
        "schema": "OTERYN_TRANSITION_AUTHORING_SHARD/v1",
    },
}


class ConvertError(Exception):
    pass


def canonical(value) -> bytes:
    text = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return (text + "\n").encode()


def slug(text: str) -> str:
    value = "_".join(re.findall(r"[a-z0-9]+", text.lower()))
    if not value or len(value) > 64:
        raise ConvertError(f"{text!r} has no production slug")
    return value


def ref(family: str, key: str) -> dict:
    return {"family": family, "key": key, "revision": REVISION}


def position(x: int, y: int, z: int) -> dict:
    return {"coordinate_frame": COORDINATE_FRAME, "floor": z, "x": x, "y": y}


def binding(family: str, key: str, namespace: str, external_id: str) -> dict:
    return {
        "disposition": "EXACT",
        "external_id": external_id,
        "identity_namespace": namespace,
        "source_key": SOURCE["source_key"],
        "source_revision": SOURCE["revision"],
        "target": ref(family, key),
    }


def committed_keys(root: Path, family: str, namespace: str) -> dict[str, str]:
    """Source id -> key already committed for `namespace`, read from the family shards."""
    directory = root / FAMILIES[family]["dir"]
    index = directory / "index.json"
    if not index.is_file():
        return {}
    found: dict[str, str] = {}
    for shard in json.loads(index.read_text(encoding="utf-8")).get("shards", []):
        shard_doc = json.loads((root / shard).read_text(encoding="utf-8"))
        for record in shard_doc["records"]:
            for row in record["source_bindings"]:
                if row["identity_namespace"] == namespace:
                    found[row["external_id"]] = row["target"]["key"]
    return found


def assign_keys(
    sources: list[tuple[str, str]], committed: dict[str, str], prefix: str, family: str
) -> dict[str, str]:
    """Stable identity: reuse the committed key per source id; mint a slug only for new ids.

    `sources` is (source id, display name). Every committed key stays reserved, so a new id
    can never take over an existing identity; a slug collision fails closed.
    """
    taken = set(committed.values())
    keys = {}
    for source_id, name in sources:
        if source_id in committed:
            keys[source_id] = committed[source_id]
    for source_id, name in sources:
        if source_id in keys:
            continue
        key = f"{prefix}{slug(name)}"
        if key in taken:
            raise ConvertError(
                f"{family}: new source id {source_id} collides with {key}"
            )
        taken.add(key)
        keys[source_id] = key
    return keys


def unique(records: list[dict], family: str) -> list[dict]:
    records.sort(key=lambda r: r["declaration"]["identity"]["key"])
    keys = [r["declaration"]["identity"]["key"] for r in records]
    if len(set(keys)) != len(keys):
        raise ConvertError(f"{family}: duplicate keys")
    return records


def in_map(facts, x: int, y: int, z: int) -> bool:
    return 0 <= x < facts.width and 0 <= y < facts.height and 0 <= z <= MAX_FLOOR


def item_keys() -> dict[int, str]:
    rows = json.loads(ITEM_BINDINGS.read_text(encoding="utf-8"))["bindings"]
    return {
        int(row["external_id"]): row["target"]["key"]
        for row in rows
        if row["identity_namespace"] == "ots/item_server_id"
    }


def cities(facts, root: Path = ROOT) -> tuple[list[dict], dict[int, str]]:
    records, by_id = [], {}
    keys = assign_keys(
        [(str(t["town_id"]), t["name"]) for t in facts.towns],
        committed_keys(root, "Area.City", "crystalserver/town-id"),
        "oteryn:area.city.",
        "Area.City",
    )
    for town in facts.towns:
        key = keys[str(town["town_id"])]
        if not in_map(facts, *town["temple"]):
            raise ConvertError(f"{key}: temple outside map")
        by_id[town["town_id"]] = key
        records.append(
            {
                "declaration": {
                    "area_kind": "city",
                    "identity": {"key": key, "revision": REVISION},
                    "kind": "Area",
                    "name": town["name"],
                    "temple": position(*town["temple"]),
                },
                "source_bindings": [
                    binding("Area", key, "crystalserver/town-id", str(town["town_id"]))
                ],
            }
        )
    return unique(records, "Area.City"), by_id


def houses(
    facts, house_xml: bytes, city_keys: dict[int, str], root: Path = ROOT
) -> list[dict]:
    records = []
    rows = ET.fromstring(house_xml).findall("house")
    xml_ids = {int(row.get("houseid")) for row in rows}
    if xml_ids != set(facts.houses):
        raise ConvertError("world-house.xml and OTBM house tiles disagree")
    keys = assign_keys(
        [(row.get("houseid"), row.get("name")) for row in rows],
        committed_keys(root, "House", "crystalserver/house-id"),
        "oteryn:house.",
        "House",
    )
    for row in rows:
        house_id = int(row.get("houseid"))
        tiles = facts.houses[house_id]
        key = keys[row.get("houseid")]
        entry = (int(row.get("entryx")), int(row.get("entryy")), int(row.get("entryz")))
        town_id = int(row.get("townid"))
        if town_id not in city_keys:
            raise ConvertError(f"{key}: unknown town {town_id}")
        if not in_map(facts, *entry):
            raise ConvertError(f"{key}: entry outside map")
        doors = sorted(tiles.doors, key=lambda d: (d[3], d[2], d[1], d[0]))
        records.append(
            {
                "declaration": {
                    "beds": int(row.get("beds")),
                    "city": ref("Area", city_keys[town_id]),
                    "declared_size": int(row.get("size")),
                    "doors": [
                        {"door_id": d[3], "position": position(d[0], d[1], d[2])}
                        for d in doors
                    ],
                    "entry": position(*entry),
                    "footprint": {
                        "coordinate_frame": COORDINATE_FRAME,
                        "floors": [
                            {"floor": z, "tiles": n}
                            for z, n in sorted(tiles.floors.items())
                        ],
                        "max_x": tiles.bbox[2],
                        "max_y": tiles.bbox[3],
                        "min_x": tiles.bbox[0],
                        "min_y": tiles.bbox[1],
                        "tile_count": tiles.tiles,
                    },
                    "guildhall": row.get("guildhall") == "true",
                    "identity": {"key": key, "revision": REVISION},
                    "kind": "House",
                    "name": row.get("name"),
                    "rent": int(row.get("rent")),
                },
                "source_bindings": [
                    binding("House", key, "crystalserver/house-id", str(house_id))
                ],
            }
        )
    return unique(records, "House")


def teleport_candidates(facts) -> tuple[list[dict], dict]:
    kept, rejected = [], {"unset_destination": 0, "destination_outside_map": 0}
    for tp in facts.teleports:
        if tp["to"] == (0, 0, 0):
            rejected["unset_destination"] += 1
        elif not in_map(facts, *tp["to"]):
            rejected["destination_outside_map"] += 1
        else:
            kept.append(tp)
    return kept, rejected


def teleports(
    candidates: list[dict], present: set, items: dict[int, str], rejected: dict
):
    records = []
    rejected["destination_tile_absent"] = 0
    unbound = 0
    ordinal: dict[tuple, int] = {}
    for tp in sorted(
        candidates,
        key=lambda t: (t["from"][2], t["from"][1], t["from"][0], t["item"], t["to"]),
    ):
        if tp["to"] not in present:
            rejected["destination_tile_absent"] += 1
            continue
        x, y, z = tp["from"]
        n = ordinal[tp["from"]] = ordinal.get(tp["from"], 0) + 1
        key = f"oteryn:transition.teleport.x{x}_y{y}_z{z}" + (f"_{n}" if n > 1 else "")
        declaration = {
            "from": position(x, y, z),
            "identity": {"key": key, "revision": REVISION},
            "kind": "Transition",
            "to": position(*tp["to"]),
            "transition_kind": "teleport",
        }
        if tp["item"] in items:
            declaration["object"] = ref("Item", items[tp["item"]])
        else:
            unbound += 1
        records.append(
            {
                "declaration": declaration,
                "source_bindings": [
                    binding(
                        "Transition",
                        key,
                        "crystalserver/map-item-position",
                        f"{x},{y},{z}:{tp['item']}" + (f"#{n}" if n > 1 else ""),
                    )
                ],
            }
        )
    return unique(records, "Transition.Teleport"), unbound


def shard_files(
    family: str, records: list[dict], source: dict = SOURCE, generator: str = GENERATOR
) -> dict[str, bytes]:
    spec = FAMILIES[family]
    out, shards = {}, []
    for index, start in enumerate(range(0, len(records), SHARD_SIZE)):
        chunk = records[start : start + SHARD_SIZE]
        end = start + len(chunk) - 1
        path = f"{spec['dir']}/{spec['stem']}-{start:05d}-{end:05d}.json"
        shards.append(path)
        out[path] = canonical(
            {
                "family": family,
                "records": chunk,
                "schema": spec["schema"],
                "shard": {
                    "count": len(chunk),
                    "end": end,
                    "index": index,
                    "start": start,
                },
            }
        )
    out[f"{spec['dir']}/index.json"] = canonical(
        {
            "coordinate_frame": COORDINATE_FRAME,
            "family": family,
            "generator": generator,
            "population_state": "POPULATED",
            "record_count": len(records),
            "schema": "OTERYN_FAMILY_INDEX/v1",
            "shard_size": SHARD_SIZE,
            "shards": shards,
            "source": source,
        }
    )
    return out


def read_source(crystal_root: Path) -> dict[str, bytes]:
    blobs = {}
    for row in SOURCE["files"]:
        data = (crystal_root / row["path"]).read_bytes()
        if hashlib.sha256(data).hexdigest() != row["sha256"]:
            raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
        blobs[row["path"]] = data
    return blobs


def build(blobs: dict[str, bytes], root: Path = ROOT) -> dict[str, bytes]:
    otbm = blobs["data-global/world/world.otbm"]
    facts = otbm_reader.read(otbm)
    if facts.unknown_item_attrs:
        raise ConvertError(
            f"unknown OTBM item attributes: {dict(facts.unknown_item_attrs)}"
        )
    city_records, city_keys = cities(facts, root)
    house_records = houses(
        facts, blobs["data-global/world/world-house.xml"], city_keys, root
    )
    candidates, rejected = teleport_candidates(facts)
    present = otbm_reader.read(otbm, probe={tp["to"] for tp in candidates}).present
    tp_records, unbound = teleports(candidates, present, item_keys(), rejected)
    out = {}
    out.update(shard_files("Area.City", city_records))
    out.update(shard_files("House", house_records))
    out.update(shard_files("Transition.Teleport", tp_records))
    summary = {
        "families": {
            "Area.City": len(city_records),
            "House": len(house_records),
            "Transition.Teleport": len(tp_records),
        },
        "map": {
            "floors": [0, MAX_FLOOR],
            "height": facts.height,
            "otbm_version": facts.version,
            "tiles": facts.tiles,
            "width": facts.width,
        },
        "not_imported": {
            "teleports": rejected,
            "waypoints": len(facts.waypoints),
        },
        "schema": "OTERYN_WORLD_METADATA_SOURCE_CAPTURE/v1",
        "source": SOURCE,
        "teleports_without_item_binding": unbound,
    }
    out[str(SUMMARY.relative_to(ROOT))] = canonical(summary)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crystal-root", type=Path, required=True)
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    args = parser.parse_args()
    try:
        out = build(read_source(args.crystal_root))
    except (ConvertError, otbm_reader.OtbmError, OSError) as error:
        print(f"FAIL {error}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        return 1 if stale else 0
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
