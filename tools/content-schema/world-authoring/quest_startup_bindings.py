"""Restore selected pinned startup selectors omitted by world.otbm.

Capture with --crystal-root, then --apply. The World converter applies the
same lossless overlay when rebuilding from source. No Lua is executed.
"""

import argparse
import hashlib
import json
import sys
from collections import Counter
from pathlib import Path

import quest_door_bindings as doors

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = Path(
    "tools/content-schema/world-authoring/samples/quest-startup-bindings.json"
)
SUMMARY = "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
SPECS = (
    ("tile.lua", "TileAction", 50022, "action"),
    ("teleport.lua", "TeleportAction", 50022, "action"),
    # Adventurers' Guild exits (adventurers_guild.lua).
    ("teleport.lua", "TeleportAction", 4253, "action"),
    ("door_level.lua", "LevelDoorAction", 1070, "action"),
    ("lever.lua", "LeverUnique", 30008, "unique"),
    ("lever.lua", "LeverUnique", 30034, "unique"),
    ("lever.lua", "LeverAction", 5633, "action"),
    ("item.lua", "ItemAction", 4503, "action"),
    ("door_key.lua", "KeyDoorAction", "Storage.Quest.Key.ID4502", "action"),
    # World change returns: the Fury Dungeon archway (fury_gates.lua) and the
    # Nightmare Isle exit (nightmare_isles.lua uid 35020).
    ("item.lua", "ItemAction", 9715, "action"),
    ("teleport.lua", "TeleportUnique", 35020, "unique"),
)

ADDITIONAL_ACTIONS = (
    ("door_key.lua", "KeyDoorAction", "Storage.Quest.Key.ID3142", "action"),
    ("tile_pick.lua", "TilePickAction", 101, "action"),
    *(
        ("item.lua", "ItemAction", aid, "action")
        for aid in (*range(64103, 64110), *range(64121, 64129))
    ),
    *(("teleport.lua", "TeleportAction", aid, "action") for aid in (64120, 64121)),
)

TIBIA_TALES_SPECS = (
    *(("item.lua", "ItemAction", action, "action") for action in range(4207, 4224)),
    ("item.lua", "ItemAction", 25018, "action"),
    ("chest.lua", "ChestUnique", 6257, "unique"),
)


def capture(source, specs=(*SPECS, *ADDITIONAL_ACTIONS), *, include_tibia_tales=False):
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    rows, sources = [], []
    if include_tibia_tales:
        specs = (*specs, *TIBIA_TALES_SPECS)
    for filename, table, selector, attribute in specs:
        path = "data-global/startup/tables/" + filename
        raw = (source / path).read_bytes()
        source_row = {"path": path, "sha256": hashlib.sha256(raw).hexdigest()}
        if source_row not in sources:
            sources.append(source_row)
        fields = lua_tables.assignments(raw.decode(), {table})[table]["fields"]
        (field,) = [
            f
            for f in fields
            if (f["key"].get("expr") if isinstance(f["key"], dict) else f["key"])
            == selector
        ]
        value = lua_tables.as_python(field["value"])
        for group in value if isinstance(value, list) else [value]:
            if (
                table == "ItemAction"
                and selector == 25018
                and group["itemId"] is not False
            ):
                # The same action also names an unrelated surface object.
                continue
            if selector == 4503 and group["itemId"] != 7042:
                # The Folda chest USE mints this key. The map restores its lock only.
                continue
            positions = group["itemPos"]
            for position in positions if isinstance(positions, list) else [positions]:
                cell = [position[k] for k in "xyz"]
                if table == "TilePickAction" and cell != [32774, 32289, 10]:
                    continue
                if selector == "Storage.Quest.Key.ID3142":
                    if cell != [32450, 32044, 8] or group["itemId"] is not False:
                        raise ValueError("Key 3142 source selector changed")
                    storage_path = "data-global/lib/core/storages.lua"
                    storage = (source / storage_path).read_bytes()
                    storage_row = {
                        "path": storage_path,
                        "sha256": hashlib.sha256(storage).hexdigest(),
                    }
                    if storage_row not in sources:
                        sources.append(storage_row)
                    node = lua_tables.assignments(storage.decode(), {"Storage"})[
                        "Storage"
                    ]
                    for key in ("Quest", "Key", "ID3142"):
                        node = next(
                            f["value"] for f in node["fields"] if f["key"] == key
                        )
                    if node != 3142:
                        raise ValueError("Key 3142 source storage changed")
                # False selectors target the existing source ground or door.
                item = (
                    group["itemId"]
                    or {
                        ("ItemAction", 25018): 104,
                        ("KeyDoorAction", "Storage.Quest.Key.ID3142"): 6248,
                        ("KeyDoorAction", "Storage.Quest.Key.ID4502"): 7033,
                        ("LevelDoorAction", 1070): 5102,
                        ("TilePickAction", 101): 355,
                    }[(table, selector)]
                )
                rows.append(
                    {
                        "position": cell,
                        "item": item,
                        "attribute": attribute,
                        "action": int(selector.rsplit("ID", 1)[1])
                        if table == "KeyDoorAction"
                        else selector,
                        "source": path,
                        "line": field["line"],
                    }
                )
    path = "data-global/lib/core/storages.lua"
    raw = (source / path).read_bytes()
    storage = lua_tables.as_python(
        lua_tables.assignments(raw.decode(), {"Storage"})["Storage"]
    )
    if storage["Quest"]["Key"]["ID4502"] != 4502:
        raise ValueError("Alawar key storage changed")
    storage_row = {"path": path, "sha256": hashlib.sha256(raw).hexdigest()}
    if storage_row not in sources:
        sources.append(storage_row)
    path = "data/libs/tables/doors.lua"
    raw = (source / path).read_bytes()
    sources.append({"path": path, "sha256": hashlib.sha256(raw).hexdigest()})
    pairs = lua_tables.assignments(raw.decode(), {"LevelDoorTable"})["LevelDoorTable"][
        "fields"
    ]
    if not any(
        lua_tables.as_python(f["value"]) == {"closedDoor": 5102, "openDoor": 5103}
        for f in pairs
    ):
        raise ValueError("expertise door source changed")
    return {
        "schema": 1,
        "repository": "zimbadev/crystalserver",
        "revision": doors.REVISION,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "sources": sources,
        "bindings": sorted(rows, key=lambda row: row["position"]),
    }


