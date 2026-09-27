#!/usr/bin/env python3
"""Check transcribed chest/door/interaction positions against the real game map.

Reads the Canary `otservbr.otbm` release map (path given on the command line, sha256
checked against a pinned constant) plus, optionally, CrystalServer's own `world.otbm`
for the two CrystalServer-only positions the format doc's open questions name, and
compares them against the positions already transcribed into
`samples/chests/manifest.json`, `samples/doors/manifest.json` and
`samples/interactions/interactions.json`. Writes `samples/map-check/report.json`.

The repository's existing OTBM-adjacent tools (`tools/game-atlas-fullworld-source/producer.py`,
`tools/tibia-worldmap-reconstruction/`) do not themselves contain a binary OTBM node-tree
parser to import: the former only delegates to `tools.otbm_atlas`, a parser module that lives
in a *different*, not-checked-out repository (`blakinio/Otheryn`) and is not present anywhere
in this repository; the latter operates purely on an already-parsed, normalized JSON document,
not on OTBM bytes. Reusing them as-is was not possible, so this file's `OtbmReader`/tile-walk is
a minimal parser written directly against Canary's own reference implementation
(`src/io/iomap.cpp`, `src/io/io_definitions.hpp`, `src/map/mapcache.cpp`), which this repository
does have checked out read-only under the reference-server clones, so the node layout, attribute
ids and the tile/item byte widths below are not re-invented, only transcribed into Python.

Usage:
    python3 ots_map_check.py <canary otservbr.otbm path> [--crystalserver <world.otbm path>]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
SAMPLES = HERE / "samples"
REPORT_DIR = SAMPLES / "map-check"
REPORT_PATH = REPORT_DIR / "report.json"

# Pinned per the task: Canary's release map, downloaded (never committed) from the URL
# `config.lua.dist` names (`mapDownloadUrl`); recorded sha256 of the exact bytes checked here.
CANARY_MAP_URL = "https://github.com/opentibiabr/canary/releases/download/v3.6.1/otservbr.otbm"
CANARY_MAP_SHA256 = "a80de1dda6a9aca3956a9d5b7fb2e0caebb451570d26853fc21beb40d5f31da2"

# CrystalServer ships its own world map inside its repository (not downloaded): a gzip-compressed
# OTBM at `data-global/world/world.otbm` (config.lua.dist: `mapName = "world"`,
# `toggleCompressedMap = true`). Decompress with `gunzip` before passing it here; sha256 below is
# of the decompressed bytes, from zimbadev/crystalserver@ff7ede59.
CRYSTALSERVER_MAP_SOURCE = (
    "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a :: "
    "data-global/world/world.otbm (gzip-compressed in the repository; decompressed before use)"
)
CRYSTALSERVER_MAP_SHA256 = "f1080fbefdea904f1b7465b7479beb98080e74f5d27dfdfd8e38e69be2399726"

# Extra maps Canary's config/loader references beyond the main release map, and whether the
# startup path (`CanaryServer::loadMaps` in `src/canary_server.cpp`) actually loads them.
EXTRA_MAP_NOTES = [
    {
        "path": "data-otservbr-global/world/custom/otservbr-custom.otbm",
        "loaded_at_startup": True,
        "reason": "toggleMapCustom=true (config.lua.dist default): CanaryServer::loadMaps loads "
        "every *.otbm under world/custom/ right after the main map.",
    },
    {
        "path": "data-otservbr-global/world/quest/soul_war/ebb_and_flow/*.otbm",
        "loaded_at_startup": False,
        "reason": "Loaded later, at runtime, by Game.loadMap(...) in lib/quests/soul_war.lua "
        "cycling the Ebb and Flow water-level overlay; not part of CanaryServer::loadMaps.",
    },
    {
        "path": "data-otservbr-global/world/quest/ferumbras_ascendant/habitats.otbm",
        "loaded_at_startup": False,
        "reason": "Loaded on demand by Game.loadMap(...) in data/libs/functions/functions.lua "
        "when the Ferumbras habitats are reset; not part of CanaryServer::loadMaps.",
    },
    {
        "path": "data-otservbr-global/world/world_changes/**/*.otbm "
        "(fury_gates, nightmare_isle, oriental_trader, full_moon)",
        "loaded_at_startup": False,
        "reason": "Loaded on demand by Game.loadMap(...) from world_changes/*.lua globalevents "
        "(gate rotation, full-moon werewolf event, etc.); not part of CanaryServer::loadMaps.",
    },
    {
        "path": "data-otservbr-global/world/quest/the_dream_courts/*.otbm, "
        "data-otservbr-global/world/quest/cults_of_tibia/misguided/*.otbm",
        "loaded_at_startup": False,
        "reason": "Same pattern: quest-triggered Game.loadMap(...) overlays, not startup-loaded.",
    },
]

# The two open map questions from OTERYN_QUEST_AUTHORING_FORMAT_V1.md §6.
CORPSE_CHEST_POSITION = (32576, 32216, 15)
CORPSE_CHEST_UID = 5016
WOTE_CANARY_ONLY_POSITION = (33074, 31170, 8)
WOTE_CANARY_ONLY_UID = 14092
WOTE_CRYSTALSERVER_POSITION = (33079, 31173, 8)
WOTE_CRYSTALSERVER_UID = 7825


# --------------------------------------------------------------------------- OTBM node types --
# From canary/src/io/io_definitions.hpp (OTBM_NodeTypes_t / OTBM_AttrTypes_t), and the identical
# item attribute ids in canary/src/items/items_definitions.hpp used by mapcache.cpp's readAttr.
OTBM_MAP_DATA = 2
OTBM_TILE_AREA = 4
OTBM_TILE = 5
OTBM_ITEM = 6
OTBM_HOUSETILE = 14
OTBM_TOWNS = 12
OTBM_TOWN = 13
OTBM_WAYPOINTS = 15
OTBM_WAYPOINT = 16
OTBM_TILE_ZONE = 19

ATTR_DESCRIPTION = 1
ATTR_ACTION_ID = 4
ATTR_UNIQUE_ID = 5
ATTR_TEXT = 6
ATTR_DESC = 7
ATTR_TELE_DEST = 8
ATTR_ITEM = 9
ATTR_DEPOT_ID = 10
ATTR_EXT_SPAWN_MONSTER_FILE = 11
ATTR_CHARGES_U8 = 12  # ATTR_RUNE_CHARGES on the map-data side; unused here but kept for parity
ATTR_EXT_HOUSE_FILE = 13
ATTR_HOUSEDOORID = 14
ATTR_COUNT = 15
ATTR_TILE_FLAGS = 3
ATTR_DURATION = 16
ATTR_DECAYING_STATE = 17
ATTR_WRITTENDATE = 18
ATTR_WRITTENBY = 19
ATTR_SLEEPERGUID = 20
ATTR_SLEEPSTART = 21
ATTR_CHARGES = 22
ATTR_EXT_SPAWN_NPC_FILE = 23
ATTR_EXT_ZONE_FILE = 24

NODE_START = 0xFE
NODE_END = 0xFF
ESCAPE = 0xFD


class OtbmError(RuntimeError):
    pass


class OtbmReader:
    """Escape-aware forward-only cursor over one OTBM node tree, mirroring
    canary/src/io/filestream.cpp's FileStream (0xFD escapes a following literal 0xFD/0xFE/0xFF;
    a raw, un-escaped 0xFE/0xFF is always a real node-start/node-end marker, never data)."""

    __slots__ = ("data", "pos", "size")

    def __init__(self, data: bytes, pos: int = 0):
        self.data = data
        self.pos = pos
        self.size = len(data)

    def read_byte(self) -> int:
        d = self.data
        p = self.pos
        if p >= self.size:
            raise OtbmError("unexpected end of file")
        b = d[p]
        if b == ESCAPE:
            p += 1
            if p >= self.size:
                raise OtbmError("unexpected end of file after escape byte")
            b = d[p]
        self.pos = p + 1
        return b

    def read_u16(self) -> int:
        d = self.data
        p = self.pos
        if p + 1 < self.size and d[p] != ESCAPE and d[p + 1] != ESCAPE:
            self.pos = p + 2
            return d[p] | (d[p + 1] << 8)
        lo = self.read_byte()
        hi = self.read_byte()
        return lo | (hi << 8)

    def read_u32(self) -> int:
        d = self.data
        p = self.pos
        if p + 3 < self.size and ESCAPE not in (d[p], d[p + 1], d[p + 2], d[p + 3]):
            self.pos = p + 4
            return d[p] | (d[p + 1] << 8) | (d[p + 2] << 16) | (d[p + 3] << 24)
        b0 = self.read_byte()
        b1 = self.read_byte()
        b2 = self.read_byte()
        b3 = self.read_byte()
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)

    def read_string(self) -> bytes:
        length = self.read_u16()
        # Raw copy, matching FileStream::getString (not escape-processed byte by byte).
        p = self.pos
        s = self.data[p : p + length]
        self.pos = p + length
        return s

    def peek(self) -> int:
        save = self.pos
        v = self.read_byte()
        self.pos = save
        return v

    def try_consume(self, tag: int) -> bool:
        save = self.pos
        v = self.read_byte()
        if v == tag:
            return True
        self.pos = save
        return False

    def expect_start(self, node_type: int | None = None) -> bool:
        """Mirrors FileStream::startNode: with node_type given, also consumes+checks the type
        byte; with node_type=None, only consumes the 0xFE and leaves the type byte for the
        caller (used where iomap.cpp reads the type itself, e.g. TILE vs HOUSETILE)."""
        if not self.try_consume(NODE_START):
            return False
        if node_type is None:
            return True
        if self.try_consume(node_type):
            return True
        self.pos -= 1  # undo the NODE_START consumption too (matches FileStream::back()x2)
        return False

    def expect_end(self) -> bool:
        return self.try_consume(NODE_END)


def _skip_item_attrs(r: OtbmReader) -> tuple[int, int]:
    """Consume one item's readAttr() attribute stream; return (action_id, unique_id).

    Stops (restoring position) at the first byte that is not a known attribute tag, exactly
    like readAttr()'s default case + stream.back(); that byte is then either another node's
    0xFE (a container's contents) or this node's own closing 0xFF, left for the caller."""
    action_id = 0
    unique_id = 0
    while True:
        save = r.pos
        attr = r.read_byte()
        if attr == ATTR_DEPOT_ID:
            r.read_u16()
        elif attr == ATTR_HOUSEDOORID:
            r.read_byte()
        elif attr == ATTR_TELE_DEST:
            r.read_u16()
            r.read_u16()
            r.read_byte()
        elif attr == ATTR_COUNT:
            r.read_byte()
        elif attr == ATTR_CHARGES:
            r.read_u16()
        elif attr == ATTR_ACTION_ID:
            action_id = r.read_u16()
        elif attr == ATTR_UNIQUE_ID:
            unique_id = r.read_u16()
        elif attr == ATTR_TEXT:
            r.read_string()
        elif attr == ATTR_DESC:
            r.read_string()
        else:
            r.pos = save
            break
    return action_id, unique_id


def _parse_item_node(r: OtbmReader, item_id: int, depth: int) -> dict[str, Any]:
    """Parse one OTBM_ITEM node's body (already past the node's 'type' and 'id' u16); mirrors
    BasicItem::unserializeItemNode. Does NOT consume this item's own closing 0xFF: like the
    reference (whose caller in iomap.cpp calls stream.endNode() itself after this returns), the
    caller here must call r.expect_end() right after this returns."""
    if r.peek() == NODE_END:
        # Empty item (isProp(END) shortcut in the reference): nothing more to read.
        return {"id": item_id, "aid": 0, "uid": 0, "children": []}
    action_id, unique_id = _skip_item_attrs(r)
    children: list[dict[str, Any]] = []
    while r.expect_start(None):
        node_type = r.read_byte()
        if node_type == OTBM_ITEM:
            child_id = r.read_u16()
            child = _parse_item_node(r, child_id, depth + 1)
            if depth < 8:
                children.append(child)
        elif node_type == OTBM_TILE_ZONE:
            count = r.read_u16()
            for _ in range(count):
                r.read_u16()
        else:
            raise OtbmError(f"unexpected child node type {node_type} inside item")
        if not r.expect_end():
            raise OtbmError("unterminated item child node")
    return {"id": item_id, "aid": action_id, "uid": unique_id, "children": children}


def walk_map(path: Path, wanted: set[tuple[int, int, int]]) -> dict[tuple[int, int, int], dict[str, Any]]:
    """Single pass over the OTBM tile tree. Returns, for every wanted (x,y,z) whose tile is
    actually present in the file, {'ground': item_id|None, 'items': [ {id,aid,uid,children}, ... ]}."""
    data = path.read_bytes()
    results: dict[tuple[int, int, int], dict[str, Any]] = {}

    r = OtbmReader(data, 4)  # skip the 4-byte OTBM identifier
    if not r.expect_start(None):
        raise OtbmError("no root node")
    r.read_byte()  # root node type byte (raw skip in the reference; value is not meaningful)
    r.read_u32()  # version
    r.read_u16()  # width
    r.read_u16()  # height
    r.read_u32()  # items major version
    r.read_u32()  # items minor version

    if not r.expect_start(OTBM_MAP_DATA):
        raise OtbmError("missing OTBM_MAP_DATA node")

    # parseMapDataAttributes: consume known small string attributes until an unrecognised byte
    # (the first child node's 0xFE) is hit, then back up.
    while True:
        attr = r.peek()
        if attr in (
            ATTR_DESCRIPTION,
            ATTR_EXT_SPAWN_MONSTER_FILE,
            ATTR_EXT_SPAWN_NPC_FILE,
            ATTR_EXT_HOUSE_FILE,
            ATTR_EXT_ZONE_FILE,
        ):
            r.read_byte()
            r.read_string()
        else:
            break

    # parseTileArea: repeated OTBM_TILE_AREA children.
    while r.expect_start(OTBM_TILE_AREA):
        base_x = r.read_u16()
        base_y = r.read_u16()
        base_z = r.read_byte()

        while r.expect_start(None):
            tile_type = r.read_byte()
            if tile_type not in (OTBM_TILE, OTBM_HOUSETILE):
                raise OtbmError(f"unexpected tile node type {tile_type}")

            tile_x_off = r.read_byte()
            tile_y_off = r.read_byte()
            x = base_x + tile_x_off
            y = base_y + tile_y_off
            z = base_z

            if tile_type == OTBM_HOUSETILE:
                r.read_u32()  # house id

            if r.try_consume(ATTR_TILE_FLAGS):
                r.read_u32()

            ground: int | None = None
            if r.try_consume(ATTR_ITEM):
                ground = r.read_u16()

            want = (x, y, z) in wanted
            items: list[dict[str, Any]] = []
            while r.expect_start(None):
                node_type = r.read_byte()
                if node_type == OTBM_ITEM:
                    item_id = r.read_u16()
                    parsed = _parse_item_node(r, item_id, 0)
                    if want:
                        items.append(parsed)
                elif node_type == OTBM_TILE_ZONE:
                    count = r.read_u16()
                    for _ in range(count):
                        r.read_u16()
                else:
                    raise OtbmError(f"unexpected tile child node type {node_type}")
                if not r.expect_end():
                    raise OtbmError("unterminated tile child node")

            if not r.expect_end():
                raise OtbmError("unterminated tile node")

            if want:
                results[(x, y, z)] = {"ground": ground, "items": items}

        if not r.expect_end():
            raise OtbmError("unterminated tile area node")

    # Every wanted tile has now been seen: OTBM_TOWNS/OTBM_WAYPOINTS (still nested inside
    # OTBM_MAP_DATA in the byte stream) carry nothing this tool needs, so parsing stops here.
    # This mirrors the reference loader's own iomap.cpp, which closes OTBM_MAP_DATA with an
    # unchecked stream.endNode() at exactly this point (it fails silently, since a towns node
    # still follows) before parsing towns/waypoints as if they were separate top-level nodes.

    return results


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for block in iter(lambda: fh.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


# --------------------------------------------------------------------------- sample loading --
def load_json(path: Path) -> Any:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def tile_items(tiles: dict[tuple[int, int, int], dict[str, Any]], pos: tuple[int, int, int]) -> list[dict[str, Any]] | None:
    """Flattened (id, aid, uid) for every item directly on the tile (ground plus stack, plus one
    level of container contents), or None if the tile itself was not present in the map at all."""
    tile = tiles.get(pos)
    if tile is None:
        return None
    flat: list[dict[str, Any]] = []
    if tile.get("ground") is not None:
        flat.append({"id": tile["ground"], "aid": 0, "uid": 0})
    for item in tile["items"]:
        flat.append({"id": item["id"], "aid": item["aid"], "uid": item["uid"]})
        for child in item.get("children", []):
            flat.append({"id": child["id"], "aid": child["aid"], "uid": child["uid"]})
    return flat


def find_uid(items: list[dict[str, Any]], uid: int) -> dict[str, Any] | None:
    for item in items:
        if item["uid"] == uid:
            return item
    return None


def find_aid(items: list[dict[str, Any]], aid: int) -> dict[str, Any] | None:
    for item in items:
        if item["aid"] == aid:
            return item
    return None


def find_item_id(items: list[dict[str, Any]], item_id: int) -> dict[str, Any] | None:
    for item in items:
        if item["id"] == item_id:
            return item
    return None


ITEM_IDS_CAP = 8


def item_id_list(items: list[dict[str, Any]]) -> list[int]:
    return [item["id"] for item in items[:ITEM_IDS_CAP]]


_APPEARANCE_RE = re.compile(r"^(canary|crystalserver):item/(\d+)$")


def parse_appearance(appearance: dict[str, Any] | None) -> tuple[str, int] | None:
    """('canary'|'crystalserver', item_id) from a placement's appearance key, or None."""
    if not appearance:
        return None
    m = _APPEARANCE_RE.match(appearance.get("key", ""))
    if not m:
        return None
    return m.group(1), int(m.group(2))


