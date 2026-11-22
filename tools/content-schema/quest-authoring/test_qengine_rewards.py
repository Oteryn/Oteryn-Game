"""Offline donor identity, schema and deterministic materialization checks."""
import copy
import json
import unittest
from jsonschema import Draft202012Validator, ValidationError
import qengine_rewards as q


class QuestRewards(unittest.TestCase):

    def test_demon_outfits_award_true_dedication_only_on_final_addon(self):
        rewards = {row['key'].split('/')[-1]: row for row in self.callbacks['callbacks']}
        self.assertIsNone(rewards['l3-demon-base']['achievement'])
        for addon, other_bit in [('shield', 2), ('helmet', 1)]:
            with self.subTest(addon=addon):
                name = f'l3-demon-{addon}'
                self.assertIsNone(rewards[name]['achievement'])
                complete = rewards[name + '-complete']
                self.assertEqual(complete['achievement'], 'oteryn:achievement/true_dedication')
                self.assertEqual(complete['requires_addons'], [
                    {'outfit': 'oteryn:outfit.demon', 'addons': other_bit},
                ])
                self.assertEqual(rewards[name]['requires'][-1]['maximum'] + 1,
                                 complete['requires'][-1]['minimum'])
