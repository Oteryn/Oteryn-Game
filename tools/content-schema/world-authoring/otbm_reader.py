"""Minimal streaming OTBM reader for world metadata (towns, houses, teleports).

Reference-only migration input (OTS_HYPOTHESIS_ONLY): it reads a pinned third-party map
and yields source facts; it never emits map bytes, terrain or placements.
"""

from __future__ import annotations

import gzip
import struct
from collections import Counter
from dataclasses import dataclass, field

ESC, START, END = 0xFD, 0xFE, 0xFF
NODE_MAP_DATA, NODE_TILE_AREA, NODE_TILE, NODE_ITEM = 2, 4, 5, 6
NODE_TOWNS, NODE_TOWN, NODE_HOUSE_TILE = 12, 13, 14
NODE_WAYPOINTS, NODE_WAYPOINT = 15, 16
ATTR_TILE_FLAGS, ATTR_ITEM, ATTR_TELE_DEST, ATTR_HOUSE_DOOR_ID = 3, 9, 8, 14

# Item attribute payload layout (forgottenserver lineage, shared by Canary/CrystalServer).
U8, U16, U32, STR = 1, 2, 4, 0
ITEM_ATTRS = {
    4: U16, 5: U16, 6: STR, 7: STR, 10: U16, 12: U8, 14: U8, 15: U8, 16: U32, 17: U8,
    18: U32, 19: STR, 20: U32, 21: U32, 22: U16, 23: U32, 24: STR, 25: STR, 26: STR,
    27: U32, 28: U32, 29: U32, 30: U32, 31: U32, 32: U8, 33: U8, 35: U32, 36: U16, 37: U8,
}  # fmt: skip


class OtbmError(ValueError):
    pass


@dataclass
class House:
    tiles: int = 0
    floors: Counter = field(default_factory=Counter)
    bbox: list[int] | None = None
    doors: list[tuple[int, int, int, int]] = field(default_factory=list)


@dataclass
class MapFacts:
    version: int = 0
    width: int = 0
    height: int = 0
    tiles: int = 0
    towns: list[dict] = field(default_factory=list)
    waypoints: list[dict] = field(default_factory=list)
    houses: dict[int, House] = field(default_factory=dict)
    teleports: list[dict] = field(default_factory=list)
    unknown_item_attrs: Counter = field(default_factory=Counter)
    present: set[tuple[int, int, int]] = field(default_factory=set)


def load_bytes(raw: bytes) -> bytes:
    data = gzip.decompress(raw) if raw[:2] == b"\x1f\x8b" else raw
    if len(data) < 6 or data[4] != START:
        raise OtbmError("not an OTBM node file")
    return data


def _string(buf: bytes, i: int) -> tuple[str, int]:
    (length,) = struct.unpack_from("<H", buf, i)
    return buf[i + 2 : i + 2 + length].decode("latin-1"), i + 2 + length


def _item_attrs(props: bytes, i: int, facts: MapFacts) -> dict:
    out: dict = {}
    while i < len(props):
        attr = props[i]
        i += 1
        if attr == ATTR_TELE_DEST:
            out["dest"] = struct.unpack_from("<HHB", props, i)
            i += 5
            continue
        size = ITEM_ATTRS.get(attr)
        if size is None:
            facts.unknown_item_attrs[attr] += 1
            return out
        if size == STR:
            out[attr], i = _string(props, i)
        else:
            out[attr] = int.from_bytes(props[i : i + size], "little")
            i += size
    return out


def _visit_items(node, pos, house_id, facts: MapFacts) -> None:
    for child in node[2]:
        if child[0] != NODE_ITEM:
            continue
        props = bytes(child[1])
        (item_id,) = struct.unpack_from("<H", props, 0)
        attrs = _item_attrs(props, 2, facts)
        if "dest" in attrs:
            facts.teleports.append(
                {"from": pos, "item": item_id, "to": tuple(attrs["dest"])}
            )
        if house_id is not None and ATTR_HOUSE_DOOR_ID in attrs:
            facts.houses[house_id].doors.append((*pos, attrs[ATTR_HOUSE_DOOR_ID]))
        _visit_items(child, pos, house_id, facts)


def _tile_area(node, facts: MapFacts, probe) -> None:
    base_x, base_y, z = struct.unpack_from("<HHB", node[1], 0)
    for tile in node[2]:
        if tile[0] not in (NODE_TILE, NODE_HOUSE_TILE):
            raise OtbmError(f"unexpected node {tile[0]} in tile area")
        pos = (base_x + tile[1][0], base_y + tile[1][1], z)
        facts.tiles += 1
        if probe is not None:
            if pos in probe:
                facts.present.add(pos)
            continue
        house_id = None
        if tile[0] == NODE_HOUSE_TILE:
            (house_id,) = struct.unpack_from("<I", tile[1], 2)
            house = facts.houses.setdefault(house_id, House())
            house.tiles += 1
            house.floors[z] += 1
            x, y = pos[0], pos[1]
            b = house.bbox
            house.bbox = (
                [x, y, x, y]
                if b is None
                else [min(b[0], x), min(b[1], y), max(b[2], x), max(b[3], y)]
            )
        _visit_items(tile, pos, house_id, facts)


def _towns(node, facts: MapFacts) -> None:
    for town in node[2]:
        props = bytes(town[1])
        (town_id,) = struct.unpack_from("<I", props, 0)
        name, i = _string(props, 4)
        facts.towns.append(
            {
                "town_id": town_id,
                "name": name,
                "temple": struct.unpack_from("<HHB", props, i),
            }
        )


def _waypoints(node, facts: MapFacts) -> None:
    for waypoint in node[2]:
        props = bytes(waypoint[1])
        name, i = _string(props, 0)
        facts.waypoints.append(
            {"name": name, "position": struct.unpack_from("<HHB", props, i)}
        )


def read(raw: bytes, probe: set[tuple[int, int, int]] | None = None) -> MapFacts:
    """Stream the node tree once. With ``probe`` only tile presence of those positions is read."""
    data = load_bytes(raw)
    facts = MapFacts()
    n = len(data)
    pos = 4
    stack: list[list] = []
    root = None
    while pos < n:
        byte = data[pos]
        if byte == START:
            node = [data[pos + 1], bytearray(), []]
            pos += 2
            if stack:
                stack[-1][2].append(node)
            else:
                root = node
            stack.append(node)
        elif byte == END:
            pos += 1
            node = stack.pop()
            if not stack:
                break
            parent = stack[-1]
            if parent[0] == NODE_MAP_DATA:
                if node[0] == NODE_TILE_AREA:
                    _tile_area(node, facts, probe)
                elif node[0] == NODE_TOWNS:
                    _towns(node, facts)
                elif node[0] == NODE_WAYPOINTS:
                    _waypoints(node, facts)
                parent[2].pop()
        elif byte == ESC:
            if not stack or pos + 1 >= n:
                raise OtbmError("dangling escape")
            stack[-1][1].append(data[pos + 1])
            pos += 2
        else:
            if not stack:
                raise OtbmError("data outside node")
            j = pos
            while j < n and data[j] not in (ESC, START, END):
                j += 1
            stack[-1][1].extend(data[pos:j])
            pos = j
    if root is None or stack:
        raise OtbmError("truncated node tree")
    facts.version, facts.width, facts.height = struct.unpack_from("<IHH", root[1], 0)
    return facts
