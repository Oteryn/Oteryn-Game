import unittest
from pathlib import Path

import quest_completion_matrix as matrix


class QuestCompletionMatrixTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.result = matrix.expected(cls.root)

    def test_all_pinned_wiki_titles_map_to_canonical_quests(self):
        self.assertEqual(373, self.result["summary"]["wiki_titles"])
        self.assertEqual(373, self.result["summary"]["wiki_titles_mapped"])
        self.assertEqual(373, len(self.result["records"]))
        self.assertTrue(all(row["canonical"] for row in self.result["records"]))

    def test_matrix_never_claims_playability(self):
        self.assertEqual(0, self.result["summary"]["playable_verified"])
        self.assertTrue(all(row["playable_verification"] == "NOT_ASSESSED"
                            for row in self.result["records"]))

    def test_donor_coverage_matches_pinned_inventory(self):
        self.assertEqual(
            {
                "DONOR_IMPLEMENTATION_AVAILABLE": 332,
                "DONOR_PARTIAL_PLUS_REFERENCE": 12,
                "REFERENCE_AUTHORED_REQUIRED": 29,
            },
            self.result["summary"]["donor_mode"],
        )

    def test_native_lowering_is_not_misclassified_as_missing_source_data(self):
        by_title = {row["wiki_title"]: row for row in self.result["records"]}
        self.assertEqual(
            "NATIVE_BINDINGS_PENDING",
            by_title["An Interest In Botany Quest"]["work_state"],
        )
        self.assertEqual(
            "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY",
            by_title["A Father's Burden Quest"]["typed_progress_state"],
        )
        self.assertEqual(
            "NATIVE_BINDINGS_PENDING",
            by_title["A Father's Burden Quest"]["work_state"],
        )
        self.assertEqual(
            "SINGLE",
            by_title["To Outfox a Fox Quest"]["canonical_mapping"],
        )
        self.assertEqual(
            ["oteryn:quest.to_outfox_a_fox_quest"],
            [row["key"] for row in by_title["To Outfox a Fox Quest"]["canonical"]],
        )
        self.assertEqual(
            "REWARD_CLAIM_USE_PENDING",
            by_title["To Outfox a Fox Quest"]["work_state"],
        )
        reward_only = [
            row for row in self.result["records"]
            if row["work_state"] == "REWARD_CLAIM_USE_PENDING"
        ]
        self.assertEqual(42, len(reward_only))
        self.assertTrue(
            all(
                {definition["kind"] for definition in row["canonical"]} == {"reward_only"}
                for row in reward_only
            )
        )
        self.assertEqual(
            {
                "NATIVE_BINDINGS_PENDING": 331,
                "REWARD_CLAIM_USE_PENDING": 42,
            },
            self.result["summary"]["implementation_state"],
        )
        self.assertEqual(59, self.result["summary"]["reward_claim_ready_refs"])
        self.assertEqual(60, self.result["summary"]["reward_claim_placements"])
        self.assertEqual(
            {
                "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY": 146,
                "CHOSEN_TYPED_PROGRESS_ONLY": 68,
                "LOWERED": 7,
                "NO_CANDIDATE": 42,
                "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY": 110,
            },
            self.result["summary"]["typed_progress_state"],
        )
        self.assertEqual(
            {
                "SOURCE_HOLDS_CLEAR": 152,
                "SOURCE_HOLDS_PRESENT": 221,
            },
            self.result["summary"]["source_fidelity_state"],
        )

    def test_reward_only_runtime_owner_requires_ready_bound_claim_content(self):
        definition = {
            "kind": "reward_only",
            "claims": [
                {
                    "family": "RewardClaim",
                    "key": "oteryn:reward-claim.test",
                    "revision": "reward-claim-r1",
                }
            ],
        }
        ready = {
            "oteryn:reward-claim.test": {
                "readiness": "ready",
                "placements": [{"source_binding": {"project_position": {"x": 1, "y": 1, "z": 1}}}],
            }
        }
        self.assertEqual(
            "REWARD_CLAIM_USE_PENDING",
            matrix.reward_claim_owner([definition], ready)["state"],
        )
        for broken in [
            {},
            {"oteryn:reward-claim.test": {"readiness": "waiting_data", "placements": [{}]}},
            {"oteryn:reward-claim.test": {"readiness": "ready", "placements": []}},
        ]:
            self.assertEqual(
                "REWARD_CLAIM_DATA_PENDING",
                matrix.reward_claim_owner([definition], broken)["state"],
            )

    def test_canonical_inventory_is_not_assumed_one_to_one(self):
        self.assertEqual(352, self.result["summary"]["canonical_definitions"])
        self.assertEqual(
            350,
            self.result["summary"]["unique_canonical_definitions_representing_wiki_titles"],
        )
        self.assertEqual(
            2,
            self.result["summary"]["canonical_definitions_not_in_wiki_title_inventory"],
        )


if __name__ == "__main__":
    unittest.main()
