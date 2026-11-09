"""M03 production admission selects the committed source object, never a fixture tile."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec


class TimeMachineAdmissionTest(unittest.TestCase):
    def test_recipe_and_native_object_match_the_production_binding(self):
        document = json.loads((ROOT / 'content/quests/authored-bindings/m03-25-years-time-machine.json').read_text())
        quest = document['quests'][0]
        index = json.loads((ROOT / 'content/quests/definitions/index.json').read_text())
        definitions = [row['definition'] for path in index['shards']
                       for row in json.loads((ROOT / path).read_text())['records']]
        definition = next(d for d in definitions if d['identity']['key'] == quest['quest'])
        recipe = definition['oteryn_recipe']['payload']['recipe']
        digest = hashlib.sha256(json.dumps(recipe, sort_keys=True, ensure_ascii=False,
                                          separators=(',', ':')).encode()).hexdigest()
        self.assertEqual(quest['recipe_sha256'], digest)
        self.assertEqual(recipe['requirements']['min_level'], 20)
        self.assertTrue(recipe['requirements']['premium'])
        stage = quest['stages'][0]
        self.assertEqual(stage['key'], 's1')
        self.assertFalse(stage['completes'])
        callback = stage['trigger']['callback']
        self.assertEqual(callback['admission'], {'regular_blessings': True, 'promotion': True, 'premium': True})
        self.assertEqual(callback['minimum_level'], 20)
        self.assertEqual(callback['selector']['position'], callback['selector']['native']['position'])
        self.assertEqual(callback['selector']['appearance'], 25098)
        self.assertTrue(callback['storage'][0]['at_least'])
        self.assertEqual(callback['storage'][0]['maximum'], 1)
        self.assertEqual(callback['teleport'], {'x': 33453, 'y': 31030, 'floor': -8})
        returning = quest['stages'][1]['trigger']['callback']
        self.assertNotIn('admission', returning)
        self.assertNotIn('minimum_level', returning)
        self.assertTrue(returning['storage'][0]['preserve'])
        self.assertEqual(returning['teleport'], {'x': 32430, 'y': 32167, 'floor': -8})
        index = json.loads((ROOT / 'content/world/placements/index.json').read_text())
        region = ROOT / 'content/world/placements/region-z08-x126-y125.b3'
        tiles = [tile for _, sector in world_region_codec.decode_region(region.read_bytes())[3]
                 for tile in sector if tile[:2] == (32430, 32166)]
        self.assertEqual(len(tiles), 1)
        self.assertFalse(tiles[0][3], 'source machine must not be a house object')
        palette, depth, attrs = tiles[0][5][callback['selector']['native']['ordinal']]
        self.assertEqual(index['palette'][palette]['source_item_id'], 25098)
        self.assertEqual(depth, 0)
        self.assertIsNone(attrs)


if __name__ == '__main__':
    unittest.main()
