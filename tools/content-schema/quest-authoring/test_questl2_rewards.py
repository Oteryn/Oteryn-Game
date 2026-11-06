"""L2 source hand-ins must survive ordinary shared-registry regeneration."""
import json
import unittest

import qengine_rewards as q


class LaneL2Rewards(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = q.build()
        cls.callbacks = {
            c['key'].removeprefix('oteryn:quest-reward/'): c
            for c in json.loads(cls.outputs['content/quests/shared-reward-callbacks.json'])['callbacks']
        }

    def test_sam_kroox_opens_both_source_tracks_without_minting_armor(self):
        callback = self.callbacks['sam-kroox']
        self.assertEqual(callback['npc'], 'oteryn:npc.kroox')
        self.assertIsNone(callback['give'])
        self.assertEqual(
            [(s['track'].rsplit('/', 1)[-1], s['set']) for s in callback['storage']],
            [('sams_old_backpack_npc', 2), ('sams_old_backpack_door', 1)],
        )

    def test_thieves_deliveries_burn_the_exact_source_items(self):
        stages = [
            ('tusks', 3044, 10, 1, 2), ('vase', 227, 1, 2, 3),
            ('invitation', 7933, 1, 2, 3), ('bridge', 7871, 1, 7, 8),
            ('goblet', 7870, 1, 1, 2), ('fish', 7936, 1, 3, 4),
            ('letter', 7935, 1, 1, 2),
        ]
        for index, (name, item, count, before, after) in enumerate(stages, 1):
            callback = self.callbacks['thieves-' + name]
            self.assertEqual(callback['take'], [{'item': f'oteryn:item.tibia.i{item}', 'count': count}])
            self.assertEqual(callback['storage'][0]['minimum'], before)
            self.assertEqual(callback['storage'][0]['maximum'], before)
            self.assertEqual(callback['storage'][0]['set'], after)
            self.assertEqual(callback['storage'][1]['minimum'], index)
            self.assertEqual(callback['storage'][1]['set'], index + 1)
            self.assertIsNone(callback['give'])

    def test_ape_city_handins_keep_source_progress_and_secondary_guards(self):
        for name, item, before, after in [
            ('moss', 4827, 1, 2), ('syrup', 4828, 3, 4),
            ('parchment', 4831, 5, 6), ('egg', 4839, 9, 10),
            ('mushroom', 4829, 11, 12), ('hair', 4832, 15, 16),
        ]:
            callback = self.callbacks['ape-' + name]
            self.assertEqual(callback['take'], [{'item': f'oteryn:item.tibia.i{item}', 'count': 1}])
            self.assertEqual(callback['storage'][0]['minimum'], before)
            self.assertEqual(callback['storage'][0]['set'], after)
        for name, track in [('moss', 'whisper_moss'), ('hair', 'holy_ape_hair')]:
            guard = self.callbacks['ape-' + name]['storage'][1]
            self.assertTrue(guard['track'].endswith('/' + track))
            self.assertEqual((guard['minimum'], guard['maximum']), (1, 1))
            self.assertTrue(guard['preserve'])

    def test_witches_cap_keeps_its_absolute_deadline(self):
        profiles = json.loads(self.outputs['content/items/definitions/spell-native-profiles.json'])
        cap = next(r for r in profiles['records']
                   if r['production_definition']['production_key'] == 'oteryn:item.tibia.i4829')
        temporal = cap['semantics']['temporal']['value']
        self.assertEqual(temporal['consumption_mode']['value'], 'DURABLE_ABSOLUTE_DEADLINE')
        self.assertEqual(temporal['duration']['value'], 86400000)
        self.assertFalse(temporal['stop_duration']['value'])

    def test_jorge_repeat_is_twenty_tokens_for_one_source_abacus(self):
        callback = self.callbacks['token-abacus']
        self.assertTrue(callback['repeatable'])
        self.assertEqual(callback['take'], [{'item': 'oteryn:item.tibia.i19083', 'count': 20}])
        self.assertEqual(callback['give'], {'item': 'oteryn:item.tibia.i19151', 'count': 1, 'contents': []})
        self.assertTrue(callback['storage'][0]['preserve'])
        self.assertIn('data-global/npc/jorge.lua:128:sha256:', callback['sources'][0])
        free = json.loads(json.dumps(callback))
        free['take'] = []
        with self.assertRaises(ValueError):
            q.validate_callbacks({'schema': 1, 'callbacks': [free]})


if __name__ == '__main__':
    unittest.main()
