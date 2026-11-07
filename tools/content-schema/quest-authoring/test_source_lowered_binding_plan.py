import copy
import unittest

from native_claim_plan_exclusions import declared_native_claim, claim_only_owner
from pathlib import Path

import source_lowered_binding_plan as plan


class SourceLoweredBindingPlanTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        cls.packet = plan.expected(cls.root)

    def test_declared_claims_are_excluded_without_qualifying_the_quest(self):
        state = plan.read(self.root, plan.QUEST_STATE)
        claims = [(quest['quest'], row) for quest in state['quests']
                  for row in quest['transitions']
                  if (row.get('source') or {}).get('owner') == 'chest'
                  and plan.completion_state(quest) == 'LOWERED']
        self.assertEqual(1, len(claims))
        self.assertEqual(self.packet['native_claim_exclusions'],
                         [declared_native_claim(self.root, quest, row)
                          for quest, row in claims])
        for quest, row in claims:
            altered = copy.deepcopy(row)
            altered['effects'][0]['effect']['value'] += 1
            with self.assertRaisesRegex(ValueError, 'differs from declared claim'):
                declared_native_claim(self.root, quest, altered)
            altered['key'] += '-unregistered'
            with self.assertRaisesRegex(ValueError, 'unregistered native chest'):
                declared_native_claim(self.root, quest, altered)

    def test_claim_only_owner_has_no_donor_mission_coverage(self):
        state = plan.read(self.root, plan.QUEST_STATE)
        owners = [quest for quest in state['quests'] if claim_only_owner(self.root, quest)]
        self.assertEqual(['oteryn:quest.formogar_mine_hoist'],
                         [quest['quest'] for quest in owners])
        altered = copy.deepcopy(owners[0])
        altered['tracks'][0]['bounds_basis'] = 'donor_mission'
        self.assertFalse(claim_only_owner(self.root, altered))
        altered = copy.deepcopy(owners[0])
        altered['transitions'][0]['effects'][0]['effect']['value'] += 1
        with self.assertRaisesRegex(ValueError, 'differs from declared claim'):
            claim_only_owner(self.root, altered)

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

    def test_action_or_movement_without_callback_is_rejected(self):
        transition = {
            "key": "oteryn:quest-transition/test/no-callback",
            "requested_by": None,
            "source": {"owner": "action", "callback": None},
        }
        with self.assertRaisesRegex(ValueError, "lacks callback evidence"):
            plan.classify_transition(transition)

    def test_source_and_chosen_plans_are_disjoint_and_cover_310_candidate_owners(self):
        chosen = plan.read(self.root, plan.CHOSEN_PLAN)
        source_owners = {row["quest"] for row in self.packet["records"]}
        chosen_owners = {row["quest"] for row in chosen["records"]}
        self.assertFalse(source_owners & chosen_owners)
        self.assertEqual(310, len(source_owners | chosen_owners))


if __name__ == "__main__":
    unittest.main()
