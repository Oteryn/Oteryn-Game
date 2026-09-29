#!/usr/bin/env python3
"""Convert the appearance-only ids of the base map into `Terrain` records.

Owner rule: ids present in the pinned CrystalServer `items.xml` belong to the Item registry
(item agent, B1b); ids that only the official client knows (appearance-only) belong to the
Terrain family and get a permanent `oteryn:terrain.a<id>` key here. Reads, all offline:

- the base map palette (`content/world/placements/index.json`) and region files (occurrence
  counts of top-level tile items),
- the official client `appearances-<sha256>.dat` under `content/assets/files/` (verified against
  `imports/official/client-assets/15.30/manifest.json`), decoded by `client_appearance_reader`,

and writes content/world/terrain/ plus `samples/terrain-capture-v1.json`. Every field comes from
the appearance of that id; nothing is guessed. `class` follows one ordered rule over the flags:

    bank flag            -> ground      (walkable surface; `speed` is the bank waypoints value)
    else clip flag       -> border      (edge pieces laid over a ground)
    else unpass flag     -> blocking    (creatures cannot pass)
    else                 -> decoration

    python convert_terrain.py [--check] [--crystal-root PATH]

The set of records is the palette entries whose key is already a Terrain key. With
`--crystal-root` (the pinned checkout) the set is derived instead: palette ids that no Item
binding covers, that `items.xml` does not declare and that the client knows; the first run
creates the records and `convert_world_base.py` then switches those palette keys.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
from collections import Counter
from pathlib import Path

import client_appearance_reader as appearances
import convert_floor_changes as floor_changes
import convert_map_regions as regions
import convert_world_base as world_base
import convert_world_metadata as base

ROOT = base.ROOT
HERE = base.HERE
FAMILY = "Terrain"
DIRECTORY = world_base.TERRAIN_DIRECTORY
KEY_PREFIX = world_base.TERRAIN_KEY_PREFIX
NAMESPACE = world_base.TERRAIN_NAMESPACE
SOURCE_KEY = regions.SOURCE_KEY
GENERATOR = "tools/content-schema/world-authoring/convert_terrain.py"
SUMMARY = "tools/content-schema/world-authoring/samples/terrain-capture-v1.json"
PLACEMENTS_INDEX = f"{world_base.DIRECTORY}/index.json"
APPEARANCES_NAME = re.compile(r"^appearances-([0-9a-f]{64})\.dat$")
ITEM_KEY_PREFIX = "oteryn:item."
CLASSES = ("ground", "border", "blocking", "decoration")
FLAG_NAMES = ("bank", "clip", "unmove", "unpass")
CLASS_RULE = [
    "bank flag: ground; the bank waypoints value is `speed`",
    "else clip flag: border",
    "else unpass flag: blocking",
    "else: decoration",
]
# The client declares no flag that marks water or lava ground: `liquidpool` (liquid splash
# decals) and `liquidcontainer` (drinkable fluids) occur on no appearance-only palette id, so
# no `liquid` class exists.


class ConvertError(base.ConvertError):
    pass


def terrain_class(flags: frozenset[str]) -> str:
    if "bank" in flags:
        return "ground"
    if "clip" in flags:
        return "border"
    if "unpass" in flags:
        return "blocking"
    return "decoration"


def record_key(appearance_id: int) -> str:
    return f"{KEY_PREFIX}{appearance_id:06d}"


def load_client(root: Path) -> tuple[dict[int, appearances.Appearance], dict]:
    """The decoded appearances and their pin, verified against the client asset manifest."""
    manifest_doc = json.loads((root / regions.MANIFEST).read_text(encoding="utf-8"))
    files = {row["name"]: row["sha256"] for row in manifest_doc["files"]}
    names = sorted(n for n in files if APPEARANCES_NAME.fullmatch(n))
    if len(names) != 1:
        raise ConvertError("the client manifest must list exactly one appearances file")
    name = names[0]
    data = (root / regions.FILES / name).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if files[name] != digest or name != f"appearances-{digest}.dat":
        raise ConvertError(f"{name}: sha256 differs from the client asset manifest")
    source = {
        "client_version": regions.CLIENT_VERSION,
        "evidence": "OfficialClient",
        "files": [{"path": f"{regions.FILES}/{name}", "sha256": digest}],
        "manifest": {
            "archive_sha256": manifest_doc["archive_sha256"],
            "path": regions.MANIFEST,
        },
        "source_key": SOURCE_KEY,
    }
    return appearances.read_appearances(data), source


def terrain_ids(
    palette: list[dict], declared: set[int] | None, known: set[int]
) -> list[int]:
    """The palette ids that get a record (see the module docstring); `known` is the client's."""
    ids = []
    for row in palette:
        server_id, key = row["source_item_id"], row["key"]
        if key.startswith(KEY_PREFIX):
            if key != record_key(server_id) or row["provisional"]:
                raise ConvertError(f"palette id {server_id}: malformed terrain key")
            ids.append(server_id)
        elif declared is not None and not key.startswith(ITEM_KEY_PREFIX):
            if row["provisional"] and server_id not in declared and server_id in known:
                ids.append(server_id)
    return sorted(ids)


