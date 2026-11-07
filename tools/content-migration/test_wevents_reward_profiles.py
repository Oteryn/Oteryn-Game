"""Physical custody required by the native World-event recovery callback."""
import unittest

import qengine_reward_item_profiles as profiles


class WorldEventRewardProfiles(unittest.TestCase):
    def test_stolen_gold_bag_has_source_bound_portable_nonstackable_custody(self):
        packet = profiles.capture()
        records = {row['production_definition']['production_key']: row
                   for row in packet['records']}
        key = 'oteryn:item.tibia.i13429'
        self.assertTrue(key in records, "missing native stolen-gold bag profile")
        self.assertEqual(records[key]['admission']['stack_class'], 'NonStackable')
        self.assertTrue(records[key]['admission']['materializable'])
        self.assertEqual(records[key]['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 500})
        self.assertEqual(records[key]['admission']['legal_destinations'],
                         ['CharacterInventory', 'Ground'])

    def test_jesse_corpse_closure_has_source_timers_and_ground_only_custody(self):
        records = {row['production_definition']['production_key']: row
                   for row in profiles.capture()['records']}
        for item, capacity, duration, target in (
                (18153, 20, 10000, 18150), (18150, 20, 300000, 18151),
                (18151, 10, 300000, 18152), (18152, None, 60000, None)):
            row = records[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['admission']['legal_destinations'], ['Ground'])
            semantics = row['semantics']
            if capacity is None:
                self.assertEqual(semantics['container']['state'], 'NOT_APPLICABLE')
            else:
                self.assertEqual(semantics['container']['value']['capacity']['value'], capacity)
            temporal = semantics['temporal']['value']
            self.assertEqual(temporal['duration']['value'], duration)
            self.assertEqual(temporal['consumption_mode']['value'], 'DURABLE_ABSOLUTE_DEADLINE')
            if target is None:
                self.assertEqual(temporal['decay_target']['state'], 'NOT_APPLICABLE')
            else:
                self.assertEqual(temporal['decay_target']['value']['key'], f'oteryn:item.tibia.i{target}')


if __name__ == '__main__':
    unittest.main()
