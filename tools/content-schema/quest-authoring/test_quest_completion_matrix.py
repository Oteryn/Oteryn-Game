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

    def test_qready2_every_owned_claim_has_runtime_proof(self):
        import json
        packet = json.loads((self.root / "tools/content-schema/quest-authoring/samples/completion-matrix/qready2-cases.json").read_text())
        titles = {case["wiki_title"] for case in packet["cases"]}
        self.assertEqual(17, len(titles))
        self.assertEqual(21, len(packet["cases"]))
        from quest_reward_use_proofs import load
        qualified = load(self.root, matrix.load_reward_claims(self.root))
        self.assertEqual(17, sum(row["wiki_title"] in titles and row["implementation_state"] == "REWARD_CLAIM_USE_VERIFIED"
                                for row in self.result["records"]))
        for row in self.result["records"]:
            if row["wiki_title"] in titles and set(row["reward_claim_owner"]["claim_keys"]) <= qualified:
                self.assertEqual("REWARD_CLAIM_USE_VERIFIED", row["implementation_state"])
                self.assertTrue(all(d["runtime_enabled"] for d in row["canonical"]))
                self.assertEqual(set(row["reward_claim_owner"]["claim_keys"]),
                                 {c["claim_key"] for c in packet["cases"] if c["wiki_title"] == row["wiki_title"]})

    def test_purple_tome_all_three_claims_have_runtime_proof(self):
        row = next(row for row in self.result["records"] if row["wiki_title"] == "Purple Tome Quest")
        self.assertEqual("REWARD_CLAIM_USE_VERIFIED", row["implementation_state"])
        self.assertEqual(3, len(row["reward_claim_owner"]["claim_keys"]))
        self.assertTrue(all(d["runtime_enabled"] for d in row["canonical"]))

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
        enabled = sum(row["work_state"] == "REWARD_CLAIM_USE_ENABLED" for row in self.result["records"])
        verified = sum(row["work_state"] == "REWARD_CLAIM_USE_VERIFIED" for row in self.result["records"])
        self.assertEqual(42, len(reward_only) + enabled + verified)
        self.assertTrue(
            all(
                {definition["kind"] for definition in row["canonical"]} == {"reward_only"}
                for row in reward_only
            )
        )
        self.assertEqual(
            {
                "NATIVE_BINDINGS_PENDING": 331,
                "REWARD_CLAIM_USE_PENDING": 42 - enabled - verified,
                "REWARD_CLAIM_USE_ENABLED": enabled,
                "REWARD_CLAIM_USE_VERIFIED": verified,
            },
            self.result["summary"]["implementation_state"],
        )
        # Crystal201189f's complete bag captures close SOURCE gaps for these
        # three quests. They remain implementation-pending, never promoted to USE.
        import sys
        sys.path.insert(0, str(self.root / 'tools/content-schema/reward-claim-authoring'))
        import container_native
        container_native.source()  # Verify the immutable donor packet bytes/cohort.
        claims = matrix.load_reward_claims(self.root)
        for title in ('Draconia Quest', 'Fire Axe Quest', 'Goblin Temple Quest'):
            owner = by_title[title]['reward_claim_owner']
            self.assertEqual(owner['state'], 'REWARD_CLAIM_IMPLEMENTATION_PENDING')
            selected = [claims[key] for key in owner['claim_keys']]
            self.assertTrue(all(not claim.get('data_holds') for claim in selected))
            containers = [claim['native_contents'] for claim in selected if 'native_contents' in claim]
            self.assertTrue(containers, title)
            for capture in containers:
                self.assertEqual(capture['revision'], container_native.REVISION)
                self.assertEqual(capture['source_sha256'], container_native.SOURCE_SHA256)
        # Native Item qualification can resolve a live claim without rewriting
        # its historical SOURCE witness or declaring the whole quest playable.
        draconia = by_title["Draconia Quest"]
        self.assertEqual(draconia["reward_claim_owner"]["state"],
                         "REWARD_CLAIM_IMPLEMENTATION_PENDING")
        self.assertIn("claim_item_semantics_missing",
                      draconia["canonical"][0]["missing_data_codes"])
        self.assertEqual(draconia["playable_verification"], "NOT_ASSESSED")
        # QUEST-FINISH admits eight additional exact full-default charge claims.
        # Resolving the remaining fluid Items moves two reward-only owners from
        # data pending to implementation pending; SOURCE and runtime stay distinct.
        self.assertEqual(132, self.result["summary"]["reward_claim_ready_refs"])
        self.assertEqual(136, self.result["summary"]["reward_claim_placements"])
        self.assertEqual(
            {
                "REWARD_CLAIM_DATA_PENDING": 3,
                "REWARD_CLAIM_IMPLEMENTATION_PENDING": 46,
                "REWARD_CLAIM_USE_PENDING": 56 - enabled - verified,
                "REWARD_CLAIM_USE_ENABLED": enabled,
                "REWARD_CLAIM_USE_VERIFIED": verified,
            },
            self.result["summary"]["reward_claim_owner_state"],
        )
        self.assertEqual(
            self.result["summary"]["reward_claim_ready_refs"],
            sum(
                (row["reward_claim_owner"] or {}).get("ready_claims", 0)
                for row in self.result["records"]
            ),
        )
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
        waiting = {
            "oteryn:reward-claim.test": {
                "readiness": "waiting_implementation",
                "placements": [{}],
            }
        }
        self.assertEqual(
            "REWARD_CLAIM_IMPLEMENTATION_PENDING",
            matrix.reward_claim_owner([definition], waiting)["state"],
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
