#!/usr/bin/env python3
"""No-network tests for validate_achievements.py."""

from __future__ import annotations

import copy
import importlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT.parent / "quest-authoring"))
v = importlib.import_module("validate_achievements")

SAMPLE = json.loads(
    (ROOT / "synthetic-valid-achievement.json").read_text(encoding="utf-8")
)


def variant(**changes: object) -> dict:
    record = copy.deepcopy(SAMPLE)
    record.update(changes)
    return record


class ValidateAchievementsTest(unittest.TestCase):
    def test_sample_is_valid(self) -> None:
        self.assertEqual(v.validate([SAMPLE])["invalid"], [])
        self.assertEqual(v.main([str(ROOT / "synthetic-valid-achievement.json")]), 0)

    def test_renamed_record_keeps_its_key(self) -> None:
        self.assertEqual(v.allocate_key("Allow Cookies?"), SAMPLE["identity"]["key"])
        renamed = variant(name="Allow Biscuits?")
        self.assertEqual(v.validate([renamed])["invalid"], [])

    def test_points_must_lie_in_grade_range(self) -> None:
        self.assertEqual(
            v.validate([variant(points=4)])["invalid"][0]["errors"],
            ["points 4 outside the grade 1 range"],
        )
        self.assertEqual(v.validate([variant(grade=4, points=10)])["invalid"], [])

    def test_retired_has_zero_points(self) -> None:
        self.assertEqual(v.validate([variant(retired=True, points=0)])["invalid"], [])
        self.assertEqual(
            v.validate([variant(retired=True, points=2)])["invalid"][0]["errors"],
            ["a retired achievement has 0 points"],
        )
        self.assertEqual(
            v.validate([variant(points=0)])["invalid"][0]["errors"],
            ["points 0 outside the grade 1 range"],
        )

    def test_duplicate_key_rejected(self) -> None:
        report = v.validate([SAMPLE, variant(description="Other text.")])
        self.assertEqual(
            report["invalid"],
            [
                {
                    "index": 1,
                    "key": SAMPLE["identity"]["key"],
                    "errors": ["duplicate key, first at record 0"],
                }
            ],
        )

    def test_schema_rejects_shape(self) -> None:
        cases = [
            variant(grade=5),
            variant(secret="no"),
            variant(extra=1),
            variant(provenance={}),
            variant(identity="oteryn:achievement/allow_cookies"),
            variant(
                identity={
                    **SAMPLE["identity"],
                    "key": "canary:achievement/allow_cookies",
                }
            ),
        ]
        for record in cases:
            errors = v.validate([record])["invalid"][0]["errors"]
            self.assertTrue(
                errors and all(e.startswith("schema:") for e in errors), errors
            )

    def test_slug_matches_quest_tooling(self) -> None:
        import ots_chests

        for name in (
            "Allow Cookies?",
            "A Study in Scarlett",
            "Zzztill Zzztanding!",
            "Aye-aye, Captain!",
        ):
            self.assertEqual(v.slug(name), ots_chests.slug(name))


if __name__ == "__main__":
    unittest.main()
