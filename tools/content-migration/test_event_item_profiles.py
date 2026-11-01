"""Event admission must preserve definitions admitted by established owners."""
import json
from pathlib import Path
import unittest

import register_spell_families

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'content/items/definitions/spell-native-profiles.json'


class EventItemProfilePrecedence(unittest.TestCase):
    def test_task_board_weapon_semantics_survive_event_registration(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/task-board-item-profiles.json').read_bytes())
        expected = {row['production_definition']['production_key']: row
                    for row in source['records']}
        generated = register_spell_families.outputs(ROOT)
        actual = {row['production_definition']['production_key']: row
                  for row in json.loads(generated[PROFILE])['records']}
        for item in (21178, 21179, 21180):
            key = f'oteryn:item.tibia.i{item}'
            self.assertEqual(actual[key], expected[key], key)

    def test_yasir_doors_keep_exact_source_geometry_without_global_item_admission(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        families = source['world_door_families']
        self.assertEqual([(row['locked'], row['closed'], row['open']) for row in families],
                         [(4912, 5007, 4911), (4913, 5006, 4914)])
        self.assertEqual([row['donor'] for row in families],
                         ['data/libs/tables/doors.lua:17', 'data/libs/tables/doors.lua:18'])
        records = {row['production_definition']['production_key']: row for row in source['records']}
        physical = {row['server_item_id']: row for row in source['world_map_physical']}
        generated = register_spell_families.outputs(ROOT)
        native = {row['production_definition']['production_key']
                  for row in json.loads(generated[PROFILE])['records']}
        for item in [4911, 4912, 4913, 4914, 5006, 5007]:
            key = f'oteryn:item.tibia.i{item}'
            self.assertNotIn(key, records)
            if item in (4912, 5006):
                # Both placed door definitions have UNKNOWN native physical policy
                # on the exact base; source geometry must not promote that policy.
                self.assertNotIn(key, native)
            self.assertIn(item, source['projection_only_item_ids'])
            self.assertEqual(physical[item]['dynamic_kind'], 'door')
            self.assertFalse(physical[item]['pickupable'])
            self.assertFalse(physical[item]['movable'])
            self.assertIn({'key': 'type', 'value': 'door'}, physical[item]['xml_attributes'])

    def test_every_pinned_world_layer_item_has_a_physical_profile_or_explicit_gap(self):
        layers = json.loads((ROOT / 'content/encounters/world-changes/layers.json').read_bytes())
        expected = {entry[0] for layer in layers['layers']
                    for tile in layer['tiles'] for entry in tile['entries']}
        self.assertEqual(len(layers['layers']), 16)
        self.assertEqual(len(expected), 492)
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        self.assertEqual(source['world_layer_item_ids'], sorted(expected))
        records = {int(row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i'))
                   for row in source['records']}
        gaps = {row['item_id']: row for row in source['unavailable']}
        projection = set(source['projection_only_item_ids'])
        self.assertFalse(records & gaps.keys())
        self.assertFalse(records & projection)
        self.assertTrue(expected <= records | projection | gaps.keys())
        self.assertEqual(source['native_item_profile_ids'], sorted(records))
        for item in expected & gaps.keys():
            self.assertEqual(gaps[item]['reason'], 'EVENT_ITEM_PHYSICAL_DEFINITION_UNAVAILABLE')
            self.assertTrue(any('.otbm@' in donor for donor in gaps[item]['donor']))
        novelty = json.loads((ROOT / 'tools/content-schema/native-gameplay/novelty-item-actions.json').read_bytes())
        events = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-actions.json').read_bytes())
        self.assertEqual(events['seasonal_inventory_item_ids'],
                         [12143, 14756, 20347, 20350, 28484, 28485, 28486])
        self.assertEqual(events['seasonal_item_ids'],
                         sorted(set(events['seasonal_inventory_item_ids']) | set(events['seasonal_map_item_ids'])))
        self.assertTrue(set(events['seasonal_inventory_item_ids']) <= records)
        self.assertEqual(novelty['map_action_item_ids'],
                         [306, 324, 330, 647, 3588, 3699, 3700, 13842, 13843, 13844, 14029, 14031, 14032])
        self.assertTrue(set(novelty['map_action_item_ids']) <= records)
        self.assertTrue({row[field] for row in novelty['house_transforms']
                         for field in ('item_id', 'target')} <= records)
        expected_map = expected | set(novelty['map_item_ids']) | set(events['seasonal_map_item_ids']) | {
            row[state] for row in source['world_door_families'] for state in ('locked', 'closed', 'open')}
        physical = source['world_map_physical']
        self.assertEqual([row['server_item_id'] for row in physical], sorted(expected_map - gaps.keys()))
        for row in physical:
            self.assertTrue({'ground', 'ground_speed', 'block_solid', 'block_projectile',
                             'block_pathfind', 'movable', 'pickupable', 'container', 'floor_change',
                             'floor_change_kind', 'has_height', 'dynamic_kind', 'magic_field',
                             'xml_attributes'} <= row.keys())
            if row['pickupable']:
                self.assertIn(row['server_item_id'], records)
            if row['server_item_id'] in projection:
                self.assertFalse(row['pickupable'])
        # Both actual pickupable daily appearances retain full native policy, even
        # while the other daily source objects retain map-only source facts.
        for item in [3020, 3655]:
            record = next(row for row in source['records']
                          if row['production_definition']['production_key'] == f'oteryn:item.tibia.i{item}')
            self.assertTrue(record['admission']['materializable'])
            self.assertIn('CharacterInventory', record['admission']['legal_destinations'])

    def test_horse_return_floor_uses_pinned_station_source_and_physical_profile(self):
        actions = json.loads((ROOT / 'tools/content-schema/native-gameplay/novelty-item-actions.json').read_bytes())
        self.assertIn(31159, actions['map_item_ids'])
        self.assertTrue(any(row['path'] == 'data-global/scripts/globalevents/others/horse_station.lua'
                            and len(row['sha256']) == 64 for row in actions['sources']))
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        self.assertFalse(any(row['production_definition']['production_key'] == 'oteryn:item.tibia.i31159'
                             for row in source['records']))
        self.assertIn(31159, source['projection_only_item_ids'])
        physical = next(row for row in source['world_map_physical'] if row['server_item_id'] == 31159)
        self.assertFalse(physical['pickupable'])
        self.assertFalse(physical['movable'])
        self.assertFalse(any(row['item_id'] == 31159 for row in source['unavailable']))

    def test_cake_bridge_source_facts_do_not_admit_missing_canonical_identity(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        captured = source['appearance_only_items']
        self.assertEqual([row['item_id'] for row in captured], [48227])
        self.assertTrue(captured[0]['physical']['ground'])
        self.assertEqual(captured[0]['physical']['ground_speed'], 150)
        self.assertFalse(captured[0]['physical']['movable'])
        self.assertFalse(captured[0]['physical']['pickupable'])
        self.assertFalse(any(row['server_item_id'] == 48227 for row in source['world_map_physical']))
        self.assertFalse(any(row['production_definition']['production_key'] == 'oteryn:item.tibia.i48227'
                             for row in source['records']))
        gap = next(row for row in source['unavailable'] if row['item_id'] == 48227)
        self.assertEqual(gap, {
            'item_id': 48227,
            'production_key': 'oteryn:item.tibia.i48227',
            'reason': 'EVENT_ITEM_CANONICAL_IDENTITY_UNAVAILABLE',
            'donor': ['data-global/scripts/lib/a_piece_of_cake_config.lua:249-292'],
            'appearance_present': True,
            'decision': 'https://github.com/Oteryn/Oteryn-Game/pull/2781#issuecomment-6549960114',
        })
        evidence = source['canonical_identity_unavailable_evidence']
        self.assertEqual(len(evidence), 1)
        self.assertEqual(evidence[0]['item_id'], 48227)
        self.assertEqual(evidence[0]['reason'], gap['reason'])
        self.assertEqual(evidence[0]['decision'], gap['decision'])
        self.assertEqual(evidence[0]['source_revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
        self.assertEqual(len(evidence[0]['sources']), 8)
        self.assertTrue(all(len(row['sha256']) == 64 for row in evidence[0]['sources']))
        self.assertFalse({'bindings', 'target', 'policy', 'index'} & evidence[0].keys())
        # ANSWER6549960114 preserves the canonical key section and sorted reference
        # contract. Neither observed physical flags nor the producer allocates a key.
        keys = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-keys.json').read_bytes())
        self.assertNotIn('oteryn:item.tibia.i48227', [row[0] for row in keys['records']])

    def test_spiritual_horseshoe_source_rule_survives_without_invented_item_profile(self):
        source = json.loads((ROOT / 'tools/content-schema/native-gameplay/event-item-profiles.json').read_bytes())
        unavailable = source['unavailable']
        self.assertEqual([row['item_id'] for row in unavailable], [44048, 48227])
        self.assertEqual(unavailable[0]['reason'], 'EVENT_ITEM_PHYSICAL_DEFINITION_UNAVAILABLE')
        self.assertTrue(unavailable[0]['appearance_present'])
        self.assertFalse(any(row['production_definition']['production_key'] == 'oteryn:item.tibia.i44048'
                             for row in source['records']))
        actions = json.loads((ROOT / 'tools/content-schema/native-gameplay/item-cosmetic-actions.json').read_bytes())
        self.assertTrue(any(row['item_id'] == 44048 for row in actions['rules']))

    def test_firewalker_boots_keep_canonical_semantics_during_registration(self):
        key = 'oteryn:item.tibia.i9019'
        definitions = []
        for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
            definitions.extend(row['definition'] for row in json.loads(path.read_bytes())['records']
                               if row['definition']['identity']['key'] == key)
        self.assertEqual(len(definitions), 1)
        generated = register_spell_families.outputs(ROOT)
        records = [row for row in json.loads(generated[PROFILE])['records']
                   if row['production_definition']['production_key'] == key]
        self.assertEqual(len(records), 1)
        # Compare the complete semantic value, including armor, market policy and
        # EQUIP transformation, rather than a small portable-admission projection.
        self.assertEqual(records[0]['semantics'], definitions[0]['semantics'])
        self.assertEqual(records[0]['production_binding']['disposition'], 'EXACT')
        self.assertEqual(records[0]['admission']['materializable'], True)


if __name__ == '__main__':
    unittest.main()
