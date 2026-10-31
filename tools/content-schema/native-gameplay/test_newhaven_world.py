"""Bounded source-backed Newhaven population and item admission regressions."""
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class NewhavenWorldTests(unittest.TestCase):
    def test_plain_walkability_has_all_bounded_newhaven_items(self):
        data = json.loads((ROOT / 'tools/content-schema/native-gameplay/movement-item-profiles.json').read_bytes())
        records = {r['production_definition']['production_key']: r for r in data['records']}
        for item in (650, 33726, 1761):
            self.assertIs(records[f'oteryn:item.tibia.i{item}']['attributes']['blocks_movement'], False)
        self.assertNotIn('oteryn:item.tibia.i22262', records)  # Solid Chief target ornament.

    def test_source_starter_profiles_preserve_admitted_spellbook_defense(self):
        data = json.loads((ROOT / 'tools/content-schema/native-gameplay/newhaven-item-profiles.json').read_bytes())
        records = {r['production_definition']['production_key']: r for r in data['records']}
        book = records['oteryn:item.tibia.i21400']
        self.assertEqual(book['semantics']['weapon']['value']['weapon_type']['value'], 'SHIELD')
        self.assertEqual(book['semantics']['weapon']['value']['defense']['value'], 13)
        for record in records.values():
            self.assertTrue(record['admission']['materializable'])
            for field in ('movable', 'pickupable'):
                self.assertEqual(record['semantics']['physical']['value'][field], {'state': 'KNOWN', 'value': True})

    def test_newhaven_spawn_inputs_restore_six_and_one_chief_without_corruptor(self):
        doc = json.loads((ROOT / 'tools/world-bundle-compiler/newhaven-spawns.json').read_bytes())
        points = [p for r in doc['records'] for p in r['declaration']['points']]
        self.assertEqual(len(points), 7)
        self.assertFalse(any('corruptor' in p['creature'] for p in points))
        self.assertFalse(any(p['cell'] == {'x': 32620, 'y': 32457, 'floor': 7} for p in points))
        chief = [p for p in points if p['creature'].endswith('.muglex_clan_chief')]
        self.assertEqual(chief, [{'cell': {'x': 32527, 'y': 32430, 'floor': 7}, 'creature': 'oteryn:creature.muglex_clan_chief', 'direction': 'north', 'respawn_ms': 60000}])


if __name__ == '__main__':
    unittest.main()