def build(
    root: Path = ROOT, declared: set[int] | None = None, workers: int = 1
) -> dict[str, bytes]:
    client, source = load_client(root)
    palette = json.loads((root / PLACEMENTS_INDEX).read_text(encoding="utf-8"))[
        "palette"
    ]
    position = {row["source_item_id"]: i for i, row in enumerate(palette)}
    ids = terrain_ids(palette, declared, set(client))
    missing = [i for i in ids if i not in client]
    if missing:
        raise ConvertError(f"terrain ids without a client appearance: {missing[:5]}")
    if not ids:
        raise ConvertError("no Terrain ids: run with --crystal-root to create them")
    counts = floor_changes.count_tile_items(root, {position[i] for i in ids}, workers)
    digest = source["files"][0]["sha256"]
    records = []
    for appearance_id in ids:
        appearance = client[appearance_id]
        key = record_key(appearance_id)
        declaration: dict = {
            "appearance_id": appearance_id,
            "class": terrain_class(appearance.flags),
            "flags": sorted(appearance.flags),
            "identity": {"key": key, "revision": base.REVISION},
            "kind": "Terrain",
            "occurrences_on_base_map": counts[position[appearance_id]],
        }
        if appearance.name:
            declaration["name"] = appearance.name
        if appearance.automap_color is not None:
            declaration["automap_color"] = appearance.automap_color
        if "bank" in appearance.flags:
            if appearance.speed is None:
                raise ConvertError(f"{key}: bank flag without a speed")
            declaration["speed"] = appearance.speed
        records.append(
            {
                "declaration": declaration,
                "source_bindings": [
                    {
                        "disposition": "EXACT",
                        "external_id": str(appearance_id),
                        "identity_namespace": NAMESPACE,
                        "source_key": SOURCE_KEY,
                        "source_revision": digest,
                        "target": base.ref(FAMILY, key),
                    }
                ],
            }
        )
    records = base.unique(records, FAMILY)
    out = base.shard_files(FAMILY, records, source, GENERATOR)
    out[SUMMARY] = base.canonical(capture_summary(records, client, source))
    return out


def capture_summary(records: list[dict], client: dict, source: dict) -> dict:
    declarations = [r["declaration"] for r in records]
    classes = {
        name: {
            "occurrences_on_base_map": sum(
                d["occurrences_on_base_map"] for d in declarations if d["class"] == name
            ),
            "records": sum(d["class"] == name for d in declarations),
        }
        for name in CLASSES
    }
    flags = Counter(f for d in declarations for f in d["flags"])
    return {
        "appearances": {
            "objects": len(client),
            "path": source["files"][0]["path"],
            "sha256": source["files"][0]["sha256"],
        },
        "class_rule": CLASS_RULE,
        "classes": classes,
        "families": {FAMILY: len(records)},
        "flags": {name: flags[name] for name in FLAG_NAMES},
        "occurrences_on_base_map": sum(
            d["occurrences_on_base_map"] for d in declarations
        ),
        "records_with": {
            "automap_color": sum("automap_color" in d for d in declarations),
            "name": sum("name" in d for d in declarations),
            "speed": sum("speed" in d for d in declarations),
        },
        "schema": "OTERYN_TERRAIN_SOURCE_CAPTURE/v1",
        "source": source,
    }


def read_declared(crystal_root: Path) -> set[int]:
    """Every id the pinned `items.xml` declares, after checking its sha256."""
    row = next(
        f for f in world_base.SOURCE["files"] if f["path"] == world_base.ITEMS_XML
    )
    xml = (crystal_root / row["path"]).read_bytes()
    if hashlib.sha256(xml).hexdigest() != row["sha256"]:
        raise ConvertError(f"{row['path']}: sha256 differs from the pinned source")
    return world_base.items_xml_ids(xml)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="fail instead of writing")
    parser.add_argument(
        "--crystal-root",
        type=Path,
        help="pinned crystalserver checkout: derive the Terrain ids from items.xml",
    )
    parser.add_argument("--workers", type=int, default=min(4, os.cpu_count() or 1))
    args = parser.parse_args()
    try:
        declared = read_declared(args.crystal_root) if args.crystal_root else None
        out = build(declared=declared, workers=args.workers)
    except (
        base.ConvertError,
        appearances.AppearanceError,
        OSError,
        KeyError,
        json.JSONDecodeError,
    ) as error:
        print(f"FAIL {error!r}", file=sys.stderr)
        return 1
    stale = [
        path
        for path, data in out.items()
        if not (ROOT / path).is_file() or (ROOT / path).read_bytes() != data
    ]
    directory = ROOT / DIRECTORY
    extra = sorted(
        str(p.relative_to(ROOT))
        for p in directory.iterdir()
        if str(p.relative_to(ROOT)) not in out
    )
    if args.check:
        for path in stale:
            print(f"STALE {path}", file=sys.stderr)
        for path in extra:
            print(f"EXTRA {path}", file=sys.stderr)
        return 1 if stale or extra else 0
    for path in extra:
        (ROOT / path).unlink()
    for path in stale:
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(out[path])
    print(f"wrote {len(stale)} of {len(out)} files, removed {len(extra)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