def affected(index, document):
    positions = {
        (z, x // 256, y // 256)
        for x, y, z in (row["position"] for row in document["bindings"])
    }
    return [
        row
        for row in index["regions"]
        if tuple(
            map(
                int,
                Path(row["path"])
                .stem.replace("region-z", "")
                .replace("-x", " ")
                .replace("-y", " ")
                .split(),
            )
        )
        in positions
    ]


def overlay_outputs(outputs, *, require_all=False, supplied_by=None):
    document = json.loads((ROOT / CAPTURE).read_bytes())
    index = json.loads(outputs[doors.INDEX.as_posix()])
    if supplied_by is None:
        owner = index["source"].get("quest_access")
        supplied_by = (
            json.loads((ROOT / owner["path"]).read_bytes())["bindings"] if owner else ()
        )
    supplied = {
        (tuple(row["position"]), row["item"], row["attribute"], row["value"])
        for row in supplied_by
    }
    owned = [
        row
        for row in document["bindings"]
        if (tuple(row["position"]), row["item"], row["attribute"], row["action"])
        not in supplied
    ]
    imported = []
    byte_delta = 0
    for region in affected(index, document):
        path = region["path"]
        original = raw = outputs[path]
        for attribute in ("action", "unique"):
            bindings = [
                row for row in document["bindings"] if row["attribute"] == attribute
            ]
            raw, rows = doors.overlay_region(
                raw,
                index["palette"],
                {"bindings": bindings},
                attribute=attribute,
                item_ids={r["item"] for r in bindings},
            )
            imported.extend(rows)
        outputs[path] = raw
        region["sha256"] = hashlib.sha256(raw).hexdigest()
        byte_delta += len(raw) - len(original)
    expected = Counter(
        (tuple(r["position"]), r["item"], r["attribute"], r["action"])
        for r in document["bindings"]
    )
    actual = Counter(
        (tuple(r["position"]), r["item"], attr, r[attr])
        for r in imported
        for attr in ("action", "unique")
        if attr in r
    )
    if any(count > expected[key] for key, count in actual.items()) or (
        require_all and actual != expected
    ):
        raise ValueError("missing or ambiguous startup placement")
    previous = index["source"].get("quest_startup", {}).get("attributes", {})
    counts = dict(Counter(r["attribute"] for r in owned))
    index["source"]["quest_startup"] = {
        "path": CAPTURE.as_posix(),
        "sha256": hashlib.sha256((ROOT / CAPTURE).read_bytes()).hexdigest(),
        "attributes": counts,
    }
    outputs[doors.INDEX.as_posix()] = doors.canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary["source"] = index["source"]
    for attribute, count in counts.items():
        summary["item_attributes"][attribute] = (
            summary["item_attributes"].get(attribute, 0)
            + count
            - previous.get(attribute, 0)
        )
    summary["bytes_on_disk"] += byte_delta
    outputs[SUMMARY] = doors.canonical(summary)
    return imported


def apply(check):
    index = json.loads((ROOT / doors.INDEX).read_bytes())
    document = json.loads((ROOT / CAPTURE).read_bytes())
    paths = [
        doors.INDEX.as_posix(),
        SUMMARY,
        *(r["path"] for r in affected(index, document)),
    ]
    outputs = {p: (ROOT / p).read_bytes() for p in paths}
    access = json.loads(
        (
            ROOT
            / "tools/content-schema/world-authoring/samples/quest-access-bindings.json"
        ).read_bytes()
    )
    rows = overlay_outputs(outputs, require_all=True, supplied_by=access["bindings"])
    summary = json.loads(outputs[SUMMARY])
    summary["bytes_on_disk"] = sum(
        len(outputs[region["path"]])
        if region["path"] in outputs
        else (ROOT / region["path"]).stat().st_size
        for region in index["regions"]
    )
    outputs[SUMMARY] = doors.canonical(summary)
    changed = [p for p, data in outputs.items() if (ROOT / p).read_bytes() != data]
    if check and changed:
        raise ValueError("stale startup bindings: " + ", ".join(changed))
    if not check:
        for path in changed:
            (ROOT / path).write_bytes(outputs[path])
    print(f"quest startup: {len(rows)} exact selectors; {len(changed)} files changed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.crystal_root:
        data = (
            json.dumps(capture(args.crystal_root, include_tibia_tales=True), indent=2)
            + "\n"
        ).encode()
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != data:
                raise ValueError("startup source capture changed")
        else:
            (ROOT / CAPTURE).write_bytes(data)
    if args.apply:
        apply(args.check)


if __name__ == "__main__":
    main()
