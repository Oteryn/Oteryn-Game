import json
import unittest

import qengine_reward_item_profiles as items
import spirithunters_item_profile as profile


class SpirithuntersItemProfileTest(unittest.TestCase):
    def test_portable_device_is_admitted_in_tree_and_native_profile(self):
        source = profile.capture(items.ROOT)
        self.assertEqual(source['production_binding']['external_id'], '4049')
        self.assertEqual(source['semantics']['physical']['value']['weight']['value'], 2550)
        shard = items.ROOT / 'content/items/definitions/items-24500-24999.json'
        definition, = (row['definition'] for row in json.loads(shard.read_bytes())['records']
                       if row['definition']['identity']['key'] == profile.KEY)
        self.assertTrue(definition['materializable'])
        self.assertEqual(definition['stack_class'], 'NonStackable')
        native, = (row for row in json.loads((items.ROOT / items.PROFILE).read_bytes())['records']
                   if row['production_definition']['production_key'] == profile.KEY)
        self.assertEqual(native['admission'], source['admission'])


if __name__ == '__main__':
    unittest.main()
