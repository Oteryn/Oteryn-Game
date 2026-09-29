"""Read the official Tibia client `map` file and its `subarea-*` mask images.

Only structure proven against the committed 15.30 files is decoded; anything else fails
closed. The formats are undocumented, so every rule below was checked against the data
(see README, "Region source"):

`map-<sha256>.dat` is a raw protobuf message. Top level fields:
- 1 (repeated): an area: 1 id, 2 name, 3 kind (1 = has children, 2 = child), 4 (repeated)
  child area ids, 5 anchor position, 6 flag, 7 (repeated) secondary name.
- 2 (repeated): a marker (name, position, icon). Counted, not decoded further.
- 3 (repeated): an image: 1 image type (0 = subarea mask, 1 = satellite, 2 = minimap),
  2 position of the image's top-left tile, 3 file name, 4 width, 5 height, 6 area id
  (type 0 only), 7 scale (types 1 and 2 only).
- 4, 5: a position each (declared minimum and maximum corner).

A subarea image is a Tibia-framed LZMA stream holding a 32-bit BMP of `width * height`
pixels, one pixel per tile, anchored at the image position; a non-zero pixel is a tile of
the area. Framing: leading zero bytes, two constant bytes (70 0A), two varints, then a
standard LZMA-alone header and stream. The file name ends in the sha256 of the decoded BMP.
"""

from __future__ import annotations

import hashlib
import lzma
import struct
from dataclasses import dataclass, field


class ClientMapError(Exception):
    pass


def varint(buf: bytes, i: int) -> tuple[int, int]:
    value = shift = 0
    while True:
        if i >= len(buf) or shift > 63:
            raise ClientMapError("truncated or oversized varint")
        byte = buf[i]
        i += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, i


def fields(buf: bytes) -> list[tuple[int, int, int | bytes]]:
    """Protobuf wire fields as (number, wire type, value); varint or length-delimited only."""
    out, i = [], 0
    while i < len(buf):
        key, i = varint(buf, i)
        number, wire = key >> 3, key & 7
        if number == 0:
            raise ClientMapError("field number 0")
        if wire == 0:
            value, i = varint(buf, i)
        elif wire == 2:
            size, i = varint(buf, i)
            if i + size > len(buf):
                raise ClientMapError("truncated length-delimited field")
            value, i = buf[i : i + size], i + size
        elif wire == 1:
            value, i = buf[i : i + 8], i + 8
        else:
            raise ClientMapError(f"unsupported wire type {wire}")
        out.append((number, wire, value))
    return out


def collect(buf: bytes, allowed: set[int], what: str) -> dict[int, list]:
    grouped: dict[int, list] = {}
    for number, wire, value in fields(buf):
        if number not in allowed:
            raise ClientMapError(f"{what}: unknown field {number}")
        grouped.setdefault(number, []).append((wire, value))
    return grouped


def single(grouped: dict, number: int, wire: int, what: str, required=True):
    rows = grouped.get(number, [])
    if len(rows) > 1 or (required and not rows):
        raise ClientMapError(f"{what}: field {number} must occur once")
    if not rows:
        return None
    if rows[0][0] != wire:
        raise ClientMapError(f"{what}: field {number} has the wrong wire type")
    return rows[0][1]


def repeated(grouped: dict, number: int, wire: int, what: str) -> list:
    rows = grouped.get(number, [])
    if any(row[0] != wire for row in rows):
        raise ClientMapError(f"{what}: field {number} has the wrong wire type")
    return [row[1] for row in rows]


def position(buf: bytes, what: str) -> tuple[int, int, int]:
    grouped = collect(buf, {1, 2, 3}, what)
    return tuple(single(grouped, n, 0, what) for n in (1, 2, 3))  # type: ignore[return-value]


def text(raw: bytes, what: str) -> str:
    try:
        return raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ClientMapError(f"{what}: not UTF-8") from error


@dataclass
class Area:
    id: int
    name: str
    kind: int
    children: list[int]
    anchor: tuple[int, int, int] | None
    flag: int | None
    secondary_names: list[str]


@dataclass
class SubareaImage:
    area: int
    x: int
    y: int
    z: int
    width: int
    height: int
    file: str


@dataclass
class MapFile:
    areas: list[Area] = field(default_factory=list)
    subareas: list[SubareaImage] = field(default_factory=list)
    markers: int = 0
    other_images: dict[str, int] = field(default_factory=dict)
    minimum: tuple[int, int, int] | None = None
    maximum: tuple[int, int, int] | None = None


def read_map(data: bytes) -> MapFile:
    facts = MapFile()
    for number, wire, value in fields(data):
        if wire != 2 and number in (1, 2, 3, 4, 5):
            raise ClientMapError(f"map: field {number} has the wrong wire type")
        if number == 1:
            facts.areas.append(read_area(value))
        elif number == 2:
            facts.markers += 1
        elif number == 3:
            image = read_image(value)
            if isinstance(image, SubareaImage):
                facts.subareas.append(image)
            else:
                facts.other_images[image] = facts.other_images.get(image, 0) + 1
        elif number in (4, 5):
            corner = position(value, f"map field {number}")
            if number == 4:
                facts.minimum = corner
            else:
                facts.maximum = corner
        else:
            raise ClientMapError(f"map: unknown field {number}")
    return facts


