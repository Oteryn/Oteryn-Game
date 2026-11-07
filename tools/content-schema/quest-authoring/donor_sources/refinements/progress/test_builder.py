import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
sys.path.insert(0, os.environ.get('QUEST_AUTHORING_ROOT', str(Path(__file__).resolve().parents[3])))
from donor_sources.refinements.fields import builder as fields
import builder as b


def n(kind, **kw):
    return {'node_type': kind, 'fields': kw, 'span': None}


def name(value):
    return n('Name', id=value)


def index(base, key, square=False):
    return n('Index', value=base, idx=n('Number', n=key) if square else name(key), notation={'name': 'SQUARE' if square else 'DOT'})


def chunk(stmts):
    return n('Chunk', body=n('Block', body=stmts))


class MissionSupplement(unittest.TestCase):
    def test_numeric_local_alias_identity(self):
        storage = index(index(index(name('Storage'), 'Quest'), 'U11_40'), 'ThreatenedDreams')
        decl = n('LocalAssign', targets=[name('TD')], values=[storage])
        use = index(index(name('TD'), 'Mission02'), 1, True)
        ast = chunk([decl, n('If', test=use)])
        engine = fields.Engine(ast, b'', {})
        result = b.source_path(use, '/fields/body/fields/body/1/fields/test', engine)
        self.assertEqual(result[0], 'Storage.Quest.U11_40.ThreatenedDreams.Mission02[1]')
        self.assertEqual(len(result[1]), 1)
        self.assertEqual(result[1][0]['binding']['name'], 'TD')

    def test_dynamic_index_is_not_guessed_one(self):
        use = index(index(name('Storage'), 'Mission02'), 'index')
        use['fields']['notation']['name'] = 'SQUARE'
        engine = fields.Engine(chunk([]), b'', {})
        self.assertIsNone(b.source_path(use, '/r', engine))

    def test_shadowed_storage_root_is_rejected(self):
        decl = n('LocalAssign', targets=[name('Storage')], values=[n('Table', fields=[])])
        use = index(name('Storage'), 'Quest')
        ast = chunk([decl, n('If', test=use)])
        self.assertIsNone(b.source_path(use, '/fields/body/fields/body/1/fields/test', fields.Engine(ast, b'', {})))

    def test_reassigned_alias_is_rejected(self):
        decl = n('LocalAssign', targets=[name('TD')], values=[index(name('Storage'), 'Quest')])
        assign = n('Assign', targets=[name('TD')], values=[n('Table', fields=[])])
        use = index(name('TD'), 'Mission02')
        ast = chunk([decl, assign, n('If', test=use)])
        self.assertIsNone(b.source_path(use, '/fields/body/fields/body/2/fields/test', fields.Engine(ast, b'', {})))

    def test_multireturn_alias_not_guessed(self):
        ast = chunk([n('LocalAssign', targets=[name('TD'), name('second')], values=[index(name('Storage'), 'Quest')]), n('If', test=name('TD'))])
        self.assertIsNone(b.source_path(name('TD'), '/fields/body/fields/body/1/fields/test', fields.Engine(ast, b'', {})))

    def test_else_and_callback_control_flow_are_preserved(self):
        write = n('Invoke', func=name('setStorageValue'), source=name('player'), args=[])
        branch = n('If', test=n('TrueExpr'), body=n('Block', body=[]), orelse=n('Else', body=n('Block', body=[write])))
        function = n('Function', body=n('Block', body=[branch]), args=[name('player')])
        engine = fields.Engine(chunk([function]), b'', {})
        pointer='/fields/body/fields/body/0/fields/body/fields/body/0/fields/orelse/fields/body/fields/body/0'
        ctx=b.enclosing_context(engine,pointer)
        self.assertEqual([v['kind'] for v in ctx], ['Function','If','Else'])
        self.assertTrue(next(v for v in ctx if v['kind']=='If')['is_orelse_path'])

    def test_mission_join_retains_two_owners(self):
        with tempfile.TemporaryDirectory() as td:
            root=Path(td);folder=root/'content/quests/definitions';folder.mkdir(parents=True)
            rows=[]
            for i,track in enumerate(b.TRACKS.values()):
                for owner in range(2 if i==2 else 1):
                    mission={'progress':track,'key':'m'+str(i)}
                    rows.append({'definition':{'identity':{'key':'quest'+str(i)+str(owner)},'source_data':{'quest':{'identity':{'key':'source'+str(i)},'missions':[mission]}}}})
            (folder/'quests-00000-00099.json').write_text(json.dumps({'records':rows}))
            refs=b.mission_bindings(root)
            self.assertEqual([len(refs[t]) for t in b.TRACKS.values()],[1,1,2])
            for links in refs.values():
                for link in links:self.assertEqual(b.digest(link['mission']),link['mission_sha256'])

    def test_colon_method_lookup_after_args_rejected(self):
        import copy
        import jsonschema
        here=Path(__file__).parent
        schema=json.loads((here/'schema.json').read_text())
        record=json.loads((here/'record-fixture.json').read_text())
        validator=jsonschema.Draft202012Validator(schema['properties']['records']['items'])
        validator.validate(record)
        altered=copy.deepcopy(record)
        altered['operation']['evaluation_order']=['READ_RECEIVER_ONCE','EVALUATE_STORAGE_KEY','EVALUATE_VALUE','LOOKUP_METHOD','CALL_WITH_RECEIVER_SELF']
        with self.assertRaises(jsonschema.ValidationError):validator.validate(altered)

    def test_fake_native_slot_or_runtime_promotion_rejected(self):
        import copy
        import jsonschema
        here=Path(__file__).parent
        schema=json.loads((here/'schema.json').read_text())
        record=json.loads((here/'record-fixture.json').read_text())
        validator=jsonschema.Draft202012Validator(schema['properties']['records']['items'])
        for field in ('runtime_enabled','native_admission','canonical_gap_replaced'):
            altered=copy.deepcopy(record);altered[field]=True
            with self.assertRaises(jsonschema.ValidationError):validator.validate(altered)
        altered=copy.deepcopy(record);altered['operation']['storage_key_operand']['native_storage_slot']=12345
        with self.assertRaises(jsonschema.ValidationError):validator.validate(altered)


if __name__ == '__main__':
    unittest.main()
