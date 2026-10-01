#!/usr/bin/env python3
"""No-network preparation checks, including evidence promotion and source defects."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path

import charm_mechanics as cm


class CharmMechanicsTests(unittest.TestCase):
    def setUp(self):
        self.sample = cm.load(cm.SOURCES)
        self.mechanics = cm.load(cm.MECHANICS)
        self.progression = cm.load(cm.PROGRESSION)

    def check(self):
        return cm.validate(self.sample, self.mechanics, self.progression)

    def charm(self, name, sample=False):
        rows = self.sample["mechanics"] if sample else self.mechanics["charms"]
        return next(r for r in rows if r["key"] == "oteryn:charm." + name)

    def test_complete_current_package_and_schema(self):
        from jsonschema import Draft202012Validator

        Draft202012Validator.check_schema(cm.load(cm.SCHEMA))
        self.assertEqual(self.check(), [])
        self.assertEqual(cm.validate_index(cm.load(cm.INDEX)), [])
        self.assertEqual(len(self.mechanics["charms"]), 25)
        self.assertEqual(
            sum(
                len(r["observed"]) for r in self.sample["catalogue_comparison"]["rows"]
            ),
            75,
        )

    def test_missing_charm_rejected(self):
        self.mechanics["charms"].pop()
        self.assertTrue(self.check())

    def test_type_and_hook_drift_rejected(self):
        for field, value in [
            ("effect_type", "critical_hit_chance"),
            ("hook", "critical_chance_resolution"),
        ]:
            with self.subTest(field=field):
                original = copy.deepcopy(self.mechanics)
                self.charm("wound")[field] = value
                self.assertTrue(self.check())
                self.mechanics = original

    def test_ots_source_promotion_rejected(self):
        source = next(
            v
            for v in self.sample["sources"].values()
            if v["repository"] in cm.OTS_REPOS
        )
        source["evidence_class"] = "PROJECT_ACCEPTED_RECORD"
        self.assertTrue(self.check())

    def test_conflict_cannot_be_silently_resolved_even_in_both_files(self):
        for sample in [True, False]:
            self.charm("cleanse", sample)["parameters"]["eligible_canary"]["status"] = (
                "OWNER_ACCEPTED"
            )
        self.assertTrue(self.check())

    def test_unknown_cannot_activate_even_in_both_files(self):
        for sample in [True, False]:
            self.charm("carnage", sample)["parameters"]["ots_area_geometry_candidate"][
                "activation"
            ] = True
        self.assertTrue(self.check())

    def test_typed_speed_and_duration_bounds(self):
        for field, value in [
            ("formula_a", "fast"),
            ("duration_ms", 0),
            ("paralysis_minimum_speed", -1),
        ]:
            with self.subTest(field=field):
                original = copy.deepcopy(self.mechanics)
                self.charm("cripple")["parameters"][field]["value"] = value
                self.assertTrue(self.check())
                self.mechanics = original

    def test_closed_shapes_reject_extra_fields(self):
        self.charm("low_blow")["parameters"]["modifier"]["value"]["magic_new_stat"] = 1
        self.assertTrue(self.check())

    def test_catalogue_bytes_are_bound(self):
        altered = json.loads(cm.CATALOGUE.read_text())
        altered["records"][0]["definition"]["stages"][0]["value"] += 1
        errors = cm.validate(
            self.sample, self.mechanics, self.progression, cm.dumps(altered).encode()
        )
        self.assertIn("catalogue SHA256 drift", errors)
        self.assertTrue(any("numeric comparison" in e for e in errors))

    def test_independent_numeric_observations_cannot_drift(self):
        self.sample["catalogue_comparison"]["rows"][0]["observed"]["crystal"]["costs"][
            0
        ] += 1
        self.assertTrue(any("numeric comparison" in e for e in self.check()))

    def test_reversed_and_zero_source_lines_rejected(self):
        reference = self.charm("cripple")["parameters"]["formula_a"]["references"][0]
        reference["lines"] = [20, 10]
        self.assertTrue(self.check())
        reference["lines"] = [0, 10]
        self.assertTrue(self.check())

    def test_unknown_reference_rejected(self):
        self.charm("cripple")["parameters"]["formula_a"]["references"][0]["source"] = (
            "unknown"
        )
        self.assertTrue(self.check())

    def test_source_hash_verification_rejects_missing_and_changed_file(self):
        sample = {
            "sources": {
                "fixture": {
                    "repository": "fixture",
                    "path": "source",
                    "sha256": cm.digest(b"expected"),
                }
            }
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertTrue(cm.verify_sources(sample, {"fixture": root}))
            (root / "source").write_bytes(b"wrong")
            self.assertTrue(cm.verify_sources(sample, {"fixture": root}))
            (root / "source").write_bytes(b"expected")
            self.assertEqual(cm.verify_sources(sample, {"fixture": root}), [])

    def test_source_parsers_require_all_25_and_numeric_triples(self):
        for planner in [True, False]:
            with self.assertRaises((ValueError, TypeError)):
                cm.source_catalogue(b"incomplete source", planner)

    def test_owner_targeting_retains_passives_on_secondary_auto_targets(self):
        targeting = self.mechanics["common_rules"]["area_targeting"]["value"]
        self.assertEqual(targeting["auto_attack_procs"], "main_target_only")
        self.assertEqual(len(targeting["secondary_auto_passives"]), 4)
        self.assertIn("area_targeting", self.charm("fatal_hold")["common_rules"])
        self.assertFalse(targeting["generated_proc_or_charm_damage_can_trigger_charms"])
        for key in ["low_blow", "savage_blow", "vampiric_embrace", "voids_call"]:
            self.assertEqual(
                self.charm(key)["parameters"]["modifier"]["value"]["operation"],
                "add_percentage_points",
            )

    def test_generated_damage_guards_cover_carnage_without_surviving_target_gate(self):
        for row in self.mechanics["charms"]:
            if row["effect_type"] in [
                "attack_proc_damage",
                "attack_proc_resource_damage",
                "kill_area_damage",
            ]:
                self.assertIn("charm_generated_damage", row["common_rules"])
        carnage = self.charm("carnage")
        self.assertIn("area_targeting", carnage["common_rules"])
        self.assertNotIn("attack_proc_commit", carnage["common_rules"])
        self.assertIn(
            "no_generated_damage_reentry", self.charm("parry")["common_rules"]
        )

    def test_ots_defects_remain_reference_only(self):
        for name, parameter in [
            ("low_blow", "canary_actual_rng_algorithm"),
            ("scavenge", "ots_actual_success_math"),
            ("voids_call", "crystal_actual_amount_math"),
            ("vampiric_embrace", "crystal_actual_amount_math"),
        ]:
            fact = self.charm(name)["parameters"][parameter]
            self.assertEqual(fact["status"], "CONFLICT")
            self.assertFalse(fact["activation"])

    def test_index_population_and_unassign_boundary(self):
        index = cm.load(cm.INDEX)
        index["population_state"] = "READY_UNPOPULATED"
        self.assertTrue(cm.validate_index(index))
        boundary = self.progression["rules"]["unassign_boundary"]["value"]
        self.assertFalse(boundary["runtime_command_available"])
        self.assertIn("GAME-ITEM-01/DUR-03", boundary["condition_for_future_runtime"])


def run_suite() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(CharmMechanicsTests)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    if not result.wasSuccessful():
        raise AssertionError("Charm mechanics preparation validation failed")
    return result.testsRun


if __name__ == "__main__":
    run_suite()
