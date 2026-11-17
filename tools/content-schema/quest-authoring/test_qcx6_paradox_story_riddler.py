"""Ordered contact outputs must feed the admitted Riddler's source guards."""
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PREFIX = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/"
SYNC = "oteryn:quest-progress/l6/paradox/source-synced"


class ParadoxStoryRiddlerTests(unittest.TestCase):
    def callbacks(self):
        rows = json.loads((ROOT / "content/quests/npc-callbacks/l6-stories.json").read_text())["callbacks"]
        return {row["key"].removeprefix("oteryn:quest-reward/l6-paradox-"): row for row in rows}

    def apply(self, callback, values):
        for effect in callback["storage"]:
            current = values.get(effect["track"], -1)
            if not effect["minimum"] <= current <= effect["maximum"]:
                return False
        for effect in callback["storage"]:
            key = effect["track"]
            current = values.get(key, -1)
            if not effect.get("preserve"):
                values[key] = max(current, effect["set"]) if effect.get("at_least") else effect["set"]
        return True

    def test_ordered_contacts_write_the_riddler_prerequisites(self):
        callbacks = json.loads((ROOT / "content/quests/npc-callbacks/l6-stories.json").read_text())["callbacks"]
        by_key = {row["key"]: row for row in callbacks}
        for contact, suffix, before, after in [
            ("oldrak", "the_feared_hugo", (-1, 0), 1),
            ("zoltan", "the_feared_hugo", (1, 1), 2),
            ("padreia", "the_feared_hugo", (2, 2), 3),
            ("lubo", "the_feared_hugo", (3, 3), 4),
            ("prisoner-lesson", "mathemagics", (-1, 0), 1),
        ]:
            with self.subTest(contact=contact):
                row = by_key["oteryn:quest-reward/l6-paradox-" + contact]
                source = [effect for effect in row["storage"] if effect["track"] == PREFIX + suffix]
                self.assertEqual(len(source), 1, "contact must write the source track read by Riddler")
                self.assertEqual(source[0]["minimum"], -1)
                self.assertGreaterEqual(source[0]["maximum"], 4)
                self.assertEqual(source[0]["set"], after)
                self.assertTrue(source[0].get("at_least"), "higher source progress must survive")

    def test_existing_mid_story_and_higher_source_progress_remain_usable(self):
        callbacks = self.callbacks()
        contacts = ["oldrak", "zoltan", "padreia", "lubo", "prisoner-lesson"]
        story = "oteryn:quest-progress/l6/paradox/story"
        for stage in range(1, 5):
            with self.subTest(legacy_stage=stage):
                values = {story: stage}
                for contact in contacts[stage:]:
                    self.assertTrue(self.apply(callbacks[contact], values), contact)
                self.assertEqual((values[PREFIX + "the_feared_hugo"], values[PREFIX + "mathemagics"]), (4, 1))
        values = {story: -1, PREFIX + "the_feared_hugo": 4, PREFIX + "mathemagics": 2}
        for contact in contacts:
            self.assertTrue(self.apply(callbacks[contact], values), contact)
        self.assertEqual((values[PREFIX + "the_feared_hugo"], values[PREFIX + "mathemagics"]), (4, 2))

    def test_completed_legacy_lesson_can_catch_up_without_another_reward(self):
        callback = self.callbacks().get("source-catch-up")
        self.assertIsNotNone(callback, "old lesson receipts need a separate compatibility binding")
        story = "oteryn:quest-progress/l6/paradox/story"
        for stage in range(-1, 5):
            self.assertFalse(self.apply(callback, {story: stage}))
        for existing in ({}, {PREFIX + "the_feared_hugo": 4, PREFIX + "mathemagics": 2}):
            values = {story: 5, **existing}
            self.assertTrue(self.apply(callback, values))
            self.assertEqual(values[story], 5)
            self.assertEqual(values[PREFIX + "the_feared_hugo"], 4)
            self.assertEqual(values[PREFIX + "mathemagics"], existing.get(PREFIX + "mathemagics", 1))
        complete = {story: 4}
        self.assertTrue(self.apply(self.callbacks()["prisoner-lesson"], complete))
        self.assertEqual(complete[SYNC], 1)
        self.assertFalse(self.apply(callback, complete), "fresh lesson replay must create no catch-up receipt")
        legacy = {story: 5}
        self.assertTrue(self.apply(callback, legacy))
        self.assertEqual(legacy[SYNC], 1)
        self.assertFalse(self.apply(callback, legacy), "legacy repair is once-only")
        self.assertEqual(callback["npc"], "oteryn:npc.a_prisoner")
        self.assertEqual(callback["keywords"], ["math"])
        self.assertEqual(callback["topic"], 0)
        self.assertEqual(callback["take"], [])
        for field in ("give", "outfit", "achievement", "experience", "fee"):
            self.assertIsNone(callback.get(field), field)


if __name__ == "__main__":
    unittest.main()
