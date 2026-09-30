#!/usr/bin/env python3
"""Convert pinned CrystalServer world metadata into Oteryn content families.

Writes teleport Transitions (content/world/transitions/) plus the committed capture
summary; each teleport `object` is the A12 4.6 family key of its item id (WorldObject or
Terrain from the WO-2 catalogue, else Item). The source is OTS_HYPOTHESIS_ONLY migration
evidence: only normalized facts are written, never map bytes. Cities and Regions are owned
by area-authoring; Terrain, objects and placements are out of scope.

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
ITEM_BINDINGS = ROOT / "imports/crystalserver/bindings/items.json"

SOURCE = {
    "evidence": "OtsHypothesisOnly",
    "files": [
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
CATALOGUES = ("content/world/objects", "content/world/terrain")
GENERATOR = "tools/content-schema/world-authoring/convert_world_metadata.py"

FAMILIES = {
    "Area.HuntingPlace": {
        "dir": "content/world/areas/hunting-places",
        "stem": "hunting-places",
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


def item_keys() -> dict[int, str]:
    rows = json.loads(ITEM_BINDINGS.read_text(encoding="utf-8"))["bindings"]
    return {
        int(row["external_id"]): row["target"]["key"]
        for row in rows
        if row["identity_namespace"] == "ots/item_server_id"
    }


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
    """A12 4.6: the family key of the id -- WorldObject, else Terrain, else Item."""
    for family, key in (
        ("WorldObject", f"oteryn:world-object.tibia.i{item_id}"),
        ("Terrain", f"oteryn:terrain.tibia.i{item_id}"),
    ):
        if catalogue.get(key) == family:
            return ref(family, key)
    if item_id in items:
        return ref("Item", items[item_id])
    raise ConvertError(
        f"teleport item {item_id} has no WorldObject, Terrain or Item key"
    )


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
                        "crystalserver/map-item-position",
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


def build(blobs: dict[str, bytes], root: Path = ROOT) -> dict[str, bytes]:
    otbm = blobs["data-global/world/world.otbm"]
    facts = otbm_reader.read(otbm)
    if facts.unknown_item_attrs:
        raise ConvertError(
            f"unknown OTBM item attributes: {dict(facts.unknown_item_attrs)}"
        )
    candidates, rejected = teleport_candidates(facts)
    present = otbm_reader.read(otbm, probe={tp["to"] for tp in candidates}).present
    tp_records, object_families = teleports(
        candidates, present, item_keys(), rejected, catalogue_keys(root)
    )
    out = {}
    out.update(shard_files("Transition.Teleport", tp_records))
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
        "source": SOURCE,
        "teleport_object_families": object_families,
    }
    out[str(SUMMARY.relative_to(ROOT))] = canonical(summary)
    return out


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
