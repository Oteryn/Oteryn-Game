"""Ordered contact outputs must feed the admitted Riddler's source guards."""
import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
PREFIX = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/"


class ParadoxStoryRiddlerTests(unittest.TestCase):
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
                self.assertEqual((source[0]["minimum"], source[0]["maximum"]), before)
                self.assertEqual(source[0]["set"], after)


if __name__ == "__main__":
    unittest.main()
