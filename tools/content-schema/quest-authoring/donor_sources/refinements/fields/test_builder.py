import unittest
import builder as b


def node(kind, **fields):
    return {'node_type': kind, 'fields': fields, 'span': None}


def name(value):
    return node('Name', id=value)


def index(base, key, dot=True):
    return node('Index', value=base, idx=key, notation={'name': 'DOT' if dot else 'SQUARE'})


def chunk(stmts):
    return node('Chunk', body=node('Block', body=stmts))


class FieldOperands(unittest.TestCase):
    def engine(self, ast):
        return b.Engine(ast, b'', {'source': 'fixture'}, property_witnesses=[])

    def test_dot_name_is_literal_not_global(self):
        ast = chunk([])
        spec = self.engine(ast).project(index(name('target'), name('uid')), '/fields/read')
        self.assertEqual(spec['key'], {'kind': 'literal_field_key', 'type': 'string', 'value': 'uid'})
        self.assertEqual(spec['receiver']['ast_pointer'], '/fields/read/fields/value')
        self.assertTrue(spec['value_type_not_inferred'])

    def test_dynamic_index_and_nested_indices_preserve_order(self):
        outer = index(name('parchmentText'), index(name('reward'), index(name('item'), name('uid')), False), False)
        spec = self.engine(chunk([])).project(outer, '/r')
        self.assertEqual(spec['key']['node_type'], 'Index')
        self.assertEqual(spec['evaluation_order'], ['receiver', 'key', 'lua_index_operation'])
        self.assertIn('NO_CONSTANT_OR_TABLE_LOOKUP_FOLDING', spec['operation_semantics'])

    def test_callee_receiver_is_dependency_not_fake_resolved(self):
        n = index(node('Invoke', source=name('creature'), func=name('getOutfit'), args=[]), name('lookBody'))
        spec = self.engine(chunk([])).project(n, '/r')
        self.assertEqual(spec['receiver'], {'kind': 'operand_reference', 'ast_pointer': '/r/fields/value', 'node_type': 'Invoke'})
        self.assertFalse(spec['native_admission'])

    def test_lexical_shadowing_initial_value_not_folded(self):
        const = node('LocalAssign', targets=[name('RAM_ITEM_ID')], values=[node('Number', n=123)])
        use = node('If', test=name('RAM_ITEM_ID'))
        ast = chunk([const, use])
        spec = self.engine(ast).project(use['fields']['test'], '/fields/body/fields/body/1/fields/test')
        self.assertEqual(spec['binding']['kind'], 'local_declaration')
        self.assertEqual(spec['initial_declaration_witness']['declared_literal']['value'], 123)
        self.assertEqual(spec['kind'], 'source_named_operand')
        self.assertNotIn('value', spec)

    def test_forward_local_does_not_bind_prior_use(self):
        ast = chunk([node('If', test=name('value')), node('LocalAssign', targets=[name('value')], values=[node('Number', n=1)])])
        self.assertIsNone(self.engine(ast).resolve('value', '/fields/body/fields/body/0/fields/test'))

    def test_metatable_and_error_semantics_preserved(self):
        spec = self.engine(chunk([])).project(index(name('unknown'), name('type')), '/r')
        self.assertIn('USERDATA_INDEX_DISPATCH_PRESERVED', spec['operation_semantics'])
        self.assertIn('INVALID_RECEIVER_INDEX_ERRORS_AND_CALLBACK_SIDE_EFFECTS_PRESERVED', spec['operation_semantics'])
        self.assertEqual(spec['property_dispatch_activation'], 'CONDITIONAL_ON_ACTUAL_RECEIVER_METATABLE_NOT_ASSUMED')
        self.assertEqual(spec['property_dispatch_candidates'], [])

    def test_multireturn_initial_value_is_not_guessed(self):
        assign = node('LocalAssign', targets=[name('a'), name('b')], values=[node('Call', func=name('fn'), args=[])])
        ast = chunk([assign, node('If', test=name('b'))])
        spec = self.engine(ast).project(name('b'), '/fields/body/fields/body/1/fields/test')
        self.assertEqual(spec['initial_declaration_witness']['state'], 'MULTIRETURN_ADJUSTMENT_NOT_FOLDED')

    def test_loop_binding_and_parameter_binding(self):
        loop = node('Forin', targets=[name('entry')], body=node('Block', body=[node('If', test=name('entry'))]))
        ast = chunk([loop])
        spec = self.engine(ast).project(name('entry'), '/fields/body/fields/body/0/fields/body/fields/body/0/fields/test')
        self.assertEqual(spec['binding']['kind'], 'loop_binding')

    def test_nonfield_unsupported_is_not_claimed(self):
        self.assertIsNone(self.engine(chunk([])).project(node('Call', func=name('unknown'), args=[]), '/r'))

    def test_property_witness_never_assigns_receiver_type(self):
        proof = {'property_names': ['uid'], 'conditional_receiver_metatables': ['Item'],
                 'uid_semantics': 'UNIQUEID_OR_TEMPORARY_SCRIPT_HANDLE'}
        engine = b.Engine(chunk([]), b'', {'source': 'fixture'}, property_witnesses=[proof])
        spec = engine.project(index(name('target'), name('uid')), '/r')
        self.assertEqual(spec['property_dispatch_candidates'], [proof])
        self.assertNotIn('receiver_type', spec)
        self.assertTrue(spec['value_type_not_inferred'])

    def test_utf8_offsets_are_bytes_not_characters(self):
        text = 'żółć target.uid'
        n = index(name('target'), name('uid'))
        n['span'] = {'start_char': 5, 'end_char_exclusive': len(text)}
        engine = b.Engine(chunk([]), text.encode(), {'source': 'fixture'})
        w = engine.project(n, '/r')['source_witness']
        self.assertEqual(w['raw_expression'], 'target.uid')
        self.assertEqual(w['byte_start'], len('żółć '.encode()))
        self.assertEqual(w['byte_end_exclusive'], len(text.encode()))

    def test_initializer_field_order_and_duplicates_retained(self):
        table = node('Table', fields=[node('Field', key=name('x'), value=node('Number', n=1)),
                                    node('Field', key=name('x'), value=node('Number', n=2))])
        ast = chunk([node('LocalAssign', targets=[name('config')], values=[table]), node('If', test=name('config'))])
        spec = self.engine(ast).project(index(name('config'), name('x')), '/fields/body/fields/body/1/fields/test')
        fields = spec['initial_receiver_declaration']['ordered_fields']
        self.assertEqual([v['order'] for v in fields], [0, 1])
        self.assertEqual([v['key_name'] for v in fields], ['x', 'x'])

    def test_item_metatable_getter_body_drift_rejected(self):
        import json
        from pathlib import Path
        fixture = json.loads(Path(__file__).with_name('proof-fixtures.json').read_text())
        self.assertEqual(b.validate_item_index_model(fixture['item_lua'].encode())['uid'], 'methods.getUniqueId(self)')
        with self.assertRaisesRegex(ValueError, 'body drift'):
            b.validate_item_index_model(fixture['item_lua'].replace('methods.getUniqueId(self)', 'methods.getId(self)').encode())

    def test_cpp_uid_handle_behavior_drift_rejected(self):
        import json
        from pathlib import Path
        fixture = json.loads(Path(__file__).with_name('proof-fixtures.json').read_text())
        for source in ('canary', 'crystalserver'):
            raw = fixture['uid_cpp_'+source].encode()
            b.validate_uid_cpp_model(raw, source)
            with self.assertRaisesRegex(ValueError, 'body drift'):
                b.validate_uid_cpp_model(raw.replace(b'addThing(item)', b'getID(item)'), source)


if __name__ == '__main__':
    unittest.main()
