#!/usr/bin/env python3
"""Pure self-tests for the bounded Thais fixture producer helpers."""

from __future__ import annotations

import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest

from public_policy import PublicPolicyError, validate_tile_shape


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("dyn_atlas_thais_export", HERE / "export.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("unable to load exporter module")
EXPORT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EXPORT)


class ExportHelperTests(unittest.TestCase):
    def test_canonical_json_is_sorted_and_lf_terminated(self) -> None:
        self.assertEqual(EXPORT._canonical_json_bytes({"z": 1, "a": 2}), b'{"a":2,"z":1}\n')

    def test_stable_ids_are_deterministic_and_domain_separated(self) -> None:
        first = EXPORT._stable_id("presentation", 1, 2, -7, 0, 100)
        second = EXPORT._stable_id("presentation", 1, 2, -7, 0, 100)
        other = EXPORT._stable_id("tile", 1, 2, -7, 0, 100)
        self.assertEqual(first, second)
        self.assertNotEqual(first, other)
        self.assertTrue(first.startswith("presentation:"))

    def test_visual_coverage_uses_south_east_anchor(self) -> None:
        self.assertEqual(EXPORT._coverage(32, 32), [{"dx_tiles": 0, "dy_tiles": 0}])
        self.assertEqual(
            EXPORT._coverage(64, 32),
            [{"dx_tiles": -1, "dy_tiles": 0}, {"dx_tiles": 0, "dy_tiles": 0}],
        )
        self.assertEqual(
            EXPORT._coverage(32, 64),
            [{"dx_tiles": 0, "dy_tiles": -1}, {"dx_tiles": 0, "dy_tiles": 0}],
        )

    def test_invalid_visual_dimensions_fail_closed(self) -> None:
        with self.assertRaises(EXPORT.ExportError):
            EXPORT._coverage(48, 32)
        with self.assertRaises(EXPORT.ExportError):
            EXPORT._coverage(0, 32)

    def test_stackable_pattern_buckets(self) -> None:
        appearance = SimpleNamespace(appearance_id=10, stackable=True, hangable=False, splash=False, fluid_container=False)
        frame = SimpleNamespace(pattern_width=4, pattern_height=2, pattern_depth=1)
        expected = {
            0: (0, 0, 0),
            1: (0, 0, 0),
            4: (3, 0, 0),
            5: (0, 1, 0),
            10: (1, 1, 0),
            25: (2, 1, 0),
            50: (3, 1, 0),
        }
        for count, patterns in expected.items():
            item = SimpleNamespace(subtype=count)
            self.assertEqual(EXPORT._item_patterns(appearance, frame, item, 32280, 32155, 7, False, False), patterns)

    def test_hangable_hook_selection(self) -> None:
        appearance = SimpleNamespace(appearance_id=11, stackable=False, hangable=True, splash=False, fluid_container=False)
        frame = SimpleNamespace(pattern_width=3, pattern_height=1, pattern_depth=1)
        item = SimpleNamespace(subtype=None)
        self.assertEqual(EXPORT._item_patterns(appearance, frame, item, 0, 0, 0, True, False), (1, 0, 0))
        self.assertEqual(EXPORT._item_patterns(appearance, frame, item, 0, 0, 0, False, True), (2, 0, 0))
        self.assertEqual(EXPORT._item_patterns(appearance, frame, item, 0, 0, 0, False, False), (0, 0, 0))

    def test_fluid_pattern_selection_is_bounded(self) -> None:
        appearance = SimpleNamespace(appearance_id=12, stackable=False, hangable=False, splash=True, fluid_container=False)
        frame = SimpleNamespace(pattern_width=4, pattern_height=3, pattern_depth=1)
        item = SimpleNamespace(subtype=10)
        self.assertEqual(EXPORT._item_patterns(appearance, frame, item, 0, 0, 0, False, False), (3, 1, 0))

    def test_default_deny_public_policy_rejects_unallowlisted_field(self) -> None:
        tile = {
            "position": {"floor": -7, "x": 32280, "y": 32155},
            "presentation": [],
            "record_type": "tile",
            "source_position": {"legacy_x": 32280, "legacy_y": 32155, "legacy_z": 7},
            "tile_record_id": "tile:test",
        }
        validate_tile_shape(tile)
        tile["text"] = "must never leak"
        with self.assertRaises(PublicPolicyError):
            validate_tile_shape(tile)


if __name__ == "__main__":
    unittest.main(verbosity=2)
