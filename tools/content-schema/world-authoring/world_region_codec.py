"""Pure encoder and decoder for OTERYN_WORLD_REGION_B3/v1 region files.

A region file holds the tiles of one 256x256 area on a single floor. The file is a fixed
header, a sector table and one zstd frame per non-empty 32x32 sector::

    header   "OTRB" | version u8 | z u8 | rx u16 | ry u16 | sector_count u16      (12 bytes, LE)
    table    sector_count x (local u8 | offset u32 | length u32)                   (9 bytes each)
    payloads zstd level-3 frames, in table order, back to back

``local`` is ``(sy % 8) * 8 + (sx % 8)`` and the table is strictly ascending. Each frame
decompresses to a sector payload (all integers are LEB128 varints unless noted)::

    tile_count
    per tile, ascending (y, x):  position_delta  control  [flags]  [house]  [zones]  items...
    position_delta = local_index - previous_local_index - 1   (local_index = y % 32 * 32 + x % 32)
    control        = item_count << 3 | zones_present << 2 | house_present << 1 | flags_present
    zones          = zone_count, then that many zone ids
    item           = registry << 1 | has_mask      then, when has_mask: mask, attribute values

Items are flattened in stacking order and container contents follow their container; the
``depth`` attribute (0 for top-level items) restores the nesting. ``registry`` is the Oteryn
item registry number, never a source server id. Attribute presence is exact: an attribute
that is present with value 0 or an empty text stays present.

In memory an item is ``(registry, depth, attrs)`` where ``attrs`` is ``None`` or a dict
using the names in ``ATTRIBUTES``, and a tile is ``(x, y, flags, house, zones, items)``
with ``zones`` a tuple of u16 tile zone ids (empty when the tile has none).
"""

from __future__ import annotations

import struct

import zstandard

MAGIC = b"OTRB"
VERSION = 1
CODEC = "OTERYN_WORLD_REGION_B3/v1"
REGION_SIZE = 256
SECTOR_SIZE = 32
SECTORS_PER_SIDE = REGION_SIZE // SECTOR_SIZE
ZSTD_LEVEL = 3
MAX_FLOOR = 15
MAX_SECTOR_BYTES = 16 * 1024 * 1024
HEADER = struct.Struct("<4sBBHHH")
ENTRY = struct.Struct("<BII")

# Attribute name -> (mask bit, kind). depth is structural and uses bit 0.
DEPTH_BIT = 1
ATTRIBUTES = {
    "count": (1 << 1, "u8"),
    "action": (1 << 2, "u16"),
    "unique": (1 << 3, "u16"),
    "door": (1 << 4, "u8"),
    "text": (1 << 5, "str"),
    "charges": (1 << 6, "u16"),
    "description": (1 << 7, "str"),
    "teleport": (1 << 8, "pos"),
    "depot": (1 << 9, "u16"),
}
KNOWN_MASK = DEPTH_BIT | sum(bit for bit, _ in ATTRIBUTES.values())
_ORDER = sorted(ATTRIBUTES.items(), key=lambda row: row[1][0])


class CodecError(ValueError):
    pass


def region_name(z: int, rx: int, ry: int) -> str:
    return f"region-z{z:02}-x{rx:03}-y{ry:03}.b3"


def _put(out: bytearray, value: int) -> None:
    while value >= 0x80:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value)


def _get(buf: bytes, pos: int) -> tuple[int, int]:
    value = shift = 0
    while True:
        byte = buf[pos]
        pos += 1
        value |= (byte & 0x7F) << shift
        if byte < 0x80:
            return value, pos
        shift += 7
        if shift > 63:
            raise CodecError("varint too long")


def _limit(value, maximum: int, what: str) -> int:
    if (
        not isinstance(value, int)
        or isinstance(value, bool)
        or not 0 <= value <= maximum
    ):
        raise CodecError(f"{what} {value!r} out of range 0..{maximum}")
    return value


def _encode_attribute(out: bytearray, kind: str, value, what: str) -> None:
    if kind == "u8":
        out.append(_limit(value, 0xFF, what))
    elif kind == "u16":
        _put(out, _limit(value, 0xFFFF, what))
    elif kind == "str":
        if not isinstance(value, str):
            raise CodecError(f"{what} must be text")
        data = value.encode("utf-8")
        _put(out, len(data))
        out.extend(data)
    else:
        x, y, z = value
        _put(out, _limit(x, 0xFFFF, what))
        _put(out, _limit(y, 0xFFFF, what))
        out.append(_limit(z, 0xFF, what))


