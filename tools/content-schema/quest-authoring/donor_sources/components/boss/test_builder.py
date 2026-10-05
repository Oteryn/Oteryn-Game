import copy
import unittest
import builder as b


def name(value):
    return {'node_type': 'Name', 'fields': {'id': value}, 'span': None}


def number(value):
    return {'node_type': 'Number', 'fields': {'n': value}, 'span': None}


class BossConversionTests(unittest.TestCase):
    def test_positions_remain_syntax_not_builtin_proof(self):
        n = {'node_type': 'Call', 'fields': {'func': name('Position'), 'args': [number(1), number(2), number(3)]}, 'span': None}
        value = b.typed(n, '')
        self.assertEqual(value['syntactic_position'], {'x': 1, 'y': 2, 'z': 3})
        self.assertEqual(value['builtin_identity'], 'NOT_ASSESSED')
        n['fields']['args'][0] = name('computedX')
        self.assertNotIn('syntactic_position', b.typed(n, ''))

    def test_duplicate_table_keys_and_order_are_preserved(self):
        n = {'node_type': 'Table', 'span': None, 'fields': {'fields': [
            {'node_type': 'Field', 'span': None, 'fields': {'key': name('custom'), 'value': number(1)}},
            {'node_type': 'Field', 'span': None, 'fields': {'key': name('custom'), 'value': number(2)}},
        ]}}
        result = b.typed(n, '')
        self.assertEqual([v['value']['value'] for v in result['fields']], [1, 2])
        self.assertEqual([v['key']['name'] for v in result['fields']], ['custom', 'custom'])

    def test_unknown_expression_exact_raw_and_ast_reference(self):
        text = 'config.time or 10 * 60'
        n = {'node_type': 'OrLoOp', 'fields': {}, 'span': {'start_char': 0, 'end_char_exclusive': len(text)}}
        value = b.typed(n, text, '/fields/value')
        self.assertEqual(value['raw_expression'], text)
        self.assertEqual(value['ast_pointer'], '/fields/value')
        self.assertEqual(value['expression_sha256'], b.sha(text.encode()))

    def test_callback_and_registration_are_preserved(self):
        callback = {'node_type': 'AnonymousFunction', 'fields': {}, 'span': {'start_char': 0, 'end_char_exclusive': 14}}
        call = {'node_type': 'Invoke', 'fields': {'func': name('uid'), 'source': name('lever'), 'args': [number(5)]}, 'span': None}
        table = {'node_type': 'Table', 'span': None, 'fields': {'fields': [{'node_type': 'Field', 'span': None, 'fields': {'key': name('onUseExtra'), 'value': callback}}]}}
        statement = {'node_type': 'LocalAssign', 'span': None, 'fields': {'targets': [name('config')], 'values': [table]}}
        ast = {'fields': {'body': {'fields': {'body': [statement, call]}}}}
        output = b.definition(ast, 'function() end')
        self.assertEqual(len(output['callback_references']), 1)
        self.assertFalse(output['callback_references'][0]['semantic_admission'])
        self.assertEqual(output['registrations'][0]['arguments'][0]['value'], 5)

    def test_assignment_witness_mismatch_rejected(self):
        from pathlib import Path
        witness = {'source': 's', 'revision': 'r', 'path': 'p', 'sha256': 'a', 'git_blob_sha1': 'b', 'byte_count': 3}
        altered = copy.deepcopy(witness)
        altered['byte_count'] = 4
        with self.assertRaisesRegex(ValueError, 'witness mismatch'):
            b.verified_raw(altered, {('s', 'r', 'p'): witness}, Path('manifest.json'))

    def test_ast_container_tamper_rejected(self):
        import tempfile
        import pathlib
        import json
        import gzip
        with tempfile.TemporaryDirectory() as td:
            root = pathlib.Path(td)
            (root / 'captures').mkdir()
            raw = b'local config = {}'
            digest = b.sha(raw)
            payload = gzip.compress(json.dumps({'status': 'PARSED', 'sha256': digest,
                'raw_bytes_base64': b.base64.b64encode(raw).decode(), 'ast': {}}).encode(), mtime=0)
            (root / 'index.json').write_text(json.dumps({'captures': [{'sha256': digest, 'container_sha256': b.sha(payload)}]}))
            (root / 'captures' / (digest + '.json.gz')).write_bytes(payload + b'changed')
            with self.assertRaisesRegex(ValueError, 'container mismatch'):
                b.ASTCache(root, {digest}).get(digest, raw)

    def test_schema_blocks_native_promotion(self):
        import json
        from pathlib import Path
        import jsonschema
        schema = json.loads(Path(__file__).with_name('schema.json').read_text())
        empty = {'schema': 'OTERYN_BOSS_COMPONENT_SOURCE/v1',
                 'scope': 'TYPED_DONOR_SOURCE_NOT_RUNTIME_OR_QUEST_COMPLETENESS',
                 'records': [], 'controllers': [], 'summary': {'records': 0, 'typed_source_records': 0, 'semantically_complete_records': 0},
                 'native_admission': True}
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(empty)


if __name__ == '__main__':
    unittest.main()