# --------------------------------------------------------------------------- chests --
# Canary's startup GlobalEvent "Map Attributes Loader" (scripts/globalevents/others/
# map_attributes_loader.lua) calls loadLuaMapUnique(ChestUnique) etc: it finds the item with the
# table's *itemId* already placed at *itemPos* on the raw map and only THEN stamps the table's
# numeric key onto it as ITEM_ATTRIBUTE_UNIQUEID (data-otservbr-global/startup/others/
# functions.lua, loadLuaMapUnique/loadLuaMapAction). So the distributed otservbr.otbm never has
# these chest/door unique or action ids baked in; the meaningful, checkable ground truth is
# whether the expected *item id* (the chest's appearance) is already sitting at the expected
# position, exactly the precondition Canary's own loader checks (and logs "Wrong item id found"
# on failure) before it would stamp the uid. That is what this check verifies; a uid found
# already-embedded in the downloaded release map (before any server ever ran the loader) would be
# surprising and is reported separately, but its absence is not counted as a mismatch.
def check_chests(tiles: dict[tuple[int, int, int], dict[str, Any]]) -> dict[str, Any]:
    claims = load_json(SAMPLES / "chests" / "claims.json")["claims"]
    manifest = load_json(SAMPLES / "chests" / "manifest.json")

    matched = 0
    mismatched: list[dict[str, Any]] = []
    tile_missing = 0
    crystalserver_only = 0
    for claim in claims:
        key = claim["identity"]["key"]
        for placement in claim["placements"]:
            pos = (placement["position"]["x"], placement["position"]["y"], placement["position"]["z"])
            parsed = parse_appearance(placement.get("appearance"))
            if parsed is None:
                continue
            namespace, expected_item_id = parsed
            if namespace != "canary":
                crystalserver_only += 1
                continue
            items = tile_items(tiles, pos)
            if items is None:
                tile_missing += 1
                mismatched.append({"key": key, "position": list(pos), "expected_item_id": expected_item_id, "found": "no tile"})
                continue
            if find_item_id(items, expected_item_id) is not None:
                matched += 1
            else:
                mismatched.append(
                    {
                        "key": key,
                        "position": list(pos),
                        "expected_item_id": expected_item_id,
                        "found_item_ids": item_id_list(items),
                    }
                )

    already_embedded_uid = 0
    for entry in manifest["entries"]:
        if "position" not in entry:
            continue
        pos = tuple(entry["position"])
        expected_uid = next((src["uid"] for src in entry["sources"] if src.get("source") == "canary" and "uid" in src), None)
        if expected_uid is None:
            continue
        items = tile_items(tiles, pos)
        if items is not None and find_uid(items, expected_uid) is not None:
            already_embedded_uid += 1

    return {
        "note": "Matched/mismatched compare the chest's expected ITEM ID (appearance) against the "
        "raw map, mirroring loadLuaMapUnique's own precondition check; the runtime unique id "
        "itself is assigned by Canary's Map Attributes Loader startup event, not baked into the "
        "distributed map, so its absence here is expected and not a mismatch.",
        "total_checked": matched + len(mismatched),
        "matched": matched,
        "mismatched_count": len(mismatched),
        "tile_missing_count": tile_missing,
        "crystalserver_only_placements_not_checked": crystalserver_only,
        "already_embedded_unique_id_in_raw_map": already_embedded_uid,
        "mismatches": mismatched[:200],
    }