def encode_tile(flags: int, house: int, zones, items) -> bytes:
    """Encode everything of a tile after its position delta."""
    _limit(flags, 0xFFFFFFFF, "tile flags")
    _limit(house, 0xFFFFFFFF, "house id")
    out = bytearray()
    _put(
        out, len(items) << 3 | (len(zones) != 0) << 2 | (house != 0) << 1 | (flags != 0)
    )
    if flags:
        _put(out, flags)
    if house:
        _put(out, house)
    if zones:
        _put(out, len(zones))
        for zone in zones:
            _put(out, _limit(zone, 0xFFFF, "tile zone id"))
    previous = -1
    for registry, depth, attrs in items:
        _limit(registry, 0xFFFFFFFF >> 1, "item registry number")
        _limit(depth, 0xFF, "item depth")
        if depth > previous + 1:
            raise CodecError("item depth skips a container level")
        previous = depth
        mask = DEPTH_BIT if depth else 0
        if attrs:
            unknown = set(attrs) - set(ATTRIBUTES)
            if unknown:
                raise CodecError(f"unknown item attributes {sorted(unknown)}")
            for name in attrs:
                mask |= ATTRIBUTES[name][0]
        _put(out, registry << 1 | (mask != 0))
        if mask:
            _put(out, mask)
            if depth:
                out.append(depth)
            for name, (bit, kind) in _ORDER:
                if mask & bit:
                    _encode_attribute(out, kind, attrs[name], name)
    return bytes(out)


def encode_sector(entries: list[tuple[int, bytes]]) -> bytes:
    """Encode ``(local_index, encode_tile(...))`` entries sorted strictly ascending."""
    out = bytearray()
    _put(out, len(entries))
    previous = -1
    for index, body in entries:
        if not previous < index < SECTOR_SIZE * SECTOR_SIZE:
            raise CodecError(
                "sector tiles must be strictly ascending inside the sector"
            )
        _put(out, index - previous - 1)
        out.extend(body)
        previous = index
    return bytes(out)


def sector_key(x: int, y: int) -> tuple[int, int]:
    return x // SECTOR_SIZE, y // SECTOR_SIZE


def encode_sector_tiles(sx: int, sy: int, tiles) -> bytes:
    """Encode the tiles ``(x, y, flags, house, zones, items)`` of sector ``(sx, sy)``."""
    entries = []
    for x, y, flags, house, zones, items in tiles:
        if sector_key(x, y) != (sx, sy):
            raise CodecError(f"tile ({x}, {y}) is outside sector ({sx}, {sy})")
        entries.append(
            (
                (y % SECTOR_SIZE) * SECTOR_SIZE + x % SECTOR_SIZE,
                encode_tile(flags, house, zones, items),
            )
        )
    return encode_sector(entries)


def _compressor() -> zstandard.ZstdCompressor:
    return zstandard.ZstdCompressor(level=ZSTD_LEVEL, write_checksum=True, threads=0)


def encode_region(z: int, rx: int, ry: int, sectors: dict[int, bytes]) -> bytes:
    """Build a region file from ``{local_index: uncompressed sector payload}``."""
    if not sectors:
        raise CodecError("a region file needs at least one sector")
    _limit(z, MAX_FLOOR, "floor")
    _limit(rx, 0xFFFF, "region x")
    _limit(ry, 0xFFFF, "region y")
    cctx = _compressor()
    frames = []
    for local in sorted(sectors):
        _limit(local, SECTORS_PER_SIDE**2 - 1, "sector index")
        frames.append((local, cctx.compress(sectors[local])))
    offset = HEADER.size + ENTRY.size * len(frames)
    out = bytearray(HEADER.pack(MAGIC, VERSION, z, rx, ry, len(frames)))
    for local, frame in frames:
        out.extend(ENTRY.pack(local, offset, len(frame)))
        offset += len(frame)
    for _, frame in frames:
        out.extend(frame)
    return bytes(out)


def parse_region(data: bytes) -> tuple[int, int, int, list[tuple[int, int, int]]]:
    """Return ``(z, rx, ry, [(local, offset, length)])`` after checking the layout."""
    if len(data) < HEADER.size:
        raise CodecError("region file shorter than its header")
    magic, version, z, rx, ry, count = HEADER.unpack_from(data)
    if magic != MAGIC or version != VERSION:
        raise CodecError("not an OTERYN_WORLD_REGION_B3/v1 file")
    if z > MAX_FLOOR or not 0 < count <= SECTORS_PER_SIDE**2:
        raise CodecError("header floor or sector count out of range")
    end_of_table = HEADER.size + ENTRY.size * count
    if len(data) < end_of_table:
        raise CodecError("sector table runs past the end of the file")
    table = []
    expected = end_of_table
    previous = -1
    for i in range(count):
        local, offset, length = ENTRY.unpack_from(data, HEADER.size + ENTRY.size * i)
        if not previous < local < SECTORS_PER_SIDE**2:
            raise CodecError("sector table must be strictly ascending within 0..63")
        if offset != expected or length == 0:
            raise CodecError("sector payloads must be non-empty and contiguous")
        table.append((local, offset, length))
        expected += length
        previous = local
    if expected != len(data):
        raise CodecError("bytes after the last sector payload")
    return z, rx, ry, table


