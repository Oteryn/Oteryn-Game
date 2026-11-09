import json
import pathlib
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
PLAN = ROOT / "content/quests/missions/completion-binding-plan.json"
SOURCE = ROOT / "tools/content-schema/quest-authoring/samples/server-completion/chosen-source-events-rewards/packet.json"


class ChosenSourceBindingPlanTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan = json.loads(PLAN.read_text(encoding="utf-8"))
        cls.source = json.loads(SOURCE.read_text(encoding="utf-8"))
        cls.source_keys = {row["quest_ref"]["key"] for row in cls.source["records"]}
        cls.by_quest = {row["quest"]: row for row in cls.plan["records"]}

    def test_population(self):
        self.assertEqual({"quests": 304, "stages": 1887}, self.plan["counts"])
        self.assertEqual(236, len(self.source_keys))
        self.assertTrue(self.source_keys <= set(self.by_quest))
        source_rows = [self.by_quest[key] for key in self.source_keys]
        self.assertEqual(1471, sum(len(row["stages"]) for row in source_rows))
        self.assertEqual(509, sum(len(row["reward_identity_associations"]) for row in source_rows))

    def test_source_rows_remain_non_executable(self):
        self.assertFalse(self.plan["runtime_enabled"])
        for key in self.source_keys:
            row = self.by_quest[key]
            self.assertFalse(row["runtime_enabled"])
            self.assertIsNone(row["native_reward_delivery_binding"])
            for stage in row["stages"]:
                self.assertFalse(stage["runtime_enabled"])
                if key == "oteryn:quest.adventures_of_galthen_quest" and stage["stage_key"] in {"s5", "s8"}:
                    stage_key = stage["stage_key"]
                    predecessor, creature = {"s5": ("s4", "ahau"), "s8": ("s7", "mitmah_vanguard")}[stage_key]
                    self.assertEqual(stage["native_dispatch_binding"], {
                        "owner": "native_kills",
                        "creature": {"family": "Creature", "key": "oteryn:creature."+creature, "revision": "definition-r1"},
                        "transition": "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/"+stage_key,
                        "predecessor": {"track": "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/"+predecessor, "value": 1},
                    })
                else:
                    self.assertIsNone(stage["native_dispatch_binding"])
                self.assertIsNone(stage["selected_NPC_branch"])
                self.assertIsNone(stage["NPC_dialogue_candidates"])

    def test_source_talk_stages_keep_dialogue_hold(self):
        talks = []
        for key in self.source_keys:
            for stage in self.by_quest[key]["stages"]:
                intent = stage["event_identity_associations"]
                if intent["kind"] == "talk":
                    talks.append(intent)
        self.assertEqual(251, len(talks))
        self.assertTrue(all("DIALOGUE_LANE_OWNS_TALK_BINDING" in row["unresolved"] for row in talks))

    def test_authored_dialogue_candidates_are_preserved(self):
        authored = [row for row in self.plan["records"] if row["quest"] not in self.source_keys]
        self.assertEqual(68, len(authored))
        talks = [
            stage for row in authored for stage in row["stages"]
            if stage["event_identity_associations"]["kind"] == "talk"
        ]
        self.assertEqual(79, len(talks))
        self.assertTrue(all(stage["NPC_dialogue_candidates"] is not None for stage in talks))

    def test_only_two_selected_native_kills_are_bound_and_rewards_remain_held(self):
        bound = {(row["quest"], stage["stage_key"])
                 for row in self.plan["records"] for stage in row["stages"]
                 if stage["native_dispatch_binding"] is not None}
        self.assertEqual(bound, {
            ("oteryn:quest.adventures_of_galthen_quest", "s5"),
            ("oteryn:quest.adventures_of_galthen_quest", "s8"),
        })
    def test_only_registered_native_talk_stage_is_enabled(self):
        bound = [(row["quest"], stage) for row in self.plan["records"]
                 for stage in row["stages"] if stage["native_dispatch_binding"] is not None]
        self.assertEqual(1, len(bound))
        quest, stage = bound[0]
        self.assertEqual("oteryn:quest.authored.a_pirate_s_death_to_me", quest)
        self.assertEqual("s1", stage["stage_key"])
        self.assertTrue(stage["runtime_enabled"])
        self.assertEqual("npc_dialogue", stage["native_dispatch_binding"]["kind"])
        self.assertEqual(
            0,
            sum(row["native_reward_delivery_binding"] is not None for row in self.plan["records"]),
        )


if __name__ == "__main__":
    unittest.main()
