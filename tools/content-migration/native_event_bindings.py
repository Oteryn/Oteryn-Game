#!/usr/bin/env python3
"""Restore selected donor startup selectors without rewriting native map regions."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
from lua_tables import assignments, as_python

REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SOURCE = "data-global/startup/tables/item.lua"
SHA = "677bffd2482d16a5c73633e3f7324c4dc16cf1e00fbb5368e56ce4b6d60402fd"


def generate():
    with tarfile.open(ROOT / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz") as archive:
        manifest = json.load(archive.extractfile("corpus-manifest.json"))
        rows = [row for row in manifest["files"] if row["source"] == "crystalserver"
                and row["revision"] == REVISION and row["path"] == SOURCE]
        if len(rows) != 1:
            raise ValueError("WEVENTS_STARTUP_IDENTITY")
        raw = archive.extractfile(rows[0]["cache_path"]).read()
    if hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError("WEVENTS_STARTUP_SHA256")
    fields = assignments(raw.decode(), {"ItemAction"})["ItemAction"]["fields"]
    bindings = []
    for field in fields:
        if field["key"] not in {50000, 50006, 50007, 50008, 50009, 50010}:
            continue
        value = as_python(field["value"])
        for position in value["itemPos"]:
            bindings.append({"position": [position[axis] for axis in ("x", "y", "z")],
                             "item": value["itemId"] or None, "action": field["key"],
                             "unique": None, "source": {"repository": rows[0]["repository"],
                             "revision": REVISION, "path": SOURCE, "line": field["line"],
                             "sha256": SHA}})
    bindings.sort(key=lambda row: (row["action"], row["position"]))
    if len(bindings) != 16:
        raise ValueError("WEVENTS_STARTUP_COUNT")
    return (json.dumps({"schema": 1, "bindings": bindings}, indent=2) + "\n").encode()


def verify_native_objects(document):
    sys.path.insert(0, str(ROOT / "tools/content-schema/world-authoring"))
    import world_region_codec as codec
    directory = ROOT / "content/world/placements"
    palette = json.loads((directory / "index.json").read_bytes())["palette"]
    cache = {}
    for binding in document["bindings"]:
        x, y, z = binding["position"]
        name = f"region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3"
        if name not in cache:
            raw = (directory / name).read_bytes()
            _, rx, ry, table = codec.parse_region(raw)
            tiles = {}
            for local, offset, length in table:
                payload = codec.zstandard.ZstdDecompressor().decompress(
                    raw[offset:offset + length], max_output_size=codec.MAX_SECTOR_BYTES)
                for tile in codec.decode_sector(payload, rx * 8 + local % 8, ry * 8 + local // 8):
                    tiles[tile[:2]] = tile
            cache[name] = tiles
        tile = cache[name].get((x, y))
        if tile is None:
            raise ValueError(f"WEVENTS_NATIVE_TILE {binding['position']}")
        items = [palette[index]["source_item_id"] for index, depth, _ in tile[5] if depth == 0]
        if not items or (binding["item"] is not None and binding["item"] not in items):
            raise ValueError(f"WEVENTS_NATIVE_ITEM {binding['position']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--map-check", action="store_true")
    args = parser.parse_args()
    target = ROOT / "content/quests/native-object-bindings.json"
    expected = generate()
    if args.map_check:
        verify_native_objects(json.loads(expected))
    if args.check:
        if target.read_bytes() != expected:
            raise SystemExit("WEVENTS_STARTUP_STALE")
    else:
        target.write_bytes(expected)
    print("WEVENTS startup selectors: 16 verified source bindings")
