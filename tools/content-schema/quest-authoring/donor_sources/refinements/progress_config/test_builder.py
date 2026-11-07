import copy
import json
import os
import sys
import unittest
from pathlib import Path
sys.path.insert(0,os.environ.get('QUEST_AUTHORING_ROOT',str(Path(__file__).resolve().parents[3])))
from donor_sources.refinements.fields import builder as fields
import builder as b


class ConfigWriteTests(unittest.TestCase):
    def setUp(self):
        fixture=json.loads(Path(__file__).with_name('fixtures.json').read_text())
        self.engines=[]
        for path in [b.SCRIPT,b.HELPER]:
            f=next(r for r in fixture['sources'] if r['provenance']['path']==path)
            self.engines.append(fields.Engine(f['ast'],f['raw'].encode(),f['provenance']))
        self.refs=fixture['mission_refs']

    def test_exact_config_storage_write_and_damaging_players(self):
        record=b.project(*self.engines,self.refs)
        self.assertEqual(record['source_target'],b.TRACK)
        self.assertEqual(record['operation']['declared_case_value'],2)
        self.assertEqual(record['case']['key'],b.BOSS)
        self.assertIn('CALL_CALLBACK_ONLY_FOR_TRUTHY_PLAYER',record['operation']['player_iteration']['steps'])
        self.assertFalse(record['runtime_enabled'])
        self.assertIsNone(record['operation']['native_storage_slot'])

    def test_changed_config_value_rejected(self):
        for _,node in self.engines[0].nodes:
            if node['node_type']=='Field' and node['fields']['key']['node_type']=='String' and b.base64.b64decode(node['fields']['key']['fields']['s']['bytes_base64']).decode()==b.BOSS:
                for field in node['fields']['value']['fields']['fields']:
                    if b.dotted(field['fields']['key'])=='value':field['fields']['value']['fields']['n']=999
        with self.assertRaisesRegex(ValueError,'storage/value differs'):b.project(*self.engines,self.refs)

    def test_changed_generic_write_operand_rejected(self):
        for _,node in self.engines[0].nodes:
            if node['node_type']=='Invoke' and b.dotted(node['fields']['func'])=='setStorageValue':node['fields']['args'][1]['fields']['idx']['fields']['id']='g_value'
        with self.assertRaisesRegex(ValueError,'write operands differ'):b.project(*self.engines,self.refs)

    def test_raw_script_guard_or_callback_drift_rejected(self):
        self.engines[0].text=self.engines[0].text.replace('boss.value then','boss.value + 1 then')
        with self.assertRaisesRegex(ValueError,'finite Source script body drift'):b.project(*self.engines,self.refs)

    def test_helper_body_drift_rejected(self):
        helper=self.engines[1]
        helper.text=helper.text.replace('Player(key)','Player(999)')
        with self.assertRaisesRegex(ValueError,'helper body drift'):b.project(*self.engines,self.refs)

    def test_runtime_and_fake_native_slot_rejected_by_schema(self):
        import jsonschema
        r=b.project(*self.engines,self.refs)
        schema=json.loads(Path(__file__).with_name('schema.json').read_text())['properties']['records']['items']
        validator=jsonschema.Draft202012Validator(schema);validator.validate(r)
        wrong=copy.deepcopy(r);wrong['native_admission']=True
        with self.assertRaises(jsonschema.ValidationError):validator.validate(wrong)
        wrong=copy.deepcopy(r);wrong['operation']['native_storage_slot']=5501
        with self.assertRaises(jsonschema.ValidationError):validator.validate(wrong)


if __name__=='__main__':unittest.main()
