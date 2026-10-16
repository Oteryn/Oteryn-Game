#!/usr/bin/env python3
"""No-network tests for validate_achievements.py."""

from __future__ import annotations

import contextlib
import copy
import importlib
import io
import json
import sys
import tempfile
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


class CoverageTest(unittest.TestCase):
    def test_without_claims_every_key_is_pending(self) -> None:
        report = v.coverage_report([SAMPLE], [])
        self.assertEqual(report["bound_keys"], [])
        self.assertEqual(
            report["unearnable_pending"],
            [
                {
                    "key": SAMPLE["identity"]["key"],
                    "status": "UNEARNABLE_PENDING",
                    "reason": "NO_ADMITTED_GRANTER; EARNING_FAMILY_UNCLASSIFIED",
                }
            ],
        )

    def test_claim_binding_uses_existing_slug_and_explicit_owner_resolution(
        self,
    ) -> None:
        professor = variant(
            identity={
                **SAMPLE["identity"],
                "key": "oteryn:achievement/the_professor_s_nut",
            }
        )
        report = v.coverage_report(
            [SAMPLE, professor],
            [
                "canary:achievement/allow_cookies",
                "canary:achievement/the_professors_nut",
            ],
        )
        self.assertEqual(report["unearnable_pending"], [])
        self.assertEqual(
            report["bound_keys"],
            sorted([SAMPLE["identity"]["key"], professor["identity"]["key"]]),
        )

    def test_unknown_refs_and_unapproved_namespaces_never_cover_a_key(self) -> None:
        report = v.coverage_report(
            [SAMPLE],
            [
                "canary:achievement/unknown",
                "tibia:achievement/allow_cookies",
                "canary:item/allow_cookies",
                "Allow Cookies?",
            ],
        )
        self.assertEqual(report["bound_keys"], [])
        self.assertEqual(len(report["unearnable_pending"]), 1)

    def test_report_is_deterministic_and_bindings_are_deduplicated(self) -> None:
        other = variant(
            identity={**SAMPLE["identity"], "key": "oteryn:achievement/another"}
        )
        refs = ["canary:achievement/allow_cookies", "oteryn:achievement/allow_cookies"]
        expected = v.coverage_report([SAMPLE, other], refs)
        self.assertEqual(
            expected, v.coverage_report([other, SAMPLE], list(reversed(refs)) * 2)
        )
        self.assertEqual(len(expected["bound_keys"]), 1)

    def test_retired_bound_key_is_not_a_claim_of_earnability(self) -> None:
        retired = variant(retired=True, points=0)
        report = v.coverage_report([retired], [SAMPLE["identity"]["key"]])
        self.assertEqual(report["retired_keys"], [SAMPLE["identity"]["key"]])
        self.assertEqual(report["bound_keys"], [SAMPLE["identity"]["key"]])
        self.assertEqual(
            report["scope"], "SUPPLIED_INPUTS_ONLY_NOT_RUNTIME_EARNABILITY"
        )
        pending = v.coverage_report([retired], [])
        self.assertEqual(
            pending["unearnable_pending"][0]["status"], "UNEARNABLE_PENDING"
        )

    def test_text_and_provenance_do_not_classify_earning_family(self) -> None:
        record = variant(
            name="Level 100", description="Kill a boss, catch fish or finish a quest."
        )
        report = v.coverage_report([record], [])
        self.assertIn(
            "EARNING_FAMILY_UNCLASSIFIED", report["unearnable_pending"][0]["reason"]
        )
        self.assertEqual(
            report["reason_families"],
            ["quest_or_quest_step", "boss_or_encounter", "counter", "one_time_action"],
        )
        self.assertEqual(
            report["unsupported_granters"],
            ["quest-transition", "encounter-binding", "counter-threshold"],
        )

    def test_invalid_catalogue_never_has_positive_coverage(self) -> None:
        for records in ([variant(points=4)], [SAMPLE, SAMPLE], [None]):
            report = v.coverage_report(records, [SAMPLE["identity"]["key"]])
            self.assertEqual(report, {"status": "INVALID_CATALOGUE"})

    def test_cli_reports_invalid_unhashable_keys_with_chest_claims(self) -> None:
        for key in ([], {}):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as directory:
                record = copy.deepcopy(SAMPLE)
                record["identity"]["key"] = key
                records = Path(directory) / "records.json"
                records.write_text(json.dumps([record]))
                claims = Path(directory) / "claims.json"
                placement = {"achievement": {"key": SAMPLE["identity"]["key"]}}
                claims.write_text(
                    json.dumps({"claims": [{"placements": [placement]}]})
                )
                output = io.StringIO()
                with contextlib.redirect_stdout(output):
                    result = v.main([str(records), "--chest-claims", str(claims)])
                report = json.loads(output.getvalue())
                self.assertEqual(result, 1)
                self.assertEqual(report["valid"], 0)
                self.assertEqual(len(report["invalid"]), 1)
                self.assertEqual(
                    report["unbound_chest_refs"], [SAMPLE["identity"]["key"]]
                )
                self.assertEqual(report["coverage"], {"status": "INVALID_CATALOGUE"})

    def test_cli_emits_pending_without_failing_valid_catalogue(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            result = v.main([str(ROOT / "synthetic-valid-achievement.json")])
        self.assertEqual(result, 0)
        report = json.loads(output.getvalue())
        self.assertEqual(report["valid"], 1)
        self.assertEqual(
            report["coverage"]["unearnable_pending"][0]["status"], "UNEARNABLE_PENDING"
        )

    def test_cli_unbound_claim_fails_and_does_not_hide_pending(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            claims = Path(directory) / "claims.json"
            claims.write_text(
                json.dumps(
                    {
                        "claims": [
                            {
                                "placements": [
                                    {
                                        "achievement": {
                                            "key": "canary:achievement/unknown"
                                        }
                                    }
                                ]
                            }
                        ]
                    }
                )
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                result = v.main(
                    [
                        str(ROOT / "synthetic-valid-achievement.json"),
                        "--chest-claims",
                        str(claims),
                    ]
                )
        self.assertEqual(result, 1)
        report = json.loads(output.getvalue())
        self.assertEqual(report["unbound_chest_refs"], ["canary:achievement/unknown"])
        self.assertEqual(report["coverage"]["bound_keys"], [])
        self.assertEqual(len(report["coverage"]["unearnable_pending"]), 1)

    def test_cli_combines_only_placement_refs_across_claim_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            files = []
            for index, document in enumerate(
                [
                    {
                        "claims": [
                            {
                                "placements": [
                                    {
                                        "achievement": {
                                            "key": "canary:achievement/allow_cookies"
                                        }
                                    }
                                ]
                            }
                        ]
                    },
                    {
                        "claims": [
                            {
                                "placements": [
                                    {"achievement": {"key": SAMPLE["identity"]["key"]}}
                                ]
                            }
                        ]
                    },
                    {
                        "claims": [],
                        "achievement_domain": {"achievement": "Allow Cookies?"},
                        "thresholds": [{"achievement": SAMPLE["identity"]["key"]}],
                    },
                ]
            ):
                path = Path(directory) / f"claims-{index}.json"
                path.write_text(json.dumps(document))
                files.extend(["--chest-claims", str(path)])
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                result = v.main(
                    [str(ROOT / "synthetic-valid-achievement.json"), *files]
                )
            self.assertEqual(result, 0)
            report = json.loads(output.getvalue())
            self.assertEqual(report["chest_refs"], 2)
            self.assertEqual(
                report["coverage"]["bound_keys"], [SAMPLE["identity"]["key"]]
            )
            self.assertEqual(report["coverage"]["unearnable_pending"], [])
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                result = v.main(
                    [str(ROOT / "synthetic-valid-achievement.json"), *files[-2:]]
                )
            self.assertEqual(result, 0)
            self.assertEqual(
                json.loads(output.getvalue())["coverage"]["bound_keys"], []
            )

    def test_empty_catalogue_is_a_complete_empty_report(self) -> None:
        report = v.coverage_report([], [])
        self.assertEqual(report["bound_keys"], [])
        self.assertEqual(report["unearnable_pending"], [])
        self.assertEqual(report["retired_keys"], [])


if __name__ == "__main__":
    unittest.main()
