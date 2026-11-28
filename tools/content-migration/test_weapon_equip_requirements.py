import copy
import json
import unittest

import weapon_equip_requirements as weapons


def record(key, level, vocations):
    unknown = {'state': 'UNKNOWN'}
    return {'production_definition': {'production_key': key}, 'semantics': {'equipment': {
        'state': 'KNOWN', 'value': {'patterns': {'state': 'KNOWN', 'value': [{
            'pattern_id': 1, 'primary_slot': {'state': 'KNOWN', 'value': 'WEAPON'},
            'level': level or unknown, 'vocations': vocations or unknown}]}}}}}


class WeaponEquipRequirements(unittest.TestCase):
    def setUp(self):
        self.profiles = {'records': [record('a', None, None), record('b', {'state': 'KNOWN', 'value': 25}, None),
                                     record('c', None, {'state': 'KNOWN', 'value': ['KNIGHT']})]}
        self.facts = {'schema': weapons.SCHEMA, 'records': [
            {'key': 'a', 'level': 0, 'vocations': []}, {'key': 'b', 'level': 25, 'vocations': []},
            {'key': 'c', 'level': 0, 'vocations': ['KNIGHT']}]}

    def test_fills_only_unknown_fields_and_is_idempotent(self):
        before = copy.deepcopy(self.profiles)
        result = weapons.complete(self.profiles, self.facts)
        self.assertEqual(self.profiles, before)
        for row in result['records']:
            pattern = row['semantics']['equipment']['value']['patterns']['value'][0]
            self.assertEqual(pattern['level']['state'], 'KNOWN')
            self.assertEqual(pattern['vocations']['state'], 'KNOWN')
        self.assertEqual(weapons.complete(result, self.facts), result)

    def test_known_conflict_refuses(self):
        self.facts['records'][1]['level'] = 30
        with self.assertRaisesRegex(ValueError, 'KNOWN_CONFLICT'):
            weapons.complete(self.profiles, self.facts)

    def test_shipped_profile_carries_every_fact(self):
        facts = json.loads((weapons.ROOT / weapons.FACTS).read_bytes())
        profiles = json.loads((weapons.ROOT / weapons.PROFILE).read_bytes())
        self.assertEqual(len(facts['records']), 91)
        self.assertEqual(weapons.complete(profiles, facts), profiles)
        by_key = {r['production_definition']['production_key']: r for r in profiles['records']}
        for source_id in (3276, 3274):
            pattern = by_key[f'oteryn:item.tibia.i{source_id}']['semantics']['equipment']['value']['patterns']['value'][0]
            self.assertEqual((pattern['level'], pattern['vocations']),
                             ({'state': 'KNOWN', 'value': 0}, {'state': 'KNOWN', 'value': []}))


if __name__ == '__main__':
    unittest.main()
