"""Finite source/negative controls for the proposed Tile chain classification."""
import pathlib
import tempfile
import unittest
import ots_interactions
from ots_interactions import typed_tile_item_removal


class TileItemRemovalTests(unittest.TestCase):
    def classify(self, body, prefix='', in_loop=False):
        text = prefix + 'local action = Action()\nfunction action.onUse(player, item)\n' + body + '\nend\naction:aid(30008)\naction:register()\n'
        with tempfile.TemporaryDirectory(dir=pathlib.Path(__file__).parent) as temp:
            path = pathlib.Path(temp) / 'action.lua'
            path.write_text(text)
            script = ots_interactions.Script('canary', pathlib.Path(temp), 'action.lua', {}, 'test/action')
            list(script.interactions())
            line = next(i for i, text in enumerate(script.lines, 1) if ':getItemById' in text)
            return typed_tile_item_removal(script, script.raw(line), line, in_loop)

    def test_finite_loop_retains_repeated_source_operation(self):
        result = self.classify('for i = 32593, 32601 do\nTile(i,32104,14):getItemById(1271):remove()\nend', in_loop=True)
        self.assertEqual(result['operation'], 'REMOVE')
        self.assertIn('value_source_line', result)
        self.assertNotIn('anchor', result)

    def test_literal_position(self):
        self.assertEqual(self.classify('Tile(32593,32104,14):getItemById(1271):remove()')['operation'], 'REMOVE')

    def test_coordinate_overflow(self):
        for xyz in ['65536,32104,14', '32593,65536,14', '32593,32104,16', '-1,32104,14']:
            self.assertIsNone(self.classify('Tile(' + xyz + '):getItemById(1271):remove()'))

    def test_loop_upper_z_is_checked(self):
        self.assertIsNone(self.classify('for i = 0, 16 do\nTile(32593,32104,i):getItemById(1271):remove()\nend', in_loop=True))

    def test_dynamic_getter_and_coordinate(self):
        for expression in ['Tile(32593,32104,14):getItemById(dynamicId):remove()', 'Tile(runtimeX,32104,14):getItemById(1271):remove()', 'Tile(32593,32104,14):getItemById(getId()):remove()']:
            self.assertIsNone(self.classify(expression))

    def test_unproven_loop_bound(self):
        self.assertIsNone(self.classify('for i = first, last do\nTile(i,32104,14):getItemById(1271):remove()\nend', in_loop=True))

    def test_modified_constructor(self):
        for prefix in ['local Tile = other\n', 'Tile = other\n', 'local shadow = Tile\n', 'function Tile() return other end\n', 'Tile.field = other\n', '_G.Tile = other\n']:
            self.assertIsNone(self.classify('Tile(32593,32104,14):getItemById(1271):remove()', prefix))

    def test_constructor_parameter(self):
        self.assertIsNone(self.classify('Tile(32593,32104,14):getItemById(1271):remove()', 'function shadow(Tile) return Tile end\n'))

    def test_not_an_exact_statement(self):
        for suffix in [':unknown()', '; unknown()', ' + unknown()']:
            self.assertIsNone(self.classify('Tile(32593,32104,14):getItemById(1271):remove()' + suffix))

    def test_literal_lookalike(self):
        self.assertIsNone(self.classify('player:say("Tile(32593,32104,14):getItemById(1271):remove()")'))

    def test_loop_mutation_not_hidden(self):
        self.assertIsNone(self.classify('for i = 32593,32601 do\nTile(i,32104,14):getItemById(1271):remove()\ni = 1\nend', in_loop=True))


if __name__ == '__main__':
    unittest.main()
