#!/usr/bin/env python3
"""No-network tests for validate_achievements.py."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT.parent / "quest-authoring"))
import validate_achievements as v  # noqa: E402

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

    def test_key_must_be_slug_of_name(self) -> None:
        errors = v.validate([variant(name="Allow Biscuits?")])["invalid"][0]["errors"]
        self.assertEqual(errors, ["key must be oteryn:achievement/allow_biscuits"])

    def test_points_must_lie_in_grade_range(self) -> None:
        self.assertEqual(
            v.validate([variant(points=4)])["invalid"][0]["errors"],
            ["points 4 outside the grade 1 range"],
        )
        self.assertEqual(v.validate([variant(grade=4, points=10)])["invalid"], [])

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
