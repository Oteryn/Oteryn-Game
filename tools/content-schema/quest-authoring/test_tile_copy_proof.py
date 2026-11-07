"""Bounded copy-only Tile constructor proof; SOURCE hypothesis, never Native."""
import tempfile
import unittest
from pathlib import Path
import ots_interactions as converter


class TileCopyProofTests(unittest.TestCase):
    def fixture(self, prelude='', body='', suffix=''):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        text = ('local config = { id = 300, pos = Position(10, 20, 7) }\n'
                + prelude + '\nlocal action = Action()\n'
                + 'function action.onUse(player, item, fromPosition, target, toPosition)\n'
                + body + '\nGame.createItem(config.id, 1, config.pos)\nend\n'
                + 'action:id(100)\n' + suffix)
        (root / 'fixture.lua').write_text(text)
        script = converter.Script('canary', root, 'fixture.lua', {}, 'fixture')
        interaction = script.interactions()[0]
        return interaction, list(converter.walk(interaction['rules']))

    def assert_blocked(self, result):
        _, children = result
        self.assertTrue(any(c.get('status') == 'blocked' for c in children))
        self.assertFalse(any(c.get('operation') == 'CREATE' for c in children))

    def test_bare_tile_coordinate_copy_preserves_known_create(self):
        interaction, children = self.fixture(body='local tile = Tile(config.pos)')
        create = next(c for c in children if c.get('operation') == 'CREATE')
        self.assertEqual(create['def']['key'], 'canary:item/300')
        self.assertEqual(interaction['anchors'][0]['source_position'], {'x': 10, 'y': 20, 'z': 7})

    def test_mutating_tile_constructor_is_not_a_copy(self):
        self.assert_blocked(self.fixture(prelude='local Tile = function(p) p.x = 99 end',
                                         body='local tile = Tile(config.pos)'))

    def test_position_constructor_shadow_is_not_literal_proof(self):
        self.assert_blocked(self.fixture(prelude='Position = customPosition',
                                         body='local tile = Tile(config.pos)'))

    def test_same_named_qualified_calls_cannot_borrow_global_proof(self):
        for call in ('other.Tile(config.pos)', 'other:Tile(config.pos)'):
            with self.subTest(call=call):
                self.assert_blocked(self.fixture(body='local tile = ' + call))

    def test_alias_and_function_escape_remain_held(self):
        for body in ('local other = config.pos\nother.x = 99', 'mutate(config.pos)',
                     'for _, value in pairs(config) do mutate(value) end'):
            with self.subTest(body=body):
                self.assert_blocked(self.fixture(body='local tile = Tile(config.pos)\n' + body))

    def test_later_member_mutation_and_inline_else_remain_held(self):
        for suffix in ('config.pos.x = 99', 'if unknown then print(1) else config.pos.x = 99 end'):
            with self.subTest(suffix=suffix):
                self.assert_blocked(self.fixture(body='local tile = Tile(config.pos)', suffix=suffix))

    def test_impure_duplicate_constructor_fields_are_not_collapsed_to_proof(self):
        for initializer in ('{ pos = unknown(), pos = Position(10,20,7), id = 300 }',
                            '{ id = Game.createMonster("test", Position(1,2,7)), id = 300, pos = Position(10,20,7) }'):
            temporary = tempfile.TemporaryDirectory()
            self.addCleanup(temporary.cleanup)
            root = Path(temporary.name)
            text = ('local config = ' + initializer + '\nlocal a = Action()\n'
                    'function a.onUse(player, item)\nlocal tile = Tile(config.pos)\n'
                    'Game.createItem(config.id,1,config.pos)\nend\na:id(100)\n')
            (root/'fixture.lua').write_text(text)
            script = converter.Script('canary', root, 'fixture.lua', {}, 'fixture')
            inter = script.interactions()[0]
            self.assert_blocked((inter, list(converter.walk(inter['rules']))))

    def test_fresh_pure_coordinate_table_copy_is_not_an_unknown_escape(self):
        interaction, children = self.fixture(body='local tile = Tile({x=1,y=2,z=7})')
        self.assertTrue(any(c.get('operation') == 'CREATE' for c in children))


if __name__ == '__main__':
    unittest.main()