# --------------------------------------------------------------------------- doors --
# Same startup-assignment mechanism as chests (loadLuaMapAction(KeyDoorAction/LevelDoorAction/
# QuestDoorAction) in map_attributes_loader.lua): checked against the door's expected item id
# (appearance) when the transcription recorded one; a null appearance means the source Lua table
# used itemId=false ("apply to the tile's top item, whatever it is"), which this checks only for
# tile/top-item presence, matching that wildcard semantics.
def check_doors(tiles: dict[tuple[int, int, int], dict[str, Any]]) -> dict[str, Any]:
    gates = load_json(SAMPLES / "doors" / "gates.json")["gates"]

    matched = 0
    mismatched: list[dict[str, Any]] = []
    tile_missing = 0
    crystalserver_only = 0
    no_expected_item_id_present = 0
    no_expected_item_id_missing: list[dict[str, Any]] = []
    for gate in gates:
        key = gate["identity"]["key"]
        for placement in gate["placements"]:
            pos = (placement["position"]["x"], placement["position"]["y"], placement["position"]["z"])
            items = tile_items(tiles, pos)
            parsed = parse_appearance(placement.get("appearance"))
            if parsed is not None and parsed[0] != "canary":
                crystalserver_only += 1
                continue
            if items is None:
                tile_missing += 1
                mismatched.append({"key": key, "position": list(pos), "found": "no tile"})
                continue
            if parsed is not None:
                _, expected_item_id = parsed
                if find_item_id(items, expected_item_id) is not None:
                    matched += 1
                else:
                    mismatched.append(
                        {
                            "key": key,
                            "position": list(pos),
                            "expected_item_id": expected_item_id,
                            "found_item_ids": item_id_list(items),
                        }
                    )
            else:
                if items:
                    no_expected_item_id_present += 1
                else:
                    no_expected_item_id_missing.append({"key": key, "position": list(pos)})
    return {
        "note": "Matched/mismatched compare the door's expected ITEM ID (appearance) against the "
        "raw map; the door's runtime action id itself is assigned by Canary's Map Attributes "
        "Loader startup event (loadLuaMapAction), not baked into the distributed map.",
        "total_checked": matched + len(mismatched) + no_expected_item_id_present + len(no_expected_item_id_missing),
        "matched": matched,
        "mismatched_count": len(mismatched),
        "tile_missing_count": tile_missing,
        "crystalserver_only_placements_not_checked": crystalserver_only,
        "no_expected_item_id_tile_has_content": no_expected_item_id_present,
        "no_expected_item_id_tile_empty_count": len(no_expected_item_id_missing),
        "no_expected_item_id_tile_empty": no_expected_item_id_missing[:200],
        "mismatches": mismatched[:200],
    }


