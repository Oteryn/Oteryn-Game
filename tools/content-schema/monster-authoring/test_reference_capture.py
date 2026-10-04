"""Declaration capture cannot execute custom casts or turn reference facts into variants."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import spell_scripts as ss


def reader(root, files):
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    result = ss.SpellScripts.__new__(ss.SpellScripts)
    result.canary, result.extra_roots = root, ()
    result.index = {'case': ('instant', root / 'data/scripts/a.lua')}
    result.areas, result.enums = '', {}
    result.player_chains, result.accepted_guards = False, {}
    result.cache, result.registration_cache = {}, {}
    result.source_git, result.source_revision = root, 'a' * 40
    return result


SCRIPT = '''
local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
local condition = Condition(CONDITION_PARALYZE)
condition:setParameter(CONDITION_PARAM_TICKS, 1000)
combat:addCondition(condition)
combat:setFormula(COMBAT_FORMULA_LEVELMAGIC, 1, 2, 3, 4)
combat:setArea(createCombatArea({{1,3,1}}))
function impossibleChain() error("chain callback was executed") end
combat:setCallback(CALLBACK_PARAM_CHAINVALUE, "impossibleChain")
local spell = Spell("instant")
function spell.onCastSpell(creature, var)
  error("custom cast was executed")
end
spell:name("case")
spell:register()
'''


class ReferenceCapture(unittest.TestCase):
    def test_reused_variable_keeps_three_instances_not_nine_and_ignores_examples(self):
        script = '''-- local spell = Spell("instant")
-- spell:name("comment")
local example = 'local fake = Spell("instant") fake:name("string")'
local spell = Spell("instant")
spell:name("one")
spell = Spell("instant")
spell:name("two")
spell = Spell("instant")
spell:name("three")
'''
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            reader(root, {'data/scripts/a.lua': script})
            found = ss.registrations(root, ('data/scripts',))
        self.assertEqual({'one', 'two', 'three'}, set(found))
        self.assertEqual(3, sum(map(len, found.values())))

    def test_symbolic_example_constructor_is_source_disabled_reference(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            r = reader(root, {'data/scripts/a.lua': SCRIPT.replace('Spell("instant")', 'Spell(SPELL_INSTANT)')})
            found = ss.registrations(root, ('data/scripts',))
            kind, path = found['case'][0]
            result = r.evaluate_registration('case', kind, path)
        self.assertEqual('@spell_instant', kind)
        self.assertTrue(result['source_disabled_reference_only'])
        self.assertNotIn('variants', result)

    def test_call_sequence_keeps_duplicates_and_getter_calls(self):
        script = SCRIPT.replace('spell:register()', 'spell:cooldown(10)\nspell:cooldown()\nspell:cooldown(20)\nspell:register()')
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': script}).evaluate('case')
        self.assertEqual([[10], [], [20]], [c['args'] for c in result['spell_call_sequence'] if c['method'] == 'cooldown'])

    def test_nil_holes_and_trailing_nil_preserve_argument_positions(self):
        script = SCRIPT.replace('spell:register()', 'spell:referenceValues(nil, false, 0, nil)\nspell:register()')
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': script}).evaluate('case')
        entry = next(c for c in result['spell_call_sequence'] if c['method'] == 'referenceValues')
        self.assertEqual([None, False, 0, None], entry['args'])

    def test_legacy_dense_probe_calls_remain_supported(self):
        self.assertEqual([('setParameter', ['COMBAT_PARAM_TYPE', 0])],
                         ss.calls({'__calls': {1: {1: 'setParameter', 2: {1: 'COMBAT_PARAM_TYPE', 2: 0}}}}))

    def test_duplicate_named_instances_are_explicitly_ambiguous(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': SCRIPT + SCRIPT}).evaluate('case')
        self.assertIn('ambiguous', result['error'])
        self.assertEqual(2, len(result['reference_spell_instances']))

    def test_p4_retains_every_combat_without_invoking_callbacks(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': SCRIPT}).evaluate('case')
        self.assertEqual('P4', result['tier'])
        self.assertNotIn('error', result)
        self.assertNotIn('variants', result)
        combat = result['reference_combats'][0]
        self.assertEqual('COMBAT_PHYSICALDAMAGE', combat['params']['COMBAT_PARAM_TYPE'])
        self.assertEqual('impossibleChain', combat['callbacks']['CALLBACK_PARAM_CHAINVALUE'])
        self.assertEqual(1, len(combat['conditions']))
        self.assertIsNotNone(combat['area'])
        self.assertIsNotNone(combat['formula'])
        self.assertEqual(['setParameter', 'addCondition', 'setFormula', 'setArea', 'setCallback'],
                         [c['method'] for c in combat['call_sequence']])
        self.assertEqual('Condition', combat['call_sequence'][1]['args'][0]['__kind'])

    def test_position_zone_and_legacy_callback_are_inert(self):
        extra = '''local p = Position(1, 2, 3)
local zone = Zone.getByName("boss.case")
local positions = zone:getPositions()
setCombatCallback(combat, CALLBACK_PARAM_TARGETTILE, "tileCallback")
'''
        script = SCRIPT.replace('local spell =', extra + 'local spell =')
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': script}).evaluate('case')
        self.assertEqual(1, result['reference_positions'][0]['x'])
        self.assertTrue(result['reference_zones'][0]['positions_requested'])
        self.assertEqual('tileCallback', result['reference_combats'][0]['callbacks']['CALLBACK_PARAM_TARGETTILE'])
        self.assertEqual('P4', result['tier'])

    def test_alternate_registration_does_not_replace_winner(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            r = reader(root, {'data/scripts/a.lua': SCRIPT,
                             'data/scripts/b.lua': SCRIPT.replace('COMBAT_PHYSICALDAMAGE', 'COMBAT_FIREDAMAGE')})
            winner = r.evaluate('case')
            other = r.evaluate_registration('case', 'instant', root / 'data/scripts/b.lua')
            self.assertIs(winner, r.evaluate('case'))
        self.assertEqual('COMBAT_FIREDAMAGE', other['reference_combats'][0]['params']['COMBAT_PARAM_TYPE'])

    def test_literal_dependency_must_match_git_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            r = reader(root, {'data/scripts/a.lua': SCRIPT, 'data/libs/functions/vocation.lua': 'VOCATION = { BASE_ID = { KNIGHT = 4 } }'})
            with patch.object(ss.subprocess, 'check_output', return_value=b'changed'):
                with self.assertRaisesRegex(ValueError, 'differs from pinned'):
                    r._verified_dependency(root, 'data/libs/functions/vocation.lua')

    def test_missing_snapshot_vocation_uses_exact_pinned_literal(self):
        library = b'VOCATION = {\n BASE_ID = { KNIGHT = 4 },\n}\nfunction Vocation.world() error("must not run") end'
        script = 'local vocation = {VOCATION.BASE_ID.KNIGHT}\n' + SCRIPT
        with tempfile.TemporaryDirectory() as tmp:
            r = reader(Path(tmp), {'data/scripts/a.lua': script})
            with patch.object(ss.subprocess, 'check_output', return_value=library):
                result = r.evaluate('case')
        self.assertNotIn('error', result)
        self.assertEqual('a' * 40, result['reference_dependencies'][0]['revision'])

    def test_allowlisted_include_is_verified_literal_only(self):
        include = b'GazVariables = {\n MinionsNow = 2,\n MaxSummons = 7,\n}\n'
        script = 'dofile(DATA_DIRECTORY .. "/scripts/spells/monster/gaz_functions.lua")\n' + SCRIPT
        with tempfile.TemporaryDirectory() as tmp:
            r = reader(Path(tmp), {'data/scripts/a.lua': script})
            with patch.object(ss.subprocess, 'check_output', return_value=include):
                result = r.evaluate('case')
        self.assertNotIn('error', result)
        self.assertEqual(1, len(result['reference_dependencies']))

    def test_infinite_declaration_is_bounded(self):
        with tempfile.TemporaryDirectory() as tmp:
            result = reader(Path(tmp), {'data/scripts/a.lua': 'while true do end\n' + SCRIPT}).evaluate('case')
        self.assertIn('capture instruction limit', result['error'])

    def test_unknown_include_retains_partial_capture_and_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            r = reader(Path(tmp), {'data/scripts/a.lua': SCRIPT + '\ndofile("/etc/passwd")'})
            result = r.evaluate('case')
        self.assertIn('allowlisted', result['error'])
        self.assertFalse(result['reference_capture_complete'])
        self.assertEqual(1, len(result['reference_combats']))

    def test_unresolved_chain_evaluation_retains_static_reference(self):
        script = SCRIPT.replace('  error("custom cast was executed")', '  return combat:execute(creature, var)')
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            r = reader(root, {'data/scripts/a.lua': script})
            result = r.evaluate_registration('case', 'instant', root / 'data/scripts/a.lua')
        self.assertIn('combat evaluation', result['error'])
        self.assertTrue(result['reference_capture_complete'])
        self.assertEqual('impossibleChain', result['reference_combats'][0]['callbacks']['CALLBACK_PARAM_CHAINVALUE'])
        self.assertNotIn('variants', result)


if __name__ == '__main__':
    unittest.main()
