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
    ("door_level.lua", "LevelDoorAction", 1070, "action"),
    ("lever.lua", "LeverUnique", 30034, "unique"),
)


def capture(source):
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    rows, sources = [], []
    for filename, table, selector, attribute in SPECS:
        path = "data-global/startup/tables/" + filename
        raw = (source / path).read_bytes()
        sources.append({"path": path, "sha256": hashlib.sha256(raw).hexdigest()})
        fields = lua_tables.assignments(raw.decode(), {table})[table]["fields"]
        (field,) = [f for f in fields if f["key"] == selector]
        value = lua_tables.as_python(field["value"])
        for group in value if isinstance(value, list) else [value]:
            positions = group["itemPos"]
            for position in positions if isinstance(positions, list) else [positions]:
                # LevelDoorAction's false means the door at this exact position.
                # Both native placements are 5102, a closed expertise door:
                # Crystal data/libs/tables/doors.lua:273 (5102 -> 5103).
                item = group["itemId"] or 5102
                rows.append(
                    {
                        "position": [position[k] for k in "xyz"],
                        "item": item,
                        "attribute": attribute,
                        "action": selector,
                        "source": path,
                        "line": field["line"],
                    }
                )
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
        data = (json.dumps(capture(args.crystal_root), indent=2) + "\n").encode()
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != data:
                raise ValueError("startup source capture changed")
        else:
            (ROOT / CAPTURE).write_bytes(data)
    if args.apply:
        apply(args.check)


if __name__ == "__main__":
    main()
