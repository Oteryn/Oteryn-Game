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
        extra_key = 'oteryn:creature.unqualified_inventory_fixture'
        extra_row = copy.deepcopy(self.rows[0])
        extra_native = copy.deepcopy(self.native[0])
        extra_row['definition']['identity']['key'] = extra_key
        extra_native['profile']['target']['key'] = extra_key
        with self.assertRaisesRegex(ValueError, 'Non-seasonal canonical inventory'):
            inventory.validate_inventory(self.root, self.rows + [extra_row],
                                         self.native + [extra_native], self.schedules)
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


    def _rows_with_source_authoring(self, *, canonical):
        from questpc7_thaian_authoring import PACKET
        packet = json.loads((self.root / PACKET).read_bytes())
        source, = [row for row in packet['profiles']
                   if row['target']['family'] == 'Creature']
        rows = list(self.rows)
        index = next(i for i, row in enumerate(rows)
                     if row['definition']['identity']['key'] == 'oteryn:creature.thaian')
        rows[index] = copy.deepcopy(rows[index])
        rows[index]['authoring'] = copy.deepcopy(source['data'])
        if canonical:
            profile = rows[index]['authoring']['profile']
            self.assertEqual(profile['immunities'], [])
            self.assertIsNone(profile['details']['reference_level'])
            del profile['immunities']
        return rows, index

    def test_tree_projection_preserves_source_closure(self):
        rows, index = self._rows_with_source_authoring(canonical=True)
        profile = rows[index]['authoring']['profile']
        self.assertNotIn('immunities', profile)
        self.assertIsNone(profile['details']['reference_level'])
        before = copy.deepcopy(rows[index])
        native, = [row for row in self.native
                   if row['profile']['target']['key'] == 'oteryn:creature.thaian']
        native_before = copy.deepcopy(native)
        self.assertEqual(inventory.validate_inventory(
            self.root, rows, self.native, self.schedules), 1)
        self.assertEqual(rows[index], before)
        self.assertEqual(native, native_before)
        self.assertEqual(native['profile']['data']['profile']['immunities'], [])
        self.assertIsNone(native['profile']['data']['profile']['details']['reference_level'])

    def test_unprojected_reference_level_omissions_are_rejected(self):
        for fields in (('reference_level',), ('immunities', 'reference_level')):
            with self.subTest(fields=fields):
                rows, index = self._rows_with_source_authoring(canonical=False)
                profile = rows[index]['authoring']['profile']
                for field in fields:
                    del (profile if field == 'immunities' else profile['details'])[field]
                with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                    inventory.validate_inventory(
                        self.root, rows, self.native, self.schedules)

    def test_nondefault_canonical_fields_are_not_normalized_away(self):
        for canonical in (False, True):
            for field, value in (('immunities', ['fire']), ('immunities', None),
                                 ('immunities', 'fire'), ('reference_level', 1),
                                 ('reference_level', False), ('reference_level', [])):
                with self.subTest(canonical=canonical, field=field):
                    rows, index = self._rows_with_source_authoring(canonical=canonical)
                    profile = rows[index]['authoring']['profile']
                    target = profile if field == 'immunities' else profile['details']
                    target[field] = value
                    with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                        inventory.validate_inventory(
                            self.root, rows, self.native, self.schedules)

    def test_native_source_default_bytes_remain_exact(self):
        for fields in (('immunities',), ('reference_level',),
                       ('immunities', 'reference_level')):
            with self.subTest(fields=fields):
                rows, _ = self._rows_with_source_authoring(canonical=True)
                native = list(self.native)
                index = next(i for i, row in enumerate(native)
                             if row['profile']['target']['key'] == 'oteryn:creature.thaian')
                native[index] = copy.deepcopy(native[index])
                profile = native[index]['profile']['data']['profile']
                for field in fields:
                    del (profile if field == 'immunities' else profile['details'])[field]
                with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                    inventory.validate_inventory(
                        self.root, rows, native, self.schedules)

    def test_missing_or_duplicate_thaian_records_remain_rejected(self):
        rows, index = self._rows_with_source_authoring(canonical=True)
        native_index = next(i for i, row in enumerate(self.native)
                            if row['profile']['target']['key'] == 'oteryn:creature.thaian')
        cases = {
            'missing_canonical': (rows[:index] + rows[index + 1:], self.native),
            'duplicate_canonical': (rows + [rows[index]], self.native),
            'missing_native': (rows, self.native[:native_index] + self.native[native_index + 1:]),
            'duplicate_native': (rows, self.native + [self.native[native_index]]),
        }
        for name, (candidate_rows, candidate_native) in cases.items():
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
                    inventory.validate_inventory(
                        self.root, candidate_rows, candidate_native, self.schedules)

    def test_closure_diagnostics_preserve_complete_operands(self):
        from questpc7_thaian_authoring import PACKET
        packet = json.loads((self.root / PACKET).read_bytes())
        rows, index = self._rows_with_source_authoring(canonical=True)
        expected_canonical = copy.deepcopy(rows[index]['authoring'])
        rows[index]['authoring']['profile']['health'] += 1
        messages = []
        for _ in range(2):
            with self.assertRaisesRegex(ValueError, '^Thaian source authoring closure: ') as raised:
                inventory.validate_inventory(
                    self.root, rows, self.native, self.schedules)
            messages.append(str(raised.exception))
        self.assertEqual(messages[0], messages[1])
        payload = json.loads(messages[0].split(': ', 1)[1])
        source_profiles = {row['target']['family']: row for row in packet['profiles']}
        source_records = {row['identity']['family']: row for row in packet['records']}
        self.assertEqual(payload['source'], packet['source'])
        self.assertEqual(payload['counts'], {'canonical': 1, 'native': 1})
        self.assertEqual(payload['expected'], {
            'definition': source_records['Creature'],
            'canonical_authoring_forms': [
                source_profiles['Creature']['data'], expected_canonical],
            'native_profile': source_profiles['Creature'],
            'native_behavior': source_profiles['Behavior'],
        })
        self.assertEqual(payload['observed'], {
            'canonical': [rows[index]],
            'native': [row for row in self.native
                       if row['profile']['target']['key'] == 'oteryn:creature.thaian'],
        })

    def test_native_behavior_remains_raw_source_exact(self):
        rows, _ = self._rows_with_source_authoring(canonical=True)
        native = list(self.native)
        index = next(i for i, row in enumerate(native)
                     if row['profile']['target']['key'] == 'oteryn:creature.thaian')
        native[index] = copy.deepcopy(native[index])
        native[index]['behavior']['data']['profile']['attacks'][0]['interval_ms'] += 1
        with self.assertRaisesRegex(ValueError, 'Thaian source authoring closure'):
            inventory.validate_inventory(self.root, rows, native, self.schedules)


if __name__ == '__main__':
    unittest.main()
