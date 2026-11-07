"""Read the object appearances of the official Tibia client `appearances-<sha256>.dat`.

The file is a raw protobuf (`Appearances`). Only what the Terrain family relies on is
decoded; every other field is skipped by its wire framing, and the reader fails closed on a
malformed frame or an unsupported wire type (`client_map_reader.fields`):

- top level field 1 (repeated): an object appearance. Fields 2 (outfits), 3 (effects) and 4
  (missiles) are other appearance kinds and are ignored.
- object: 1 id, 2 frame groups (skipped), 3 flags, 4 name, 5 description (skipped).
- flags (only the fields below are read): 1 `bank` (1 waypoints = ground speed), 2 `clip`,
  13 `unpass`, 14 `unmove`, 30 `automap` (1 colour). A boolean flag is present with value 1;
  an absent field is false.

The field numbers are those of Tibia's public `appearances.proto`. They were checked against
the pinned CrystalServer `items.xml` (README, "Terrain"): appearance ids equal server ids.
"""

from __future__ import annotations

from dataclasses import dataclass

import client_map_reader as reader

OBJECT_FIELD = 1
FLAG_BOOLEANS = {2: "clip", 13: "unpass", 14: "unmove"}
FLAG_BANK, FLAG_AUTOMAP = 1, 30
OBJECT_FIELDS = {1, 2, 3, 4, 5}


class AppearanceError(reader.ClientMapError):
    pass


@dataclass(frozen=True)
class Appearance:
    id: int
    name: str | None
    flags: frozenset[str]  # subset of bank, clip, unmove, unpass
    speed: int | None  # bank waypoints
    automap_color: int | None


def one_varint(raw: bytes, what: str) -> int | None:
    """The varint field 1 of a small submessage, or None when it is absent."""
    grouped = reader.collect(raw, {1}, what)
    return reader.single(grouped, 1, 0, what, required=False)


def decode_flags(
    raw: bytes, what: str
) -> tuple[frozenset[str], int | None, int | None]:
    names: set[str] = set()
    speed = color = None
    seen: set[int] = set()
    for number, wire, value in reader.fields(raw):
        if number in seen and (number in FLAG_BOOLEANS or number in (1, 30)):
            raise AppearanceError(f"{what}: flag {number} occurs twice")
        if number in FLAG_BOOLEANS:
            if wire != 0 or value not in (0, 1):
                raise AppearanceError(f"{what}: flag {number} is not a boolean")
            seen.add(number)
            if value:
                names.add(FLAG_BOOLEANS[number])
        elif number == FLAG_BANK:
            if wire != 2 or not isinstance(value, bytes):
                raise AppearanceError(f"{what}: bank is not a message")
            seen.add(number)
            names.add("bank")
            speed = one_varint(value, f"{what} bank")
        elif number == FLAG_AUTOMAP:
            if wire != 2 or not isinstance(value, bytes):
                raise AppearanceError(f"{what}: automap is not a message")
            seen.add(number)
            color = one_varint(value, f"{what} automap")
    return frozenset(names), speed, color


def decode_object(raw: bytes) -> Appearance:
    grouped: dict[int, list] = {}
    for number, wire, value in reader.fields(raw):
        if number in OBJECT_FIELDS:
            grouped.setdefault(number, []).append((wire, value))
    object_id = reader.single(grouped, 1, 0, "appearance")
    what = f"appearance {object_id}"
    name = reader.single(grouped, 4, 2, what, required=False)
    flags_raw = reader.single(grouped, 3, 2, what, required=False)
    flags, speed, color = (
        decode_flags(flags_raw, what)
        if flags_raw is not None
        else (frozenset(), None, None)
    )
    return Appearance(
        id=object_id,
        name=reader.text(name, what) if name is not None else None,
        flags=flags,
        speed=speed,
        automap_color=color,
    )


def read_appearances(data: bytes) -> dict[int, Appearance]:
    """Every object appearance by id; an id that occurs twice fails closed."""
    found: dict[int, Appearance] = {}
    for number, wire, value in reader.fields(data):
        if number != OBJECT_FIELD:
            continue
        if wire != 2 or not isinstance(value, bytes):
            raise AppearanceError("object appearance is not a message")
        appearance = decode_object(value)
        if appearance.id in found:
            raise AppearanceError(f"appearance id {appearance.id} occurs twice")
        found[appearance.id] = appearance
    return found
