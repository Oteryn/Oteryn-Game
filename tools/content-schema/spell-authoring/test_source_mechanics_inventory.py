import unittest
from source_mechanics_inventory import scan, build
from unittest.mock import patch
from types import SimpleNamespace


class MechanicsCaptureTests(unittest.TestCase):
    def test_reserved_boolean_operators_are_not_calls(self):
        facts = scan('if not (foo()) or (bar()) and (baz()) then sink("and", "or", "not") end')
        self.assertEqual(['foo', 'bar', 'baz', 'sink'], [c['call_identity'] for c in facts['calls']])
        self.assertEqual(['"and"', '"or"', '"not"'], [a['token'] for a in facts['calls'][-1]['arguments']])

    def test_actual_source_boolean_operator_patterns_keep_nested_calls(self):
        facts = scan('return itemType:isStackable() and (item:getCount()) or (getItemWeight(item))')
        self.assertEqual(['itemType:isStackable', 'item:getCount', 'getItemWeight'], [c['call_identity'] for c in facts['calls']])
        facts = scan('if not target:isPlayer() and not (master or master:isPlayer()) then end')
        self.assertEqual(['target:isPlayer', 'master:isPlayer'], [c['call_identity'] for c in facts['calls']])

    def test_comments_strings_and_nested_area(self):
        facts = scan('''-- Combat:setParameter(fake)
local s = "fake:addDamage(5)"
local c = Combat()
c:setArea(createCombatArea({{1, 2}, {3, 0}}))
c:setParameter(COMBAT_PARAM_TYPE, COMBAT_FIREDAMAGE)
''')
        self.assertEqual(['Combat', 'c:setArea', 'createCombatArea', 'c:setParameter'], [x['call_identity'] for x in facts['calls']])
        self.assertEqual('table_expression', facts['calls'][2]['arguments'][0]['kind'])
        self.assertEqual(5, facts['calls'][3]['line'])

    def test_scheduled_anonymous_body_is_not_exported_but_calls_retained(self):
        facts = scan('addEvent(function() world:remove() end, 500, uid)')
        self.assertEqual('opaque_expression', facts['calls'][0]['arguments'][0]['kind'])
        self.assertEqual('anonymous_function_body_not_exported', facts['calls'][0]['arguments'][0]['reason'])
        self.assertEqual('unsupported_call_reference', facts['calls'][1]['category'])
        self.assertTrue(facts['calls'][0]['scheduled'])

    def test_reused_spell_variable_keeps_constructor_order_and_getter_shape(self):
        facts = scan('local spell = Spell("instant")\nspell:name("one")\nlocal spell = Spell("instant")\nspell:cooldown()\nspell:name("two")')
        self.assertEqual([0, 1], [c['spell_constructor_declaration']['constructor_ordinal'] for c in facts['calls'] if c['call_identity'] == 'Spell'])
        self.assertEqual(0, facts['calls'][1]['preceding_receiver_spell_constructor']['constructor_ordinal'])
        self.assertEqual(1, facts['calls'][3]['preceding_receiver_spell_constructor']['constructor_ordinal'])
        self.assertEqual('getter_candidate', facts['calls'][3]['registrar_method_shape'])
        self.assertEqual('setter_candidate', facts['calls'][4]['registrar_method_shape'])

    def test_chained_and_indexed_receivers_are_not_dropped(self):
        facts = scan('creature:getPosition():sendMagicEffect(CONST_ME_FIRE)\ncombats[i]:execute(creature, var)')
        self.assertEqual(['creature:getPosition', 'creature:getPosition():sendMagicEffect', 'combats[i]:execute'], [c['call_identity'] for c in facts['calls']])
        self.assertEqual('expression_tokens', facts['calls'][2]['receiver_expression']['kind'])

    def test_callback_body_commas_do_not_split_outer_arguments(self):
        facts = scan('addEvent(function() local a,b=1,2; if a then f(a,b) end end,100)')
        self.assertEqual(2, facts['calls'][0]['argument_count'])
        self.assertEqual('anonymous_function_body_not_exported', facts['calls'][0]['arguments'][0]['reason'])
        self.assertNotIn('function', [c['call_identity'] for c in facts['calls']])
        self.assertIn('f', [c['call_identity'] for c in facts['calls']])

    def test_whole_tree_constructor_and_literal_helper_population(self):
        inputs = [('data/scripts/quests/soulpit/unreferenced.lua', b'local spell = Spell("instant"); dofile("lib/helper.lua")'), ('data/lib/helper.lua', b'local x = 5'), ('data/lib/comments.lua', b'-- Spell("instant")')]
        tree = '\n'.join('100644 blob ' + str(i) + '\t' + p for i, (p, _) in enumerate(inputs)).encode()
        batch = b''.join(str(i).encode()+b' blob '+str(len(b)).encode()+b'\n'+b+b'\n' for i, (_, b) in enumerate(inputs))
        with patch('source_mechanics_inventory.git', return_value=tree), patch('source_mechanics_inventory.subprocess.run', return_value=SimpleNamespace(stdout=batch)):
            result = build('/unused-local', {})
        self.assertEqual(4, result['counts']['files'])
        self.assertEqual({'whole_tree_spell_constructor', 'source_helper_dependency'}, {f['selection'] for f in result['files']})

    def test_long_expression_order_and_complete_dependencies(self):
        names = ['coefficient_' + str(i) for i in range(150)]
        facts = scan('condition:setFormula(' + ' + '.join(names) + ')')
        arg = facts['calls'][0]['arguments'][0]
        self.assertEqual('expression_tokens', arg['kind'])
        self.assertEqual(names, arg['tokens'][::2])
        callback = scan('addEvent(function() return ' + ' + '.join(names) + ' end, 100)')['calls'][0]['arguments'][0]
        self.assertEqual('opaque_expression', callback['kind'])
        self.assertEqual(150, len([x for x in callback['symbol_dependencies'] if x.startswith('coefficient_')]))

    def test_assigned_cast_declaration_and_source_order(self):
        facts = scan('spell.onCastSpell = function(creature, var) return combat:execute(creature, var) end')
        self.assertEqual('spell.onCastSpell', facts['function_declarations'][0]['identity'])
        self.assertEqual('spell.onCastSpell', facts['calls'][0]['lexical_declaration_context'])
        self.assertEqual(0, facts['calls'][0]['source_order'])

    def test_function_declaration_not_call_and_damage_literals(self):
        facts = scan('function onTarget(creature, pos)\nif creature:isPlayer() then condition:addDamage(3, 1000, -20) end\nend')
        self.assertEqual('onTarget', facts['function_declarations'][0]['identity'])
        self.assertNotIn('onTarget', [x['call_identity'] for x in facts['calls']])
        self.assertEqual(-20, facts['calls'][-1]['arguments'][2]['value'])
        self.assertEqual(1, len(facts['branches']))


if __name__ == '__main__':
    unittest.main()
