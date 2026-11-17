"""Capture selected pinned quest-route startup bindings without rewriting binary maps."""

import argparse
import hashlib
import json
import re
import sys
import tarfile
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = (
    ROOT / "tools/content-schema/world-authoring/samples/quest-key-door-bindings.json"
)
CORPUS = (
    ROOT / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
)
REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SELECTED = ("ID3940", "ID3980")
TABLE = "data-global/startup/tables/door_key.lua"
STORAGES = "data-global/lib/core/storages.lua"
sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
import lua_tables


def document():
    sources = []
    with tarfile.open(CORPUS) as archive:
        manifest = json.load(archive.extractfile("corpus-manifest.json"))

        def source(path):
            (row,) = [
                f
                for f in manifest["files"]
                if f["source"] == "crystalserver"
                and f["revision"] == REVISION
                and f["path"] == path
            ]
            raw = archive.extractfile(row["cache_path"]).read()
            if (
                hashlib.sha256(raw).hexdigest() != row["sha256"]
                or hashlib.sha1(
                    b"blob " + str(len(raw)).encode() + b"\0" + raw
                ).hexdigest()
                != row["git_blob_sha1"]
            ):
                raise ValueError("key door source digest mismatch")
            sources.append(
                {
                    k: row[k]
                    for k in (
                        "repository",
                        "revision",
                        "path",
                        "sha256",
                        "git_blob_sha1",
                    )
                }
            )
            return raw.decode()

        routes = []
        for path, group, action, selected in [
            (
                "data-global/startup/tables/teleport_item.lua",
                "TeleportItemAction",
                15005,
                {(32171, 32439, 7): 1930, (32172, 32439, 7): 1931},
            ),
            (
                "data-global/startup/tables/door_level.lua",
                "LevelDoorAction",
                1050,
                {(32212, 32435, 10): 1648},
            ),
        ]:
            fields = lua_tables.assignments(source(path), {group})[group]["fields"]
            (field,) = [f for f in fields if f["key"] == action]
            values = lua_tables.as_python(field["value"])
            values = values if isinstance(values, list) else [values]
            found = set()
            for value in values:
                positions = value["itemPos"]
                for position in (
                    positions if isinstance(positions, list) else [positions]
                ):
                    xyz = tuple(position[k] for k in "xyz")
                    if xyz in selected:
                        routes.append(
                            {
                                "position": list(xyz),
                                "item": selected[xyz],
                                "attribute": "action",
                                "value": action,
                                "source_path": path,
                                "source_line": field["line"],
                            }
                        )
                        found.add(xyz)
            if found != set(selected):
                raise ValueError("quest route source selector drift")
        table = lua_tables.assignments(source(TABLE), {"KeyDoorAction"})[
            "KeyDoorAction"
        ]
        storage = lua_tables.assignments(source(STORAGES), {"Storage"})["Storage"]
    for key in ("Quest", "Key"):
        storage = next(f["value"] for f in storage["fields"] if f["key"] == key)
    numbers = {f["key"]: (f["value"], f["line"]) for f in storage["fields"]}
    index = json.loads((ROOT / "content/world/placements/index.json").read_bytes())
    registry = (ROOT / "crates/gameplay/world/src/map/door_registry.rs").read_text()
    triples = registry.split("const KEY_DOORS:", 1)[1].split("];", 1)[0]
    locked_ids = {int(a) for a, _, _ in re.findall(r"\((\d+), (\d+), (\d+)\)", triples)}
    bindings = []
    for field in table["fields"]:
        expression = (
            field["key"].get("expr") if isinstance(field["key"], dict) else None
        )
        if expression not in {"Storage.Quest.Key." + key for key in SELECTED}:
            continue
        number, storage_line = numbers[expression.rsplit(".", 1)[1]]
        group = lua_tables.as_python(field["value"])
        for position in group["itemPos"]:
            x, y, z = (position[k] for k in "xyz")
            (region,) = [
                r
                for r in index["regions"]
                if r["path"].endswith(codec.region_name(z, x // 256, y // 256))
            ]
            raw = (ROOT / region["path"]).read_bytes()
            if hashlib.sha256(raw).hexdigest() != region["sha256"]:
                raise ValueError("key door map digest mismatch")
            (tile,) = [
                tile
                for _, tiles in codec.decode_region(raw)[3]
                for tile in tiles
                if tile[:2] == (x, y)
            ]
            if tile[3] != 0:
                raise ValueError("key door is a House tile")
            (item,) = [
                index["palette"][i]["source_item_id"]
                for i, depth, _ in tile[-1]
                if depth == 0 and index["palette"][i]["source_item_id"] in locked_ids
            ]
            if group["itemId"] not in (False, item):
                raise ValueError("key door source item differs")
            bindings.append(
                {
                    "position": [x, y, z],
                    "item": item,
                    "attribute": "action",
                    "value": number,
                    "source_line": field["line"],
                    "storage_line": storage_line,
                    "map_path": region["path"],
                    "map_sha256": region["sha256"],
                }
            )
    if len(bindings) != len(SELECTED):
        raise ValueError("key door capture incomplete")
    for binding in routes:
        x, y, z = binding["position"]
        (region,) = [
            r
            for r in index["regions"]
            if r["path"].endswith(codec.region_name(z, x // 256, y // 256))
        ]
        raw = (ROOT / region["path"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != region["sha256"]:
            raise ValueError("quest route map digest mismatch")
        (tile,) = [
            tile
            for _, tiles in codec.decode_region(raw)[3]
            for tile in tiles
            if tile[:2] == (x, y)
        ]
        matches = [
            attrs
            for palette, depth, attrs in tile[-1]
            if depth == 0
            and index["palette"][palette]["source_item_id"] == binding["item"]
        ]
        if tile[3] != 0 or len(matches) != 1:
            raise ValueError("quest route native item or House drift")
        binding.update(map_path=region["path"], map_sha256=region["sha256"])
        bindings.append(binding)
    return {
        "schema": 1,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "sources": sources,
        "bindings": sorted(bindings, key=lambda row: row["position"]),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = json.dumps(document(), indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if OUTPUT.read_text() != raw:
            raise SystemExit("KEY_DOOR_BINDING_CAPTURE_DRIFT")
    else:
        OUTPUT.write_text(raw)
