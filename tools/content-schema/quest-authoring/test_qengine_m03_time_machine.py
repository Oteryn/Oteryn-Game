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
    def test_all_nine_machine_pieces_have_one_live_guard_binding(self):
        import qengine_access
        document = json.loads((ROOT / 'content/quests/authored-bindings/m03-25-years-time-machine.json').read_text())
        callbacks = [stage['trigger']['callback'] for stage in document['quests'][0]['stages']
                     if stage['trigger']['callback'].get('admission')]
        self.assertFalse(any(row['quest'] == 'oteryn:quest.25_years_of_tibia_quest'
                             for row in qengine_access.document()['callbacks']))
        self.assertEqual(sorted(row['selector']['appearance'] for row in callbacks), list(range(25091, 25100)))
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        tiles = {(tile[0], tile[1]): tile for _, sector in world_region_codec.decode_region(
            (ROOT / 'content/world/placements/region-z08-x126-y125.b3').read_bytes())[3] for tile in sector}
        for callback in callbacks:
            native = callback['selector']['native']
            tile = tiles[native['position']['x'], native['position']['y']]
            self.assertFalse(tile[3])
            entry = tile[5][native['ordinal']]
            self.assertEqual(palette[entry[0]]['source_item_id'], native['appearance'])
            self.assertEqual(callback['admission'], {'regular_blessings': True, 'promotion': True, 'premium': True})
            self.assertEqual(callback['minimum_level'], 20)
            self.assertTrue(callback['effects']['scripted_teleport'])
            self.assertEqual(callback['storage'][0]['minimum'], 0)
            self.assertTrue(callback['storage'][0]['at_least'])

    def test_all_nine_return_pieces_are_ungated_and_keep_progress(self):
        import qengine_access
        document = json.loads((ROOT / 'content/quests/authored-bindings/m03-25-years-time-machine.json').read_text())
        rows = [stage['trigger']['callback'] for stage in document['quests'][0]['stages']
                if not stage['trigger']['callback'].get('admission')]
        self.assertEqual(sorted(row['selector']['appearance'] for row in rows), list(range(25091, 25100)))
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_text())['palette']
        tiles = {(tile[0],tile[1]):tile for _,sector in world_region_codec.decode_region(
            (ROOT / 'content/world/placements/region-z08-x130-y121.b3').read_bytes())[3] for tile in sector}
        for row in rows:
            self.assertFalse(row.get('requires_world_admission'))
            self.assertFalse(row.get('admission'))
            self.assertFalse(row.get('minimum_level'))
            self.assertTrue(row['effects']['scripted_teleport'])
            self.assertTrue(row['storage'][0]['preserve'])
            self.assertFalse(row['storage'][0].get('at_least'))
            native = row['selector']['native']
            tile = tiles[native['position']['x'],native['position']['y']]
            self.assertFalse(tile[3])
            self.assertEqual(palette[tile[5][native['ordinal']][0]]['source_item_id'], native['appearance'])
            self.assertEqual(row['teleport'], {'x':32430,'y':32167,'floor':-8})

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
