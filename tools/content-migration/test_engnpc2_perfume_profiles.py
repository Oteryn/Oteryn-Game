"""Source-qualified physical Items for the Goblin Merchant perfume samples."""
import unittest

import qengine_reward_item_profiles as profiles


class PerfumeCustody(unittest.TestCase):
    def test_perfume_inputs_and_outputs_keep_exact_portable_source_facts(self):
        records = {row['production_definition']['production_key']: row
                   for row in profiles.capture()['records']}
        for item_id in (17670, 17672, 17673, 17674):
            key = f'oteryn:item.tibia.i{item_id}'
            self.assertIn(key, set(records), 'registered sample must have qualified input and output')
            row = records[key]
            self.assertEqual(row['production_binding']['external_id'], str(item_id))
            self.assertEqual(row['admission'], {
                'materializable': True, 'stack_class': 'NonStackable',
                'legal_destinations': ['CharacterInventory', 'Ground']})
            self.assertEqual(row['semantics']['physical']['value']['weight'],
                             {'state': 'KNOWN', 'value': 350})
            self.assertEqual(row['semantics']['stack']['value']['stackable'],
                             {'state': 'KNOWN', 'value': False})
            for field in ('charges', 'temporal'):
                self.assertEqual(row['semantics'][field], {'state': 'NOT_APPLICABLE'})


if __name__ == '__main__':
    unittest.main()
