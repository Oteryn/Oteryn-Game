"""Native charged chest admission cannot rewrite frozen SOURCE quest witnesses."""
import unittest
import sys
from pathlib import Path

import quest_tree_authoring as tree


class NativeClaimSourceReadinessTests(unittest.TestCase):
    def test_native_admission_and_historical_source_holds_remain_separate(self):
        root = Path(__file__).resolve().parents[3]
        index = tree.read(root, tree.CLAIM_INDEX)
        native = {r['definition']['identity']['key']: r['definition']
                  for path in index['shards'] for r in tree.read(root, path)['records']}
        source = {r['identity']['key']: r for r in tree.claim_catalogue(root)}
        for suffix in ('timering3', 'trollcave1', 'weddingring1', 'edrongoblin1', 'ghoulroom1', 'silveramulet', 'stealhring1'):
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
        sys.path.insert(0, str(root / 'tools/content-schema/reward-claim-authoring'))
        import native_source_leaf
        leaves, _ = native_source_leaf.build(root)
        expected_leaves = {r['definition']['identity']['key']: r['definition'] for r in leaves}
        self.assertEqual(set(native) - set(source), {'oteryn:reward-claim.source.formorgar_hoist_skeleton'})
        self.assertEqual({key: native[key] for key in set(native) - set(source)}, expected_leaves)
        self.assertEqual(set(source) - set(native), set())
        # This private classifier witness is not part of a RewardClaim definition.
        for record in source.values():
            self.assertIsInstance(record.pop('_source_baseline_item_hold'), bool)
        changed = {key for key in source if native[key] != source[key]}
        self.assertEqual(changed, {
            'oteryn:reward-claim.kv.madmageroom3', triangle, desert,
            'oteryn:reward-claim.kv.edrongoblin1', 'oteryn:reward-claim.kv.ghoulroom1',
            'oteryn:reward-claim.kv.silveramulet', 'oteryn:reward-claim.kv.stealhring1',
            'oteryn:reward-claim.kv.ornamentedshield',
            'oteryn:reward-claim.kv.present_box', 'oteryn:reward-claim.kv.powerring2',
            'oteryn:reward-claim.kv.timering3', 'oteryn:reward-claim.kv.trollcave1',
            'oteryn:reward-claim.kv.weddingring1',
        })


if __name__ == '__main__':
    unittest.main()
