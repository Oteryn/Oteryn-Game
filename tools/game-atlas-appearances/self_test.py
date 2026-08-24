#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

import export as subject


def v(value: int) -> bytes:
    if value < 0:
        value &= (1 << 64) - 1
    out = bytearray()
    while True:
        byte = value & 0x7F
        value >>= 7
        out.append(byte | (0x80 if value else 0))
        if not value:
            return bytes(out)


def field_varint(number: int, value: int) -> bytes:
    return v(number << 3) + v(value)


def field_bytes(number: int, payload: bytes) -> bytes:
    return v((number << 3) | 2) + v(len(payload)) + payload


def phase(low: int, high: int) -> bytes:
    return field_varint(1, low) + field_varint(2, high)


def animation(phases: list[tuple[int, int]], *, loop_type: int = 0, loop_count: int = 0, synchronized: bool = False) -> bytes:
    payload = field_varint(1, 0)
    payload += field_varint(2, int(synchronized))
    payload += field_varint(3, 0)
    payload += field_varint(4, loop_type)
    payload += field_varint(5, loop_count)
    for low, high in phases:
        payload += field_bytes(6, phase(low, high))
    return payload


def frame_group(group_type: int, width: int, height: int, depth: int, layers: int, sprites: list[int], phases: list[tuple[int, int]] | None = None, *, loop_type: int = 0, loop_count: int = 0) -> bytes:
    sprite = field_varint(1, width) + field_varint(2, height) + field_varint(3, depth) + field_varint(4, layers)
    for sprite_id in sprites:
        sprite += field_varint(5, sprite_id)
    if phases is not None:
        sprite += field_bytes(6, animation(phases, loop_type=loop_type, loop_count=loop_count))
    return field_varint(1, group_type) + field_varint(2, group_type) + field_bytes(3, sprite)


def appearance(appearance_id: int, groups: list[bytes]) -> bytes:
    payload = field_varint(1, appearance_id)
    for group in groups:
        payload += field_bytes(2, group)
    return payload


def source(*, bad_timing: bool = False, missing_sprite: bool = False) -> tuple[bytes, bytes]:
    catalog_last = 90 if missing_sprite else 500
    catalog = json.dumps([{"type":"sprite","file":"sheet.bin","firstspriteid":1,"lastspriteid":catalog_last,"spritetype":0}], separators=(",", ":")).encode()
    object_sprites = list(range(1, 5))
    object_phases = [(100, 200), (200, 100) if bad_timing else (300, 400)]
    obj = appearance(100, [frame_group(2, 2, 1, 1, 1, object_sprites, object_phases)])
    outfit_sprites = list(range(50, 98))
    idle = frame_group(0, 4, 3, 1, 2, outfit_sprites, [(0, 0), (100, 200)], loop_type=-1)
    moving_sprites = list(range(100, 132))
    moving = frame_group(1, 4, 1, 1, 1, moving_sprites, [(100, 100)] * 8)
    outfit = appearance(200, [idle, moving])
    top = field_bytes(1, obj) + field_bytes(2, outfit)
    return catalog, top


class ProductTests(unittest.TestCase):
    def test_object_and_outfit_programs_are_deterministic(self):
        catalog, data = source()
        first = subject.build_product_from_bytes(catalog, data)
        second = subject.build_product_from_bytes(catalog, data)
        self.assertEqual(first, second)
        self.assertEqual(first["manifest"]["statistics"]["animated_object_programs"], 1)
        self.assertEqual(first["manifest"]["statistics"]["outfit_programs"], 2)
        self.assertEqual(first["object_programs"][0]["animation"]["duration_ranges_ms"], [[100, 200], [300, 400]])
        self.assertEqual(first["outfit_programs"][0]["animation"]["effective_duration_ranges_ms"], [[100, 200], [100, 200]])
        self.assertEqual(first["outfit_programs"][0]["animation"]["loop_type"], "pingpong")

    def test_outfit_resolution_is_explicit_for_direction_addons_colors(self):
        catalog, data = source()
        product = subject.build_product_from_bytes(catalog, data)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subject.write_product(product, root)
            resolved = subject.resolve_outfit_presentation(root, look_type=200, head=0, body=19, legs=38, feet=57, addons=2)
        idle = resolved["groups"][0]
        self.assertEqual(idle["directions"], {"north":0,"east":1,"south":2,"west":3})
        self.assertEqual(idle["enabled_addon_pattern_y"], [0, 2])
        self.assertEqual(resolved["colors_rgb"]["head"], [255, 255, 255])
        self.assertTrue(resolved["outfit_presentation_id"].startswith("outfit-presentation:sha256:"))

    def test_outfit_direction_width_two_fails_closed(self):
        catalog = json.dumps([{"type":"sprite","file":"sheet.bin","firstspriteid":1,"lastspriteid":500,"spritetype":0}], separators=(",", ":")).encode()
        idle = frame_group(0, 4, 1, 1, 1, list(range(50, 54)))
        moving = frame_group(1, 2, 1, 1, 1, list(range(100, 116)), [(100, 100)] * 8)
        data = field_bytes(2, appearance(201, [idle, moving]))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subject.write_product(subject.build_product_from_bytes(catalog, data), root)
            with self.assertRaisesRegex(subject.ProductError, "unsupported direction pattern width 2"):
                subject.resolve_outfit_presentation(root, look_type=201, head=0, body=0, legs=0, feet=0, addons=0)

    def test_object_variant_ref_rejects_out_of_range_pattern(self):
        catalog, data = source()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subject.write_product(subject.build_product_from_bytes(catalog, data), root)
            ref = subject.resolve_object_animation_ref(root, 100, {"x":1,"y":0,"z":0})
            self.assertIsNotNone(ref)
            with self.assertRaises(subject.ProductError):
                subject.resolve_object_animation_ref(root, 100, {"x":2,"y":0,"z":0})

    def test_malformed_timing_rejected(self):
        catalog, data = source(bad_timing=True)
        with self.assertRaisesRegex(subject.ProductError, "malformed animation timing"):
            subject.build_product_from_bytes(catalog, data)

    def test_missing_phase_sprite_rejected(self):
        catalog, data = source(missing_sprite=True)
        with self.assertRaisesRegex(subject.ProductError, "references missing sprite"):
            subject.build_product_from_bytes(catalog, data)

    def test_unknown_look_type_rejected(self):
        catalog, data = source()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subject.write_product(subject.build_product_from_bytes(catalog, data), root)
            with self.assertRaisesRegex(subject.ProductError, "unknown lookType"):
                subject.resolve_outfit_presentation(root, look_type=999, head=0, body=0, legs=0, feet=0, addons=0)

    def test_source_digest_mismatch_rejected_before_decode(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "wrong.zip"
            path.write_bytes(b"not the accepted source")
            with self.assertRaisesRegex(subject.ProductError, "ZIP SHA-256 mismatch"):
                subject.read_exact_source_zip(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
