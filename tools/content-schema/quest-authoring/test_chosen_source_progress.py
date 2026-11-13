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
        self.assertEqual(1471, summary["tracks"])
        self.assertEqual(1471, summary["transitions"])
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

        self.assertEqual(
            base["counts"]["quests"] + self.packet["summary"]["new_quests"],
            merged["counts"]["quests"],
        )
        self.assertEqual(
            base["counts"]["tracks"] + self.packet["summary"]["tracks"] - 4,
            merged["counts"]["tracks"],
        )
        self.assertEqual(
            base["counts"]["transitions"] + self.packet["summary"]["transitions"] - 2,
            merged["counts"]["transitions"],
        )
        self.assertEqual(
            base["counts"]["completes"] + self.packet["summary"]["completion_transitions"],
            merged["counts"]["completes"],
        )
        self.assertEqual(
            {
                "CHOSEN_SOURCE_TYPED_PROGRESS_ONLY": 146,
                "LOWERED": 6,
                "SOURCE_PLUS_CHOSEN_TYPED_PROGRESS_ONLY": 90,
                "standalone_source_claim": 5,
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
                [row for row in overlay["tracks"]
                 if row["key"] not in {row["key"] for row in source["tracks"]}],
                after["tracks"][len(source["tracks"]) :],
                key,
            )
            self.assertEqual(
                [row for row in overlay["transitions"]
                 if row["key"] not in {row["key"] for row in source["transitions"]}],
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

    def test_native_galthen_overlap_is_exact_and_partial_or_changed_is_refused(self):
        base = json.loads(SOURCE_STATE.read_text(encoding="utf-8"))
        quest = next(q for q in base['quests']
                     if q['quest'] == 'oteryn:quest.adventures_of_galthen_quest')
        overlay = next(q for q in self.packet['overlays'] if q['quest'] == quest['quest'])
        for family, stages in [('tracks', ['s4', 's5', 's7', 's8']),
                               ('transitions', ['s5', 's8'])]:
            intersection = {r['key'] for r in quest[family]} & {r['key'] for r in overlay[family]}
            prefix = ('oteryn:quest-progress/' if family == 'tracks'
                      else 'oteryn:quest-transition/')
            self.assertEqual(intersection, {
                prefix + 'chosen-source/adventures_of_galthen_quest/' + stage for stage in stages})
        for changed in ['track', 'transition', 'partial']:
            fixture = copy.deepcopy(base)
            target = next(q for q in fixture['quests'] if q['quest'] == quest['quest'])
            if changed == 'track':
                next(t for t in target['tracks'] if t['key'].endswith('/s5'))['max'] = 2
            elif changed == 'transition':
                next(t for t in target['transitions'] if t['key'].endswith('/s5'))['effects'][0]['from']['value'] = 0
            else:
                target['transitions'] = [t for t in target['transitions'] if not t['key'].endswith('/s5')]
            with self.assertRaises(ValueError):
                self.builder.merge(fixture, self.packet)


if __name__ == "__main__":
    unittest.main()
