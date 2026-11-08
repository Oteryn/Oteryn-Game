"""Source cell and immutable approved-recipe checks for the finite Kingsday binding."""
import hashlib
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
import world_region_codec as codec


class KingsdayBindings(unittest.TestCase):
    def test_approved_recipe_is_bound_without_rewriting_it(self):
        key = 'oteryn:quest.authored.kingsday_mini_world_change'
        recipe = next(r['definition']['recipe']
                      for p in (ROOT / 'content/quests/definitions').glob('quests-*.json')
                      for r in json.loads(p.read_bytes()).get('records', [])
                      if r['definition']['identity']['key'] == key)
        bindings = json.loads((ROOT / 'content/quests/questpc-authored-bindings.json').read_bytes())
        bound = next((q for q in bindings['quests'] if q['quest'] == key), None)
        self.assertIsNotNone(bound, 'approved Kingsday recipe has no native bindings')
        self.assertEqual(bound['recipe_sha256'], hashlib.sha256(json.dumps(
            recipe, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest())
        self.assertEqual([s['key'] for s in bound['stages']], [s['key'] for s in recipe['stages']])
        self.assertEqual(bound['stages'][0]['trigger']['callback']['selector']['position'],
                         {'x': 32313, 'y': 32182, 'floor': -7})
        self.assertTrue(all(s['trigger']['callback']['npc'] == 'oteryn:npc.king_tibianus'
                            for s in bound['stages'][1:]))
        self.assertEqual([s['completes'] for s in bound['stages']], [False, False, True])
        self.assertEqual(bound['stages'][2]['trigger']['callback']['achievement'],
                         'oteryn:achievement/loyal_subject')

    def test_king_uses_the_captured_placement(self):
        index = json.loads((ROOT / 'content/world/npc-placements/index.json').read_bytes())
        rows = [r['declaration'] for path in index['shards']
                for r in json.loads((ROOT / path).read_bytes())['records']]
        king = [r for r in rows if r['npc'] == 'oteryn:npc.king_tibianus']
        self.assertEqual(len(king), 1)
        self.assertEqual(king[0]['cell'], {'x': 32311, 'y': 32172, 'floor': 6})
        self.assertEqual(king[0]['direction'], 'north')
        self.assertEqual({p['origin'] for p in king[0]['provenance']}, {'canary', 'crystal'})

    def test_captured_courtyard_and_king_native_cells_exist(self):
        palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
        for x, y, z, expected in [(32313, 32182, 7, [9575]),
                                  (32312, 32182, 7, [9579]),
                                  (32311, 32172, 6, [464, 2576]),
                                  (32311, 32171, 6, [464, 2573])]:
            path = ROOT / f'content/world/placements/region-z{z:02}-x{x // 256:03}-y{y // 256:03}.b3'
            tiles = [t for _, rows in codec.decode_region(path.read_bytes())[3]
                     for t in rows if t[:2] == (x, y)]
            self.assertEqual(len(tiles), 1, (x, y, z))
            self.assertEqual(tiles[0][3], 0, 'public castle cells must not be private houses')
            self.assertEqual([palette[o[0]]['source_item_id'] for o in tiles[0][5] if o[1] == 0], expected)


if __name__ == '__main__':
    unittest.main()
