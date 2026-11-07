"""Regression checks for accepted ITEM-ADD-1 self-decay removal and complete corpse chains."""
import tempfile
import unittest
from pathlib import Path

import canary_batch
from validate_monster import validate


class CorpseDecay(unittest.TestCase):
    def convert(self, attributes, corpse=52559):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            constants = root / canary_batch.EFFECT_CONSTANTS
            constants.parent.mkdir(parents=True)
            constants.write_text('enum MagicEffectClasses {};\nenum ShootType_t {};\n', encoding='utf-8')
            path = root / canary_batch.MONSTER_DIR / 'synthetic.lua'
            path.parent.mkdir(parents=True)
            path.write_text('local mType = Game.createMonsterType("Synthetic")\n'
                            'local monster = {}\n'
                            'monster.health = 100\nmonster.maxHealth = 100\n'
                            f'monster.corpse = {corpse}\n'
                            'monster.outfit = {lookType = 1}\n'
                            'mType:register(monster)\n', encoding='utf-8')
            items = {i: {'name': f'corpse {i}', 'article': 'a', 'attributes': attrs}
                     for i, attrs in attributes.items()}
            objects = {i: {'flags': {'corpse': True}} for i in attributes}
            names, index = canary_batch.name_index(objects, items)
            converter = canary_batch.Converter(root, objects, items, names, index)
            converter.pending_definitions = set()
            return converter.convert('synthetic')

    def assert_valid(self, result):
        _, monster, deps, catalog, manifest, _ = result
        self.assertEqual([], validate(monster, deps, catalog, manifest))

    def test_self_decay_is_terminal_removal_with_exact_source_and_decision(self):
        result = self.convert({52559: {'decayto': '52559', 'duration': '300'}})
        self.assert_valid(result)
        items = result[2]['items']
        self.assertEqual(1, len(items))
        self.assertEqual({'decay_action': 'remove', 'stop_duration': False, 'duration_ms': 300000},
                         items[0]['temporal'])
        corpse_row = next(row for row in result[4]['entries'] if row['source_field'] == 'corpse')
        self.assertIn('items.xml item 52559: decayTo=52559, duration=300 seconds', corpse_row['resolution'])
        self.assertIn('ITEM-ADD-1 accepted owner decision 3a', corpse_row['resolution'])
        self.assertEqual(canary_batch.REVISION, result[4]['sources'][0]['revision'])

    def test_complete_transform_chain_can_end_in_self_decay(self):
        result = self.convert({52559: {'decayto': '52560', 'duration': '5'},
                               52560: {'decayto': '52561', 'duration': '10'},
                               52561: {'decayto': '52561', 'duration': '300'}})
        self.assert_valid(result)
        items = result[2]['items']
        self.assertEqual([f'canary:item/{i}' for i in (52559, 52560, 52561)],
                         [item['identity']['key'] for item in items])
        self.assertEqual([5000, 10000, 300000], [item['temporal']['duration_ms'] for item in items])
        self.assertEqual(canary_batch.ref('Item', 'canary:item/52560'), items[0]['temporal']['decay_target'])
        self.assertEqual(canary_batch.ref('Item', 'canary:item/52561'), items[1]['temporal']['decay_target'])
        self.assertEqual('remove', items[2]['temporal']['decay_action'])
        self.assertNotIn('decay_target', items[2]['temporal'])

    def test_zero_target_remains_terminal_removal(self):
        result = self.convert({52559: {'decayto': '0', 'duration': '20'}})
        self.assert_valid(result)
        self.assertEqual({'decay_action': 'remove', 'stop_duration': False, 'duration_ms': 20000},
                         result[2]['items'][0]['temporal'])

    def test_multi_item_cycles_remain_invalid_and_keep_all_edges(self):
        result = self.convert({52559: {'decayto': '52560', 'duration': '5'},
                               52560: {'decayto': '52559', 'duration': '10'}})
        _, monster, deps, catalog, manifest, _ = result
        self.assertEqual(2, len(deps['items']))
        self.assertEqual(canary_batch.ref('Item', 'canary:item/52559'),
                         deps['items'][1]['temporal']['decay_target'])
        self.assertTrue(any(error.startswith('corpse decay: cycle')
                            for error in validate(monster, deps, catalog, manifest)))


if __name__ == '__main__':
    unittest.main()
