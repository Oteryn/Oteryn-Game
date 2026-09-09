#!/usr/bin/env python3
"""Prepare an untracked real-pixel cache for Issue #509.

The adapter verifies the exact 15.32 ZIP before reading proprietary bytes,
reuses the Game-owned appearance exporter as the semantic parser/normalizer,
and emits only local target/ cache data. The tracked repository receives no
proprietary pixels.
"""
from __future__ import annotations

from dataclasses import dataclass
import argparse
import hashlib
import importlib.util
import json
import lzma
from pathlib import Path
import shutil
import sys
import zipfile
from typing import Any, Iterable

SOURCE_ZIP_SHA256 = "1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f"
SOURCE_CATALOG_SHA256 = "35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85"
SOURCE_APPEARANCE_SHA256 = "dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075"
SOURCE_CATALOG_NAME = "assets/catalog-content.json"
SOURCE_APPEARANCE_NAME = f"assets/appearances-{SOURCE_APPEARANCE_SHA256}.dat"
SPRITES_PER_PAGE = 64
SLOT_SIZE = 64
SHEET_SIZE = 384
MAX_REQUIRED_SPRITES = 8_192
MAX_REQUIRED_SHEETS = 2_048
MANIFEST_NAME = "real-content-manifest.json"


class PrepareError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class SheetRange:
    first: int
    last: int
    sprite_type: int
    archive_name: str

    @property
    def sprite_size(self) -> tuple[int, int]:
        try:
            return ((32, 32), (32, 64), (64, 32), (64, 64))[self.sprite_type]
        except IndexError as exc:
            raise PrepareError(f"unsupported sprite type {self.sprite_type}") from exc


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_game_exporter(repo_root: Path) -> Any:
    path = repo_root / "tools" / "game-atlas-appearances" / "export.py"
    if not path.is_file():
        raise PrepareError(f"Game-owned appearance exporter missing: {path}")
    spec = importlib.util.spec_from_file_location("oteryn_game_appearance_export", path)
    if spec is None or spec.loader is None:
        raise PrepareError(f"cannot load Game-owned appearance exporter: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def verified_source_bytes(asset_zip: Path) -> tuple[bytes, bytes]:
    actual = sha256_file(asset_zip)
    if actual != SOURCE_ZIP_SHA256:
        raise PrepareError(
            f"15.32 ZIP SHA-256 mismatch: expected {SOURCE_ZIP_SHA256}, got {actual}"
        )
    # Proprietary bytes are read only after the whole-source identity is proven.
    with zipfile.ZipFile(asset_zip) as archive:
        try:
            catalog = archive.read(SOURCE_CATALOG_NAME)
            appearances = archive.read(SOURCE_APPEARANCE_NAME)
        except KeyError as exc:
            raise PrepareError(f"required 15.32 archive entry missing: {exc}") from exc
    if sha256_bytes(catalog) != SOURCE_CATALOG_SHA256:
        raise PrepareError("15.32 catalog SHA-256 mismatch")
    if sha256_bytes(appearances) != SOURCE_APPEARANCE_SHA256:
        raise PrepareError("15.32 appearance SHA-256 mismatch")
    return catalog, appearances


def catalog_ranges(catalog_bytes: bytes) -> tuple[SheetRange, ...]:
    try:
        catalog = json.loads(catalog_bytes)
    except json.JSONDecodeError as exc:
        raise PrepareError(f"invalid catalog JSON: {exc}") from exc
    if not isinstance(catalog, list):
        raise PrepareError("catalog root must be an array")
    result: list[SheetRange] = []
    for entry in catalog:
        if not isinstance(entry, dict) or entry.get("type") != "sprite":
            continue
        try:
            first = int(entry["firstspriteid"])
            last = int(entry["lastspriteid"])
            sprite_type = int(entry["spritetype"])
            file_name = str(entry["file"])
        except (KeyError, TypeError, ValueError) as exc:
            raise PrepareError("malformed sprite catalog entry") from exc
        if first < 0 or last < first or sprite_type not in (0, 1, 2, 3):
            raise PrepareError("invalid sprite catalog range")
        if not file_name or Path(file_name).is_absolute() or ".." in Path(file_name).parts:
            raise PrepareError("unsafe sprite catalog path")
        archive_name = file_name if file_name.startswith("assets/") else f"assets/{file_name}"
        result.append(SheetRange(first, last, sprite_type, archive_name))
    result.sort(key=lambda row: (row.first, row.last, row.archive_name))
    previous_last = -1
    for row in result:
        if row.first <= previous_last:
            raise PrepareError("overlapping sprite catalog ranges")
        previous_last = row.last
    if not result:
        raise PrepareError("catalog contains no sprite sheets")
    return tuple(result)


def sheet_for_sprite(ranges: tuple[SheetRange, ...], sprite_id: int) -> SheetRange:
    left, right = 0, len(ranges)
    while left < right:
        middle = (left + right) // 2
        if ranges[middle].last < sprite_id:
            left = middle + 1
        else:
            right = middle
    if left == len(ranges) or sprite_id < ranges[left].first:
        raise PrepareError(f"sprite {sprite_id} is not present in the pinned catalog")
    return ranges[left]


def decode_sheet_bytes(data: bytes) -> tuple[int, int, bytes]:
    """Decode one pinned CipSoft LZMA-wrapped 32-bit BMP to top-down RGBA."""
    position = 0
    while position < len(data) and data[position] == 0:
        position += 1
    if data[position : position + 5] != b"\x70\x0a\xfa\x80\x24":
        raise PrepareError("invalid CipSoft sprite sheet header")
    position += 5
    while position < len(data) and data[position] & 0x80:
        position += 1
    position += 1
    if position + 13 > len(data):
        raise PrepareError("truncated LZMA properties")
    properties = data[position]
    lc = properties % 9
    remainder = properties // 9
    lp, pb = remainder % 5, remainder // 5
    if pb > 4:
        raise PrepareError("invalid LZMA properties")
    dictionary = int.from_bytes(data[position + 1 : position + 5], "little")
    if dictionary <= 0 or dictionary > 1 << 30:
        raise PrepareError("invalid LZMA dictionary size")
    position += 13
    try:
        bmp = lzma.decompress(
            data[position:],
            format=lzma.FORMAT_RAW,
            filters=[
                {
                    "id": lzma.FILTER_LZMA1,
                    "dict_size": dictionary,
                    "lc": lc,
                    "lp": lp,
                    "pb": pb,
                }
            ],
        )
    except lzma.LZMAError as exc:
        raise PrepareError(f"sprite sheet LZMA decode failed: {exc}") from exc
    if len(bmp) < 54 or bmp[:2] != b"BM":
        raise PrepareError("decoded sprite sheet is not BMP")
    pixel_offset = int.from_bytes(bmp[10:14], "little")
    width = int.from_bytes(bmp[18:22], "little", signed=True)
    height = int.from_bytes(bmp[22:26], "little", signed=True)
    bits_per_pixel = int.from_bytes(bmp[28:30], "little")
    compression = int.from_bytes(bmp[30:34], "little")
    if width != SHEET_SIZE or abs(height) != SHEET_SIZE:
        raise PrepareError(f"unexpected sprite sheet dimensions {width}x{height}")
    if bits_per_pixel != 32 or compression != 0:
        raise PrepareError(
            f"unsupported BMP format: bpp={bits_per_pixel}, compression={compression}"
        )
    pixel_size = width * abs(height) * 4
    if pixel_offset < 0 or pixel_offset + pixel_size > len(bmp):
        raise PrepareError("truncated sprite pixels")
    pixels = bmp[pixel_offset : pixel_offset + pixel_size]
    rows = [
        pixels[index * width * 4 : (index + 1) * width * 4]
        for index in range(abs(height))
    ]
    if height > 0:
        rows.reverse()
    rgba = bytearray(pixel_size)
    target = 0
    for row in rows:
        for index in range(0, len(row), 4):
            blue, green, red, alpha = row[index : index + 4]
            if red == 0xFF and green == 0x00 and blue == 0xFF:
                rgba[target : target + 4] = b"\x00\x00\x00\x00"
            else:
                rgba[target : target + 4] = bytes((red, green, blue, alpha))
            target += 4
    return width, abs(height), bytes(rgba)


def extract_sprite(sheet: SheetRange, rgba: bytes, sprite_id: int) -> tuple[int, int, bytes]:
    if not sheet.first <= sprite_id <= sheet.last:
        raise PrepareError(f"sprite {sprite_id} is outside selected sheet")
    width, height = sheet.sprite_size
    columns = SHEET_SIZE // width
    rows = SHEET_SIZE // height
    offset = sprite_id - sheet.first
    if offset >= columns * rows:
        raise PrepareError(
            f"sprite {sprite_id} exceeds physical sheet capacity for {width}x{height}"
        )
    x = (offset % columns) * width
    y = (offset // columns) * height
    result = bytearray(width * height * 4)
    target = 0
    for row in range(y, y + height):
        start = (row * SHEET_SIZE + x) * 4
        end = start + width * 4
        result[target : target + width * 4] = rgba[start:end]
        target += width * 4
    return width, height, bytes(result)


def place_in_slot(width: int, height: int, rgba: bytes) -> bytes:
    if width > SLOT_SIZE or height > SLOT_SIZE or len(rgba) != width * height * 4:
        raise PrepareError("invalid sprite RGBA geometry")
    slot = bytearray(SLOT_SIZE * SLOT_SIZE * 4)
    # Source geometry occupies the lower-right of the logical 64x64 cell,
    # matching Tibia-style visual overhang anchored to the gameplay tile.
    origin_x = SLOT_SIZE - width
    origin_y = SLOT_SIZE - height
    for row in range(height):
        source = row * width * 4
        target = ((origin_y + row) * SLOT_SIZE + origin_x) * 4
        slot[target : target + width * 4] = rgba[source : source + width * 4]
    return bytes(slot)


def collect_sprite_ids(value: Any) -> set[int]:
    result: set[int] = set()
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"sprite_id", "sprite_source_id"} and isinstance(child, int) and child > 0:
                result.add(child)
            elif key in {"sprite_ids", "sprite_source_ids"} and isinstance(child, list):
                for item in child:
                    if isinstance(item, int) and item > 0:
                        result.add(item)
            result.update(collect_sprite_ids(child))
    elif isinstance(value, list):
        for child in value:
            result.update(collect_sprite_ids(child))
    return result


def deterministic_program(programs: Iterable[dict[str, Any]], *, animated: bool) -> dict[str, Any]:
    candidates = []
    for program in programs:
        if not isinstance(program, dict):
            continue
        phase_count = int(program.get("phase_count", 1))
        sprite_ids = program.get("sprite_source_ids")
        if not isinstance(sprite_ids, list) or not sprite_ids:
            continue
        if animated and phase_count <= 1:
            continue
        candidates.append(program)
    if not candidates:
        raise PrepareError("no suitable normalized appearance program")
    candidates.sort(
        key=lambda row: (
            int(row.get("appearance_source_id", 0)),
            int(row.get("frame_group", {}).get("type", 0)),
            int(row.get("frame_group", {}).get("id", 0)),
        )
    )
    return candidates[0]


def make_effect_or_missile_program(
    game_exporter: Any,
    category: str,
    appearance_bytes: bytes,
    sprite_geometry: dict[int, tuple[int, int]],
) -> dict[str, Any]:
    decoded = game_exporter.decode_category(appearance_bytes, category)
    programs = [
        game_exporter._frame_program(category, appearance.appearance_id, frame, sprite_geometry)
        for appearance in decoded
        for frame in appearance.frame_groups
    ]
    return deterministic_program(programs, animated=False)


def build_required_set(
    atlas_slice: Any,
    outfit_program: dict[str, Any],
    effect_program: dict[str, Any],
    missile_program: dict[str, Any],
) -> list[int]:
    required = collect_sprite_ids(atlas_slice)
    required.update(collect_sprite_ids(outfit_program))
    required.update(collect_sprite_ids(effect_program))
    required.update(collect_sprite_ids(missile_program))
    if not required:
        raise PrepareError("real-content selection resolved no source sprites")
    if len(required) > MAX_REQUIRED_SPRITES:
        raise PrepareError(
            f"required sprite count {len(required)} exceeds cap {MAX_REQUIRED_SPRITES}"
        )
    return sorted(required)


def emit_pages(
    asset_zip: Path,
    output: Path,
    ranges: tuple[SheetRange, ...],
    required_ids: list[int],
) -> tuple[list[dict[str, Any]], int]:
    page_ids = sorted({sprite_id // SPRITES_PER_PAGE for sprite_id in required_ids})
    pages_dir = output / "pages"
    pages_dir.mkdir(parents=True, exist_ok=True)
    required_set = set(required_ids)
    sheet_cache: dict[str, bytes] = {}
    sheet_count = 0
    pages: list[dict[str, Any]] = []
    with zipfile.ZipFile(asset_zip) as archive:
        for page_id in page_ids:
            page = bytearray(SPRITES_PER_PAGE * SLOT_SIZE * SLOT_SIZE * 4)
            members: list[dict[str, Any]] = []
            for local in range(SPRITES_PER_PAGE):
                sprite_id = page_id * SPRITES_PER_PAGE + local
                if sprite_id not in required_set:
                    continue
                sheet = sheet_for_sprite(ranges, sprite_id)
                decoded = sheet_cache.get(sheet.archive_name)
                if decoded is None:
                    if sheet_count >= MAX_REQUIRED_SHEETS:
                        raise PrepareError("required sprite sheet count exceeds cap")
                    try:
                        compressed = archive.read(sheet.archive_name)
                    except KeyError as exc:
                        raise PrepareError(
                            f"catalog sprite sheet missing from ZIP: {sheet.archive_name}"
                        ) from exc
                    _width, _height, decoded = decode_sheet_bytes(compressed)
                    sheet_cache[sheet.archive_name] = decoded
                    sheet_count += 1
                width, height, sprite = extract_sprite(sheet, decoded, sprite_id)
                slot = place_in_slot(width, height, sprite)
                slot_bytes = SLOT_SIZE * SLOT_SIZE * 4
                start = local * slot_bytes
                page[start : start + slot_bytes] = slot
                members.append(
                    {
                        "sprite_source_id": sprite_id,
                        "source_geometry": [width, height],
                        "slot": local,
                    }
                )
            path = pages_dir / f"page-{page_id:06d}.rgba"
            path.write_bytes(page)
            pages.append(
                {
                    "page_id": page_id,
                    "path": f"pages/{path.name}",
                    "sha256": sha256_bytes(page),
                    "byte_length": len(page),
                    "members": members,
                }
            )
    return pages, sheet_count


def prepare(
    repo_root: Path,
    asset_zip: Path,
    atlas_slice_path: Path,
    output: Path,
) -> dict[str, Any]:
    catalog_bytes, appearance_bytes = verified_source_bytes(asset_zip)
    game_exporter = load_game_exporter(repo_root)

    # This call is the Game-owned normalized semantics authority for object/outfit.
    normalized = game_exporter.build_product_from_bytes(catalog_bytes, appearance_bytes)
    outfit_program = deterministic_program(normalized["outfit_programs"], animated=True)

    source_ranges = game_exporter._catalog_sprite_ranges(catalog_bytes)
    sprite_geometry = game_exporter._sprite_geometry_index(source_ranges)
    effect_program = make_effect_or_missile_program(
        game_exporter, "effect", appearance_bytes, sprite_geometry
    )
    missile_program = make_effect_or_missile_program(
        game_exporter, "missile", appearance_bytes, sprite_geometry
    )

    ranges = catalog_ranges(catalog_bytes)
    try:
        atlas_slice = json.loads(atlas_slice_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise PrepareError(f"invalid Atlas slice JSON: {exc}") from exc

    required_ids = build_required_set(
        atlas_slice, outfit_program, effect_program, missile_program
    )
    if output.exists():
        shutil.rmtree(output)
    output.mkdir(parents=True, exist_ok=True)
    pages, decoded_sheet_count = emit_pages(asset_zip, output, ranges, required_ids)

    manifest: dict[str, Any] = {
        "schema": "oteryn-world-vfx-real-content-cache-v1",
        "source": {
            "label": "15.32",
            "zip_sha256": SOURCE_ZIP_SHA256,
            "catalog_sha256": SOURCE_CATALOG_SHA256,
            "appearance_sha256": SOURCE_APPEARANCE_SHA256,
        },
        "semantic_authority": {
            "path": "tools/game-atlas-appearances/export.py",
            "contract_id": normalized["manifest"]["contract_id"],
            "semantic_revision": normalized["manifest"]["semantic_revision"],
            "product_root": normalized["manifest"]["product_root"],
        },
        "atlas_slice_sha256": sha256_file(atlas_slice_path),
        "sprite_page": {
            "sprites_per_page": SPRITES_PER_PAGE,
            "slot_size": SLOT_SIZE,
            "page_count": len(pages),
            "required_sprite_count": len(required_ids),
            "decoded_sheet_count": decoded_sheet_count,
        },
        "bindings": {
            "outfit": outfit_program,
            "effect": effect_program,
            "missile": missile_program,
        },
        "required_sprite_ids": required_ids,
        "pages": pages,
        "proprietary_pixels_committed": False,
    }
    manifest_bytes = (
        json.dumps(manifest, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")
    (output / MANIFEST_NAME).write_bytes(manifest_bytes)
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--asset-zip", required=True, type=Path)
    parser.add_argument("--atlas-slice", required=True, type=Path)
    parser.add_argument(
        "--output",
        type=Path,
        help="untracked output directory (defaults inside this experiment)",
    )
    args = parser.parse_args()
    experiment_root = Path(__file__).resolve().parent
    output = args.output or experiment_root / "target" / "real-content-cache"
    repo_root = Path(__file__).resolve().parents[2]
    try:
        manifest = prepare(repo_root, args.asset_zip, args.atlas_slice, output)
    except (PrepareError, OSError, zipfile.BadZipFile) as exc:
        print(f"prepare-real-content-error: {exc}", file=sys.stderr)
        return 1
    summary = {
        "manifest": str(output / MANIFEST_NAME),
        "required_sprite_count": manifest["sprite_page"]["required_sprite_count"],
        "page_count": manifest["sprite_page"]["page_count"],
        "decoded_sheet_count": manifest["sprite_page"]["decoded_sheet_count"],
        "source_zip_sha256": manifest["source"]["zip_sha256"],
        "proprietary_pixels_committed": False,
    }
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
