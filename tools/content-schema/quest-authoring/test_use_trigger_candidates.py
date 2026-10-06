import unittest
from pathlib import Path

import use_trigger_candidates as candidates


class UseTriggerCandidatesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.packet = candidates.build(cls.root)

    def test_population_is_closed_and_inactive(self):
        self.assertEqual(
            {
                candidates.CLASS_EXACT: 4,
                candidates.CLASS_NO_MATCH: 40,
                candidates.CLASS_TARGET_INCOMPLETE: 387,
            },
            self.packet["summary"]["classification_counts"],
        )
        self.assertEqual(431, self.packet["summary"]["use_stages"])
        self.assertEqual(0, self.packet["native_dispatch_bindings"])
        self.assertFalse(self.packet["runtime_admitted"])

    def test_four_literal_item_id_source_trigger_candidates_are_exactly_known(self):
        exact = [
            row
            for row in self.packet["records"]
            if row["classification"] == candidates.CLASS_EXACT
        ]
        self.assertEqual(
            {
                (
                    "oteryn:quest.the_cursed_crystal",
                    "s5",
                    "canary:interaction/the_cursed_crystal/actions_medusa_oil",
                ),
                (
                    "oteryn:quest.the_cursed_crystal",
                    "s7",
                    "canary:interaction/the_cursed_crystal/actions_medusa_oil",
                ),
                (
                    "oteryn:quest.brotherhood_outfits_quest",
                    "s3",
                    "canary:interaction/dreamers_challenge_quest/actions_documents",
                ),
                (
                    "oteryn:quest.brotherhood_outfits_quest",
                    "s4",
                    "canary:interaction/dreamers_challenge_quest/actions_documents",
                ),
            },
            {
                (row["quest"], row["stage_key"], row["candidate"]["source_graph"])
                for row in exact
            },
        )
        for row in exact:
            self.assertTrue(row["exact_item_targets"])
            self.assertTrue(
                all(
                    target["identity_basis"] == "A12_CANONICAL_TIBIA_ITEM_ID"
                    for target in row["exact_item_targets"]
                )
            )
            self.assertEqual(1, len(row["same_quest_literal_id_use_trigger_candidates"]))
            trigger = row["same_quest_literal_id_use_trigger_candidates"][0]
            self.assertTrue(trigger["registration_covers_stage_targets"])
            self.assertTrue(trigger["registration_has_extra_item_ids"])
            self.assertFalse(row["candidate"]["stage_branch_semantics_proven"])
            self.assertFalse(row["candidate"]["native_placement_binding_proven"])
            self.assertFalse(row["candidate"]["command_occurrence_binding_proven"])
            self.assertIsNone(row["native_dispatch_binding"])
            self.assertFalse(row["runtime_admitted"])

    def test_no_non_literal_registration_is_used_as_identity_proof(self):
        exact = [
            row
            for row in self.packet["records"]
            if row["classification"] == candidates.CLASS_EXACT
        ]
        for row in exact:
            for trigger in row["same_quest_literal_id_use_trigger_candidates"]:
                self.assertTrue(trigger["matched_literal_id_registrations"])
                self.assertTrue(
                    all(
                        registration["registration"].startswith("id(")
                        for registration in trigger["matched_literal_id_registrations"]
                    )
                )


if __name__ == "__main__":
    unittest.main()
