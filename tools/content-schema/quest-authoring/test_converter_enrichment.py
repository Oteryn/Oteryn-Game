"""Static converter enrichment: synthetic fixtures, no reference dialogue text."""
import json
import tempfile
import unittest
from pathlib import Path

import ots_interactions as converter


class ConverterEnrichmentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        (self.repo / 'data/items').mkdir(parents=True)
        (self.repo / 'data/items/items.xml').write_text(
            '<items><item id="300" name="test reward"/><item id="301" name="duplicate"/>'
            '<item id="302" name="duplicate"/><item fromid="310" toid="320" name="range"/></items>')

    def script(self, globals='', body=''):
        text = globals + '\nlocal action = Action()\nfunction action.onUse(player, item, fromPosition, target, toPosition)\n' + body + '\nend\naction:id(100)\n'
        (self.repo / 'fixture.lua').write_text(text)
        script = converter.Script('canary', self.repo, 'fixture.lua', {}, 'fixture')
        number = next(n for n, line in enumerate(script.lines, 1) if line.startswith('function action'))
        script.bind(number, 'onUse')
        script.current_line = len(script.lines)
        return script

    def test_complete_call_respects_quotes(self):
        text = '"name, (x)", Position(1, 2, 7)) trailing'
        args = converter.call_complete(text, 0)
        self.assertEqual(converter.split_args(args), ['"name, (x)"', 'Position(1, 2, 7)'])
        self.assertIsNone(converter.call_complete('Position(1, 2, 7)', 0))

    def test_static_position_alias_and_table(self):
        s = self.script('local config = { destination = Position(10, 20, 7) }',
                        'player:teleportTo(config.destination)')
        self.assertEqual(s.position('config.destination'), (10, 20, 7))
        self.assertEqual(s.position('Position(config.destination)'), (10, 20, 7))
        result = s.interactions()[0]
        self.assertEqual(list(converter.walk(result['rules']))[0]['target']['kind'], 'anchor')
        self.assertEqual(result['anchors'][0]['source_position'], {'x': 10, 'y': 20, 'z': 7})

    def test_dynamic_mutated_shadowed_positions_stay_blocked(self):
        cases = ['local pos = Position(1, 2, 7)\npos.x = pos.x + 1',
                 'local pos = Position(1, 2, 7)\nlocal pos = Position(3, 4, 7)',
                 'local pos = Position(1, 2, 7)\npos:moveUpstairs()',
                 'local pos = config[player:getId()]']
        for declarations in cases:
            with self.subTest(declarations=declarations):
                s = self.script(declarations, 'player:teleportTo(pos)')
                child = list(converter.walk(s.interactions()[0]['rules']))[-1]
                self.assertEqual(child['status'], 'blocked')
        s = self.script('local config = { {x=1,y=2,z=7} }')
        self.assertIsNone(s.position('config[index]'))
        self.assertIsNone(s.position('Position(1 + 2, 3, 7)'))

    def test_unsupported_table_does_not_erase_sibling(self):
        s = self.script('local broken = { otherCall() }\nlocal good = { stage = 3, storage = Storage.Test.Stage }')
        self.assertEqual(s.literal('good.stage'), 3)
        self.assertEqual(s.resolve_storage('good.storage'), 'Storage.Test.Stage')
        self.assertIsNone(s.literal('broken'))

    def test_item_names_and_constants_resolve_exactly(self):
        s = self.script('local ITEM = "test reward"\nlocal COUNT = 2', 'player:addItem(ITEM, COUNT)\nplayer:removeItem(ITEM, COUNT)')
        children = list(converter.walk(s.interactions()[0]['rules']))
        self.assertEqual([c['request'] for c in children], ['hand_out', 'consume'])
        self.assertTrue(all(c['item']['key'] == 'canary:item/300' and c['count'] == 2 for c in children))
        self.assertIsNone(s.item_id('"duplicate"'))
        self.assertIsNone(s.item_id('"range"'))
        self.assertIsNone(s.item_id('"missing"'))
        self.assertIsNone(s.item_id('300 + 1'))
        self.assertEqual(s.item_id('0x12c'), 300)

    def test_boolean_precedence_parentheses_and_negation(self):
        s = self.script('local threshold = 3')
        c = s.condition('(player:getLevel() >= threshold or item:getId() == 300) and not (player:getLevel() < 2)', 999)
        self.assertIn('all', c)
        self.assertIn('any', c['all'][0])
        self.assertTrue(c['all'][1]['negate'])
        self.assertEqual(c['all'][0]['any'][1]['object']['field'], 'item_type')
        c = s.condition('player:getLevel() >= 3 or item:getId() == 300 and player:getLevel() < 10', 999)
        self.assertIn('all', c['any'][1])

    def test_getid_does_not_confuse_actor_or_use_target(self):
        s = self.script()
        for receiver in ('player', 'target'):
            self.assertIn('unresolved', s.condition(receiver + ':getId() == 300', 999))
        self.assertIn('actor_item_count', s.condition('player:getItemCount("test reward") >= 1', 999))
        self.assertIn('unresolved', s.condition('player:getItemCount("duplicate") >= 1', 999))

    def test_runtime_and_sibling_aliases_stay_unknown(self):
        s = self.script('', 'player:teleportTo(pos)\nlocal pos = Position(1, 2, 7)')
        first = list(converter.walk(s.interactions()[0]['rules']))[0]
        self.assertEqual(first['status'], 'blocked')
        self.assertIn('unresolved', s.condition('player:getLevel() >= math.random(10)', 999))
        self.assertIn('unresolved', s.condition('player:getLevel() >= 1 and unknownCall()', 999))
        s = self.script('', 'player:teleportTo(config.pos)\nlocal config = {pos={x=1,y=2,z=7}}')
        self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_branch_local_does_not_leak_to_sibling(self):
        s = self.script('', 'if player:getLevel() >= 1 then\nlocal pos = Position(1, 2, 7)\n'
                        'else\nplayer:teleportTo(pos)\nend')
        self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_alias_escape_and_inline_else_mutation_are_held(self):
        mutations = ['local other = config\nother.pos.x = 9',
                     'mutate(config)', 'mutate(config.pos)',
                     'local other = config.pos\nother.x = 9',
                     'if unknown then print(1) else config.pos.x = 9 end',
                     'print("--") ; config.pos.x = 9']
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                s = self.script('local config = {pos={x=1,y=2,z=7}}\n' + mutation,
                                'player:teleportTo(config.pos)')
                self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_forward_scalar_and_global_branch_aliases_are_held(self):
        cases = ['local COUNT = LATE\nlocal LATE = 2',
                 'if unknown then\nlocal COUNT = 2\nend',
                 'local COUNT=2\nif unknown then print(1) else COUNT=3 end']
        for declarations in cases:
            s = self.script(declarations, 'player:addItem(300, COUNT)')
            child = list(converter.walk(s.interactions()[0]['rules']))[0]
            self.assertIn('value_source_line', child)
            self.assertNotIn('count', child)

    def test_primitive_table_leaf_escape_is_safe_and_forward_leaf_is_held(self):
        s = self.script('local config={item=300,count=2}\nlocal COUNT=config.count',
                        'local reward=player:addItem(config.item, COUNT)')
        child = list(converter.walk(s.interactions()[0]['rules']))[0]
        self.assertEqual(child['count'], 2)
        s = self.script('local config={item=300,count=LATE}\nlocal LATE=2',
                        'player:addItem(config.item, config.count)')
        self.assertIn('value_source_line', list(converter.walk(s.interactions()[0]['rules']))[0])

    def test_multiline_table_and_position_alias_escapes_are_held(self):
        for declarations, target in [
            ('local config={pos={x=1,y=2,z=7}}\nlocal other=\nconfig', 'config.pos'),
            ('local pos=Position(1,2,7)\nlocal other=pos\nother.x=9', 'pos')]:
            s = self.script(declarations, 'player:teleportTo(' + target + ')')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_table_iteration_can_escape_mutable_values(self):
        for iterate in ('pairs', 'ipairs'):
            s = self.script('local config={pos={x=1,y=2,z=7}}\n'
                            'for _,p in ' + iterate + '(config) do p.x=9 end',
                            'player:teleportTo(config.pos)')
            self.assertEqual(list(converter.walk(s.interactions()[0]['rules']))[0]['status'], 'blocked')

    def test_enriched_output_matches_existing_schema(self):
        import jsonschema
        s = self.script('local pos = Position(1, 2, 7)',
                        'if player:getItemCount("test reward") >= 1 and item:getId() == 100 then\n'
                        'player:teleportTo(pos)\nplayer:addItem("test reward", 2)\nend')
        schema = json.loads((Path(converter.ROOT) / 'interaction.schema.json').read_text())
        output = s.interactions()[0]
        for key in ('script', 'callback_line'):
            output.pop(key)
        jsonschema.validate({'interactions': [output]}, schema)