# --------------------------------------------------------------------------- interactions --
_AID_RE = re.compile(r"^aid\((\d+)\)$")
_UID_RE = re.compile(r"^uid\((\d+)\)$")


def check_interactions(tiles: dict[tuple[int, int, int], dict[str, Any]]) -> dict[str, Any]:
    interactions = load_json(SAMPLES / "interactions" / "interactions.json")["interactions"]
    tile_present = 0
    tile_missing = 0
    use_aid_uid_matched = 0
    use_aid_uid_mismatched: list[dict[str, Any]] = []
    use_no_literal_registration = 0
    missing_tiles: list[dict[str, Any]] = []
    for interaction in interactions:
        anchors = interaction.get("anchors") or []
        if not anchors:
            continue
        key = interaction["identity"]["key"]
        edge = interaction["source"]["edge"]
        registrations = interaction.get("source", {}).get("target_registrations") or []
        literal_aid = None
        literal_uid = None
        for reg in registrations:
            m = _AID_RE.match(reg)
            if m:
                literal_aid = int(m.group(1))
            m = _UID_RE.match(reg)
            if m:
                literal_uid = int(m.group(1))
        for anchor in anchors:
            pos = (anchor["source_position"]["x"], anchor["source_position"]["y"], anchor["source_position"]["z"])
            items = tile_items(tiles, pos)
            if items is None:
                tile_missing += 1
                missing_tiles.append({"key": key, "anchor": anchor["key"], "position": list(pos)})
                continue
            tile_present += 1
            if edge != "USE":
                continue
            if literal_uid is not None:
                if find_uid(items, literal_uid) is not None:
                    use_aid_uid_matched += 1
                else:
                    use_aid_uid_mismatched.append(
                        {
                            "key": key,
                            "anchor": anchor["key"],
                            "position": list(pos),
                            "expected_uid": literal_uid,
                            "found_item_ids": item_id_list(items),
                        }
                    )
            elif literal_aid is not None:
                if find_aid(items, literal_aid) is not None:
                    use_aid_uid_matched += 1
                else:
                    use_aid_uid_mismatched.append(
                        {
                            "key": key,
                            "anchor": anchor["key"],
                            "position": list(pos),
                            "expected_aid": literal_aid,
                            "found_item_ids": item_id_list(items),
                        }
                    )
            else:
                use_no_literal_registration += 1
    return {
        "note": "aid()/uid() matched here against a literal number in the source registration. "
        "Some of these belong to the same generic Chest/Door/Item/Lever/Teleport/Tile tables "
        "check_chests/check_doors document as runtime-assigned by Canary's Map Attributes Loader "
        "(not baked into the raw map, so an expected mismatch); others are one-off map-editor "
        "unique/action ids the checked-out Lua does not route through that loader (a real, "
        "meaningful mismatch). This report does not attempt to separate the two classes.",
        "anchors_with_tile_present": tile_present,
        "anchors_with_tile_missing": tile_missing,
        "missing_tiles": missing_tiles[:200],
        "use_edges_with_literal_aid_or_uid_matched": use_aid_uid_matched,
        "use_edges_with_literal_aid_or_uid_mismatched_count": len(use_aid_uid_mismatched),
        "use_edges_with_literal_aid_or_uid_mismatches": use_aid_uid_mismatched[:200],
        "use_edges_without_a_literal_aid_or_uid_registration": use_no_literal_registration,
    }


