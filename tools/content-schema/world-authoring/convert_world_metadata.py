#!/usr/bin/env python3
"""Convert pinned CrystalServer world metadata into Oteryn content families.

Writes teleport Transitions (content/world/transitions/) plus the committed capture
summary; each teleport `object` is the canonical A12 key of its item id: the Item key when
the id has an Item record (the runtime follows the 4.6 routed_to pointer), else the WO-2
WorldObject or Terrain catalogue key, else the run fails closed. The source is OTS_HYPOTHESIS_ONLY migration
evidence: only normalized facts are written, never map bytes. Cities and Regions are owned
by area-authoring; Terrain, objects and placements are out of scope.

A few teleports live in CrystalServer step-in scripts instead of map attributes (the New
Haven vocation forcefield). `SCRIPT_TELEPORTS` pins each one with its script's sha256; the
run verifies the script still registers that tile and teleports to that destination, and
writes the Transition under the `crystalserver/script-step-in-teleport` namespace.

    python convert_world_metadata.py --crystal-root /path/to/crystalserver [--check]
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import otbm_reader

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "samples/source-capture-v1.json"
ITEM_DEFINITIONS = "content/items/definitions"
ITEM_KEY = re.compile(r"^oteryn:item\.tibia\.i([1-9][0-9]*)$")

MAP_NAMESPACE = "crystalserver/map-item-position"
SCRIPT_NAMESPACE = "crystalserver/script-step-in-teleport"
# Teleports a pinned step-in script performs on a map tile that carries no destination.
SCRIPT_TELEPORTS = (
    {
        "from": (32538, 32514, 7),
        "item": 37001,
        "path": "data-global/scripts/quests/newhaven/movement_select_vocation.lua",
        "sha256": "1d44beffed174b6b3025f0fd4fc00393725445e3bb1a6729a633d9f891ac563b",
        "to": (32544, 32515, 7),
    },
)
SOURCE = {
    "evidence": "OtsHypothesisOnly",
    "files": [
        {
            "path": "data-global/world/world.otbm",
            "sha256": "dcb735549bd11de526c4bd441bbf62e4490efbb60ff7aa334530f1692345d8d7",
        },
        *(
            {"path": path, "sha256": sha256}
            for path, sha256 in sorted(
                {(s["path"], s["sha256"]) for s in SCRIPT_TELEPORTS}
            )
        ),
    ],
    "ref": "summer-update",
    "repository": "zimbadev/crystalserver",
    "revision": "00ce02a57ca5a12e48f32a3476e37471167e4c3f",
    "source_key": "oteryn:source.crystalserver",
}
OTBM = "data-global/world/world.otbm"
COORDINATE_FRAME = "global-target-2026-09-27"
REVISION = "definition-r1"
SHARD_SIZE = 500
MAX_FLOOR = 15
UNSET = (0, 0, 0)
CATALOGUES = ("content/world/objects", "content/world/terrain")
GENERATOR = "tools/content-schema/world-authoring/convert_world_metadata.py"

FAMILIES = {
    "Area.HuntingPlace": {
        "dir": "content/world/areas/hunting-places",
        "stem": "hunting-places",
        "schema": "OTERYN_AREA_AUTHORING_SHARD/v1",
    },
    "Area.Island": {
        "dir": "content/world/areas/islands",
        "stem": "islands",
        "schema": "OTERYN_AREA_AUTHORING_SHARD/v1",
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


def item_keys(root: Path = ROOT) -> dict[int, str]:
    """Item id -> key for every Item record in content/items/definitions."""
    found: dict[int, str] = {}
    for shard in sorted((root / ITEM_DEFINITIONS).glob("items-*.json")):
        for record in json.loads(shard.read_text(encoding="utf-8"))["records"]:
            key = record["definition"]["identity"]["key"]
            match = ITEM_KEY.match(key)
            if match:
                found[int(match.group(1))] = key
    return found


def teleport_candidates(facts) -> tuple[list[dict], dict]:
    kept, rejected = [], {"unset_destination": 0, "destination_outside_map": 0}
    for tp in facts.teleports:
        if tp["to"] == UNSET:
            rejected["unset_destination"] += 1
        elif not in_map(facts, *tp["to"]):
            rejected["destination_outside_map"] += 1
        else:
            kept.append(tp)
    return kept, rejected


def catalogue_keys(root: Path = ROOT) -> dict[str, str]:
    """Record key -> family for the WO-2 WorldObject and Terrain catalogues."""
    found: dict[str, str] = {}
    for directory in CATALOGUES:
        for shard in sorted((root / directory).glob("*-*.json")):
            document = json.loads(shard.read_text(encoding="utf-8"))
            for record in document["records"]:
                found[record["identity"]["key"]] = document["family"]
    return found


def object_reference(item_id: int, catalogue: dict[str, str], items: dict[int, str]):
    """Canonical A12 key: the Item record first, else the WO-2 catalogue, else fail."""
    if item_id in items:
        return ref("Item", items[item_id])
    for family, key in (
        ("WorldObject", f"oteryn:world-object.tibia.i{item_id}"),
        ("Terrain", f"oteryn:terrain.tibia.i{item_id}"),
    ):
        if catalogue.get(key) == family:
            return ref(family, key)
    raise ConvertError(f"teleport item {item_id} has no Item record or catalogue key")


SCRIPT_POSITION = re.compile(
    r":position\(\s*Position\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)\s*\)"
)
SCRIPT_TELEPORT_TO = re.compile(
    r"teleportTo\(\s*Position\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)"
)


def verify_script(entry: dict, data: bytes | None) -> None:
    """Fail unless `data` is the pinned script of `entry` and still registers its tile
    and teleports to its destination."""
    if data is None:
        raise ConvertError(f"{entry['path']}: script teleport source is not loaded")
    if hashlib.sha256(data).hexdigest() != entry["sha256"]:
        raise ConvertError(f"{entry['path']}: sha256 differs from the pinned script")
    text = data.decode("utf-8")
    registered = {tuple(map(int, m)) for m in SCRIPT_POSITION.findall(text)}
    targets = {tuple(map(int, m)) for m in SCRIPT_TELEPORT_TO.findall(text)}
    if registered != {entry["from"]} or targets != {entry["to"]}:
        raise ConvertError(
            f"{entry['path']}: script no longer teleports {entry['from']} -> {entry['to']}"
        )
    if entry["from"] == entry["to"]:
        raise ConvertError(f"{entry['path']}: script teleport to its own tile")


def script_candidates(
    facts, blobs: dict[str, bytes], scripts=SCRIPT_TELEPORTS
) -> list[dict]:
    """The pinned script teleports, each verified against its script text and the map.

    The script gives the destination of a map teleport attribute left unset (0, 0, 0) on
    an item of the pinned id. A script that no longer registers the tile or teleports
    elsewhere, a position outside the map, or a tile without exactly that one unset
    attribute fails the run closed.
    """
    found = []
    for entry in scripts:
        verify_script(entry, blobs.get(entry["path"]))
        if not (in_map(facts, *entry["from"]) and in_map(facts, *entry["to"])):
            raise ConvertError(f"{entry['path']}: script teleport outside the map")
        on_tile = [tp for tp in facts.teleports if tp["from"] == entry["from"]]
        if [(tp["item"], tp["to"]) for tp in on_tile] != [(entry["item"], UNSET)]:
            raise ConvertError(
                f"{entry['path']}: script teleport tile {entry['from']} needs exactly one "
                f"unset teleport attribute of item {entry['item']}, has {on_tile}"
            )
        found.append(
            {
                "from": entry["from"],
                "item": entry["item"],
                "namespace": SCRIPT_NAMESPACE,
                "to": entry["to"],
            }
        )
    if len({c["from"] for c in found}) != len(found):
        raise ConvertError("two script teleports share a tile")
    return found


def teleports(
    candidates: list[dict],
    present: set,
    items: dict[int, str],
    rejected: dict,
    catalogue: dict[str, str],
):
    records = []
    rejected["destination_tile_absent"] = 0
    families = {"Item": 0, "Terrain": 0, "WorldObject": 0}
    ordinal: dict[tuple, int] = {}
    for tp in sorted(
        candidates,
        key=lambda t: (t["from"][2], t["from"][1], t["from"][0], t["item"], t["to"]),
    ):
        if tp["to"] not in present:
            if "namespace" in tp:
                raise ConvertError(
                    f"script teleport destination {tp['to']} has no tile"
                )
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
        declaration["object"] = object_reference(tp["item"], catalogue, items)
        families[declaration["object"]["family"]] += 1
        records.append(
            {
                "declaration": declaration,
                "source_bindings": [
                    binding(
                        "Transition",
                        key,
                        tp.get("namespace", MAP_NAMESPACE),
                        f"{x},{y},{z}:{tp['item']}" + (f"#{n}" if n > 1 else ""),
                    )
                ],
            }
        )
    return unique(records, "Transition.Teleport"), families


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


def build(
    blobs: dict[str, bytes], root: Path = ROOT, scripts=SCRIPT_TELEPORTS
) -> dict[str, bytes]:
    otbm = blobs[OTBM]
    facts = otbm_reader.read(otbm)
    if facts.unknown_item_attrs:
        raise ConvertError(
            f"unknown OTBM item attributes: {dict(facts.unknown_item_attrs)}"
        )
    candidates, rejected = teleport_candidates(facts)
    scripted = script_candidates(facts, blobs, scripts)
    # a script destination replaces the unset attribute of its tile
    rejected["unset_destination"] -= len(scripted)
    probe = {tp["to"] for tp in candidates + scripted}
    present = otbm_reader.read(otbm, probe=probe).present
    tp_records, object_families = teleports(
        candidates + scripted, present, item_keys(root), rejected, catalogue_keys(root)
    )
    out = {}
    out.update(shard_files("Transition.Teleport", tp_records, source_of(scripts)))
    summary = {
        "families": {"Transition.Teleport": len(tp_records)},
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
        "script_teleports": len(scripted),
        "source": source_of(scripts),
        "teleport_object_families": object_families,
    }
    out[str(SUMMARY.relative_to(ROOT))] = canonical(summary)
    return out


def source_of(scripts=SCRIPT_TELEPORTS) -> dict:
    """SOURCE with exactly the script files `scripts` reads (all of them by default)."""
    otbm = [row for row in SOURCE["files"] if row["path"] == OTBM]
    rows = sorted({(s["path"], s["sha256"]) for s in scripts})
    return {
        **SOURCE,
        "files": otbm + [{"path": path, "sha256": sha256} for path, sha256 in rows],
    }


def generated_extras(out: dict[str, bytes], root: Path = ROOT) -> list[str]:
    """Generator-owned shards on disk that this run no longer generates.

    Owned means the family's own `<stem>-NNNNN-NNNNN.json` names in a directory whose index
    this run writes; the index and any other file are never touched.
    """
    extras = []
    for spec in FAMILIES.values():
        if f"{spec['dir']}/index.json" not in out:
            continue
        pattern = re.compile(rf"^{re.escape(spec['stem'])}-\d{{5}}-\d{{5}}\.json$")
        directory = root / spec["dir"]
        for path in sorted(directory.iterdir() if directory.is_dir() else []):
            relative = f"{spec['dir']}/{path.name}"
            if pattern.fullmatch(path.name) and relative not in out:
                extras.append(relative)
    return extras


def apply(out: dict[str, bytes], check: bool, root: Path = ROOT) -> int:
    """Check or write `out`; both report and (in write mode) delete extra owned shards."""
    stale = [
        path
        for path, data in out.items()
        if not (root / path).is_file() or (root / path).read_bytes() != data
    ]
    extras = generated_extras(out, root)
    if check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        for path in extras:
            print(f"EXTRA {path}", file=sys.stderr)
        return 1 if stale or extras else 0
    for path in stale:
        (root / path).parent.mkdir(parents=True, exist_ok=True)
        (root / path).write_bytes(out[path])
    for path in extras:
        (root / path).unlink()
    print(f"wrote {len(stale)} of {len(out)} files, removed {len(extras)}")
    return 0


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
    return apply(out, args.check)


if __name__ == "__main__":
    raise SystemExit(main())
