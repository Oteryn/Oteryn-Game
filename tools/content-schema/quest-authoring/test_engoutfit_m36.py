"""M36: source-faithful Rock Steady collection and Deepling addon binding."""
import json
import unittest
import qengine_rewards as q


class RockSteady(unittest.TestCase):
    def test_every_admitted_outfit_has_explicit_sex_scoped_grants(self):
        output = q.build()
        authoring = json.loads(output['content/cosmetics/outfits/authoring.json'])
        document = authoring['document']
        mappings = {row['grant_key']: row for row in document['grant_mappings']}
        for outfit in document['outfits']:
            if not outfit['key'].startswith('oteryn:outfit.'):
                continue  # Historical item-owner aliases keep their own authored grants.
            for sex in ('female', 'male'):
                for addon in range(outfit['addon_count'] + 1):
                    key = outfit['key'] + f'/quest-grant/{sex}/{addon}'
                    self.assertIn(key, mappings)
                    self.assertEqual(mappings[key]['sex'], sex)
                    self.assertEqual(mappings[key]['compatible_provenance'], [q.PROVENANCE])

    def test_collection_is_persisted_and_the_final_item_earns_the_weapon(self):
        output = q.build()
        document = json.loads(output['content/quests/shared-reward-callbacks.json'])
        rows = {r['key'].rsplit('/', 1)[-1]: r for r in document['callbacks']}
        for suffix, before, after, item, topic in [
            ('start', -1, 1, None, 0), ('confirm', 1, 2, None, 1),
            ('first', 2, 3, 14022, 2), ('second', 3, 4, 14023, 3),
            ('weapon', 4, 5, 14021, 0),
        ]:
            row = rows['m36-rock-steady-' + suffix]
            state = row['storage'][0]
            self.assertEqual((state['minimum'], state['set']), (before, after))
            self.assertEqual(row['topic'], topic)
            self.assertEqual(row['take'], [] if item is None else [{'item': f'oteryn:item.tibia.i{item}', 'count': 1}])
            self.assertTrue(all('rock_steady.lua' in s for s in row['sources']))
        self.assertEqual(rows['m36-rock-steady-weapon']['outfit'], {'outfit': 'oteryn:outfit.deepling', 'addons': 1})
        declared = [r for r in document['tracks'] if r['key'] == rows['m36-rock-steady-start']['storage'][0]['track']]
        self.assertEqual(len(declared), 1)
        self.assertEqual(declared[0]['quest'], 'oteryn:quest.rock_steady_collection')
        self.assertEqual((declared[0]['initial'], declared[0]['max']), (-1, 5))


if __name__ == '__main__':
    unittest.main()
