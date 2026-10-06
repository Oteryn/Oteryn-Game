import json
import pathlib
import unittest

from builder import PIN, WORLD_SHA, kill_binding_candidates, norm, resolve

ROOT = pathlib.Path(__file__).parent
PACKET = ROOT / "packet.json"


class ChosenSourceEventsRewardsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads(PACKET.read_text(encoding="utf-8"))

    def test_population_and_epoch(self):
        self.assertEqual("OTERYN_CHOSEN_SOURCE139_EVENT_REWARD_ASSOCIATIONS/v1", self.packet["schema"])
        self.assertEqual(PIN, self.packet["epoch"])
        self.assertEqual(WORLD_SHA, self.packet["input_refs"]["qualified_world_sha256"])
        self.assertEqual(139, self.packet["counts"]["quests"])
        self.assertEqual(695, self.packet["counts"]["stages"])
        self.assertEqual(611, self.packet["counts"]["non_dialogue_stages"])
        self.assertEqual(301, self.packet["counts"]["reward_intents"])
        self.assertEqual(283, self.packet["counts"]["exact_stage_target_refs"])
        self.assertEqual(232, self.packet["counts"]["exact_reward_refs"])
        self.assertEqual(8, self.packet["counts"]["encounter_outcome_seams"])

    def test_no_runtime_promotion(self):
        self.assertFalse(self.packet["runtime_admitted"])
        self.assertEqual(139, len(self.packet["records"]))
        self.assertEqual(
            139,
            len({row["quest_ref"]["key"] for row in self.packet["records"]}),
        )
        for quest in self.packet["records"]:
            self.assertTrue(quest["native_readiness_unchanged"])
            for stage in quest["stages"]:
                self.assertFalse(stage["runtime_admitted"])
                self.assertTrue(stage["unresolved"])
            for reward in quest["reward_intents"]:
                self.assertFalse(reward["runtime_admitted"])

    def test_no_fuzzy_matching(self):
        table = {
            ("Item", norm("Gold Coin")): [
                {"ref": {"family": "Item", "key": "accepted", "revision": "1"}}
            ]
        }
        self.assertEqual(1, len(resolve(table, ["Item"], "GOLD COIN")))
        self.assertEqual([], resolve(table, ["Item"], "Gold Coins"))

    def test_exact_rows_have_one_evidenced_ref(self):
        rows = [
            target
            for quest in self.packet["records"]
            for stage in quest["stages"]
            for target in stage["targets"]
        ] + [
            reward["mapping"]
            for quest in self.packet["records"]
            for reward in quest["reward_intents"]
        ]
        for row in rows:
            if row["status"] == "EXACT_CANONICAL_IDENTITY_ASSOCIATION":
                self.assertEqual(1, len(row["refs"]))
                self.assertTrue(row["evidence"])
                self.assertTrue(all(ev["epoch"] == PIN for ev in row["evidence"]))


    def test_exact_kill_encounter_seams_are_qualified_but_not_promoted(self):
        rows = kill_binding_candidates(self.packet)
        self.assertEqual(8, len(rows))
        self.assertEqual(
            8,
            len({
                (
                    row["quest_ref"]["key"],
                    row["stage_key"],
                    row["target"],
                    row["encounter_ref"]["key"],
                    row["outcome"],
                )
                for row in rows
            }),
        )
        for row in rows:
            self.assertEqual("Encounter", row["encounter_ref"]["family"])
            self.assertTrue(row["outcome"])
            self.assertTrue(row["rule_key"])
            self.assertFalse(row["execution_verified"])
            self.assertIsNone(row["native_dispatch_binding"])
            self.assertFalse(row["runtime_admitted"])

    def test_all_completion_stages_remain_unbound(self):
        completion = [
            stage
            for quest in self.packet["records"]
            for stage in quest["stages"]
            if stage["kind"] == "complete"
        ]
        self.assertEqual(139, len(completion))
        self.assertTrue(
            all("NATIVE_COMPLETION_REDUCER_BINDING_PENDING" in stage["unresolved"]
                for stage in completion)
        )


if __name__ == "__main__":
    unittest.main()
