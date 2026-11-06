"""R–S script-owned rewards must survive the canonical authoring boundary."""
import unittest

import reward_claim_authoring as authoring


class ScriptedChestTests(unittest.TestCase):
    def test_small_sapphire_preserves_all_four_rewards_and_both_book_texts(self):
        records = authoring.scripted_chests_rs.derive(
            authoring.load_json(authoring.ROOT / authoring.MANIFEST_REL),
            authoring.load_items(), authoring.build_records)
        definition = next(row['definition'] for row in records
                          if row['definition']['identity']['key'].endswith('.smallsapphire'))
        self.assertEqual(definition['claim'], {'per': 'character', 'repeat': {'kind': 'once'}})
        self.assertEqual(definition['placements'][0]['source_binding'], {
            'project_position': {'x': 32962, 'y': 32429, 'z': 7}, 'legacy_unique_ids': []})
        recipe = definition['native_contents']['placements'][0]
        self.assertEqual(recipe['root']['key'], 'oteryn:item.tibia.i2853')
        self.assertEqual([(row['item']['key'], row['count']) for row in recipe['contents']], [
            ('oteryn:item.tibia.i3029', 1), ('oteryn:item.tibia.i3031', 50),
            ('oteryn:item.tibia.i2822', 1), ('oteryn:item.tibia.i637', 1)])
        self.assertEqual([row['item']['key'] for row in recipe['contents'] if 'text' in row],
                         ['oteryn:item.tibia.i2822', 'oteryn:item.tibia.i637'])

    def test_small_sapphire_never_drops_an_unadmitted_child(self):
        import copy
        items = copy.deepcopy(authoring.load_items())
        for ident in (3029, 3031, 2822, 637):
            changed = copy.deepcopy(items)
            changed[f'oteryn:item.tibia.i{ident}']['materializable'] = False
            with self.subTest(ident=ident), self.assertRaisesRegex(ValueError, 'Item is not admitted'):
                authoring.scripted_chests_rs.small_sapphire(changed)

    def test_rapier_is_an_executable_once_claim_at_the_source_position(self):
        generated = authoring.generate()
        records = [row['definition']
                   for path, text in generated.items()
                   if path.endswith('.json') and '/reward_claims/reward-' in path
                   for row in authoring.json.loads(text)['records']]
        rapier = next((r for r in records if r['identity']['key'] ==
                       'oteryn:reward-claim.kv.rapier'), None)
        self.assertIsNotNone(rapier, 'the script reward is missing from RewardClaim')
        self.assertEqual(rapier['readiness'], 'ready')
        self.assertEqual(rapier['claim'], {'per': 'character', 'repeat': {'kind': 'once'}})
        self.assertEqual(rapier['placements'], [{
            'appearance_tibia_id': 2473,
            'source_binding': {
                'legacy_unique_ids': [{'server': 'crystalserver', 'unique_id': 14042}],
                'project_position': {'x': 32099, 'y': 32198, 'z': 9}},
            'reward': {'items': [{'count': 1, 'item': authoring.item_ref(
                'oteryn:item.tibia.i3272')}]},
        }])


if __name__ == '__main__':
    unittest.main()
