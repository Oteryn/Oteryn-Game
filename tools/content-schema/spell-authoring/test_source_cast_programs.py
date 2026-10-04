import copy
import json
from pathlib import Path
import unittest

from luaparser import ast
import jsonschema
import source_cast_programs as producer
import source_syntax

REPO = Path(__file__).resolve().parents[3]


def fixture(text):
    tree, counts, depth = source_syntax.serialize(ast.parse(text), text)
    return {'syntax_valid': True, 'parse_status': 'parsed', 'ast': tree,
            'node_count': len(tree['nodes'])}


class SourceCastProgramTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.records = producer.build(REPO)
        cls.by_key = {row['registration_key']: row for row in cls.records}

    def test_exact_population_and_schema(self):
        self.assertEqual(len(self.records), 21)
        self.assertEqual(set(self.by_key), set(producer.EXPECTED_KEYS))
        validator = jsonschema.Draft202012Validator(producer.schema())
        for record in self.records:
            validator.validate(record)
            self.assertEqual(record['completeness']['same_file_ast_nodes_reachable'], record['syntax']['node_count'])
            self.assertTrue(record['completeness']['same_file_typed_statement_program_complete'])
            self.assertFalse(record['completeness']['transitive_external_helper_source_closure_complete'])
            self.assertFalse(record['complete_spell_candidate'])
            self.assertFalse(record['native_execution_qualified'])
            self.assertFalse(record['runtime_activation'])

    def test_refusal_has_nested_branch_return_before_combat(self):
        row = fixture('function spell.onCastSpell(c, v) if reject(c) then c:sendCancelMessage("No") return false else return combat:execute(c,v) end end')
        program = producer.lower(row); ins = {n['source_node_ref']: n for n in program['instructions']}
        fn = program['functions'][0]; body = ins[fn['body_instruction_ref']]
        branch = ins[body['operands']['body'][0]['instruction_ref']]
        self.assertEqual(branch['opcode'], 'if')
        refusal = ins[branch['operands']['body']['instruction_ref']]
        self.assertEqual([ins[x['instruction_ref']]['opcode'] for x in refusal['operands']['body']], ['method_call', 'return'])
        false_ref = ins[refusal['operands']['body'][1]['instruction_ref']]['operands']['values'][0]['syntax_node_ref']
        self.assertEqual(row['ast']['nodes'][false_ref]['kind'], 'FalseExpr')
        self.assertTrue(producer.verify_lowering(program, row))

    def test_removal_before_combat_is_statement_order(self):
        row = next(record for key, record in self.by_key.items() if key.startswith('canary') and key.endswith('support/cancel_magic_shield.lua#1'))
        ins = {n['source_node_ref']: n for n in row['instructions']}
        fn = next(fn for fn in row['functions'] if fn['is_cast_entrypoint'])
        body = ins[fn['body_instruction_ref']]['operands']['body']
        self.assertEqual([ins[x['instruction_ref']]['opcode'] for x in body], ['method_call', 'return'])
        first = next(call for call in row['dependency_calls'] if call['source_node_ref'] == body[0]['instruction_ref'])
        self.assertEqual(first['callee_label'], 'creature:removeCondition')
        returned = ins[body[1]['instruction_ref']]['operands']['values'][0]['instruction_ref']
        call = next(call for call in row['dependency_calls'] if call['source_node_ref'] == returned)
        self.assertEqual(call['callee_label'], 'combat:execute')

    def test_order_mutation_and_dangling_ref_fail(self):
        row = fixture('function spell.onCastSpell(c,v) c:removeCondition(1) return combat:execute(c,v) end')
        program = producer.lower(row); changed = copy.deepcopy(program)
        fn = changed['functions'][0]
        body = next(node for node in changed['instructions'] if node['source_node_ref'] == fn['body_instruction_ref'])
        body['operands']['body'].reverse()
        with self.assertRaises(ValueError): producer.verify_lowering(changed, row)
        bad = copy.deepcopy(row); bad['ast']['nodes'][0]['fields']['body']['node_ref'] = 999999
        with self.assertRaises(ValueError): producer.lower(bad)

    def test_loops_default_step_and_helpers_preserved(self):
        row = fixture('local function helper(c) for i=1,3 do c:ping(i) end return nil end function spell.onCastSpell(c,v) helper(c) return false end')
        program = producer.lower(row)
        self.assertEqual(len(program['functions']), 2)
        loop = next(node for node in program['instructions'] if node['opcode'] == 'numeric_for')
        self.assertEqual(loop['operands']['step'], 1)
        call = next(call for call in program['dependency_calls'] if call['callee_label'] == 'helper')
        self.assertEqual(len(call['same_file_function_candidates']), 1)
        self.assertFalse(call['name_match_is_lexical_binding_qualified'])
        self.assertFalse(call['external_api_semantics_qualified'])

    def test_secondary_function_graph_and_multicombat_retained(self):
        nature = next(row for key,row in self.by_key.items() if key.startswith('crystal') and key.endswith("healing/nature's_embrace.lua#1"))
        helper = next(fn for fn in nature['functions'] if fn['name'] == 'shareConservationHeal')
        calls = [call for call in nature['dependency_calls'] if call['callee_label'] == helper['name']]
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0]['same_file_function_candidates'], [helper['source_node_ref']])
        fork = next(row for key,row in self.by_key.items() if key.endswith('forked_glacier.lua#1'))
        self.assertEqual(len(fork['source_combat_constructors']), 1)
        self.assertTrue(any(call['callee_label'].endswith(':getWheelSpellAdditionalTarget') for call in fork['dependency_calls']))
        self.assertTrue(any(node['opcode'] == 'if' for node in fork['instructions']))

    def test_strict_instruction_unknown_fields_and_runtime_flags_rejected(self):
        row = copy.deepcopy(self.records[0]); row['instructions'][0]['operands']['unknown'] = 42
        with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, producer.schema())
        row = copy.deepcopy(self.records[0]); row['native_execution_qualified'] = True
        with self.assertRaises(jsonschema.ValidationError): jsonschema.validate(row, producer.schema())


if __name__ == '__main__':
    unittest.main()