def decode_sector(
    payload: bytes, sx: int, sy: int, registries: set[int] | None = None
) -> list[tuple]:
    """Decode a sector payload to tiles ``(x, y, flags, house, zones, items)``.

    Every registry number met is added to ``registries`` when it is given.
    """
    try:
        tiles = _decode_sector(payload, sx, sy, registries)
    except (IndexError, struct.error) as error:
        raise CodecError("sector payload is truncated") from error
    return tiles


def _decode_sector(payload: bytes, sx: int, sy: int, registries) -> list[tuple]:
    get = _get
    pos = 0
    count, pos = get(payload, pos)
    if count > SECTOR_SIZE * SECTOR_SIZE:
        raise CodecError("sector claims more tiles than it can hold")
    base_x, base_y = sx * SECTOR_SIZE, sy * SECTOR_SIZE
    tiles = []
    previous = -1
    for _ in range(count):
        delta, pos = get(payload, pos)
        index = previous + 1 + delta
        if index >= SECTOR_SIZE * SECTOR_SIZE:
            raise CodecError("tile index outside the sector")
        previous = index
        control, pos = get(payload, pos)
        flags = house = 0
        if control & 1:
            flags, pos = get(payload, pos)
            if not 0 < flags <= 0xFFFFFFFF:
                raise CodecError("tile flags out of range")
        if control & 2:
            house, pos = get(payload, pos)
            if not 0 < house <= 0xFFFFFFFF:
                raise CodecError("house id out of range")
        zones = ()
        if control & 4:
            zone_count, pos = get(payload, pos)
            if not 0 < zone_count <= 0xFFFF:
                raise CodecError("tile zone count out of range")
            zone_ids = []
            for _ in range(zone_count):
                zone, pos = get(payload, pos)
                if zone > 0xFFFF:
                    raise CodecError("tile zone id out of range")
                zone_ids.append(zone)
            zones = tuple(zone_ids)
        items = []
        depth_limit = 0
        for _ in range(control >> 3):
            word = payload[pos]
            if word < 0x80:
                pos += 1
            else:
                word, pos = get(payload, pos)
            registry = word >> 1
            depth = 0
            attrs = None
            if word & 1:
                mask, pos = get(payload, pos)
                if not mask or mask & ~KNOWN_MASK:
                    raise CodecError("item attribute mask is empty or has unknown bits")
                if mask & DEPTH_BIT:
                    depth = payload[pos]
                    pos += 1
                    if depth == 0:
                        raise CodecError("depth attribute must be non-zero")
                attrs = {}
                for name, (bit, kind) in _ORDER:
                    if not mask & bit:
                        continue
                    if kind == "u8":
                        attrs[name] = payload[pos]
                        pos += 1
                    elif kind == "u16":
                        attrs[name], pos = get(payload, pos)
                        if attrs[name] > 0xFFFF:
                            raise CodecError(f"{name} out of range")
                    elif kind == "str":
                        length, pos = get(payload, pos)
                        raw = payload[pos : pos + length]
                        if len(raw) != length:
                            raise CodecError("text runs past the payload")
                        pos += length
                        try:
                            attrs[name] = raw.decode("utf-8")
                        except UnicodeDecodeError as error:
                            raise CodecError(f"{name} is not UTF-8") from error
                    else:
                        x, pos = get(payload, pos)
                        y, pos = get(payload, pos)
                        z = payload[pos]
                        pos += 1
                        if x > 0xFFFF or y > 0xFFFF:
                            raise CodecError("teleport destination out of range")
                        attrs["teleport"] = (x, y, z)
            if not attrs:
                attrs = None
            if depth > depth_limit:
                raise CodecError("item depth skips a container level")
            depth_limit = depth + 1
            if registries is not None:
                registries.add(registry)
            items.append((registry, depth, attrs))
        tiles.append(
            (base_x + (index & 31), base_y + (index >> 5), flags, house, zones, items)
        )
    if pos != len(payload):
        raise CodecError("bytes after the last tile of a sector")
    return tiles


def decode_region(
    data: bytes, registries: set[int] | None = None
) -> tuple[int, int, int, list[tuple[int, list[tuple]]]]:
    """Decode a region file to ``(z, rx, ry, [(local_index, tiles)])``."""
    z, rx, ry, table = parse_region(data)
    dctx = zstandard.ZstdDecompressor()
    sectors = []
    for local, offset, length in table:
        try:
            payload = dctx.decompress(
                data[offset : offset + length], max_output_size=MAX_SECTOR_BYTES
            )
        except zstandard.ZstdError as error:
            raise CodecError(f"sector {local}: {error}") from error
        sx = rx * SECTORS_PER_SIDE + local % SECTORS_PER_SIDE
        sy = ry * SECTORS_PER_SIDE + local // SECTORS_PER_SIDE
        sectors.append((local, decode_sector(payload, sx, sy, registries)))
    return z, rx, ry, sectors
