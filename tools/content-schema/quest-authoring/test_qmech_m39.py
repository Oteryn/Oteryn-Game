"""M39 source-bound sample lifecycle and disjoint carried-quantity branches."""
import copy
import json
import unittest

import qengine_rewards
import qmech_m39


class M39SampleAuthoring(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.inputs = qengine_rewards.donor_inputs()
        cls.outputs = qengine_rewards.build()
        cls.document = json.loads(cls.outputs[qmech_m39.SHARD])
        cls.callbacks = {c['key'].rsplit('/', 1)[1]: c for c in cls.document['callbacks']}

    def test_return_and_loss_are_disjoint_and_only_the_real_sample_pays(self):
        returned = self.callbacks['l2-bone-return']
        lost = self.callbacks['bone-pit-lost-sample']
        self.assertEqual(returned['inventory'], [
            {'item': 'oteryn:item.tibia.i131', 'minimum': 1, 'maximum': 4294967295}])
        self.assertEqual(lost['inventory'], [
            {'item': 'oteryn:item.tibia.i131', 'minimum': 0, 'maximum': 0}])
        self.assertEqual(returned['take'], [{'item': 'oteryn:item.tibia.i131', 'count': 1}])
        self.assertEqual(returned['give']['item'], 'oteryn:item.tibia.i6299')
        self.assertEqual(returned['experience'], 1000)
        self.assertEqual(lost['take'], [])
        self.assertIsNone(lost['give'])
        self.assertNotIn('experience', lost)
        for callback in (returned, lost, self.callbacks['l2-bone-start']):
            self.assertEqual(callback['minimum_level'], 35)
            self.assertTrue(callback['failure_reply'])

    def test_collection_preserves_the_issued_instance_and_removes_only_the_clicked_bones(self):
        [callback] = qmech_m39.worlds(self.inputs)['callbacks']
        self.assertEqual(callback['selector']['action'], 4206)
        self.assertEqual(callback['selector']['appearance'], 3133)
        self.assertEqual(callback['item_on_target']['item'], 'oteryn:item.tibia.i4852')
        self.assertEqual(callback['item_on_target']['replacement'], 'oteryn:item.tibia.i131')
        self.assertTrue(callback['item_on_target']['target_remove'])
        source = self.inputs['data-global/scripts/quests/others/actions_ectoplasm_container.lua']
        self.assertIn(b'131', source)
        self.assertIn(b'remove', source)

    def test_authoring_rejects_reversed_inventory_ranges(self):
        document = copy.deepcopy(self.document)
        callback = next(c for c in document['callbacks'] if c['key'].endswith('/l2-bone-return'))
        callback['inventory'][0].update(minimum=2, maximum=1)
        with self.assertRaisesRegex(ValueError, 'invalid inventory range'):
            qengine_rewards.validate_callbacks(document)


if __name__ == '__main__':
    unittest.main()
