"""The published Ahau boss cell keeps the donor's passable throne physics."""
import json
import copy
from pathlib import Path
import unittest
import galthen_boss_furniture as producer

ROOT = Path(__file__).resolve().parents[2]


class GalthenFurnitureTests(unittest.TestCase):
    def test_known_throne_policy_cannot_be_replaced(self):
        packet = json.loads((ROOT / producer.PATH).read_bytes())
        record = copy.deepcopy(packet['records'][0])
        record['attributes']['blocks_movement'] = True
        generated = {'profiles': json.dumps({'records': [record]}).encode(),
                     'manifest': b'{"item_profiles":{}}'}
        with self.assertRaisesRegex(ValueError, 'M25_THRONE_KNOWN_POLICY_CONFLICT'):
            producer.admit(ROOT, generated, 'profiles', 'manifest',
                           lambda row: row['production_definition']['production_key'],
                           producer.base.encode)

    def test_actual_ahau_throne_has_complete_ground_only_source_policy(self):
        document = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
        rows = [row for row in document['records']
                if row['production_definition']['production_key'] == 'oteryn:item.tibia.i31483']
        self.assertEqual(len(rows), 1, 'the real Ahau boss cell must not retain UNKNOWN solidity')
        row = rows[0]
        self.assertEqual(row, json.loads((ROOT / producer.PATH).read_bytes())['records'][0],
                         'later publishers must preserve the complete source record')
        self.assertIs(row['attributes']['blocks_movement'], False)
        self.assertIs(row['attributes']['blocks_projectile'], False)
        self.assertEqual(row['admission'], {'materializable': True,
                         'stack_class': 'NonStackable', 'legal_destinations': ['Ground']})
        self.assertEqual(row['semantics']['physical'], {'state': 'KNOWN', 'value': {
            'weight': {'state': 'KNOWN', 'value': 0},
            'movable': {'state': 'KNOWN', 'value': False},
            'pickupable': {'state': 'KNOWN', 'value': False}}})
        room = json.loads((ROOT / 'content/encounters/galthen-ahau/source-room.json').read_bytes())
        boss = next(tile for tile in room['tiles'] if tile['source_position'] == [34008, 31696, 10])
        self.assertIn(31483, [item['server_item_id'] for item in boss['items']])
        self.assertIs(boss['semantics']['flags']['block_solid'], False)


if __name__ == '__main__':
    unittest.main()
