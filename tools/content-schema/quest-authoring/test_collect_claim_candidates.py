import copy
import unittest
from pathlib import Path
from unittest import mock

import collect_claim_candidates as candidates


class CollectClaimCandidatesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.packet = candidates.build(cls.root)

    def test_population_is_closed_and_runtime_inactive(self):
        self.assertEqual(
            {
                candidates.CLASS_AMBIGUOUS: 2,
                candidates.CLASS_EXACT: 2,
                candidates.CLASS_NO_MATCH: 143,
                candidates.CLASS_TARGET_INCOMPLETE: 130,
            },
            self.packet["summary"]["classification_counts"],
        )
        self.assertEqual(277, self.packet["summary"]["collect_stages"])
        self.assertEqual(0, self.packet["native_dispatch_bindings"])
        self.assertFalse(self.packet["runtime_admitted"])

    def test_only_two_exact_same_quest_reward_claim_acquisition_candidates(self):
        exact = [
            row
            for row in self.packet["records"]
            if row["classification"] == candidates.CLASS_EXACT
        ]
        self.assertEqual(
            {
                (
                    "oteryn:quest.hunter_outfits_quest",
                    "s2",
                    "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow",
                ),
                (
                    "oteryn:quest.oriental_outfits_quest",
                    "s5",
                    "oteryn:reward-claim.quest.u7_8.oriental_outfits.coral_comb",
                ),
            },
            {
                (row["quest"], row["stage_key"], row["candidate"]["claim"])
                for row in exact
            },
        )
        for row in exact:
            self.assertEqual(1, row["chosen_occurrence_count"])
            self.assertEqual(1, len(row["exact_item_targets"]))
            self.assertTrue(row["exact_item_targets"][0]["materializable"])
            self.assertEqual("NonStackable", row["exact_item_targets"][0]["stack_class"])
            self.assertEqual("ready", row["candidate"]["claim_readiness"])
            self.assertFalse(row["candidate"]["quantity_semantics_proven"])
            self.assertFalse(row["candidate"]["quest_transition_cause_binding_proven"])
            self.assertIsNone(row["native_dispatch_binding"])
            self.assertFalse(row["runtime_admitted"])

    def test_ambiguous_rows_are_not_promoted(self):
        ambiguous = [
            row
            for row in self.packet["records"]
            if row["classification"] == candidates.CLASS_AMBIGUOUS
        ]
        self.assertEqual(
            {
                ("oteryn:quest.dawnport_quest", "s2"),
                ("oteryn:quest.forgotten_knowledge_quest", "s2"),
            },
            {(row["quest"], row["stage_key"]) for row in ambiguous},
        )
        self.assertTrue(all("candidate" not in row for row in ambiguous))

    def build_with(self, mutate):
        real = candidates.read

        def patched(root, path):
            data = copy.deepcopy(real(root, path))
            if path.startswith("content/interactions/reward_claims/") and "records" in data:
                for record in data["records"]:
                    mutate(record["definition"])
            return data

        with mock.patch.object(candidates, "read", patched):
            return candidates.build(self.root)

    def exact_claims(self, packet):
        return {
            row["candidate"]["claim"]
            for row in packet["records"]
            if row["classification"] == candidates.CLASS_EXACT
        }

    def test_item_revision_drift_drops_the_exact_candidate(self):
        hunter = "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow"

        def mutate(definition):
            if definition["identity"]["key"] == hunter:
                for placement in definition["placements"]:
                    for item in placement["reward"]["items"]:
                        item["item"]["revision"] = "definition-r2"

        with self.assertRaises(ValueError):
            self.build_with(mutate)

    def test_quest_revision_drift_drops_the_exact_candidate(self):
        hunter = "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow"

        def mutate(definition):
            if definition["identity"]["key"] == hunter:
                definition["quest"]["revision"] = "other-revision"

        with self.assertRaises(ValueError):
            self.build_with(mutate)

    def test_targets_split_across_placements_are_not_one_acquisition(self):
        pairs = [
            row for row in self.packet["records"] if len(row["exact_item_targets"]) > 1
        ]
        for row in pairs:
            for cover in row["same_quest_claims_covering_all_exact_targets"]:
                self.assertIn("placement_index", cover)


if __name__ == "__main__":
    unittest.main()