def read_area(buf: bytes) -> Area:
    grouped = collect(buf, {1, 2, 3, 4, 5, 6, 7}, "area")
    anchor = single(grouped, 5, 2, "area", required=False)
    return Area(
        id=single(grouped, 1, 0, "area"),
        name=text(single(grouped, 2, 2, "area"), "area name"),
        kind=single(grouped, 3, 0, "area"),
        children=repeated(grouped, 4, 0, "area"),
        anchor=position(anchor, "area anchor") if anchor is not None else None,
        flag=single(grouped, 6, 0, "area", required=False),
        secondary_names=[
            text(raw, "area secondary name") for raw in repeated(grouped, 7, 2, "area")
        ],
    )


IMAGE_KINDS = {1: "satellite", 2: "minimap"}


def read_image(buf: bytes) -> SubareaImage | str:
    """A subarea mask image, or the kind name of a satellite/minimap image (counted only)."""
    grouped = collect(buf, {1, 2, 3, 4, 5, 6, 7}, "image")
    kind = single(grouped, 1, 0, "image")
    if kind in IMAGE_KINDS:
        return IMAGE_KINDS[kind]
    if kind != 0:
        raise ClientMapError(f"image: unknown type {kind}")
    if 7 in grouped:
        raise ClientMapError("subarea image: unexpected scale")
    x, y, z = position(single(grouped, 2, 2, "image"), "image position")
    image = SubareaImage(
        area=single(grouped, 6, 0, "subarea image"),
        x=x,
        y=y,
        z=z,
        width=single(grouped, 4, 0, "image"),
        height=single(grouped, 5, 0, "image"),
        file=text(single(grouped, 3, 2, "image"), "image file"),
    )
    if not image.file.startswith(f"subarea-{image.area:04d}-"):
        raise ClientMapError(f"{image.file}: name does not match area {image.area}")
    return image


@dataclass
class Mask:
    """Tile membership of a subarea: `rows[r][c]` is 1 when tile (x + c, y + r) belongs."""

    x: int
    y: int
    z: int
    rows: list[bytes]

    @property
    def width(self) -> int:
        return len(self.rows[0])

    @property
    def tile_count(self) -> int:
        return sum(sum(row) for row in self.rows)

    def bounds(self) -> tuple[int, int, int, int]:
        """Tight (min_x, min_y, max_x, max_y) over the set tiles."""
        used = [r for r, row in enumerate(self.rows) if any(row)]
        columns = [c for row in self.rows for c, cell in enumerate(row) if cell]
        return (
            self.x + min(columns),
            self.y + used[0],
            self.x + max(columns),
            self.y + used[-1],
        )

    def contains(self, x: int, y: int, z: int) -> bool:
        column, row = x - self.x, y - self.y
        return (
            z == self.z
            and 0 <= row < len(self.rows)
            and 0 <= column < self.width
            and bool(self.rows[row][column])
        )


def unframe(raw: bytes) -> bytes:
    """The LZMA-alone payload of a Tibia-framed file, decompressed."""
    i = 0
    while i < len(raw) and raw[i] == 0:
        i += 1
    if raw[i : i + 2] != b"\x70\x0a":
        raise ClientMapError("unknown file framing")
    i += 2
    for _ in range(2):
        while i < len(raw) and raw[i] & 0x80:
            i += 1
        i += 1
    if i + 13 > len(raw):
        raise ClientMapError("truncated LZMA header")
    # Props and dictionary size, then a stream that is read until its own end.
    decoder = lzma.LZMADecompressor(format=lzma.FORMAT_ALONE)
    try:
        return decoder.decompress(
            raw[i : i + 5] + (2**64 - 1).to_bytes(8, "little") + raw[i + 13 :]
        )
    except lzma.LZMAError as error:
        raise ClientMapError(f"LZMA stream: {error}") from error


def read_mask(raw: bytes, image: SubareaImage) -> Mask:
    """The mask of `image`. Its file name ends in the sha256 of the decoded BMP; that is checked."""
    bmp = unframe(raw)
    if not image.file.endswith(f"-{hashlib.sha256(bmp).hexdigest()}.bmp.lzma"):
        raise ClientMapError(f"{image.file}: decoded BMP sha256 differs from the name")
    if len(bmp) < 54 or bmp[:2] != b"BM":
        raise ClientMapError(f"{image.file}: not a BMP")
    size, offset = struct.unpack("<I4xI", bmp[2:14])
    width, height, planes, bits, compression = struct.unpack("<iiHHI", bmp[18:34])
    top_down = height < 0
    height = abs(height)
    if (
        size != len(bmp)
        or planes != 1
        or bits != 32
        or compression not in (0, 3)
        or (width, height) != (image.width, image.height)
        or offset + width * height * 4 != len(bmp)
    ):
        raise ClientMapError(f"{image.file}: BMP shape differs from the map entry")
    rows = [
        bytes(
            int(bmp[o + 4 * c : o + 4 * c + 4] != b"\x00\x00\x00\x00")
            for c in range(width)
        )
        for o in range(offset, len(bmp), width * 4)
    ]
    if not top_down:
        rows.reverse()
    return Mask(image.x, image.y, image.z, rows)
