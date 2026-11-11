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

    def thaian_rows(self):
        rows = list(self.rows)
        index = next(i for i, row in enumerate(rows)
                     if row['definition']['identity']['key'] == 'oteryn:creature.thaian')
        rows[index] = copy.deepcopy(rows[index])
        return rows, rows[index]

    def test_canonical_omitted_empty_immunities_qualifies_same_source(self):
        from questpc7_thaian_authoring import PACKET
        packet, _ = inventory.cake.read_canonical(self.root / PACKET)
        creature = next(row for row in packet['profiles']
                        if row['target']['family'] == 'Creature')
        self.assertEqual(creature['data']['profile']['immunities'], [])
        rows, thaian = self.thaian_rows()
        self.assertEqual(thaian['authoring']['profile'].pop('immunities', []), [])
        before = copy.deepcopy(thaian)
        self.assertEqual(inventory.validate_inventory(
            self.root, rows, self.native, self.schedules), 1)
        self.assertEqual(thaian, before)

    def test_explicit_empty_immunities_qualifies_same_source(self):
        rows, thaian = self.thaian_rows()
        thaian['authoring']['profile']['immunities'] = []
        self.assertEqual(inventory.validate_inventory(
            self.root, rows, self.native, self.schedules), 1)

    def test_canonical_immunities_fact_changes_remain_rejected(self):
        for immunities in (['physical'], None):
            with self.subTest(immunities=immunities):
                rows, thaian = self.thaian_rows()
                thaian['authoring']['profile']['immunities'] = immunities
                with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                    inventory.validate_inventory(self.root, rows, self.native, self.schedules)

    def test_omitted_empty_immunities_keeps_other_source_guards(self):
        for changed in ('health', 'reference_level', 'identity_revision'):
            with self.subTest(changed=changed):
                rows, thaian = self.thaian_rows()
                profile = thaian['authoring']['profile']
                profile.pop('immunities', None)
                if changed == 'health':
                    profile['health'] += 1
                elif changed == 'reference_level':
                    del profile['details']['reference_level']
                else:
                    thaian['definition']['identity']['revision'] += '-changed'
                with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                    inventory.validate_inventory(self.root, rows, self.native, self.schedules)

    def test_native_empty_immunities_omission_remains_rejected(self):
        rows, thaian = self.thaian_rows()
        thaian['authoring']['profile'].pop('immunities', None)
        native = list(self.native)
        index = next(i for i, row in enumerate(native)
                     if row['profile']['target']['key'] == 'oteryn:creature.thaian')
        native[index] = copy.deepcopy(native[index])
        self.assertEqual(native[index]['profile']['data']['profile'].pop('immunities'), [])
        with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
            inventory.validate_inventory(self.root, rows, native, self.schedules)

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
