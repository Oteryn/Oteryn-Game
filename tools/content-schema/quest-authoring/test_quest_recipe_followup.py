import copy
import json
import unittest
from pathlib import Path

import quest_recipe_followup as followup
from quest_recipe_refinements import apply_refinements


class FollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[3]
        packet = json.loads((cls.root / 'tools/content-schema/quest-authoring/samples/completion242/recipes.json').read_text())
        cls.rows, _ = apply_refinements(cls.root, packet)

    def test_only_task_board_action_changes(self):
        rows, provenance = followup.apply(self.root, self.rows)
        before = copy.deepcopy(self.rows)
        selected = next(r for r in before if r['identity']['key'] == 'oteryn:quest.falconer_outfits_quest')
        self.assertEqual(selected['recipe']['stages'][0]['targets'], ['Task Board'])
        selected['recipe']['stages'][0]['kind'] = 'use'
        self.assertEqual(rows, before)
        self.assertEqual(provenance['sha256'], followup.SHA256)
        self.assertEqual(next(r for r in self.rows if r['identity']['key'] == selected['identity']['key'])['recipe']['stages'][0]['kind'], 'talk')

    def test_rejects_stale_recipe(self):
        rows = copy.deepcopy(self.rows)
        next(r for r in rows if r['identity']['key'] == 'oteryn:quest.falconer_outfits_quest')['recipe']['stages'][0]['count'] += 1
        with self.assertRaisesRegex(ValueError, 'recipe fence'):
            followup.apply(self.root, rows)

    def test_rejects_missing_owner(self):
        rows = [r for r in self.rows if r['identity']['key'] != 'oteryn:quest.falconer_outfits_quest']
        with self.assertRaisesRegex(ValueError, 'recipe fence'):
            followup.apply(self.root, rows)

    def test_rejects_duplicate_owner(self):
        row = next(r for r in self.rows if r['identity']['key'] == 'oteryn:quest.falconer_outfits_quest')
        with self.assertRaisesRegex(ValueError, 'recipe fence'):
            followup.apply(self.root, self.rows + [row])

    def test_rejects_mutated_packet(self):
        class BadPath:
            def __truediv__(self, _):
                return self

            def read_bytes(self):
                return b'{}'

        with self.assertRaisesRegex(ValueError, 'packet differs'):
            followup.apply(BadPath(), self.rows)


if __name__ == '__main__':
    unittest.main()
