"""The current Crystal mana variants must reach the existing potion consumer."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

class CurrentPotionVariants(unittest.TestCase):
    def test_current_variants_have_production_consumption(self):
        rows = {r['production_definition']['production_key']: r for r in json.loads(
            (ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())['records']}
        rows.update({r['production_definition']['production_key']: r for r in json.loads(
            (ROOT / 'content/items/definitions/srx1-item-updates.json').read_bytes())['records']})
        for item, level, low, high, vocations in [
            (53162, 100, 240, 360, ['DRUID', 'MONK', 'PALADIN', 'SORCERER']),
            (53163, 130, 240, 360, None), (53164, 200, 425, 575, None)]:
            with self.subTest(item=item):
                row = rows[f'oteryn:item.tibia.i{item}']
                self.assertTrue(row['admission']['materializable'])
                self.assertEqual(row['semantics']['consumption']['value']['value']['restores'],
                                 [{'resource': 'MANA', 'min': low, 'max': high}])
                requirements = row['semantics']['use_requirements']['value']
                self.assertEqual(requirements['min_level'], {'state': 'KNOWN', 'value': level})
                self.assertEqual(requirements['vocations'], {'state': 'KNOWN', 'value': vocations}
                                 if vocations else {'state': 'NOT_APPLICABLE'})

if __name__ == '__main__':
    unittest.main()
