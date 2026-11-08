"""M09 content admission: real native policies, canonical guards, separate caches."""
import json
import sys
import unittest

import m09_rewards as m
import qengine_rewards


class M09Rewards(unittest.TestCase):
    def test_generated_callbacks_match_their_authoring(self):
        for path, expected in m.outputs().items():
            self.assertEqual(json.loads((m.ROOT/path).read_bytes()), expected, path)

    def test_flask_uses_the_qualified_ordinary_physical_profile(self):
        key = 'oteryn:item.tibia.i12785'
        source = json.loads((m.ROOT/'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json').read_bytes())
        runtime = json.loads((m.ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
        observed = next(r for r in source['records'] if r['production_definition']['production_key'] == key)
        admitted = next(r for r in runtime['records'] if r['production_definition']['production_key'] == key)
        self.assertEqual(admitted['production_definition'], observed['production_definition'])
        self.assertEqual(admitted['admission'], observed['admission'])
        self.assertEqual(admitted['semantics']['physical'], observed['semantics']['physical'])
        self.assertEqual(admitted['semantics']['physical']['value']['weight'], {'state': 'KNOWN', 'value': 180})
        self.assertEqual(admitted['admission']['stack_class'], 'NonStackable')
        for field in ('charges', 'temporal', 'container'):
            self.assertEqual(admitted['semantics'][field]['state'], 'NOT_APPLICABLE')

    def test_treasure_selectors_match_retained_source_chests(self):
        sys.path.insert(0, str(m.ROOT/'tools/content-schema/world-authoring'))
        import world_region_codec
        palette = json.loads((m.ROOT/'content/world/placements/index.json').read_bytes())['palette']
        _, _, _, sectors = world_region_codec.decode_region((m.ROOT/
            'content/world/placements/region-z13-x125-y128.b3').read_bytes())
        tiles = {(x, y): items for _, rows in sectors
                 for x, y, _, _, _, items in rows}
        for callback in m.outputs()['content/quests/world-callbacks/m09-treasure.json']['callbacks']:
            selector = callback['selector']
            position = selector['position']
            self.assertEqual(position['floor'], -13)
            self.assertTrue(any(depth == 0 and palette[index]['source_item_id'] == selector['appearance']
                                for index, depth, _ in tiles[position['x'], position['y']]))

    def test_rookie_callback_schema_and_canonical_prerequisites(self):
        document = m.outputs()['content/quests/npc-callbacks/m09-rookie.json']
        qengine_rewards.validate_callbacks(document)
        tracks = {t['key']: t for q in json.loads(
            (m.ROOT/'content/quests/missions/quest-state.json').read_bytes())['quests']
                  for t in q['tracks']}
        for guard in document['callbacks'][0]['storage']:
            track = tracks[guard['track']]
            self.assertLessEqual(track['min'], guard['minimum'])
            self.assertGreaterEqual(track['max'], max(guard['maximum'], guard['set']))
        self.assertEqual(tracks[m.TRACK+'mission11']['initial'], -1)

    def test_every_reward_has_an_admitted_native_item_profile(self):
        profiles = json.loads((m.ROOT/
            'content/items/definitions/spell-native-profiles.json').read_bytes())
        keys = {r['production_definition']['production_key'] for r in profiles['records']}
        for document in m.outputs().values():
            if 'callbacks' not in document:
                continue
            for callback in document['callbacks']:
                gift = callback.get('give') or callback['effects']['item_reward']
                for item in [gift, *gift.get('additional', [])]:
                    self.assertTrue(item['item'] in keys,
                                    'unqualified runtime reward: '+item['item'])

    def test_treasure_caches_have_distinct_once_tracks_and_no_daily_floor_loot(self):
        callbacks = m.outputs()['content/quests/world-callbacks/m09-treasure.json']['callbacks']
        self.assertEqual(len({c['storage'][0]['track'] for c in callbacks}), 2)
        self.assertEqual(len({tuple(c['selector']['position'].values()) for c in callbacks}), 2)
        self.assertEqual([(c['effects']['item_reward']['item'],
                           c['effects']['item_reward']['count']) for c in callbacks],
                         [('oteryn:item.tibia.i9616', 1), ('oteryn:item.tibia.i9058', 3)])
        self.assertTrue(all(not c['effects']['item_reward'].get('additional') for c in callbacks))


if __name__ == '__main__':
    unittest.main()
