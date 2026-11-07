import json
import pathlib
import unittest

import quest_reward_only_binding_candidates as tool


ROOT = pathlib.Path(__file__).resolve().parents[3]


class RewardOnlyCompletionCandidatesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = tool.expected(ROOT)
        cls.by_key = {
            row["quest_ref"]["key"]: row for row in cls.packet["records"]
        }

    def test_population_is_exact_and_fail_closed(self):
        self.assertEqual(
            {
                "quests": 42,
                "claims": 59,
                "classes": {
                    "MULTI_KV": 10,
                    "MULTI_STORAGE": 1,
                    "SINGLE_KV": 24,
                    "SINGLE_STORAGE": 7,
                },
                "native_completion_transitions": 0,
                "native_claim_bindings": 0,
            },
            self.packet["summary"],
        )
        self.assertFalse(self.packet["runtime_admitted"])
        self.assertEqual(42, len(self.by_key))
        self.assertTrue(
            all(row["native_completion_transition"] is None for row in self.packet["records"])
        )
        self.assertTrue(
            all(row["native_claim_binding"] is None for row in self.packet["records"])
        )

    def test_all_source_claims_are_once_and_have_exact_native_refs(self):
        for row in self.packet["records"]:
            self.assertEqual(
                len(row["source_claims"]),
                len(row["native_reward_refs"]),
                row["quest_ref"]["key"],
            )
            self.assertTrue(
                all(claim["repeat"] == "once" for claim in row["source_claims"]),
                row["quest_ref"]["key"],
            )
            self.assertTrue(
                all(ref["family"] == "RewardClaim" for ref in row["native_reward_refs"]),
                row["quest_ref"]["key"],
            )

    def test_single_and_multi_claims_keep_different_policy_holds(self):
        for row in self.packet["records"]:
            if row["candidate_class"].startswith("SINGLE_"):
                self.assertIn(
                    "CLAIM_AS_QUEST_COMPLETION_POLICY_REQUIRED",
                    row["holds"],
                    row["quest_ref"]["key"],
                )
                self.assertNotIn(
                    "MULTI_CLAIM_COMPLETION_AGGREGATION_POLICY_REQUIRED",
                    row["holds"],
                    row["quest_ref"]["key"],
                )
            else:
                self.assertIn(
                    "MULTI_CLAIM_COMPLETION_AGGREGATION_POLICY_REQUIRED",
                    row["holds"],
                    row["quest_ref"]["key"],
                )

    def test_storage_and_kv_shapes_do_not_mix_within_one_quest(self):
        for row in self.packet["records"]:
            writes = [
                claim["progress_write"] is not None for claim in row["source_claims"]
            ]
            if row["candidate_class"].endswith("_STORAGE"):
                self.assertTrue(all(writes), row["quest_ref"]["key"])
                self.assertIn(
                    "SOURCE_PROGRESS_WRITE_EXISTS_BUT_CANONICAL_QUEST_HAS_NO_PROGRESS_TRACK",
                    row["holds"],
                )
            else:
                self.assertFalse(any(writes), row["quest_ref"]["key"])
                self.assertIn("NO_SOURCE_PROGRESS_WRITE", row["holds"])

    def test_canonical_child_outside_373_title_inventory_is_retained(self):
        row = self.by_key["oteryn:quest.to_outfox_a_fox_quest"]
        self.assertEqual("SINGLE_STORAGE", row["candidate_class"])
        self.assertEqual(1, len(row["native_reward_refs"]))

    def test_committed_packet_matches_generator(self):
        target = ROOT / tool.OUTPUT
        self.assertTrue(target.is_file())
        self.assertEqual(target.read_bytes(), tool.compact(tool.expected(ROOT)))


if __name__ == "__main__":
    unittest.main()
