"""Restore Crystal's startup quest-door actions by position, without inventing ids.

Capture: --crystal-root <201189f checkout> --canary-root <47dfd51f checkout>. Apply/check the committed capture with
--apply [--check]. convert_world_base calls overlay_region for from-scratch builds.
Only normalized facts and source line witnesses are retained (OTS_HYPOTHESIS_ONLY).
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
CAPTURE = Path("tools/content-schema/world-authoring/samples/quest-door-bindings.json")
INDEX = Path("content/world/placements/index.json")
REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
TABLE = "data-global/startup/tables/door_quest.lua"
STORAGES = "data-global/lib/core/storages.lua"
DOORS = "data/libs/tables/doors.lua"
SCRIPT = "data/scripts/actions/doors/quest_door.lua"


def canonical(value):
    return (json.dumps(value, separators=(",", ":"), sort_keys=True) + "\n").encode()


def capture(
    source,
    datapack="data-global",
    revision=REVISION,
    repository="zimbadev/crystalserver",
):
    table_path = TABLE.replace("data-global", datapack)
    storages_path = STORAGES.replace("data-global", datapack)
    sys.path.insert(0, str(ROOT / "tools/content-schema/quest-authoring"))
    import lua_tables

    texts = {
        p: (source / p).read_text() for p in [table_path, storages_path, DOORS, SCRIPT]
    }
    storage = lua_tables.assignments(texts[storages_path], {"Storage"})["Storage"]
    fields = lua_tables.assignments(texts[table_path], {"QuestDoorAction"})[
        "QuestDoorAction"
    ]["fields"]
    rows = {}
    for field in fields:
        key = field["key"]
        number, storage_line, track = key, None, f"storage/{key}"
        if isinstance(key, dict):
            node = storage
            parts = re.findall(r"[A-Za-z_][A-Za-z_0-9]*|\d+", key["expr"])[1:]
            for part in parts:
                part = int(part) if part.isdigit() else part
                child = next(f for f in node["fields"] if f["key"] == part)
                node, storage_line = child["value"], child["line"]
            number = node
            track = "/".join(
                re.sub(
                    r"[^a-z0-9]+",
                    "_",
                    re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", p).lower(),
                ).strip("_")
                for p in parts
            )
        if not isinstance(number, int) or not 0 < number <= 65535:
            raise ValueError(f"invalid action {key}: {number}")
        value = lua_tables.as_python(field["value"])
        for group in value if isinstance(value, list) else [value]:
            for position in group["itemPos"]:
                pos = tuple(position[k] for k in "xyz")
                row = {
                    "position": list(pos),
                    "action": number,
                    "track": "oteryn:quest-progress/" + track,
                    "item": group["itemId"] or None,
                    "table_line": field["line"],
                    "storage_line": storage_line,
                    "repository": repository,
                }
                if pos in rows and rows[pos]["action"] != number:
                    raise ValueError(f"conflicting door at {pos}")
                rows[pos] = row
    unique = lua_tables.assignments(texts[table_path], {"QuestDoorUnique"})[
        "QuestDoorUnique"
    ]
    uniques = []
    for field in unique["fields"]:
        value = lua_tables.as_python(field["value"])
        position = value["itemPos"]
        uniques.append(
            {
                "position": [position[k] for k in "xyz"],
                "unique": field["key"],
                "table_line": field["line"],
            }
        )
    door_table = lua_tables.assignments(texts[DOORS], {"QuestDoorTable"})[
        "QuestDoorTable"
    ]
    pairs = [
        {
            "closed": lua_tables.as_python(f["value"])["closedDoor"],
            "open": lua_tables.as_python(f["value"])["openDoor"],
            "line": f["line"],
        }
        for f in door_table["fields"]
    ]
    return {
        "revision": revision,
        "repository": repository,
        "classification": "OTS_HYPOTHESIS_ONLY",
        "predicate": {
            "operator": "not_equal",
            "value": -1,
            "source": SCRIPT,
            "line": 16,
        },
        "sources": [
            {
                "repository": repository,
                "revision": revision,
                "path": p,
                "sha256": hashlib.sha256(texts[p].encode()).hexdigest(),
            }
            for p in texts
        ],
        "unique_bindings": uniques,
        "pairs": pairs,
        "bindings": [rows[k] for k in sorted(rows)],
    }


def capture_sources(crystal, canary):
    document = capture(crystal)
    other = capture(
        canary,
        "data-otservbr-global",
        "47dfd51f45280a59a1d3e50ba7edd573d7234446",
        "opentibiabr/canary",
    )
    decisions = json.loads(
        (
            ROOT / "tools/content-schema/quest-authoring/conflict_decisions.json"
        ).read_bytes()
    )["doors"]
    by_position = {tuple(r["position"]): r for r in document["bindings"]}
    aliases = {}
    # The accepted quest catalogue uses Canary track identities. D25's six
    # Kilmaresh decisions explicitly select Global's per-mask access over Crystal's counter.
    for row in other["bindings"]:
        position = tuple(row["position"])
        if position not in by_position:
            continue
        decision = decisions.get(",".join(map(str, position)))
        if decision:
            assert decision["decision"] == "canary"
            row["decision"] = decision
            previous = by_position[position]
            if (
                decision["category"] == "implementation detail"
                and previous["action"] == row["action"]
            ):
                aliases[previous["track"]] = row["track"]
        by_position[position] = row
    for row in by_position.values():
        if row["track"] in aliases and row["track"] != aliases[row["track"]]:
            row["track_alias"] = row["track"]
            row["track"] = aliases[row["track"]]
    tracks = {}
    for row in by_position.values():
        if tracks.setdefault(row["action"], row["track"]) != row["track"]:
            raise ValueError(f"ambiguous storage action {row['action']}")
    document["bindings"] = [by_position[k] for k in sorted(by_position)]
    document["sources"].extend(other["sources"])
    return document


def overlay_region(raw, palette, document):
    """Preserve every tile/item fact; add only a matching top-level quest action."""
    z, rx, ry, table = codec.parse_region(raw)
    bindings = {
        tuple(r["position"]): r
        for r in document["bindings"]
        if r["position"][2] == z
        and r["position"][0] // 256 == rx
        and r["position"][1] // 256 == ry
    }
    if not bindings:
        return raw, []
    quest_ids = {r["closed"] for r in document["pairs"]}
    sectors = {}
    imported = []
    changed = False
    for local, offset, length in table:
        compressed = raw[offset : offset + length]
        payload = codec.zstandard.ZstdDecompressor().decompress(
            compressed, max_output_size=codec.MAX_SECTOR_BYTES
        )
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        tiles = codec.decode_sector(payload, sx, sy)
        sector_changed = False
        for x, y, flags, house, zones, items in tiles:
            binding = bindings.get((x, y, z))
            if binding is None:
                continue
            for slot, (index, depth, attrs) in enumerate(items):
                item = palette[index]["source_item_id"]
                if depth or item not in quest_ids or house:
                    continue
                if binding["item"] not in (None, item):
                    continue
                if (attrs or {}).get("action", binding["action"]) != binding["action"]:
                    raise ValueError(f"door action conflict at {(x, y, z)}")
                imported.append(
                    {"position": [x, y, z], "item": item, "action": binding["action"]}
                )
                replacement = {**(attrs or {}), "action": binding["action"]}
                if attrs != replacement:
                    items[slot] = (index, depth, replacement)
                    sector_changed = True
        sectors[local] = (
            codec.encode_sector_tiles(sx, sy, tiles) if sector_changed else payload
        )
        changed |= sector_changed
    return (codec.encode_region(z, rx, ry, sectors) if changed else raw), imported


def overlay_outputs(outputs):
    document = json.loads((ROOT / CAPTURE).read_bytes())
    index = json.loads(outputs[INDEX.as_posix()])
    imported = []
    byte_delta = 0
    for region in index["regions"]:
        path = region["path"]
        previous = outputs[path]
        raw, rows = overlay_region(previous, index["palette"], document)
        imported.extend(rows)
        outputs[path] = raw
        byte_delta += len(raw) - len(previous)
        region["sha256"] = hashlib.sha256(raw).hexdigest()
    previous_count = index["source"].get("quest_doors", {}).get("imported", 0)
    index["source"]["quest_doors"] = {
        "path": CAPTURE.as_posix(),
        "sha256": hashlib.sha256((ROOT / CAPTURE).read_bytes()).hexdigest(),
        "imported": len(imported),
    }
    outputs[INDEX.as_posix()] = canonical(index)
    summary_path = (
        "tools/content-schema/world-authoring/samples/world-base-capture-v1.json"
    )
    summary = json.loads(outputs[summary_path])
    summary["source"] = index["source"]
    if len(imported) != previous_count:
        summary["item_attributes"]["action"] = (
            summary["item_attributes"].get("action", 0) + len(imported) - previous_count
        )
    summary["bytes_on_disk"] += byte_delta
    outputs[summary_path] = canonical(summary)
    return imported


def census(check):
    document = json.loads((ROOT / CAPTURE).read_bytes())
    index = json.loads((ROOT / INDEX).read_bytes())
    quest_ids = {r["closed"] for r in document["pairs"]}
    bound = {tuple(r["position"]): r for r in document["bindings"]}
    unique = {tuple(r["position"]): r for r in document["unique_bindings"]}
    imported, unbound = [], []
    for region in index["regions"]:
        z, _, _, sectors = codec.decode_region((ROOT / region["path"]).read_bytes())
        for _, tiles in sectors:
            for x, y, _, house, _, items in tiles:
                for palette, depth, attrs in items:
                    item = index["palette"][palette]["source_item_id"]
                    if depth or item not in quest_ids:
                        continue
                    position = (x, y, z)
                    row = {"position": list(position), "item": item}
                    binding = bound.get(position)
                    if binding and not house and binding["item"] in (None, item):
                        assert (attrs or {}).get("action") == binding["action"], (
                            position
                        )
                        imported.append(row)
                    else:
                        uid = unique.get(position)
                        if uid:
                            row["unique_binding"] = uid
                            row["reason"] = (
                                "dedicated vocation interaction, not a storage door"
                                if 22001 <= uid["unique"] <= 22004
                                else "unique id only; no generic storage-action predicate"
                            )
                        else:
                            row["reason"] = (
                                "no donor QuestDoorAction or QuestDoorUnique at this position"
                            )
                        unbound.append(row)
    result = {
        "placed": len(imported) + len(unbound),
        "imported": len(imported),
        "without_storage_action": len(unbound),
        "unbound": sorted(unbound, key=lambda r: r["position"]),
    }
    path = ROOT / CAPTURE.with_name("quest-door-coverage.json")
    data = (json.dumps(result, indent=2) + "\n").encode()
    if check:
        assert path.read_bytes() == data, "QUEST_DOOR_COVERAGE_DRIFT"
    else:
        path.write_bytes(data)
    print(
        f"quest doors: {result['placed']} placed, {len(imported)} imported, {len(unbound)} without storage action"
    )


def apply(check):
    index = json.loads((ROOT / INDEX).read_bytes())
    paths = [
        INDEX.as_posix(),
        *(r["path"] for r in index["regions"]),
        "tools/content-schema/world-authoring/samples/world-base-capture-v1.json",
    ]
    outputs = {p: (ROOT / p).read_bytes() for p in paths}
    imported = overlay_outputs(outputs)
    for path, data in outputs.items():
        if (ROOT / path).read_bytes() != data:
            if check:
                raise ValueError(f"QUEST_DOOR_BINDING_DRIFT: {path}")
            (ROOT / path).write_bytes(data)
    print(f"quest door actions imported: {len(imported)}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--crystal-root", type=Path)
    parser.add_argument("--canary-root", type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--census", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.crystal_root:
        if args.canary_root is None:
            parser.error("--canary-root is required with --crystal-root")
        data = (
            json.dumps(capture_sources(args.crystal_root, args.canary_root), indent=2)
            + "\n"
        ).encode()
        if args.check:
            assert (ROOT / CAPTURE).read_bytes() == data, "QUEST_DOOR_SOURCE_DRIFT"
        else:
            (ROOT / CAPTURE).write_bytes(data)
    if args.apply:
        apply(args.check)

    if args.census:
        census(args.check)
