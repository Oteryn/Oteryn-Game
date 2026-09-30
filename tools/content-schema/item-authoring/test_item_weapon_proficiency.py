"""Tests for item_weapon_proficiency (ITEM-PROF-1). Run with `python test_item_weapon_proficiency.py`."""

import json
import unittest

import item_weapon_proficiency as iwp


class ThresholdClassTests(unittest.TestCase):
    def test_bolt_ammunition_is_crossbow_even_in_a_shared_profile(self):
        shared = "Generic 2H Distance"  # crossbow 3349, bow 3350 share this profile
        self.assertEqual(iwp.threshold_class(shared, "bolt"), "crossbow")
        self.assertEqual(iwp.threshold_class(shared, "arrow"), "standard")
        # D198: profile names without a Crossbow word are still crossbows on bolt evidence
        for name in (
            "Distance 2H Arbalest",
            "Distance 2H Chain Bolter",
            "Distance 2H The Devileye",
        ):
            self.assertEqual(iwp.threshold_class(name, "bolt"), "crossbow")

    def test_ranged_without_ammunition_evidence_is_unknown(self):
        self.assertEqual(
            iwp.threshold_class("Replica Mayhem Distance", None), "unknown"
        )
        self.assertEqual(iwp.threshold_class("Amber 2H Crossbow", None), "unknown")

    def test_knight_only_for_knight_restricted_melee(self):
        sword = "Sword 1H Crimson Sword"
        self.assertEqual(iwp.threshold_class(sword, None, "Knights"), "knight")
        self.assertEqual(
            iwp.threshold_class(sword, None, "players without vocation"), "standard"
        )
        self.assertEqual(iwp.threshold_class(sword, None, "Monks"), "standard")
        self.assertEqual(iwp.threshold_class(sword, None, None), "unknown")
        self.assertEqual(
            iwp.threshold_class("Grand Sanguine 2H Axe", None, "Knights"), "knight"
        )

    def test_other_weapons_are_standard(self):
        self.assertEqual(iwp.threshold_class("Wand 1H Wand of Decay"), "standard")
        self.assertEqual(iwp.threshold_class("Throw - Small Stone"), "standard")
        self.assertEqual(
            iwp.threshold_class("Fist 1H Light Jo Staff", None, "Monks"), "standard"
        )

    def test_threshold_tables_have_nine_increasing_levels(self):
        self.assertEqual(set(iwp.THRESHOLDS), {"standard", "knight", "crossbow"})
        self.assertEqual(set(iwp.CLASSES), set(iwp.THRESHOLDS) | {"unknown"})
        for table in iwp.THRESHOLDS.values():
            self.assertEqual(len(table), 9)
            self.assertEqual(table, sorted(table))


class ArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = iwp.build()

    def test_counts_and_keys(self):
        document = self.document
        self.assertEqual(len(document["profiles"]), iwp.EXPECTED_PROFILES)
        self.assertEqual(len(document["bindings"]), iwp.EXPECTED_BINDINGS)
        keys = [b["item_key"] for b in document["bindings"]]
        self.assertEqual(len(keys), len(set(keys)))
        self.assertTrue(all(k.startswith("oteryn:item.tibia.i") for k in keys))
        ids = {p["proficiency_id"] for p in document["profiles"]}
        self.assertTrue(all(b["proficiency_id"] in ids for b in document["bindings"]))
        self.assertEqual(document["counts"]["referenced_profiles"], 430)

    def test_bindings_are_classified_per_item(self):
        by_id = {b["client_object_id"]: b for b in self.document["bindings"]}
        self.assertEqual(by_id[3349]["threshold_class"], "crossbow")  # shared profile
        self.assertEqual(
            by_id[3350]["threshold_class"], "standard"
        )  # bow, same profile
        self.assertEqual(by_id[5803]["threshold_class"], "crossbow")  # arbalest
        self.assertEqual(by_id[26004]["threshold_class"], "unknown")  # replica crossbow
        self.assertEqual(by_id[26067]["threshold_class"], "unknown")
        self.assertEqual(by_id[3265]["threshold_class"], "knight")  # two handed sword
        self.assertEqual(
            by_id[3294]["threshold_class"], "unknown"
        )  # short sword, no vocation evidence
        counts = self.document["counts"]["bindings_by_threshold_class"]
        self.assertEqual(sum(counts.values()), iwp.EXPECTED_BINDINGS)
        self.assertEqual(set(counts), set(iwp.CLASSES))

    def test_perk_mapping_covers_every_raw_value(self):
        mapping = self.document["perk_mapping"]
        for name, key in (
            ("Type", "Type"),
            ("SkillId", "SkillId"),
            ("AugmentType", "AugmentType"),
        ):
            mapped = {row["value"] for row in mapping[key]}
            raw = {row["value"] for row in self.document["perk_raw_enums"][name]}
            self.assertLessEqual(raw, mapped, name)
        combat = {row["value"] for row in mapping["ElementId_DamageType"]}
        for name in ("ElementId", "DamageType"):
            raw = {row["value"] for row in self.document["perk_raw_enums"][name]}
            self.assertLessEqual(raw, combat, name)
        self.assertNotIn(-1, {row["value"] for row in mapping["Type"]})
        by_type = {row["value"]: row for row in mapping["Type"]}
        self.assertEqual(by_type[13 + 6]["meaning"], "hit points on hit")
        self.assertEqual(by_type[22]["unit"], "flat_tiles")

    def test_mastery_is_top_level_plus_two(self):
        for profile in self.document["profiles"]:
            self.assertEqual(profile["top_level"], len(profile["levels"]))
            self.assertEqual(profile["mastery_level"], profile["top_level"] + 2)
            self.assertLessEqual(profile["mastery_level"], 9)

    def test_committed_file_is_current_and_valid_json(self):
        text = iwp.render(self.document)
        self.assertEqual(json.loads(text)["schema"], iwp.SCHEMA)
        self.assertEqual(iwp.DEFAULT_OUTPUT.read_text(encoding="utf-8"), text)


if __name__ == "__main__":
    unittest.main()
