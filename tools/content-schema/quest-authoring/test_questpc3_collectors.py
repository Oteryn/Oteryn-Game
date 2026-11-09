import unittest
import questpc3_collectors as source


class CollectorDelivery(unittest.TestCase):
    def test_pilgrimage_requires_six_blessers_in_order_before_twenty_platinum(self):
        quest = source.build_pilgrimage()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.pilgrimage_of_ashes')
        self.assertEqual(quest['admission'], {'minimum_level': 0, 'premium': True})
        expected = ['guide_jonathan', 'norf', 'humphrey', 'edala', 'kawill', 'pydar', 'eremo', 'guide_jonathan']
        self.assertEqual(len(quest['stages']), 8)
        for index, (stage, npc) in enumerate(zip(quest['stages'], expected)):
            callback = stage['trigger']['callback']
            self.assertEqual(callback['npc'], 'oteryn:npc.' + npc)
            self.assertEqual(callback['storage'], [{'track': quest['tracks'][0]['key'], 'minimum': index, 'maximum': index, 'set': index + 1}])
            self.assertEqual(stage['completes'], index == 7)
            self.assertEqual(callback['completes'], index == 7)
            self.assertEqual(callback['take'], [])
            self.assertIsNone(callback['achievement'])
            if index == 7:
                self.assertEqual(callback['give'], {'item': 'oteryn:item.tibia.i3035', 'count': 20, 'contents': [], 'premium': True})
            else:
                self.assertIsNone(callback['give'])


    def test_stampede_personal_trial_has_five_kills_before_collector_achievement(self):
        quest = source.build_stampede()['quests'][0]
        self.assertEqual(quest['quest'], 'oteryn:quest.stampede_mini_world_change')
        self.assertEqual([s['trigger']['kind'] for s in quest['stages']], ['step_in', 'kill', 'npc_dialogue'])
        entry, kill, finish = [s['trigger'] for s in quest['stages']]
        encounter = entry['callback']['effects']['encounter']
        self.assertEqual(len(encounter['monsters']), 5)
        self.assertEqual({m['creature'] for m in encounter['monsters']}, {'oteryn:creature.terrified_elephant'})
        self.assertNotIn('world_change', entry['callback'])
        # Check chosen trial cells against retained map data, not merely the generated recipe.
        import sys
        sys.path.insert(0, str(source.ROOT / 'tools/content-schema/world-authoring'))
        import world_region_codec as codec
        _, _, _, sectors = codec.decode_region((source.ROOT / 'content/world/placements/region-z07-x128-y127.b3').read_bytes())
        tiles = {(t[0], t[1]): t for _, sector in sectors for t in sector}
        import json
        palette = json.loads((source.ROOT / 'content/world/placements/index.json').read_bytes())['palette']
        positions = [entry['callback']['selector']['position'], *[m['position'] for m in encounter['monsters']]]
        for position in positions:
            tile = tiles[(position['x'], position['y'])]
            self.assertEqual(position['floor'], -7)
            self.assertEqual(tile[2], 0)
            self.assertEqual(len(tile[5]), 1, 'trial cells must not contain a retained obstacle')
            self.assertEqual(palette[tile[5][0][0]]['source_item_id'], 1019)

        self.assertEqual((kill['counter']['maximum'], kill['counter']['advance_gate_to']), (5, 2))
        callback = finish['callback']
        self.assertEqual(callback['npc'], 'oteryn:npc.guide_tiko')
        self.assertEqual(callback['achievement'], 'oteryn:achievement/trail_of_the_ape_god')
        self.assertIsNone(callback['give'])
        self.assertEqual(callback['storage'][0]['minimum'], 2)
        self.assertTrue(quest['stages'][-1]['completes'])


if __name__ == '__main__':
    unittest.main()
