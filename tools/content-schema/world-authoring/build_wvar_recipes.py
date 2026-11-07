"""Author WVAR recipes from the pinned Crystal map and placement sources.

The small native recipe output is derived content, not a donor-map distribution.
Inferred rules are written into each definition's assumption field. Run with
--donor pointing at Crystal 201189f and --check to verify an authored output.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import struct
from pathlib import Path
import xml.etree.ElementTree as ET

from otbm_reader import read_tiles

REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
MAP_SHA = "c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85"
SPAWN_SHA = "f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e"
PREFIX = f"https://github.com/zimbadev/crystalserver/blob/{REVISION}/"
WIKI = {
    "awash": (54770, 1149364),
    "down_the_drain": (76219, 887142),
    "river_runs_deep": (76260, 887153),
    "jungle_camp": (76243, 887148),
    "warpath": (76270, 944151),
    "chakoya_iceberg": (76213, 887139),
    "spider_nest": (76262, 887154),
    "spirit_grounds": (76264, 887160),
    "lumberjack": (76247, 893906),
    "bored": (76211, 1135336),
    "hive_outpost": (76241, 887146),
    "poacher_caves": (76258, 1175805),
    "thornfire": (54126, 1020842),
}
CALLBACK_SOURCES = {
    "river_runs_deep": (
        "data-global/scripts/actions/other/fishing.lua:148-175,203-209 sha256:"
        "75bde535d8b9530fd57b4dde5f741e19264f6ed118789d4641321933a628e02f; "
        "data/items/items.xml:384 sand231 sha256:"
        "8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509"
    ),
    "bored": (
        "data/scripts/creaturescripts/monster/giant_spider_wyda_death.lua:1-12 sha256:"
        "4b78cf145d150264e8630ec73381e66b0fa4e1dca855292b93505805acacf0c2"
    ),
    "thornfire": (
        "data/scripts/actions/tools/prepared_bucket.lua:3-83 sha256:"
        "5e0d0253f913cb788a7fcbd6be41ad465b56ab1026c68226f80f79c31cefe01f; "
        "data/scripts/movements/thornfire_wolf.lua:3-18 sha256:"
        "010270cc7a3ad1ecdd960b645d1013bc61f24feace4df38092295b91aca2b7cf"
    ),
}


def checked(path: Path, sha: str) -> bytes:
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != sha:
        raise ValueError(f"{path}: does not match pinned Crystal {REVISION}")
    return raw


def creature(name: str) -> str:
    return "oteryn:creature." + name.lower().replace(" ", "_").replace("-", "_")


def spawn(name: str, position, respawn_ms=90_000, after_kills=None) -> dict:
    row = {"position": list(position), "creature": creature(name), "respawn_ms": respawn_ms, "multiplicity": "CHANNEL_LOCAL_REPEATABLE"}
    if after_kills is not None:
        row["after_kills"] = after_kills
    return row


def variant(key, *, weight=1, tiles=(), spawns=(), npcs=(), portals=()) -> dict:
    return {"key": key, "weight": weight, "layers": [], "tiles": list(tiles),
            "tags": [], "spawns": list(spawns), "npcs": list(npcs), "portals": list(portals)}


def portal(position, destination, appearance=1949, *, exit=False) -> dict:
    return {"exit": exit, "position": list(position), "destination": list(destination),
            "premium": False, "promoted": False, "min_level": 0,
            "appearance": appearance, "action": None, "unique": None}


def source(key: str, placements: list[dict]) -> str:
    lines = sorted({p["line"] for p in placements if "line" in p})
    witness = ",".join(map(str, lines)) or "tile-coordinate witnesses in authored recipe"
    page, revision = WIKI[key]
    wiki = (f"; https://tibia.fandom.com/index.php?curid={page}&oldid={revision}"
            if page else "")
    callback = "; Crystal@" + REVISION + ":" + CALLBACK_SOURCES[key] if key in CALLBACK_SOURCES else ""
    return (f"{PREFIX}data-global/world/world.otbm sha256:{MAP_SHA}; "
            f"{PREFIX}data-global/world/world-monster.xml lines:{witness} sha256:{SPAWN_SHA}{wiki}{callback}")


def placements(raw: bytes) -> list[dict]:
    """Retain source line witnesses as well as exact absolute source positions."""
    rows = []
    center = None
    for line, text in enumerate(raw.decode().splitlines(), 1):
        if "centerx=" in text:
            a = ET.fromstring(text.strip().replace(">", "/>"))
            center = (int(a.attrib["centerx"]), int(a.attrib["centery"]))
        elif "<monster name=" in text and center is not None:
            a = ET.fromstring(text.strip())
            rows.append({"position": [center[0] + int(a.attrib["x"]),
                                      center[1] + int(a.attrib["y"]), int(a.attrib["z"])],
                         "name": a.attrib["name"], "respawn_ms": int(a.attrib["spawntime"]) * 1000,
                         "line": line})
    return rows


def region_nodes(data: bytes, base: tuple[int, int, int]) -> bytes:
    """Extract bounded area nodes from an already SHA-verified map.

    Crystal repeats a 256-square area header for its 16-square chunks. Every
    matching node is captured; stopping at its first header would lose tiles.
    Escaped delimiters never affect depth. The ordinary OTBM decoder interprets
    the captured nodes; this helper only avoids decoding unrelated map regions.
    """
    props = struct.pack("<HHB", *base)
    escaped = b"".join(b"\xfd" + bytes([v]) if v >= 253 else bytes([v]) for v in props)
    prefix = b"\xfe\x04" + escaped
    nodes = []
    for found in re.finditer(re.escape(prefix), data):
        start = found.start()
        if start > 0 and data[start - 1] == 253:
            continue
        depth = 0
        for token in re.finditer(rb"\xfd[\s\S]|\xfe[\s\S]|\xff", memoryview(data)[start:]):
            if token[0][0] == 254:
                depth += 1
            elif token[0][0] == 255:
                depth -= 1
                if depth == 0:
                    nodes.append(data[start:start + token.end()])
                    break
        else:
            raise ValueError(f"unbalanced pinned map region {base}")
    if not nodes:
        raise ValueError(f"missing pinned map region {base}")
    return (b"\0" * 4 + b"\xfe\0" + struct.pack("<IHH", 2, 1, 1)
            + b"\xfe\x02" + b"".join(nodes) + b"\xff\xff")


def author(donor: Path) -> list[dict]:
    rows = placements(checked(donor / "data-global/world/world-monster.xml", SPAWN_SHA))
    packed = checked(donor / "data-global/world/world.otbm", MAP_SHA)
    data = gzip.decompress(packed)

    def select(names, box, limit=None):
        x1, y1, x2, y2, z1, z2 = box
        result = [r for r in rows if r["name"] in names and
                  x1 <= r["position"][0] <= x2 and y1 <= r["position"][1] <= y2 and
                  z1 <= r["position"][2] <= z2]
        return result[:limit]

    def native(rs):
        return [spawn(r["name"], r["position"], r["respawn_ms"]) for r in rs]

    scouts = select({"Deepling Scout"}, (32500, 32000, 32600, 32070, 10, 10), 12)
    water = select({"Water Elemental"}, (32570, 32300, 32600, 32330, 8, 8))
    hunter = select({"Hunter", "Poacher"}, (32900, 32700, 33000, 32800, 7, 7), 4)
    chakoya = select({"Chakoya Toolshaper", "Chakoya Tribewarden", "Chakoya Windcaller"},
                     (32600, 32540, 32650, 32595, 5, 7))
    wolves = select({"Gloom Wolf", "Ghost Wolf"}, (32700, 31850, 32810, 31980, 8, 10), 12)
    fire = select({"Firestarter"}, (33050, 32130, 33150, 32210, 1, 8), 24)
    vandura = select({"Tortoise", "Crocodile", "Coral Frog", "Azure Frog", "Tarantula"},
                     (32370, 32800, 32440, 32910, 7, 7), 6)
    if [len(g) for g in [scouts, water, hunter, chakoya, wolves, fire, vandura]] != [12, 7, 2, 63, 12, 24, 6]:
        raise ValueError("pinned placement group count differs from authored recipe")

    guards = [spawn(n, p) for n, p in zip(
        ["Orc Warlord", "Orc Warlord", "Orc Warlord", "Orc Berserker", "Orc Berserker", "Orc Leader"],
        [[32620, 31825, 7], [32622, 31830, 7], [32620, 31830, 7],
         [32625, 31830, 7], [32625, 31828, 7], [32628, 31831, 7]], strict=True)]
    drain_spawns = native(water) + [spawn("Water Elemental", p) for p in
        [[32582, 32301, 8], [32590, 32303, 8], [32599, 32303, 8], [32598, 32314, 8]]]
    spirit_entries = [[33142, 32545, 1], [32910, 31541, 7], [32179, 31778, 7]]
    spirit_destination = [32857, 32425, 10]
    spirit_return = [32857, 32426, 10]
    spirit_pads = [[32855, 32425, 10], [32859, 32425, 10], [32857, 32423, 10], [32857, 32428, 10]]
    special = [[32516, 32018, 9], [32516, 32018, 10],
               [32588, 32303, 7], [32588, 32303, 8], [32589, 32303, 8],
               [32817, 32280, 7], [32817, 32280, 8], [32818, 32280, 8],
               [32418, 31699, 7], [32767, 31974, 7], [32622, 31826, 7],
               *spirit_entries, spirit_destination, spirit_return, *spirit_pads]
    requested = {tuple(p) for p in special}
    requested.update(tuple(r["position"]) for rs in [scouts, water, hunter, chakoya, wolves, fire, vandura] for r in rs)
    requested.update(tuple(r["position"]) for r in guards + drain_spawns)
    bases = {(x // 256 * 256, y // 256 * 256, z) for x, y, z in requested}
    bases.update({(33280, 31488, 11), (32256, 31488, 7), (33024, 32000, 7)})
    bases.update((33024, 32000, z) for z in range(1, 9))
    tiles = {}
    fish, stumps, bog, flames, iceberg_tiles = [], [], [], [], []
    flame_ids = {232, 5061, 5062, 5063, 5064, 5065, 5066, 5067,
                 6288, 6289, 7803, 19096, 32021}

    def capture(x, y, z, flags, house, zones, items):
        if house is not None or zones or any(depth or attrs.get(15, 0) != 0 or set(attrs) - {4, 5, 15, "dest"}
                                               for _, depth, attrs in items):
            return
        row = {"position": [x, y, z], "flags": flags,
               "entries": [[i, a.get(4), a.get(5)] for i, _, a in items]}
        if not row["entries"]:
            return
        if (x, y, z) in requested:
            tiles[(x, y, z)] = row
        if 33400 <= x < 33500 and 31500 <= y < 31600 and z == 11 and any(i == 13989 for i, _, _ in items):
            fish.append(row)
        if 32410 <= x <= 32430 and 31690 <= y <= 31710 and z == 7 and any(i in {7957, 7958, 7960} for i, _, _ in items):
            stumps.append(row)
        if 33137 <= x <= 33144 and 32146 <= y <= 32153 and z == 7 and any(i == 12721 for i, _, _ in items):
            bog.append(row)
        if 33050 <= x <= 33150 and 32130 <= y <= 32210 and any(i in flame_ids for i, _, _ in items):
            flames.append(row)
        if 32600 <= x <= 32650 and 32540 <= y <= 32585 and 5 <= z <= 7 and (
                row["entries"][0][0] == 799 or 6580 <= row["entries"][0][0] <= 7157):
            iceberg_tiles.append(row)

    for base in sorted(bases):
        read_tiles(region_nodes(data, base), capture)
    if (len(fish), len(stumps), len(bog)) != (102, 8, 56):
        raise ValueError("pinned fishing/stump/bog layer membership changed")

    def at(position):
        try:
            return tiles[tuple(position)]
        except KeyError:
            raise ValueError(f"authored anchor lacks supported non-House source terrain: {position}") from None

    def footprint(rs):
        return [at(r["position"]) for r in rs]

    def ground(position, appearance, decorations=()):
        row = at(position)
        return {**row, "entries": [[appearance, None, None], *decorations]}

    def swapped(row, before, after):
        return {**row, "entries": [[after if i == before else i, a, u] for i, a, u in row["entries"]]}

    definitions = []

    def add(slot, key, chance, variants, assumed, witnesses=()):
        if any(v["portals"] for v in variants):
            assumed += " Explicit return remains usable after closure; if a shared return has no current selected entrance, the first authored safe surface return is used."
        definitions.append({"slot": slot, "key": key, "chance_permille": chance,
                            "source": source(key, list(witnesses)), "assumption": assumed,
                            "variants": variants})

    add(3, "awash", 1000, [
        variant("drained", spawns=native(scouts), tiles=footprint(scouts),
                portals=[portal([32516, 32018, 9], [32516, 32018, 10]),
                         portal([32516, 32018, 10], [32516, 32018, 9], exit=True)]),
        variant("flooded", tiles=[ground([32516, 32018, 9], 622)]),
    ], "Source gives202coal/3pumps/next-save transition but no native binding. Equal daily drained/flooded selection and adjacent-floor entry are conservative owner-authorized assumptions; Scout membership uses the first12 exact source dungeon placements.", scouts)

    add(4, "down_the_drain", 307, [variant("flooded", spawns=drain_spawns,
        tiles=footprint(drain_spawns), portals=[portal([32588, 32303, 7], [32588, 32303, 8]),
                                              portal([32589, 32303, 8], [32588, 32303, 7], exit=True)])],
        "W15 fixes307permille; wiki says11WaterElementals. Seven positions are sourced; four nearby offsets and surface/return selectors are conservative assumptions. The source cave terrain is preserved.", water)

    add(5, "river_runs_deep", 269, [
        variant("flowing", tiles=fish),
        variant("dry", tiles=[swapped(t, 13989, 231) for t in fish]),
    ], "W15 fixes269permille. Exact source desert-fishing objects13989 in the334xx/315xx floor11 river are the bounded membership. Equal flowing/dry alternatives are conservative; dry replaces fishing water with source items.xml's sand231, removing fishing-object membership and changing ground topology.")

    dworcs = [spawn(["Dworc Fleshhunter", "Dworc Venomsniper", "Dworc Voodoomaster"][i % 3], r["position"], r["respawn_ms"]) for i, r in enumerate(hunter)]
    rare_hunters = native(hunter)
    rare_hunters[0] = spawn("Arthom the Hunter", hunter[0]["position"], hunter[0]["respawn_ms"])
    add(6, "jungle_camp", 500, [
        variant("hunter", weight=49, spawns=native(hunter), tiles=footprint(hunter)),
        variant("hunter_arthom", weight=1, spawns=rare_hunters, tiles=footprint(hunter)),
        variant("dworc", weight=50, spawns=dworcs, tiles=footprint(hunter)),
    ], "Wiki identifies Hunter/Dworc alternatives and conditional Arthom.500permille,49:1:50 weights and reusing the exact Hunter camp's pads for Dworc membership are conservative assumptions; rare boss replaces one guard rather than overlaps it.", hunter)

    add(7, "warpath", 373, [variant("east_femor_hills", tiles=footprint(guards),
        spawns=guards + [spawn("Bibby Bloodbath", [32622, 31826, 7], 1768000, after_kills=6)])],
        "W15 fixes373permille. The retained Femor camp and Bibby anchor are used; other possible camp locations are not invented. Wiki inside roster2Berserkers/3Warlords/oneLeader drives six distinct guard sources. Two nearby guard pads are inferred. Bibby requires all six guard identities, not six farmed kills.")

    iceberg = list({tuple(t["position"]): t for t in iceberg_tiles + footprint(chakoya)}.values())
    open_water = [{**t, "entries": [[622, None, None]] if t["position"][2] == 7 else []} for t in iceberg]
    add(8, "chakoya_iceberg", 1000, [
        variant("iceberg", tiles=iceberg, spawns=native(chakoya)),
        variant("open_water", tiles=open_water),
    ], "The exact southern326xx/325xx source Chakoya placements establish membership; snow/ice terrain in32600..32650,32540..32585,floors5..7 supplies the bounded iceberg layer. Equal daily iceberg/open-water alternatives are conservative. Open water removes owned upper-floor tiles; floor7 becomes water622.", chakoya)

    nest = [32817, 32280, 8]
    add(9, "spider_nest", 1000, [
        variant("open", tiles=[at(nest)], spawns=[spawn("Mamma Longlegs", nest, 90000)],
                portals=[portal([32817, 32280, 7], nest), portal([32818, 32280, 8], [32817, 32280, 7], exit=True)]),
        variant("sealed", tiles=[{**at([32817, 32280, 7]), "entries": [at([32817, 32280, 7])["entries"][0], [12747, None, None]]}]),
    ], "Exact source GiantSpider pad is conservatively assigned to Mamma; its source surface anchor is the entry and adjacent cave tile the return. Equal open/sealed alternatives and90second respawn are assumptions. Personal third Mamma kill denies subsequent entry; the sealed variant has a physical large-web barrier.")

    spirit = []
    for town, entry in zip(["darashia", "vengoth", "ghostlands"], spirit_entries, strict=True):
        for monster in ["Nightmare", "Nightmare Scion", "Spectre", "Phantasm"]:
            spirit.append(variant(town + "_" + monster.lower().replace(" ", "_"),
                tiles=[at(entry), at(spirit_return), *[at(p) for p in spirit_pads]],
                spawns=[spawn(monster, p) for p in spirit_pads],
                portals=[portal(entry, spirit_destination), portal(spirit_return, entry, exit=True)]))
    add(10, "spirit_grounds", 1000, spirit,
        "Three entrance coordinates and shared destination are exact donor teleport attributes. Source wiki says creature selection is independent of portal. The Cartesian product3entrances×4populations uses equal weights. Four nearby population pads and the adjacent return selector are conservative assumptions.")

    add(11, "lumberjack", 90, [variant("felled_trees", tiles=stumps,
        npcs=[{"position": [32418, 31699, 7], "key": "oteryn:npc.chip"}])],
        "W15 fixes90permille. Eight exact source stump tiles form the bounded Fields of Glory layer; Chip's nearby source-terrain pad is inferred. Chip→Queen Eloise report ordering and50gold wood sale are wiki-sourced; quantity is not invented.")

    add(12, "bored", 82, [variant("illusion_spider", tiles=[at([32767, 31974, 7])],
        spawns=[spawn("Giant Spider Wyda", [32767, 31974, 7], 90000)])],
        "W15 fixes82permille. Existing GreenClaw GiantSpider pad is conservatively assigned to admitted GiantSpiderWyda identity. Its retained onDeath callback supplies the illusion message andSomeone's Bored achievement; no ordinary GiantSpider receives that credit.")

    insect_names = ["Insectoid Worker", "Crawler", "Swarmer", "Spitter", "Spidris", "Kollos"]
    hive = [spawn(insect_names[i], r["position"], r["respawn_ms"]) for i, r in enumerate(vandura)]
    hive_boss = hive[:-1] + [spawn("Hive Overseer", vandura[-1]["position"], 90000)]
    add(13, "hive_outpost", 500, [variant("invasion", weight=49, tiles=footprint(hive), spawns=hive),
        variant("invasion_overseer", weight=1, tiles=footprint(hive_boss), spawns=hive_boss)],
        "Wiki places the outpost on Vandura. Six exact native Vandura spawn pads are conservatively repurposed for hive-born membership;500permille and49:1optionalOverseer weights are assumptions. Native creature definitions remain unchanged.", vandura)

    poacher_variants = []
    for state, names in [("game", ["Wolf", "Bear", "Boar", "War Wolf"]),
                         ("poachers", ["Poacher"]), ("gloomy", ["Gloom Wolf", "Ghost Wolf"])]:
        population = [spawn(names[i % len(names)], r["position"], r["respawn_ms"]) for i, r in enumerate(wolves)]
        poacher_variants.append(variant(state, tiles=footprint(population), spawns=population))
    add(14, "poacher_caves", 1000, poacher_variants,
        "Source wiki gives Game/Poachers/Gloomy populations but no accepted executable kill threshold/order. Equal daily selection is the conservative assumption. The same12exact source Gloom/GhostWolf pads define exclusive population membership and60second source timers.", wolves)

    burning = {tuple(t["position"]): t for t in footprint(fire) + bog + flames}
    restored = []
    for t in burning.values():
        entries = [e for e in t["entries"] if e[0] not in flame_ids]
        restored.append({**t, "entries": entries or [[4515, None, None]]})
    add(15, "thornfire", 1000, [
        variant("burning", tiles=burning.values(), spawns=native(fire)),
        variant("restored", tiles=restored),
    ], "Exact source Firestarters, flame tiles from prepared-bucket's target IDs and bog-water12721 establish bounded Shadowthorn membership. Restored removes only those flame entries, preserving source geometry and bog water. Equal burning/restored daily variants are conservative until the shared three-prison/all-elf contribution chooses the next-save variant. Prepared-bucket and wolf-transform callbacks retain existing source rules; no guaranteed wolf/mount is invented.", fire)
    if len(definitions) != 13 or {d["slot"] for d in definitions} != set(range(3, 16)):
        raise ValueError("WVAR immutable slot allocation changed")
    return definitions


def encode(definitions: list[dict]) -> str:
    """Keep one native placement per line, as the existing layer author does."""
    compact = []
    presentation = []
    for definition in definitions:
        authored = {**definition, "variants": []}
        for original in definition["variants"]:
            recipe = dict(original)
            for field in ["tiles", "tags", "spawns", "npcs", "portals"]:
                recipe[field] = []
                for row in original[field]:
                    recipe[field].append(f"__WVAR_ROW_{len(compact)}__")
                    compact.append(json.dumps(row, separators=(",", ":")))
            authored["variants"].append(recipe)
        presentation.append(authored)
    expanded = json.dumps(presentation, indent=2)
    return re.sub(r'"__WVAR_ROW_(\d+)__"', lambda m: compact[int(m[1])], expanded) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--out", type=Path, default=Path("content/encounters/world-changes/variants-extra.json"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = encode(author(args.donor))
    if args.check:
        if args.out.read_text() != result:
            raise SystemExit(f"{args.out}: WVAR recipes differ from pinned inputs")
    else:
        args.out.write_text(result)


if __name__ == "__main__":
    main()
