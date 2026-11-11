"""The Thaian merge must not open the seasonal source inventory to arbitrary additions."""
import copy
import json
from pathlib import Path
import unittest

import refresh_season_creature_inventory as inventory


class SeasonalInventory(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = Path(__file__).resolve().parents[2]
        read = lambda path: json.loads((cls.root / path).read_bytes())
        cls.rows = [row for path in read('content/creatures/definitions/index.json')['shards']
                    for row in read(path)['records']]
        cls.native = [row for row in read('content/creatures/definitions/spell-native-profiles.json')['records']
                      if row['profile']['target']['key'].startswith('oteryn:creature.')]
        cls.schedules = sum(len(row['behavior']['data']['profile'].get(key, []))
                            for row in cls.native for key in ('attacks', 'defenses'))

    def test_exact_merged_cohort_qualifies_one_melee_successor(self):
        self.assertEqual(inventory.validate_inventory(
            self.root, self.rows, self.native, self.schedules), 1)

    def test_unrelated_creature_and_schedule_additions_remain_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Non-seasonal canonical inventory'):
            inventory.validate_inventory(self.root, self.rows + [self.rows[0]],
                                         self.native + [self.native[0]], self.schedules)
        with self.assertRaisesRegex(ValueError, 'Non-seasonal canonical inventory'):
            inventory.validate_inventory(self.root, self.rows, self.native, self.schedules + 1)

    def test_same_count_thaian_profile_mutation_remains_rejected(self):
        rows = list(self.rows)
        index = next(i for i, row in enumerate(rows)
                     if row['definition']['identity']['key'] == 'oteryn:creature.thaian')
        rows[index] = copy.deepcopy(rows[index])
        rows[index]['authoring']['profile']['health'] += 1
        with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
            inventory.validate_inventory(self.root, rows, self.native, self.schedules)


if __name__ == '__main__':
    unittest.main()
