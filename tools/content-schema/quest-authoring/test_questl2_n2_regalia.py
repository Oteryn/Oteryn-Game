"""Regalia assembly follows the pinned four-part NPC branch."""
import copy
import hashlib
import json
import unittest

import qengine_rewards as engine
import questl2_n2_regalia as regalia


class RegaliaSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.outputs = engine.build()
        cls.callback = json.loads(cls.outputs[regalia.SHARD])['callbacks'][0]
        cls.inputs = engine.donor_inputs()

    def test_four_parts_are_conserved_without_invented_progress(self):
        callback = self.callback
        self.assertEqual(callback['take'], [
            {'item': f'oteryn:item.tibia.i{i}', 'count': 1} for i in range(31572, 31576)])
        self.assertEqual(callback['give'], {
            'item': 'oteryn:item.tibia.i31576', 'count': 1, 'contents': []})
        self.assertEqual(callback['storage'], [])
        self.assertIsNone(callback.get('fee'))
        self.assertIsNone(callback['outfit'])
        engine.validate_callbacks({'schema': 1, 'tracks': [], 'callbacks': [callback]})

    def test_dialogue_and_witness_match_the_actual_donor(self):
        raw = self.inputs[regalia.SOURCE]
        self.assertEqual(hashlib.sha256(raw).hexdigest(), regalia.SHA256)
        self.assertEqual(self.callback['sources'], [
            f'crystal:{engine.PIN}:{regalia.SOURCE}:198-209:sha256:{regalia.SHA256}'])
        offers = json.loads(self.outputs['content/quests/shared-reward-offers.json'])['offers']
        offer = next(r for r in offers if r['npc'] == 'oteryn:npc.yonan')
        self.assertEqual((offer['topic'], offer['next_topic']), (0, 5))
        self.assertIn(offer['reply'][0], raw.decode().splitlines()[198])
        self.assertIn(self.callback['reply'][0], raw.decode().splitlines()[207])

    def test_known_canonical_weight_requires_the_reviewed_conflict(self):
        definition = {'identity': {'key': 'oteryn:item.tibia.i31576'},
            'semantics': {'physical': {'value': {'weight': {'state': 'KNOWN', 'value': 3400}}}}}
        record = copy.deepcopy(definition)
        record['semantics']['physical']['value']['weight']['value'] = 850
        regalia.preserve_weight(definition, record)
        self.assertEqual(record['semantics'], definition['semantics'])
        record['semantics']['physical']['value']['weight']['value'] = 851
        with self.assertRaisesRegex(ValueError, 'WEIGHT_PRECEDENCE_DRIFT'):
            regalia.preserve_weight(definition, record)


if __name__ == '__main__':
    unittest.main()
