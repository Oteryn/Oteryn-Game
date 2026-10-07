import copy
import importlib.util
import json
import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[3]
BUILDER = ROOT / "tools/content-schema/quest-authoring/samples/server-completion/chosen-source-progress/builder.py"
SOURCE_STATE = ROOT / "content/quests/missions/quest-state.json"


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
        summary = self.packet["summary"]
        self.assertEqual(236, summary["quests"])
        self.assertEqual(146, summary["new_quests"])
        self.assertEqual(90, summary["overlay_quests"])
        self.assertEqual(6, summary["source_lowered_skipped"])
        self.assertEqual(1475, summary["tracks"])
        self.assertEqual(1475, summary["transitions"])
        self.assertEqual(236, summary["completion_transitions"])
        self.assertEqual(0, summary["held_quests"])
        self.assertFalse(self.packet["native_admission"])
        self.assertFalse(self.packet["runtime_enabled"])

    def test_nine_terminal_count_recipes_are_normalized_only_in_candidate_projection(self):
        expected = {
            "oteryn:quest.barbarian_arena_quest",
            "oteryn:quest.bear_room_quest",
            "oteryn:quest.behemoth_quest",
            "oteryn:quest.demon_helmet_quest",
            "oteryn:quest.dragon_tower_quest",
            "oteryn:quest.edron_goblin_quest",
            "oteryn:quest.opticording_sphere_quest",
            "oteryn:quest.rift_warrior_outfits_quest",
            "oteryn:quest.the_ancient_tombs_quest",
        }
        self.assertEqual([], self.packet["held"])
        projected = self.packet["quests"] + self.packet["overlays"]
        normalized = {
            row["quest"]
            for row in projected
            if row["completion"].get("terminal_stage_normalization") is not None
        }
        self.assertEqual(expected, normalized)
        for row in projected:
            if row["quest"] not in expected:
                continue
            provenance = row["completion"]["terminal_stage_normalization"]
            self.assertFalse(provenance["runtime_enabled"])
            self.assertTrue(provenance["source_holds_preserved"])
            self.assertEqual(row["quest"], provenance["canonical_key"])
            self.assertEqual("complete", row["transitions"][-1]["source"]["chosen_stage"]["kind"])
            self.assertEqual(1, row["transitions"][-1]["source"]["chosen_stage"]["count"])

    def test_six_source_completed_quests_are_not_overlaid(self):
        self.assertEqual(
            {
                "oteryn:quest.hot_cuisine_quest",
                "oteryn:quest.oramond_quest",
                "oteryn:quest.sam_s_old_backpack_quest",
                "oteryn:quest.spirithunters_quest",
                "oteryn:quest.the_ape_city_quest",
                "oteryn:quest.the_outlaw_camp_quest",
            },
            {row["quest"] for row in self.packet["skipped_source_lowered"]},
        )
        self.assertTrue(
            all(
                row["source_completion_state"] == "LOWERED"
                for row in self.packet["skipped_source_lowered"]
            )
        )

    def test_merge_adds_overlay_without_mutating_source_prefix(self):
        base = json.loads(SOURCE_STATE.read_text(encoding="utf-8"))
        before = {quest["quest"]: copy.deepcopy(quest) for quest in base["quests"]}

        merged = self.builder.merge(base, self.packet)

        self.assertEqual(242, merged["counts"]["quests"])
        self.assertEqual(2803, merged["counts"]["tracks"])
        self.assertEqual(4738, merged["counts"]["transitions"])
        self.assertEqual(240, merged["counts"]["completes"])
        self.assertEqual(
            {
                "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY": 146,
                "LOWERED": 6,
                "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY": 90,
            },
            merged["counts"]["completion"],
        )

        by_owner = {quest["quest"]: quest for quest in merged["quests"]}
        for overlay in self.packet["overlays"]:
            key = overlay["quest"]
            source = before[key]
            after = by_owner[key]
            self.assertEqual(
                source["tracks"],
                after["tracks"][: len(source["tracks"])],
                key,
            )
            self.assertEqual(
                source["transitions"],
                after["transitions"][: len(source["transitions"])],
                key,
            )
            self.assertEqual(
                overlay["tracks"],
                after["tracks"][len(source["tracks"]) :],
                key,
            )
            self.assertEqual(
                overlay["transitions"],
                after["transitions"][len(source["transitions"]) :],
                key,
            )
            self.assertEqual(
                "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY",
                after["completion"]["state"],
            )
            self.assertTrue(after["completion"]["source_progress_preserved"])
            self.assertFalse(after["completion"]["runtime_enabled"])
            self.assertFalse(after["completion"]["source_equivalence"])


if __name__ == "__main__":
    unittest.main()