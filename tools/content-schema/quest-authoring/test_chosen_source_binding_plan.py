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
        self.assertEqual({"quests": 295, "stages": 1818}, self.plan["counts"])
        self.assertEqual(227, len(self.source_keys))
        self.assertTrue(self.source_keys <= set(self.by_quest))
        source_rows = [self.by_quest[key] for key in self.source_keys]
        self.assertEqual(1402, sum(len(row["stages"]) for row in source_rows))
        self.assertEqual(486, sum(len(row["reward_identity_associations"]) for row in source_rows))

    def test_source_rows_remain_non_executable(self):
        self.assertFalse(self.plan["runtime_enabled"])
        for key in self.source_keys:
            row = self.by_quest[key]
            self.assertFalse(row["runtime_enabled"])
            self.assertIsNone(row["native_reward_delivery_binding"])
            for stage in row["stages"]:
                self.assertFalse(stage["runtime_enabled"])
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
        self.assertEqual(248, len(talks))
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

    def test_no_native_bindings_anywhere(self):
        self.assertEqual(
            0,
            sum(stage["native_dispatch_binding"] is not None
                for row in self.plan["records"] for stage in row["stages"]),
        )
        self.assertEqual(
            0,
            sum(row["native_reward_delivery_binding"] is not None for row in self.plan["records"]),
        )


if __name__ == "__main__":
    unittest.main()
