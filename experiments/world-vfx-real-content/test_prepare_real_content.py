#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import json
import lzma
from pathlib import Path
import sys
import tempfile
import unittest


MODULE_PATH = Path(__file__).resolve().with_name("prepare-real-content.py")
SPEC = importlib.util.spec_from_file_location("prepare_real_content", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load prepare-real-content.py")
M = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = M
SPEC.loader.exec_module(M)


class PrepareRealContentTests(unittest.TestCase):
    def test_collect_sprite_ids_walks_atlas_and_program_shape(self) -> None:
        value = {
            "tiles": [{"presentations": [{"primitives": [{"sprite_id": 7}]}]}],
            "program": {"sprite_source_ids": [11, 12, 11]},
            "ignored": {"sprite_id": 0},
        }
        self.assertEqual(M.collect_sprite_ids(value), {7, 11, 12})

    def test_catalog_ranges_reject_path_traversal(self) -> None:
        raw = json.dumps(
            [
                {
                    "type": "sprite",
                    "firstspriteid": 1,
                    "lastspriteid": 2,
                    "spritetype": 0,
                    "file": "../secret.bmp.lzma",
                }
            ]
        ).encode()
        with self.assertRaises(M.PrepareError):
            M.catalog_ranges(raw)

    def test_sheet_lookup_uses_non_overlapping_ranges(self) -> None:
        ranges = (
            M.SheetRange(1, 4, 0, "assets/a.bmp.lzma"),
            M.SheetRange(8, 12, 3, "assets/b.bmp.lzma"),
        )
        self.assertEqual(M.sheet_for_sprite(ranges, 10).archive_name, "assets/b.bmp.lzma")
        with self.assertRaises(M.PrepareError):
            M.sheet_for_sprite(ranges, 6)

    def test_place_in_slot_bottom_right_anchors_small_sprite(self) -> None:
        source = bytes((1, 2, 3, 4)) * (32 * 32)
        slot = M.place_in_slot(32, 32, source)
        self.assertEqual(len(slot), 64 * 64 * 4)
        self.assertEqual(slot[:4], b"\x00\x00\x00\x00")
        first = ((32 * 64) + 32) * 4
        self.assertEqual(slot[first : first + 4], b"\x01\x02\x03\x04")
        self.assertEqual(slot[-4:], b"\x01\x02\x03\x04")

    def test_digest_mismatch_fails_before_zip_parse(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "not-a-zip.bin"
            path.write_bytes(b"not a zip")
            with self.assertRaisesRegex(M.PrepareError, "ZIP SHA-256 mismatch"):
                M.verified_source_bytes(path)

    def test_decode_sheet_and_extract_sprite_from_synthetic_wrapper(self) -> None:
        bmp = synthetic_bmp()
        filters = [
            {
                "id": lzma.FILTER_LZMA1,
                "dict_size": 1 << 20,
                "lc": 3,
                "lp": 0,
                "pb": 2,
            }
        ]
        compressed = lzma.compress(bmp, format=lzma.FORMAT_RAW, filters=filters)
        properties = 3 + 9 * (0 + 5 * 2)
        wrapper = (
            b"\x70\x0a\xfa\x80\x24"
            + b"\x00"
            + bytes((properties,))
            + (1 << 20).to_bytes(4, "little")
            + len(bmp).to_bytes(8, "little")
            + compressed
        )
        width, height, rgba = M.decode_sheet_bytes(wrapper)
        self.assertEqual((width, height), (384, 384))
        self.assertEqual(rgba[:4], b"\x03\x02\x01\x04")

        sheet = M.SheetRange(100, 243, 0, "assets/test.bmp.lzma")
        sw, sh, sprite = M.extract_sprite(sheet, rgba, 100)
        self.assertEqual((sw, sh), (32, 32))
        self.assertEqual(sprite[:4], b"\x03\x02\x01\x04")

    def test_decode_sheet_keys_magenta_transparent(self) -> None:
        bmp = bytearray(synthetic_bmp())
        bmp[54:58] = bytes((255, 0, 255, 255))
        filters = [
            {
                "id": lzma.FILTER_LZMA1,
                "dict_size": 1 << 20,
                "lc": 3,
                "lp": 0,
                "pb": 2,
            }
        ]
        compressed = lzma.compress(bytes(bmp), format=lzma.FORMAT_RAW, filters=filters)
        properties = 3 + 9 * (0 + 5 * 2)
        wrapper = (
            b"\x70\x0a\xfa\x80\x24"
            + b"\x00"
            + bytes((properties,))
            + (1 << 20).to_bytes(4, "little")
            + len(bmp).to_bytes(8, "little")
            + compressed
        )
        _width, _height, rgba = M.decode_sheet_bytes(wrapper)
        self.assertEqual(rgba[:4], b"\x00\x00\x00\x00")

    def test_vfx_selection_prefers_deterministic_infinite_animation(self) -> None:
        counted = {
            "appearance_source_id": 1,
            "frame_group": {"type": 0, "id": 0},
            "phase_count": 6,
            "sprite_source_ids": [101],
            "animation": {"loop_type": "counted", "random_start_phase": False},
        }
        static = {
            "appearance_source_id": 2,
            "frame_group": {"type": 0, "id": 0},
            "phase_count": 1,
            "sprite_source_ids": [102],
            "animation": None,
        }
        infinite = {
            "appearance_source_id": 78,
            "frame_group": {"type": 0, "id": 0},
            "phase_count": 8,
            "sprite_source_ids": [103],
            "animation": {"loop_type": "infinite", "random_start_phase": False},
        }
        selected = M.deterministic_vfx_program([counted, static, infinite])
        self.assertEqual(selected["appearance_source_id"], 78)

    def test_vfx_selection_falls_back_to_static_when_needed(self) -> None:
        counted = {
            "appearance_source_id": 1,
            "frame_group": {"type": 0, "id": 0},
            "phase_count": 6,
            "sprite_source_ids": [101],
            "animation": {"loop_type": "counted", "random_start_phase": False},
        }
        static = {
            "appearance_source_id": 9,
            "frame_group": {"type": 0, "id": 0},
            "phase_count": 1,
            "sprite_source_ids": [102],
            "animation": None,
        }
        selected = M.deterministic_vfx_program([counted, static])
        self.assertEqual(selected["appearance_source_id"], 9)

    def test_decode_sheet_accepts_exact_bitfield_masks(self) -> None:
        bmp = synthetic_bitfields_bmp()
        wrapper = synthetic_wrapper(bmp)
        width, height, rgba = M.decode_sheet_bytes(wrapper)
        self.assertEqual((width, height), (384, 384))
        self.assertEqual(rgba[:4], b"\x03\x02\x01\x04")

    def test_decode_sheet_rejects_unknown_bitfield_masks(self) -> None:
        bmp = bytearray(synthetic_bitfields_bmp())
        bmp[54:58] = (0x000000FF).to_bytes(4, "little")
        with self.assertRaisesRegex(M.PrepareError, "BI_BITFIELDS channel masks"):
            M.decode_sheet_bytes(synthetic_wrapper(bytes(bmp)))


def synthetic_wrapper(bmp: bytes) -> bytes:
    filters = [
        {
            "id": lzma.FILTER_LZMA1,
            "dict_size": 1 << 20,
            "lc": 3,
            "lp": 0,
            "pb": 2,
        }
    ]
    compressed = lzma.compress(bmp, format=lzma.FORMAT_RAW, filters=filters)
    properties = 3 + 9 * (0 + 5 * 2)
    return (
        b"\x70\x0a\xfa\x80\x24"
        + b"\x00"
        + bytes((properties,))
        + (1 << 20).to_bytes(4, "little")
        + len(bmp).to_bytes(8, "little")
        + compressed
    )


def synthetic_bitfields_bmp() -> bytes:
    width = height = 384
    pixel_bytes = bytes((1, 2, 3, 4)) * (width * height)
    pixel_offset = 122
    file_size = pixel_offset + len(pixel_bytes)
    header = bytearray(pixel_offset)
    header[0:2] = b"BM"
    header[2:6] = file_size.to_bytes(4, "little")
    header[10:14] = pixel_offset.to_bytes(4, "little")
    header[14:18] = (108).to_bytes(4, "little")
    header[18:22] = width.to_bytes(4, "little", signed=True)
    header[22:26] = (-height).to_bytes(4, "little", signed=True)
    header[26:28] = (1).to_bytes(2, "little")
    header[28:30] = (32).to_bytes(2, "little")
    header[30:34] = (3).to_bytes(4, "little")
    header[34:38] = len(pixel_bytes).to_bytes(4, "little")
    for offset, mask in zip(
        (54, 58, 62, 66),
        (0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000),
        strict=True,
    ):
        header[offset : offset + 4] = mask.to_bytes(4, "little")
    return bytes(header) + pixel_bytes


def synthetic_bmp() -> bytes:
    width = height = 384
    pixel_bytes = bytes((1, 2, 3, 4)) * (width * height)
    pixel_offset = 54
    file_size = pixel_offset + len(pixel_bytes)
    header = bytearray(pixel_offset)
    header[0:2] = b"BM"
    header[2:6] = file_size.to_bytes(4, "little")
    header[10:14] = pixel_offset.to_bytes(4, "little")
    header[14:18] = (40).to_bytes(4, "little")
    header[18:22] = width.to_bytes(4, "little", signed=True)
    # Negative height means top-down, avoiding row inversion in the assertion.
    header[22:26] = (-height).to_bytes(4, "little", signed=True)
    header[26:28] = (1).to_bytes(2, "little")
    header[28:30] = (32).to_bytes(2, "little")
    header[30:34] = (0).to_bytes(4, "little")
    header[34:38] = len(pixel_bytes).to_bytes(4, "little")
    return bytes(header) + pixel_bytes


if __name__ == "__main__":
    unittest.main()
