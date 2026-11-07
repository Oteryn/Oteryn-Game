"""SOURCE syntax retention; donor-free controls for scope and side effects."""
from pathlib import Path
import tempfile
import unittest
import ots_interactions as converter


class SharedActorTableTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = Path(self.tmp.name)

    def script(self, body, prefix=''):
        text = (prefix + '\nlocal move = MoveEvent()\n'
                'function move.onStepIn(creature, item, position, fromPosition)\n' + body +
                '\nend\nmove:aid(100)\nmove:register()\n')
        (self.repo / 'fixture.lua').write_text(text)
        return converter.Script('canary', self.repo, 'fixture.lua', {}, 'fixture')

    def children(self, result):
        return list(converter.walk(result['rules']))

    def test_same_actor_userdata_or_id_constructor(self):
        for expression in ('creature', 'creature:getId()'):
            with self.subTest(expression=expression):
                s = self.script(('if not creature:isPlayer() then\nreturn false\nend\n' if ':' in expression else '') + 'local player = Player(' + expression + ')\n'
                                'if not player then\nreturn false\nend\n'
                                'player:setStorageValue(Storage.Test.Stage, 2)')
                r = s.interactions()[0]
                self.assertEqual(r['rules'][0]['branch'][0]['when'],
                                 {'actor_is_player': True, 'negate': True})
                self.assertEqual(self.children(r)[0]['to'], 2)
                self.assertEqual(r['unresolved'], [])

    def test_numeric_id_constructor_needs_same_actor_guard(self):
        for guard in ['', 'if not other:isPlayer() then\nreturn false\nend\n',
                      'if not creature:isPlayer() then\nunknownCall()\nend\n']:
            r = self.script(guard + 'local player = Player(creature:getId())\n'
                            'player:setStorageValue(Storage.Test.Stage, 2)').interactions()[0]
            self.assertFalse(any(c.get('owner') == 'Quest' for c in self.children(r)))
        r = self.script('if creature:isPlayer() then\nlocal player = Player(creature:getId())\n'
                        'player:setStorageValue(Storage.Test.Stage, 2)\nend').interactions()[0]
        self.assertEqual(self.children(r)[0]['to'], 2)

    def test_nullable_alias_uses_existing_predicate(self):
        for op, negate in (('==', True), ('~=', False)):
            s = self.script(f'local player = Player(creature)\nif player {op} nil then\nreturn false\nend')
            self.assertEqual(s.interactions()[0]['rules'][0]['branch'][0]['when'],
                             {'actor_is_player': True, 'negate': negate})

    def test_constructor_rejects_other_actor_and_changed_bindings(self):
        cases = [('', 'local player = Player(123)'),
                 ('', 'local player = Player(other)'),
                 ('local Player = replacement', 'local player = Player(creature)'),
                 ('Player = replacement', 'local player = Player(creature)'),
                 ('local alias = Player', 'local player = Player(creature)'),
                 ('_G["Player"] = replacement', 'local player = Player(creature)'),
                 ('debug.getupvalue(f, 1)', 'local player = Player(creature)'),
                 ('', 'creature = other\nlocal player = Player(creature)'),
                 ('', 'local player = Player(creature)\nplayer = other')]
        for prefix, declaration in cases:
            with self.subTest(prefix=prefix, declaration=declaration):
                r = self.script(declaration + '\nplayer:setStorageValue(Storage.Test.Stage, 2)', prefix).interactions()[0]
                self.assertFalse(any(c.get('owner') == 'Quest' for c in self.children(r)))
                self.assertTrue(r['unresolved'])

    def test_branch_alias_does_not_leak_to_sibling_or_before_declaration(self):
        for body in ['if creature:isPlayer() then\nlocal player = Player(creature)\n'
                     'else\nplayer:setStorageValue(Storage.Test.Stage, 2)\nend',
                     'player:setStorageValue(Storage.Test.Stage, 2)\nlocal player = Player(creature)']:
            r = self.script(body).interactions()[0]
            self.assertFalse(any(c.get('owner') == 'Quest' for c in self.children(r)))
            self.assertTrue(r['unresolved'])

    def test_multiline_pure_position_table_is_not_gameplay(self):
        body = ('local positions = {\nPosition(1, 2, 7),\nPosition(3, 4, 7),\n}\n'
                'if creature:isPlayer() then\nlocal player = Player(creature)\n'
                'player:setStorageValue(Storage.Test.Stage, 2)\nend')
        s = self.script(body);r = s.interactions()[0]
        self.assertEqual(r['unresolved'], [])
        self.assertEqual(self.children(r)[0]['to'], 2)
        self.assertEqual(len(s.lines), 15)
        self.assertEqual(r['callback_line'], 3)

    def test_table_does_not_erase_effect_constructor_or_trailing_effect(self):
        for body in ['local objects = {\nGame.createMonster("test", Position(1, 2, 7)),\n}',
                     'local objects = {Position(1,2,7)}; Game.createMonster("test", Position(1,2,7))']:
            r = self.script(body).interactions()[0]
            self.assertTrue(any(c.get('owner') == 'Ability' for c in self.children(r)))

    def test_table_effect_function_incomplete_or_expression_stays_opaque(self):
        for body in ['local values = {\nunknownCall(),\n}',
                     'local values = {\nPosition(1, 2, 7),',
                     'local values = {\nPosition(value, 2, 7),\n}',
                     'local values = {\nfunction() doEffect() end,\n}']:
            with self.subTest(body=body):
                s = self.script(body)
                self.assertFalse(s.readonly_tables)
                if 'function' not in body:
                    self.assertTrue(s.interactions()[0]['unresolved'])

    def test_duplicate_unknown_fields_are_checked_before_collapse(self):
        bodies = ['local cfg = {\nvalue = unknownGlobal,\nvalue = 1,\n}',
                  'local cfg = {\nvalue = externalTable.member,\nvalue = 1,\n}',
                  'local cfg = {\nvalue = {x=externalTable.member, x=1},\nvalue = 1,\n}',
                  'local cfg = {\nPosition({x=externalTable.member, x=1, y=2, z=7}),\n}',
                  'local cfg = {\nvalue = unknownCall(),\nvalue = 1,\n}',
                  'local cfg = {\n[externalTable.member] = 1,\nvalue = 1,\n}']
        for body in bodies:
            with self.subTest(body=body):
                self.assertFalse(self.script(body).readonly_tables)
        r = self.script(bodies[3]).interactions()[0]
        self.assertEqual(r['unresolved'], [{'line': 5, 'reason': 'statement outside the transcribed vocabulary'}])

    def test_duplicate_effect_is_not_erased_but_pure_duplicates_are_allowed(self):
        r = self.script('local cfg = {\nvalue = Game.createMonster("test", Position(1,2,7)),\nvalue=1,\n}').interactions()[0]
        self.assertTrue(any(c.get('owner') == 'Ability' for c in self.children(r)))
        s = self.script('local cfg = {\nvalue=1,\nvalue=2,\n}')
        self.assertEqual(s.readonly_tables, {4, 5, 6, 7})

    def test_shadowed_position_constructor_is_not_discarded(self):
        for prefix in ['local Position = evil', 'Position = evil',
                       '_G["Position"] = evil', 'local alias = Position']:
            s = self.script('local pos = {\nPosition(1, 2, 7),\n}', prefix)
            self.assertFalse(s.readonly_tables)
            self.assertTrue(s.interactions()[0]['unresolved'])

    def test_nested_literals_and_physical_line_retention(self):
        body = ('local values = {\n[1] = {message="fake player:setStorageValue(1, 2)", pos=Position(1,2,7)},\n}\n'
                'unknownCall()')
        s = self.script(body);r = s.interactions()[0]
        self.assertEqual(r['unresolved'], [{'line': 7, 'reason': 'statement outside the transcribed vocabulary'}])
        self.assertEqual(self.children(r), [])


if __name__ == '__main__':
    unittest.main()
