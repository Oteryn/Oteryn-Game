"""Native charged chest admission cannot rewrite frozen SOURCE quest witnesses."""
import unittest
from pathlib import Path

import quest_tree_authoring as tree


class NativeClaimSourceReadinessTests(unittest.TestCase):
    def test_draconia_preserves_donor_reward_and_historical_hold(self):
        # Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
        # data-global/startup/tables/chest.lua:748-756 gives UID 6023 a bag
        # containing one ice rapier (3284) and one serpent sword (3297).
        # data/items/items.xml:8546-8559 defines the rapier with one charge.
        # The CLASSIC-5/Q09 historical review hold is a frozen SOURCE witness,
        # not a donor prohibition on the separately authored native reward.
        root = Path(__file__).resolve().parents[3]
        index = tree.read(root, tree.CLAIM_INDEX)
        native = {row['definition']['identity']['key']: row['definition']
                  for path in index['shards'] for row in tree.read(root, path)['records']}
        source = {row['identity']['key']: row for row in tree.claim_catalogue(root)}
        key = 'oteryn:reward-claim.kv.draconia1'
        self.assertEqual(native[key]['data_holds'], [])
        self.assertEqual(source[key]['data_holds'], [{
            'category': 'item', 'code': 'ITEM_SEMANTICS_MISSING',
            'item': 'oteryn:item.tibia.i3284', 'placement_index': 0,
            'reason': 'waiting', 'reward_field': 'items', 'reward_index': 0,
        }])
        self.assertEqual(source[key]['_source_hold_categories'], ['item'])
        self.assertNotIn('_source_hold_categories', native[key])
        contents = native[key]['native_contents']
        self.assertEqual(contents['revision'], '201189f871d23597b82caf88fd611454a2ddefb8')
        self.assertEqual(len(contents['placements']), 1)
        placement = contents['placements'][0]
        self.assertEqual((placement['unique_id'], placement['line']), (6023, 748))
        self.assertEqual(placement['root']['key'], 'oteryn:item.tibia.i2853')
        self.assertEqual([(item['item']['key'], item['count'])
                          for item in placement['contents']], [
            ('oteryn:item.tibia.i3284', 1), ('oteryn:item.tibia.i3297', 1),
        ])
        charges = native[key]['source_variant']['charge_dispositions']
        self.assertEqual(len(charges), 1)
        self.assertEqual(charges[0]['definition_charges'], {'state': 'KNOWN', 'value': 1})
        self.assertEqual(charges[0]['normalized_quantity'], {'state': 'KNOWN', 'value': 1})
        self.assertEqual(source[key]['native_contents'], contents)
        self.assertEqual(source[key]['placements'], native[key]['placements'])

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
        draconia = 'oteryn:reward-claim.kv.draconia1'
        self.assertEqual(native[draconia]['data_holds'], [])
        self.assertIn('native_contents', native[draconia])
        self.assertEqual(source[draconia]['data_holds'][0]['item'], 'oteryn:item.tibia.i3284')
        changed = {key for key in native if native[key] != source[key]}
        self.assertEqual(changed, {
            'oteryn:reward-claim.kv.madmageroom3', triangle, desert, draconia,
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
