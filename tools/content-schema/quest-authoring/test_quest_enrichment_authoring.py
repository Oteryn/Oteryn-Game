"""SPDX-License-Identifier: MPL-2.0; bounded portable proof regressions."""
import copy
import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from jsonschema import Draft202012Validator, ValidationError
import quest_enrichment_authoring as q

BASE = Path(os.environ.get('QUEST_ENRICHMENT_TEST_ROOT', Path(__file__).resolve().parents[3]))

class EnrichmentTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        shutil.copytree(BASE / q.DIRECTORY, self.root / q.DIRECTORY)
        selected = self.root / q.SELECTION; selected.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(BASE / q.SELECTION, selected)
        self.schema = q.read(Path(q.__file__).with_name('quest_enrichment.schema.json'))
    def load(self): return q.load_enrichment(self.root)
    def packet(self): return q.read(self.root / q.DIRECTORY / 'enrichment.json')
    def write(self, name, value): (self.root / q.DIRECTORY / name).write_bytes(q.encoded(value))
    def test_exact242_and5418_portable(self):
        result=self.load(); self.assertEqual(len(result),242)
        self.assertEqual(sum(r['facts_count'] for r in result.values()),5418)
        self.assertEqual(sum(r['candidate_count'] for r in result.values()),3828)
        self.assertTrue(all(r['source_hold_resolved'] is False for r in result.values()))
    def test_capture_without_local_wiki_or_donors(self):
        original=Path.read_bytes
        def only_portable(path):
            self.assertFalse('raw-captures' in str(path) or '/quest-sources/' in str(path))
            return original(path)
        with patch.object(Path,'read_bytes',only_portable): self.assertEqual(len(self.load()),242)
        self.assertNotIn('/tmp/',json.dumps(self.packet()))
        self.assertNotIn('/workspace/',json.dumps(self.packet()))
    def test_output_tampering(self):
        v=self.packet();v['records'][0]['facts'][0]['value']={'foreign_field':True}
        self.write('enrichment.json',v)
        with self.assertRaises(ValueError):self.load()
    def test_capture_rebinding_requires_record_proof(self):
        capture=q.read(self.root/q.DIRECTORY/'capture.json');receipt=q.read(self.root/q.DIRECTORY/'receipt.json')
        capture['records'][0]['facts'][0]['evidence']['span_sha256']='0'*64
        self.write('capture.json',capture);self.write('enrichment.json',q.build(capture))
        receipt['capture_sha256']=q.sha(q.encoded(capture));receipt['output_sha256']=q.sha(q.encoded(q.build(capture)))
        self.write('receipt.json',receipt)
        with self.assertRaisesRegex(ValueError,'record proof mismatch'):self.load()
    def test_scope_missing_foreign_or_duplicate(self):
        original=self.packet()
        for mode in ('missing','foreign','duplicate'):
            v=copy.deepcopy(original)
            if mode=='missing':v['records'].pop()
            else:v['records'][0]['canonical_key']='oteryn:quest.foreign' if mode=='foreign' else v['records'][1]['canonical_key']
            self.write('enrichment.json',v)
            with self.assertRaises(ValueError):self.load()
    def test_schema_closed_deep_and_native_flags(self):
        for path,value in [('unknown',1),('runtime_enabled',True),('raw_body_rechecked',True),('source_hold_resolved',True)]:
            p=self.packet();p[path]=value
            with self.assertRaises(ValidationError):Draft202012Validator(self.schema).validate(p)
        p=self.packet();p['records'][0]['facts'][0]['value']['invented']=1
        with self.assertRaises(ValidationError):Draft202012Validator(self.schema).validate(p)
    def test_reducer_references_and_counts(self):
        counts=[];text='NPC dialogue: Bring your gold and return tomorrow.'
        result=q.reduce_text({'statement':text,'entity':'Gold Coin','quantity':20},counts)
        self.assertNotIn(text,json.dumps(result));self.assertEqual(counts,[len(text)])
        self.assertEqual(result['statement']['text_reference']['sha256'],q.sha(text.encode()))
        self.assertEqual(result['entity'],'Gold Coin');self.assertEqual(result['quantity'],20)
    def test_novelty_counter_mutation(self):
        v=q.read(self.root/q.DIRECTORY/'receipt.json');v['records'][0]['candidate_count']+=1
        self.write('receipt.json',v)
        with self.assertRaisesRegex(ValueError,'novelty inventory'):self.load()
    def test_import_rejects_unaccepted_original(self):
        p=self.root/'wrong.json';p.write_text('{}')
        with self.assertRaisesRegex(ValueError,'unaccepted enrichment input'):
            q.capture_import(p, self.root/'absent-proof',[],self.root)

if __name__=='__main__': unittest.main()