class StorageAliasEnrichmentTests(unittest.TestCase):
    def test_numeric_storage_alias_records_original_line(self):
        import lua_writers
        text = 'local ObedienceStorage = 44502\nlocal a = Action()\nfunction a.onUse(player)\nplayer:setStorageValue(ObedienceStorage, 1)\nend\n'
        self.assertEqual(lua_writers.expand_aliases('local ObedienceStorage = 44502', {'ObedienceStorage': '44502'}),
                         'local ObedienceStorage = 44502')
        result = lua_writers.scan(text, 'scripts/quests/test.lua')
        self.assertEqual(result[0]['target'], '44502')
        self.assertEqual(result[0]['line'], 4)
        self.assertEqual(result[0]['to'], 1)

    def test_mutated_shadowed_or_late_alias_is_not_resolved(self):
        import lua_writers
        cases = ['local Key=44502\nKey=44503\nplayer:setStorageValue(Key,1)',
                 'local Key=44502\nlocal Key=44503\nplayer:setStorageValue(Key,1)',
                 'player:setStorageValue(Key,1)\nlocal Key=44502']
        for text in cases:
            self.assertNotIn('Key', lua_writers.storage_aliases(text.splitlines()))

    def test_config_storage_alias_preserves_named_key(self):
        import lua_writers
        text = ('local config = { storageKey = Storage.Quest.Test.Obedience }\n'
                'local a = Action()\nfunction a.onUse(player)\n'
                'player:setStorageValue(config.storageKey, 1)\nend\n')
        self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua')[0]['target'], 'Storage.Quest.Test.Obedience')
        self.assertNotIn('config.storageKey', lua_writers.storage_aliases(
            (text + 'config.storageKey = Storage.Quest.Other\n').splitlines()))

    def test_storage_alias_sibling_branch_and_escape_are_held(self):
        import lua_writers
        cases = [
            'local a=Action()\nfunction a.first(player)\nlocal Key=44502\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(Key,1)\nend',
            'if condition then\nlocal Key=44502\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(Key,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nlocal other=config\n'
            'other.storageKey=Storage.Quest.Test.B\nfunction a.onUse(player)\n'
            'player:setStorageValue(config.storageKey,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nmutate(config)\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend',
            'local config={storageKey=Storage.Quest.Test.A}\n'
            'if unknown then print(1) else config.storageKey=Storage.Quest.Test.B end\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend',
            'if condition then\nlocal config={storageKey=Storage.Quest.Test.A}\nend\n'
            'function a.onUse(player)\nplayer:setStorageValue(config.storageKey,1)\nend']
        for text in cases:
            with self.subTest(text=text):
                self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua'), [])

    def test_storage_alias_parameter_and_loop_shadows_are_held(self):
        import lua_writers
        cases = [
            'local Key=44502\nfunction a.onUse(player,Key)\nplayer:setStorageValue(Key,1)\nend',
            'local Key=44502\nfunction a.onUse(player)\nfor _,Key in pairs(config) do\n'
            'player:setStorageValue(Key,1)\nend\nend',
            'local config={storageKey=Storage.Quest.Test.A}\nfunction a.onUse(player,config)\n'
            'player:setStorageValue(config.storageKey,1)\nend']
        for text in cases:
            self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua'), [])

    def test_storage_scalar_table_leaf_is_not_parent_alias(self):
        import lua_writers
        text = ('local config={item=300,storageKey=Storage.Quest.Test.A}\n'
                'function a.onUse(player)\nlocal found=lookup(config.item)\n'
                'player:setStorageValue(config.storageKey,1)\nend')
        self.assertEqual(lua_writers.scan(text, 'scripts/quests/test.lua')[0]['target'], 'Storage.Quest.Test.A')

    def test_storage_alias_never_expands_quoted_text(self):
        import lua_writers
        line = 'player:say("Key ) StorageAlias.Stage") player:setStorageValue(Key, 1)'
        expanded = lua_writers.expand_aliases(line, {'Key': '44502', 'StorageAlias': 'Storage.Quest.Test'})
        self.assertEqual(expanded, 'player:say("Key ) StorageAlias.Stage") player:setStorageValue(44502, 1)')


if __name__ == '__main__':
    unittest.main()
