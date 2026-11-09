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
        for suffix in ('timering3', 'trollcave1', 'weddingring1'):
            key = 'oteryn:reward-claim.kv.' + suffix
            with self.subTest(claim=key):
                self.assertEqual(native[key]['readiness'], 'ready')
                self.assertEqual(source[key]['readiness'], 'waiting_item_semantics')
        triangle = 'oteryn:reward-claim.kv.triangletower'
        self.assertEqual(native[triangle]['data_holds'], [])
        self.assertIn('native_contents', native[triangle])
        self.assertEqual(source[triangle]['data_holds'][0]['category'], 'item')
        desert = 'oteryn:reward-claim.kv.desert2'
        self.assertEqual(native[desert]['data_holds'], [])
        self.assertIn('native_contents', native[desert])
        self.assertEqual(source[desert]['data_holds'][0]['item'], 'oteryn:item.tibia.i3084')
        changed = {key for key in native if native[key] != source[key]}
        self.assertEqual(changed, {
            'oteryn:reward-claim.kv.madmageroom3', triangle, desert,
            'oteryn:reward-claim.kv.ornamentedshield',
            'oteryn:reward-claim.kv.present_box', 'oteryn:reward-claim.kv.powerring2',
            'oteryn:reward-claim.kv.timering3', 'oteryn:reward-claim.kv.trollcave1',
            'oteryn:reward-claim.kv.weddingring1',
            'oteryn:reward-claim.kv.fireaxe1',
            'oteryn:reward-claim.kv.draconia1',
            'oteryn:reward-claim.kv.goblintemple2',
            'oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine',
        })


if __name__ == '__main__':
    unittest.main()
