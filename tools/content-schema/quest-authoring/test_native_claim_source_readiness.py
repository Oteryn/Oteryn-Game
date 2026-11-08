"""Native charged chest admission cannot rewrite frozen SOURCE quest witnesses."""
import unittest
from pathlib import Path

import quest_tree_authoring as tree


class NativeClaimSourceReadinessTests(unittest.TestCase):
    def test_native_admission_and_historical_source_holds_remain_separate(self):
        root = Path(__file__).resolve().parents[3]
        index = tree.read(root, tree.CLAIM_INDEX)
        native = {r['definition']['identity']['key']: r['definition']
                  for path in index['shards'] for r in tree.read(root, path)['records']}
        source = {r['identity']['key']: r for r in tree.claim_catalogue(root)}
        for suffix in ('timering3', 'trollcave1', 'weddingring1', 'madmageroom3',
                       'powerring2', 'edrongoblin1', 'ghoulroom1',
                       'silveramulet', 'stealhring1'):
            key = 'oteryn:reward-claim.kv.' + suffix
            with self.subTest(claim=key):
                self.assertEqual(native[key]['readiness'], 'ready')
                self.assertEqual(source[key]['readiness'], 'waiting_item_semantics')
        for suffix in ('draconia1', 'fireaxe1', 'lifering'):
            key = 'oteryn:reward-claim.kv.' + suffix
            with self.subTest(variant=key):
                self.assertEqual(native[key]['readiness'], 'waiting_implementation')
                self.assertIn('native_contents', native[key])
                self.assertTrue(source[key]['_source_baseline_item_hold'])
        draconia = 'oteryn:reward-claim.kv.draconia1'
        triangle = 'oteryn:reward-claim.kv.triangletower'
        self.assertEqual(native[triangle]['data_holds'], [])
        self.assertIn('native_contents', native[triangle])
        self.assertEqual(source[triangle]['data_holds'][0]['category'], 'item')
        desert = 'oteryn:reward-claim.kv.desert2'
        self.assertEqual(native[desert]['data_holds'], [])
        self.assertIn('native_contents', native[desert])
        self.assertEqual(source[desert]['data_holds'][0]['item'], 'oteryn:item.tibia.i3084')
        # These two private SOURCE compiler annotations are not native claim
        # definition fields. Check their exact historical values separately.
        for suffix, categories in {
            'ornamentedshield': ['item', 'source'],
            'present_box': ['item'], 'triangletower': ['item'],
        }.items():
            self.assertEqual(source['oteryn:reward-claim.kv.' + suffix]['_source_hold_categories'], categories)
        # Explicit false compiler annotations do not change a SOURCE witness.
        # Retain every true hold and every other field in the exact comparison.
        source_witnesses = {}
        for key, record in source.items():
            self.assertIs(type(record['_source_baseline_item_hold']), bool)
            witness = dict(record)
            witness.pop('_source_hold_categories', None)
            if witness['_source_baseline_item_hold'] is False:
                del witness['_source_baseline_item_hold']
            source_witnesses[key] = witness
        changed = {key for key in native if native[key] != source_witnesses[key]}
        self.assertEqual(changed, {
            'oteryn:reward-claim.kv.madmageroom3', triangle, desert,
            'oteryn:reward-claim.kv.ornamentedshield',
            'oteryn:reward-claim.kv.present_box',
            'oteryn:reward-claim.kv.edrongoblin1', 'oteryn:reward-claim.kv.ghoulroom1',
            'oteryn:reward-claim.kv.silveramulet', 'oteryn:reward-claim.kv.stealhring1',
            'oteryn:reward-claim.kv.powerring2',
            'oteryn:reward-claim.kv.timering3', 'oteryn:reward-claim.kv.trollcave1',
            'oteryn:reward-claim.kv.weddingring1',
            'oteryn:reward-claim.kv.draconia1',
            'oteryn:reward-claim.kv.edrongoblin1',
            'oteryn:reward-claim.kv.fireaxe1',
            'oteryn:reward-claim.kv.ghoulroom1',
            'oteryn:reward-claim.kv.goblintemple2',
            'oteryn:reward-claim.kv.lifering',
            'oteryn:reward-claim.kv.silveramulet',
            'oteryn:reward-claim.kv.stealhring1',
            'oteryn:reward-claim.quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield',
            'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine',
        })


if __name__ == '__main__':
    unittest.main()
