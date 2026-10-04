import base64
import gzip
import hashlib
import json
from pathlib import Path
import unittest
import tempfile
from luaparser import ast,astnodes
import source_syntax as exporter

ROOT=Path(__file__).resolve().parents[3]


def item(data):
    return {'source':'canary','revision':exporter.PINS['canary'],'path':'fixture.lua','sha256':exporter.sha(data),'bytes':len(data),
            'git_blob':hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()}


class SourceSyntaxTests(unittest.TestCase):
    def parse(self,text):
        data=text.encode()
        return exporter.parse_record(item(data),data)

    def test_anonymous_body_and_ordered_false_nil_zero_are_preserved(self):
        row=self.parse('local fn = function(x) return false, nil, 0, x+2 end')
        nodes=row['ast']['nodes']
        anonymous=next(n for n in nodes if n['kind']=='AnonymousFunction')
        body=nodes[anonymous['fields']['body']['node_ref']]
        ret=nodes[body['fields']['body'][0]['node_ref']]
        values=[nodes[r['node_ref']] for r in ret['fields']['values']]
        self.assertEqual([v['kind'] for v in values],['FalseExpr','Nil','Number','AddOp'])
        self.assertIs(values[0]['fields']['value'],False)
        self.assertEqual(values[2]['fields']['n'],0)
        self.assertFalse(row['binding_qualified'])
        self.assertFalse(row['source_code_activation'])

    def test_empty_list_optional_node_nil_key_and_positional_index_are_distinct(self):
        row=self.parse('local x={false, [nil]=true}; local f=function() end')
        fields=[n for n in row['ast']['nodes'] if n['kind']=='Field']
        self.assertIsNone(fields[0]['fields']['key'])
        self.assertEqual(fields[0]['fields']['index'],1)
        self.assertEqual(row['ast']['nodes'][fields[1]['fields']['key']['node_ref']]['kind'],'Nil')
        self.assertIsNone(fields[1]['fields']['index'])
        fn=next(n for n in row['ast']['nodes'] if n['kind']=='AnonymousFunction')
        self.assertEqual(fn['fields']['args'],[])
        self.assertEqual(row['ast']['nodes'][fn['fields']['body']['node_ref']]['fields']['body'],[])

    def test_wrapping_attributes_and_numeric_literal_spellings(self):
        row=self.parse('local x <const> = (1.0); local y=0x10; local z=1e3; local a=1')
        self.assertTrue(any(n['kind']=='Attribute' for n in row['ast']['nodes']))
        numbers=[n for n in row['ast']['nodes'] if n['kind']=='Number']
        self.assertEqual([n['numeric_literal_spelling'] for n in numbers],['1.0','0x10','1e3','1'])
        self.assertTrue(numbers[0]['fields']['wrapped'])
        self.assertEqual(numbers[1]['fields']['n'],16)

    def test_strings_preserve_upstream_bytes_and_authoritative_literal_spelling(self):
        row=self.parse('local x="\\xFF"; local y=[=[\nlong]=]')
        strings=[n for n in row['ast']['nodes'] if n['kind']=='String']
        self.assertEqual(strings[0]['string_literal_spelling'],'"\\xFF"')
        self.assertEqual(base64.b64decode(strings[0]['fields']['s']['data']),b'\xc3\xbf')
        self.assertEqual(strings[1]['string_literal_spelling'],'[=[\nlong]=]')
        self.assertTrue(all(n['parser_decoded_string_qualified'] is False for n in strings))
        self.assertEqual(strings[1]['fields']['delimiter'],'DOUBLE_SQUARE')

    def test_upstream_anchor_span_never_claims_full_call_coverage(self):
        row=self.parse('f "a"')
        call=next(n for n in row['ast']['nodes'] if n['kind']=='Call')
        self.assertEqual(call['source_span_coverage'],'upstream_token_interval_not_full_node_qualified')
        self.assertEqual(call['source_span']['char_start'],2)
        names=[n for n in row['ast']['nodes'] if n['kind']=='Name']
        self.assertTrue(any(n['source_span'] is None for n in names))

    def test_syntax_errors_and_upstream_builder_limits_are_distinct(self):
        invalid=self.parse('local x = function(')
        self.assertFalse(invalid['syntax_valid'])
        self.assertEqual(invalid['parse_status'],'syntax_error')
        limitation=self.parse('local x=0x1p2')
        self.assertIsNone(limitation['syntax_valid'])
        self.assertEqual(limitation['parse_status'],'upstream_builder_error')
        self.assertIsNone(limitation['ast'])

    def test_closed_mapping_rejects_new_semantic_fields_and_is_deterministic(self):
        tree=ast.parse('local x=1')
        first=exporter.serialize(tree,'local x=1')
        self.assertEqual(first,exporter.serialize(tree,'local x=1'))
        tree.body.body[0].unexpected_semantic_field=0
        with self.assertRaisesRegex(ValueError,'unmapped upstream attributes'):
            exporter.serialize(tree,'local x=1')
        with self.assertRaises(ValueError):exporter.parse_record(item(b'local x=1'),b'local x=2')

    def test_operator_call_and_fornum_step_distinctions(self):
        row=self.parse('for i=1,2 do obj:run(i//2 & 1) end; for j=1,2,1 do f(j) end')
        nodes=row['ast']['nodes']
        loops=[n for n in nodes if n['kind']=='Fornum']
        self.assertEqual(loops[0]['fields']['step'],1)
        self.assertIsInstance(loops[1]['fields']['step'],dict)
        kinds={n['kind'] for n in nodes}
        self.assertTrue({'Invoke','Call','FloorDivOp','BAndOp'}.issubset(kinds))

    def test_actual_pinned_source_subset_generation_is_byte_deterministic(self):
        inventory_path=ROOT/'docs/reference/spells/r28-source-closure/source-mechanics-inventory.json.gz'
        inventory=json.loads(gzip.decompress(inventory_path.read_bytes()))
        selected=[]
        for source in exporter.PINS:
            files=sorted([f for f in inventory['files'] if f['source']==source],key=lambda f:f['path'])
            selected.extend(files[:3])
        inventory['files']=selected
        with tempfile.TemporaryDirectory() as directory:
            temp=Path(directory)
            cohort=temp/'cohort.json.gz'
            cohort.write_bytes(gzip.compress(json.dumps(inventory).encode(),mtime=0))
            schema=Path(__file__).parent/'source-syntax.schema.json'
            first=exporter.generate(cohort,temp/'first',schema)
            second=exporter.generate(cohort,temp/'second',schema)
            self.assertEqual(first['record_count'],6)
            self.assertEqual(first['gzip_sha256'],second['gzip_sha256'])
            self.assertEqual(first['payload_sha256'],second['payload_sha256'])
            self.assertEqual(first['counts']['parsed'],6)

    def test_dependency_provenance_binds_raw_installed_metadata_bytes(self):
        from importlib.metadata import distribution
        proof=exporter.dependency_provenance()
        self.assertEqual(set(proof),set(exporter.DEPENDENCIES))
        for name,row in proof.items():
            installed=distribution(name)
            for filename,key in (('METADATA','metadata_sha256'),('RECORD','record_sha256')):
                entries=[entry for entry in installed.files if str(entry).endswith('.dist-info/'+filename)]
                self.assertEqual(len(entries),1)
                self.assertEqual(row[key],hashlib.sha256(Path(installed.locate_file(entries[0])).read_bytes()).hexdigest())

    def test_strict_schema_validates_fixture_and_rejects_arbitrary_fields(self):
        from jsonschema import Draft202012Validator
        schema=json.loads((Path(__file__).parent/'source-syntax.schema.json').read_text())
        validator=Draft202012Validator(schema)
        row=self.parse('local x = false')
        validator.validate(row)
        row['ast']['nodes'][0]['fields']['unknown']=None
        self.assertTrue(list(validator.iter_errors(row)))


if __name__=='__main__':unittest.main()
