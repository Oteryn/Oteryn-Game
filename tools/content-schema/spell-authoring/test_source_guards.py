import json
import gzip
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import jsonschema
import export_source_guards as guards


class SourceGuards(unittest.TestCase):
    def test_nested_controls_source_order_and_cancellation(self):
        text = '''function spell.onCastSpell(player, var)
if not player:isPremium() then
 player:sendCancelMessage("Premium required")
 return false
elseif player:getSoul() < 20 then
 return false
else
 player:addSoul(-20)
end
end
'''
        result = guards.capture(text)
        conditions = [e for e in result['events'] if e['kind'] == 'guard']
        self.assertEqual(['not player:isPremium()', 'player:getSoul() < 20'], [e['expression'] for e in conditions])
        self.assertEqual([2, 5], [e['source_line'] for e in conditions])
        self.assertTrue(all(e['expression_resolution'] == 'unresolved_expression' for e in conditions))
        self.assertTrue(any('cancellation' in e['domains'] for e in result['events']))
        self.assertEqual(['player:isPremium'], conditions[0]['identifiers'])
        self.assertFalse(result['source_execution'])

    def test_literal_args_nil_false_zero_and_symbolic_expression(self):
        result = guards.capture('player:check(nil, false, 0, "value", VOCATION.BASE_ID.KNIGHT, player:getTarget())')
        call = result['events'][0]
        self.assertEqual(['nil', 'boolean', 'number', 'string', 'unresolved_expression', 'unresolved_expression'],
                         [a['kind'] for a in call['arguments']])
        self.assertEqual('VOCATION.BASE_ID.KNIGHT', call['arguments'][4]['identifier'])
        self.assertEqual(False, call['arguments'][1]['value'])
        self.assertEqual(0, call['arguments'][2]['value'])

    def test_comments_and_strings_do_not_invent_guards(self):
        text = '-- if false then\nlocal text = "if fake then"\n--[[ elseif fake then ]]\nif player:isPremium() then return false end'
        self.assertEqual(1, guards.capture(text)['guard_count'])

    def test_nested_argument_commas_are_not_split(self):
        call = guards.capture('combat:setFormula(TYPE, math.min(1,2), {3,4})')['events'][0]
        self.assertEqual(3, len(call['arguments']))
        self.assertEqual('math.min(1,2)', call['arguments'][1]['expression'])

    def test_size_bound_is_explicit_refusal(self):
        with self.assertRaises(ValueError):
            guards.capture(' ' * 1048577)

    def test_schema_refuses_activation_and_extra_fields(self):
        schema = json.loads(Path(guards.__file__).with_name('player-source-guards.schema.json').read_text())
        row = {'schema':guards.SCHEMA,'registration_key':'case','snapshot':'canary-main-current',
               'source_revision':'a'*40,'source_file':'case.lua','source_sha256':'b'*64,'git_blob':'c'*40,
               'callback_facts_present':False,'runtime_activation':False,
               'event_scope':'file_unresolved','callback_binding_resolution':'unresolved_whole_file',
               'scope':'whole_source_file_lexical_evidence_not_registration_control_flow',**guards.capture('if x then return false end')}
        jsonschema.validate(row,schema)
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate({**row,'runtime_activation':True},schema)
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate({**row,'invented_mechanic':True},schema)

    def test_source_hash_mismatch_and_population_mismatch_fail(self):
        registrar={'registration_key':'canary-main-current/case.lua#1','snapshot':'canary-main-current',
                   'source_revision':'a'*40,'source_file':'case.lua','source_sha256':'b'*64,'git_blob':'c'*40}
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);rp=root/'registrars.gz';cp=root/'callbacks.gz'
            rp.write_bytes(gzip.compress((json.dumps(registrar)+'\n').encode()))
            cp.write_bytes(gzip.compress(b''))
            with patch.object(guards.subprocess,'check_output',return_value=b'if x then end'):
                with self.assertRaisesRegex(ValueError,'digest mismatch'):
                    guards.export(rp,cp,root,root/'out.gz')
            rp.write_bytes(gzip.compress(b''))
            cp.write_bytes(gzip.compress((json.dumps(registrar)+'\n').encode()))
            with self.assertRaisesRegex(ValueError,'population mismatch'):
                guards.export(rp,cp,root,root/'out.gz')

    def test_repeated_variables_nested_callbacks_have_file_scope_only(self):
        result=guards.capture('local s=Spell("instant")\nfunction s.onCastSpell(p,v) if p:isPremium() then return false end end\ns=Spell("instant")\nfunction s.onCastSpell(p,v) local function nested() if p:getSoul()<2 then return nil end end end')
        self.assertEqual(2,result['guard_count'])
        self.assertEqual('unresolved_lexical_only',result['control_structure_resolution'])
        self.assertTrue(any(a['kind']=='nil' for e in result['events'] for a in e['arguments']))


if __name__ == '__main__':
    unittest.main()
