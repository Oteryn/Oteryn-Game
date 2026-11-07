#!/usr/bin/env python3
"""Self-test for the `apps/` synthetic-key rule of item_key_references.py (A12 §5)."""

from __future__ import annotations

import unittest

from item_key_references import MAX_APPEARANCE_ID, is_dangling_tibia


def key(tibia_id: int) -> str:
    return f"oteryn:item.tibia.i{tibia_id}"


class AppsSyntheticKeyTest(unittest.TestCase):
    def test_apps_key_above_appearance_range_is_synthetic(self) -> None:
        for tibia_id in (MAX_APPEARANCE_ID + 1, 70000):
            self.assertFalse(
                is_dangling_tibia("apps", key(tibia_id), set(), set(), set())
            )

    def test_apps_key_in_range_and_not_admitted_dangles(self) -> None:
        for tibia_id in (1, MAX_APPEARANCE_ID):
            self.assertTrue(is_dangling_tibia("apps", key(tibia_id), set(), set(), {2}))

    def test_apps_key_in_range_and_admitted_passes(self) -> None:
        self.assertFalse(is_dangling_tibia("apps", key(2), set(), set(), {2}))

    def test_content_and_imports_rules_are_unchanged(self) -> None:
        for base in ("content", "imports"):
            self.assertTrue(is_dangling_tibia(base, key(70000), set(), set(), {70000}))
        self.assertFalse(
            is_dangling_tibia("imports", key(70000), set(), {key(70000)}, set())
        )
        self.assertFalse(is_dangling_tibia("content", key(5), {key(5)}, set(), set()))


if __name__ == "__main__":
    unittest.main()