# --------------------------------------------------------------------------- map decisions --
def check_map_decisions(
    canary_tiles: dict[tuple[int, int, int], dict[str, Any]],
    crystalserver_tiles: dict[tuple[int, int, int], dict[str, Any]] | None,
) -> dict[str, Any]:
    result: dict[str, Any] = {}

    canary_items = tile_items(canary_tiles, CORPSE_CHEST_POSITION)
    result["corpse_chest"] = {
        "note": "Both servers claim uid 5016 at this position (D25: 'equivalent', canary kept); "
        "the raw map's own item id at that position (not the runtime-assigned uid, which is not "
        "baked into the file; see check_chests) is what actually decides which appearance is real.",
        "position": list(CORPSE_CHEST_POSITION),
        "uid": CORPSE_CHEST_UID,
        "canary_claimed_item_id": 3204,
        "crystalserver_claimed_item_id": 4240,
        "canary_map_item_ids_at_position": item_id_list(canary_items) if canary_items is not None else None,
        "crystalserver_map_item_ids_at_position": None,
        "crystalserver_map_checked": crystalserver_tiles is not None,
    }
    if crystalserver_tiles is not None:
        cs_corpse_items = tile_items(crystalserver_tiles, CORPSE_CHEST_POSITION)
        result["corpse_chest"]["crystalserver_map_item_ids_at_position"] = (
            item_id_list(cs_corpse_items) if cs_corpse_items is not None else None
        )

    canary_wote_items = tile_items(canary_tiles, WOTE_CANARY_ONLY_POSITION)
    canary_at_crystalserver_spot = tile_items(canary_tiles, WOTE_CRYSTALSERVER_POSITION)
    wote: dict[str, Any] = {
        "note": "Canary-only chest_items claim (uid 14092, present_only_in_canary) vs a "
        "CrystalServer-only chest_items claim (uid 7825) a few tiles away, same reward. Each "
        "server's own map is also checked at the OTHER server's position, to tell 'one chest "
        "moved' apart from 'both servers place a chest at both spots but only one is wired up'.",
        "canary_only_position": list(WOTE_CANARY_ONLY_POSITION),
        "canary_only_uid": WOTE_CANARY_ONLY_UID,
        "canary_map_item_ids_at_canary_only_position": (
            item_id_list(canary_wote_items) if canary_wote_items is not None else None
        ),
        "canary_map_item_ids_at_crystalserver_position": (
            item_id_list(canary_at_crystalserver_spot) if canary_at_crystalserver_spot is not None else None
        ),
        "crystalserver_position": list(WOTE_CRYSTALSERVER_POSITION),
        "crystalserver_only_uid": WOTE_CRYSTALSERVER_UID,
        "crystalserver_map_item_ids_at_crystalserver_position": None,
        "crystalserver_map_item_ids_at_canary_only_position": None,
        "crystalserver_map_checked": crystalserver_tiles is not None,
    }
    if crystalserver_tiles is not None:
        cs_items = tile_items(crystalserver_tiles, WOTE_CRYSTALSERVER_POSITION)
        wote["crystalserver_map_item_ids_at_crystalserver_position"] = item_id_list(cs_items) if cs_items is not None else None
        cs_at_canary_spot = tile_items(crystalserver_tiles, WOTE_CANARY_ONLY_POSITION)
        wote["crystalserver_map_item_ids_at_canary_only_position"] = (
            item_id_list(cs_at_canary_spot) if cs_at_canary_spot is not None else None
        )
    result["wrath_of_the_emperor_chest"] = wote
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("canary_map", type=Path, help="path to a downloaded otservbr.otbm (Canary)")
    parser.add_argument(
        "--crystalserver",
        type=Path,
        default=None,
        help="path to CrystalServer's decompressed world.otbm, for the CrystalServer-only cross-check",
    )
    args = parser.parse_args()

    canary_path: Path = args.canary_map
    if not canary_path.is_file():
        print(f"error: canary map not found: {canary_path}", file=sys.stderr)
        return 2
    canary_sha256 = sha256_of(canary_path)
    if canary_sha256 != CANARY_MAP_SHA256:
        print(
            f"error: canary map sha256 mismatch: expected {CANARY_MAP_SHA256}, got {canary_sha256}",
            file=sys.stderr,
        )
        return 2

    crystalserver_path: Path | None = args.crystalserver
    crystalserver_sha256 = None
    if crystalserver_path is not None:
        if not crystalserver_path.is_file():
            print(f"error: crystalserver map not found: {crystalserver_path}", file=sys.stderr)
            return 2
        crystalserver_sha256 = sha256_of(crystalserver_path)
        if crystalserver_sha256 != CRYSTALSERVER_MAP_SHA256:
            print(
                f"error: crystalserver map sha256 mismatch: expected {CRYSTALSERVER_MAP_SHA256}, "
                f"got {crystalserver_sha256}",
                file=sys.stderr,
            )
            return 2

    chests_manifest = load_json(SAMPLES / "chests" / "manifest.json")
    chests_claims = load_json(SAMPLES / "chests" / "claims.json")["claims"]
    empty_containers = load_json(SAMPLES / "chests" / "empty_containers.json")["empty_containers"]
    doors_manifest = load_json(SAMPLES / "doors" / "manifest.json")
    doors_gates = load_json(SAMPLES / "doors" / "gates.json")["gates"]
    interactions = load_json(SAMPLES / "interactions" / "interactions.json")["interactions"]

    wanted: set[tuple[int, int, int]] = set()
    for entry in chests_manifest["entries"]:
        if "position" in entry:
            wanted.add(tuple(entry["position"]))
    for claim in chests_claims:
        for placement in claim["placements"]:
            wanted.add((placement["position"]["x"], placement["position"]["y"], placement["position"]["z"]))
    for container in empty_containers:
        wanted.add((container["position"]["x"], container["position"]["y"], container["position"]["z"]))
    for entry in doors_manifest["entries"]:
        if "position" in entry:
            wanted.add(tuple(entry["position"]))
    for gate in doors_gates:
        for placement in gate["placements"]:
            wanted.add((placement["position"]["x"], placement["position"]["y"], placement["position"]["z"]))
    for interaction in interactions:
        for anchor in interaction.get("anchors") or []:
            wanted.add((anchor["source_position"]["x"], anchor["source_position"]["y"], anchor["source_position"]["z"]))
    wanted.add(CORPSE_CHEST_POSITION)
    wanted.add(WOTE_CANARY_ONLY_POSITION)
    wanted.add(WOTE_CRYSTALSERVER_POSITION)  # also check canary's OWN map at crystalserver's spot

    canary_tiles = walk_map(canary_path, wanted)

    crystalserver_tiles = None
    if crystalserver_path is not None:
        crystalserver_tiles = walk_map(
            crystalserver_path,
            {WOTE_CRYSTALSERVER_POSITION, CORPSE_CHEST_POSITION, WOTE_CANARY_ONLY_POSITION},
        )

    report = {
        "format": "oteryn-ots-map-check-v1",
        "classification": "OTS_HYPOTHESIS_ONLY",
        "map_sources": {
            "canary": {
                "url": CANARY_MAP_URL,
                "sha256": canary_sha256,
                "path_basename": canary_path.name,
            },
            "crystalserver": (
                {
                    "source": CRYSTALSERVER_MAP_SOURCE,
                    "sha256": crystalserver_sha256,
                    "path_basename": crystalserver_path.name,
                }
                if crystalserver_path is not None
                else {"checked": False, "reason": "no --crystalserver path given"}
            ),
        },
        "extra_maps_canary_config_references": EXTRA_MAP_NOTES,
        "chests": check_chests(canary_tiles),
        "doors": check_doors(canary_tiles),
        "interactions": check_interactions(canary_tiles),
        "map_decisions": check_map_decisions(canary_tiles, crystalserver_tiles),
    }

    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    with REPORT_PATH.open("w", encoding="utf-8") as fh:
        json.dump(report, fh, indent=2, sort_keys=False)
        fh.write("\n")

    print(f"wrote {REPORT_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
