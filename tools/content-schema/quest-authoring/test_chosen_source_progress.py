import importlib.util
import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[3]
BUILDER = ROOT / "tools/content-schema/quest-authoring/samples/server-completion/chosen-source-progress/builder.py"
CANDIDATE = ROOT / "content/quests/missions/quest-state-completion-candidate.json"


def load_builder():
    spec = importlib.util.spec_from_file_location("chosen_source_progress", BUILDER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ChosenSourceProgressTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.builder = load_builder()
        cls.packet = cls.builder.build(ROOT)

    def test_closed_source_recipe_population(self):
        self.assertEqual(139, self.packet["summary"]["quests"])
        self.assertEqual(699, self.packet["summary"]["tracks"])
        self.assertEqual(699, self.packet["summary"]["transitions"])
        self.assertEqual(139, self.packet["summary"]["completion_transitions"])
        self.assertEqual(7, self.packet["summary"]["held_quests"])
        self.assertFalse(self.packet["native_admission"])
        self.assertFalse(self.packet["runtime_enabled"])

    def test_seven_non_unit_terminal_recipes_remain_held(self):
        self.assertEqual(
            {
                "oteryn:quest.bear_room_quest",
                "oteryn:quest.behemoth_quest",
                "oteryn:quest.demon_helmet_quest",
                "oteryn:quest.dragon_tower_quest",
                "oteryn:quest.edron_goblin_quest",
                "oteryn:quest.opticording_sphere_quest",
                "oteryn:quest.rift_warrior_outfits_quest",
            },
            {row["quest"] for row in self.packet["held"]},
        )
        self.assertTrue(all(row["reason"] == "Terminal completion count must be one" for row in self.packet["held"]))

    def test_merge_extends_existing_candidate_without_activation(self):
        source = json.loads(CANDIDATE.read_text(encoding="utf-8"))
        incoming = {q["quest"] for q in self.packet["quests"]}
        source = dict(source, quests=[q for q in source["quests"] if q["quest"] not in incoming])
        merged = self.builder.merge(source, self.packet)
        self.assertEqual(303, merged["counts"]["quests"])
        self.assertEqual(2445, merged["counts"]["tracks"])
        self.assertEqual(4378, merged["counts"]["transitions"])
        self.assertEqual(139, merged["counts"]["completion"]["CHOSEN_SOURCE_TYPED_PROGRESS_ONLY"])
        chosen = [q for q in merged["quests"] if isinstance(q["completion"], dict)
                  and q["completion"].get("state") == "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY"]
        self.assertEqual(139, len(chosen))
        self.assertTrue(all(q["completion"]["runtime_enabled"] is False for q in chosen))
        self.assertTrue(all(q["completion"]["source_equivalence"] is False for q in chosen))


if __name__ == "__main__":
    unittest.main()
