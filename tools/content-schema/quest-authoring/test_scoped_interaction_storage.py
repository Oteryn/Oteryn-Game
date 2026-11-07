"""SOURCE-only exact actor/scoped storage regression controls."""
import tempfile
import unittest
from pathlib import Path

import ots_interactions as converter


class ScopedInteractionStorageTests(unittest.TestCase):
    def graph(self, body, prefix='', callback='onUse', params='player, item, fromPosition, target, toPosition'):
        # A closed helper intentionally defeats legacy unconditional-prefix aliases.
        text = 'local function helper()\nend\n' + prefix + '\nlocal a = Action()\nfunction a.' + callback + '(' + params + ')\n' + body + '\nend\na:id(300)\n'
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as directory:
            root = Path(directory)
            (root / 'fixture.lua').write_text(text)
            graph = converter.Script('canary', root, 'fixture.lua', {}, 'fixture').interactions()[0]
        return graph

    def conditions(self, graph):
        return list(converter.conditions(graph['rules']))

    def test_after_closed_function_named_alias_adds_exact_condition_and_write(self):
        graph = self.graph('if player:getStorageValue(K.Stage) == 3 then\nplayer:setStorageValue(K.Stage, 4)\nend', 'local K = Storage.Quest.Fixture')
        condition = self.conditions(graph)[0]['quest_stage']
        child = list(converter.walk(graph['rules']))[0]
        self.assertEqual(condition['value'], 3)
        self.assertEqual(child['to'], 4)
        self.assertEqual(condition['progress'], child['progress'])
        self.assertEqual(child['progress'], 'canary:quest-progress/quest/fixture/stage')

    def test_callback_numeric_alias_does_not_join_world_namespace(self):
        graph = self.graph('local K = 12345\nif player:getStorageValue(K) == 2 then\nplayer:setStorageValue(K, 3)\nend\nGame.setStorageValue(K, 4)')
        child = list(converter.walk(graph['rules']))[0]
        self.assertEqual(child['to'], 3)
        self.assertIn('12345', child['progress'])
        world = next(c for c in converter.walk(graph['rules']) if c.get('request') == 'set_world_state')
        self.assertEqual(world['key'], 'canary:world-state/k')  # Existing symbolic SOURCE world key.
        self.assertNotIn('12345', world['key'])

    def test_scope_forward_sibling_and_callback_separation(self):
        for body in ('if player:getStorageValue(K) == 1 then\nend\nlocal K = 12345',
                     'if unknown then\nlocal K = 12345\nelse\nif player:getStorageValue(K) == 1 then\nend\nend',
                     'if unknown then\nlocal K = 12345\nend\nif player:getStorageValue(K) == 1 then\nend'):
            with self.subTest(body=body):
                self.assertTrue(any('unresolved' in c for c in self.conditions(self.graph(body))))
        graph = self.graph('if player:getStorageValue(K) == 1 then\nend', 'function other()\nlocal K = 12345\nend')
        self.assertIn('unresolved', self.conditions(graph)[0])

    def test_alias_mutation_shadow_escape_and_reflection_stay_opaque(self):
        for tail in ('K = 12', 'local other = K', 'mutate(K)', 'K.Stage = 12',
                     'local K = K', 'debug.setupvalue(helper, 1, 12)', 'loadfile("mutator.lua")()',
                     'Storage.Quest.Fixture = 12'):
            with self.subTest(tail=tail):
                graph = self.graph('if player:getStorageValue(K.Stage) == 1 then\nend\n' + tail,
                                   'local K = Storage.Quest.Fixture')
                self.assertIn('unresolved', self.conditions(graph)[0])

    def test_receiver_reassign_shadow_escape_and_foreign_player_stay_opaque(self):
        for tail in ('player = other', 'local player = other', 'mutate(player)', 'player.getStorageValue = helper'):
            with self.subTest(tail=tail):
                graph = self.graph('if player:getStorageValue(K.Stage) == 1 then\nend\n' + tail,
                                   'local K = Storage.Quest.Fixture')
                self.assertIn('unresolved', self.conditions(graph)[0])
        graph = self.graph('if other:getStorageValue(K.Stage) == 1 then\nend', 'local K = Storage.Quest.Fixture')
        self.assertIn('unresolved', self.conditions(graph)[0])

    def test_same_actor_factory_alias_is_scoped_and_getter_snapshot_safe(self):
        body = 'local player = creature:getPlayer()\nlocal K = Storage.Quest.Fixture\nlocal state = player:getStorageValue(K.Stage)\nif state == 3 then\nplayer:setStorageValue(K.Stage, 4)\nend'
        graph = self.graph(body, callback='onStepIn', params='creature, item, position, fromPosition')
        self.assertIn('quest_stage', self.conditions(graph)[0])
        graph = self.graph(body.replace('if state == 3', 'player:setStorageValue(K.Stage, 2)\nif state == 3'), callback='onStepIn', params='creature, item, position, fromPosition')
        self.assertIn('unresolved', self.conditions(graph)[0])

    def test_computed_selector_and_unknown_write_value_remain_separate_holds(self):
        graph = self.graph('local K = Storage.Quest.Fixture\nplayer:setStorageValue(K.Stage, count)\nif player:getStorageValue(K[index]) == 1 then\nend')
        # Mixed unsafe use invalidates the alias entirely; cannot guess a target.
        self.assertTrue(graph['unresolved'])
        self.assertIn('unresolved', self.conditions(graph)[0])
        graph = self.graph('local K = Storage.Quest.Fixture\nplayer:setStorageValue(K.Stage, count)')
        child = list(converter.walk(graph['rules']))[0]
        self.assertIn('value_source_line', child)
        self.assertNotIn('to', child)


if __name__ == '__main__':
    unittest.main()
