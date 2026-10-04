import copy
import json
import os
import unittest
from pathlib import Path
import quest_requirement_followup as f


class RequirementFollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        here = Path(__file__).parent
        root = Path(os.environ.get('QUEST_PRODUCT_ROOT', Path(__file__).resolve().parents[3]))
        cls.rows = json.loads((here / 'samples/recipe-followup/requirement-fixtures.json').read_text())
        cls.packet = json.loads((here / 'samples/recipe-followup/requirements.json').read_text())
        cls.spec = (root / f.SPEC_PATH).read_bytes()

    def test_only_three_chosen_minima_and_notes_change(self):
        result = f.apply_packet(self.rows, self.packet, self.spec)
        expected = copy.deepcopy(self.rows)
        for row in expected:
            recipe = row['definition']['oteryn_recipe']['payload']['recipe']
            fix = next(v for v in self.packet['changes'] if v['canonical_key'] == row['definition']['identity']['key'])
            recipe['requirements']['min_level'] = fix['new_value']
            recipe['source_notes'].append(fix['source_note'])
        self.assertEqual(expected, result)
        self.assertTrue(all(row['definition']['requirements'].get('min_level') is None for row in result))
        self.assertTrue(all({'code': 'requirement_unknown', 'field': 'min_level'} in row['definition']['missing_data'] for row in result))

    def test_missing_owner_rejected(self):
        with self.assertRaisesRegex(ValueError, 'owner differs'):
            f.apply_packet(self.rows[1:], self.packet, self.spec)

    def test_duplicate_owner_rejected(self):
        with self.assertRaisesRegex(ValueError, 'owner differs'):
            f.apply_packet(self.rows + [self.rows[0]], self.packet, self.spec)

    def test_changed_recipe_rejected(self):
        rows = copy.deepcopy(self.rows)
        rows[0]['definition']['oteryn_recipe']['payload']['recipe']['requirements']['min_level'] = 1
        with self.assertRaisesRegex(ValueError, 'recipe fence'):
            f.apply_packet(rows, self.packet, self.spec)

    def test_stale_cached_source_rejected(self):
        with self.assertRaisesRegex(ValueError, 'qualification differs'):
            f.apply_packet(self.rows, self.packet, self.spec + b' ')

    def test_premium_change_not_permitted(self):
        packet = copy.deepcopy(self.packet)
        packet['changes'][0]['path'] = '/requirements/premium'
        with self.assertRaisesRegex(ValueError, 'change differs'):
            f.apply_packet(self.rows, packet, self.spec)

    def test_runtime_promotion_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['runtime_enabled'] = True
        with self.assertRaisesRegex(ValueError, 'qualification differs'):
            f.apply_packet(self.rows, packet, self.spec)


if __name__ == '__main__':
    unittest.main()
