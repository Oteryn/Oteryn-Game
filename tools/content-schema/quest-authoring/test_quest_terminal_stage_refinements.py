import copy
import hashlib
import json
import pathlib
import unittest

import quest_recipe_refinements as base_refinements
import quest_terminal_stage_refinements as terminal_refinements


ROOT = pathlib.Path(__file__).resolve().parents[3]
RECIPES = ROOT / "tools/content-schema/quest-authoring/samples/completion242/recipes.json"


class TerminalStageRefinementsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        payload = json.loads(RECIPES.read_text(encoding="utf-8"))
        cls.original = payload
        cls.refined, _ = base_refinements.apply_refinements(ROOT, payload)
        cls.normalized, cls.provenance = terminal_refinements.apply_terminal_normalizations(
            ROOT, cls.refined
        )
        cls.before = {row["identity"]["key"]: row for row in cls.refined}
        cls.after = {row["identity"]["key"]: row for row in cls.normalized}
        cls.keys = set(cls.provenance["canonical_keys"])

    def test_exact_nine_records_are_normalized(self):
        self.assertEqual(9, len(self.keys))
        changed = {
            key for key in self.before
            if terminal_refinements.digest(self.before[key])
            != terminal_refinements.digest(self.after[key])
        }
        self.assertEqual(self.keys, changed)
        self.assertEqual(233, len(self.before) - len(changed))

    def test_all_normalized_recipes_have_unit_terminal_completion(self):
        for key in self.keys:
            recipe = self.after[key]["recipe"]
            terminal = recipe["stages"][-1]
            self.assertEqual("complete", terminal["kind"], key)
            self.assertEqual(1, terminal["count"], key)
            self.assertEqual([], terminal["next"], key)
            self.assertEqual([], terminal["targets"], key)
            self.assertEqual(
                terminal["key"],
                self.after[key]["title_stage_keys"][recipe["wiki_title"]][-1],
                key,
            )

    def test_pre_completion_action_keeps_original_terminal_objective_and_targets(self):
        packet = json.loads(
            (ROOT / terminal_refinements.PACKET).read_text(encoding="utf-8")
        )
        for entry in packet["records"]:
            key = entry["canonical_key"]
            action = self.after[key]["recipe"]["stages"][-2]
            old = entry["old_terminal"]
            self.assertEqual(old["key"], action["key"], key)
            self.assertEqual(old["objective"], action["objective"], key)
            self.assertEqual(old["targets"], action["targets"], key)
            self.assertEqual(entry["terminal_action_kind"], action["kind"], key)
            self.assertEqual(entry["terminal_action_count"], action["count"], key)

    def test_rift_token_handoffs_are_single_dialogue_occurrences(self):
        recipe = self.after[
            "oteryn:quest.rift_warrior_outfits_quest"
        ]["recipe"]
        stages = {stage["key"]: stage for stage in recipe["stages"]}
        self.assertEqual(("talk", 1), (stages["s3"]["kind"], stages["s3"]["count"]))
        self.assertEqual(("collect", 100), (stages["s4"]["kind"], stages["s4"]["count"]))
        self.assertEqual(("talk", 1), (stages["s5"]["kind"], stages["s5"]["count"]))

    def test_ancient_tombs_final_combination_is_one_use_after_seven_trials(self):
        recipe = self.after["oteryn:quest.the_ancient_tombs_quest"]["recipe"]
        stages = {stage["key"]: stage for stage in recipe["stages"]}
        self.assertEqual(("use", 1), (stages["s13"]["kind"], stages["s13"]["count"]))
        self.assertEqual(("complete", 1), (stages["s14"]["kind"], stages["s14"]["count"]))

    def test_reward_and_source_evidence_are_unchanged(self):
        for key in self.keys:
            before = copy.deepcopy(self.before[key])
            after = copy.deepcopy(self.after[key])
            self.assertEqual(before["source_identity"], after["source_identity"], key)
            self.assertEqual(before["donor_source_refs"], after["donor_source_refs"], key)
            self.assertEqual(
                before["recipe"]["reward_intents"],
                after["recipe"]["reward_intents"],
                key,
            )
            self.assertEqual(before["recipe"]["source_refs"], after["recipe"]["source_refs"], key)
            self.assertEqual(before["recipe"]["source_notes"], after["recipe"]["source_notes"], key)

    def test_packet_sha_is_fenced(self):
        raw = (ROOT / terminal_refinements.PACKET).read_bytes()
        self.assertEqual(
            terminal_refinements.APPROVED_SHA256,
            hashlib.sha256(raw).hexdigest(),
        )
        self.assertFalse(self.provenance["runtime_enabled"])
        self.assertTrue(self.provenance["source_holds_preserved"])


if __name__ == "__main__":
    unittest.main()
