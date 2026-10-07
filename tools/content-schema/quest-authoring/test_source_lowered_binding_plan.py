import unittest
from pathlib import Path

import source_lowered_binding_plan as plan


class SourceLoweredBindingPlanTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.packet = plan.expected(cls.root)

    def test_exact_source_lowered_gap_is_six_owners_and_121_transitions(self):
        self.assertEqual(
            {
                "oteryn:quest.hot_cuisine_quest",
                "oteryn:quest.oramond_quest",
                "oteryn:quest.sam_s_old_backpack_quest",
                "oteryn:quest.spirithunters_quest",
                "oteryn:quest.the_ape_city_quest",
                "oteryn:quest.the_outlaw_camp_quest",
            },
            {row["quest"] for row in self.packet["records"]},
        )
        self.assertEqual(
            {
                "quests": 6,
                "transitions": 121,
                "completion_transitions": 4,
                "requested_by_transitions": 106,
                "native_bindings": 0,
                "consumer_lanes": {"NPC_QUEST_1": 106, "QUEST_TRIGGER_1": 15},
                "source_owners": {"action": 14, "movement": 1, "npc": 106},
            },
            self.packet["summary"],
        )

    def test_rows_are_exact_existing_quest_state_transitions(self):
        state = plan.read(self.root, plan.QUEST_STATE)
        source = {
            quest["quest"]: {transition["key"]: transition for transition in quest["transitions"]}
            for quest in state["quests"]
        }
        for quest in self.packet["records"]:
            self.assertEqual("LOWERED", quest["source_completion_state"])
            self.assertFalse(quest["runtime_enabled"])
            self.assertEqual(0, quest["native_binding_count"])
            for row in quest["transitions"]:
                original = source[quest["quest"]][row["quest_transition_key"]]
                self.assertEqual(bool(original["completes"]), row["completes"])
                self.assertEqual(original.get("requested_by"), row["requested_by"])
                self.assertEqual(original.get("source") or {}, row["source"])
                self.assertIsNone(row["native_binding"])
                self.assertFalse(row["runtime_enabled"])

    def test_consumer_lanes_are_closed_and_fail_closed(self):
        for quest in self.packet["records"]:
            for row in quest["transitions"]:
                owner = row["source"]["owner"]
                if row["consumer_lane"] == "NPC_QUEST_1":
                    self.assertEqual("npc", owner)
                    self.assertIsInstance(row["requested_by"], dict)
                    self.assertTrue(row["requested_by"]["npc"])
                    self.assertEqual(
                        "QUEST_STATE_REQUESTED_BY_EVIDENCE",
                        row["binding_basis"],
                    )
                elif row["consumer_lane"] == "QUEST_TRIGGER_1":
                    self.assertIn(owner, {"action", "movement"})
                    self.assertIsNone(row["requested_by"])
                    self.assertEqual(
                        "QUEST_STATE_SOURCE_CALLBACK_EVIDENCE",
                        row["binding_basis"],
                    )
                else:
                    self.fail("unexpected consumer lane " + row["consumer_lane"])

    def test_source_and_chosen_plans_are_disjoint_and_cover_310_candidate_owners(self):
        chosen = plan.read(self.root, plan.CHOSEN_PLAN)
        source_owners = {row["quest"] for row in self.packet["records"]}
        chosen_owners = {row["quest"] for row in chosen["records"]}
        self.assertFalse(source_owners & chosen_owners)
        self.assertEqual(310, len(source_owners | chosen_owners))


if __name__ == "__main__":
    unittest.main()
