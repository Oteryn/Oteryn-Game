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

    def test_load_reads_a_catalogue_shard(self) -> None:
        shard = Path(self.id().replace(".", "_") + ".json")
        path = ROOT / shard
        try:
            path.write_text(json.dumps({"family": "Achievement", "records": [SAMPLE]}))
            self.assertEqual(v.load([path]), [SAMPLE])
        finally:
            path.unlink()

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

    def test_refs_bind_by_slug_or_explicitly_never_guessed(self) -> None:
        keys = {
            "oteryn:achievement/annihilator",
            "oteryn:achievement/the_professor_s_nut",
        }
        for ref, key in (
            ("canary:achievement/annihilator", "oteryn:achievement/annihilator"),
            ("crystalserver:achievement/annihilator", "oteryn:achievement/annihilator"),
            ("crystal:achievement/annihilator", "oteryn:achievement/annihilator"),
            ("oteryn:achievement/annihilator", "oteryn:achievement/annihilator"),
            (
                "canary:achievement/the_professors_nut",
                "oteryn:achievement/the_professor_s_nut",
            ),
            ("canary:achievement/annihilator_2", None),
            ("oteryn:achievement/the_professors_nut", None),
            ("tibia:achievement/annihilator", None),
            ("canary:item/annihilator", None),
        ):
            self.assertEqual(v.bind_ref(ref, keys), key, ref)

    def test_chest_sample_refs_bind_to_the_catalogue(self) -> None:
        catalogue = sorted(
            (ROOT.parents[2] / "content" / "achievements").glob("achievements-*.json")
        )
        claims = ROOT.parent / "quest-authoring" / "samples" / "chests" / "claims.json"
        refs = v.chest_refs(json.loads(claims.read_text(encoding="utf-8")))
        self.assertEqual(refs, ["canary:achievement/annihilator"])
        self.assertEqual(v.unbound_refs(refs, v.load(catalogue)), [])
        args = [str(path) for path in catalogue] + ["--chest-claims", str(claims)]
        self.assertEqual(v.main(args), 0)

    def test_an_unbound_chest_ref_fails(self) -> None:
        claims = {
            "claims": [
                {"placements": [{"position": {}}]},
                {
                    "placements": [
                        {"achievement": {"key": "canary:achievement/unknown"}}
                    ]
                },
            ]
        }
        path = ROOT / (self.id().replace(".", "_") + ".json")
        try:
            path.write_text(json.dumps(claims))
            args = [str(ROOT / "synthetic-valid-achievement.json")]
            self.assertEqual(v.main(args + ["--chest-claims", str(path)]), 1)
        finally:
            path.unlink()
        self.assertEqual(
            v.unbound_refs(v.chest_refs(claims), [SAMPLE]),
            ["canary:achievement/unknown"],
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
