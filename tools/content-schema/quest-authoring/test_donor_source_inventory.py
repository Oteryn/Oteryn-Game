import ast,hashlib,json,tempfile,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
HERE=Path(__file__).parent
SOURCES=HERE/'donor_sources'
PACKETS=HERE/'samples/donor-source/inventory'
class InventoryEvidenceTests(unittest.TestCase):
 def source_reader(self):
  tree=ast.parse((SOURCES/'donor_inventory.py').read_text());node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='source_text')
  ns={'pathlib':__import__('pathlib'),'hashlib':hashlib,'verified_reads':{}};exec(compile(ast.Module(body=[node],type_ignores=[]),'<source_reader>','exec'),ns);return ns['source_text']
 def test_referenced_bytes_fences(self):
  with tempfile.TemporaryDirectory() as tmp:
   p=Path(tmp)/'source.lua';raw=b'local quest = {name = "Exact"}\n';p.write_bytes(raw)
   f={'cache_path':str(p),'path':'source.lua','byte_count':len(raw),'sha256':hashlib.sha256(raw).hexdigest(),'git_blob_sha1':hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()}
   self.assertEqual(self.source_reader()(f),raw.decode())
   for key,value in [('byte_count',len(raw)+1),('sha256','0'*64),('git_blob_sha1','0'*40)]:
    with self.subTest(key=key),self.assertRaisesRegex(ValueError,'raw source bytes differ'):self.source_reader()({**f,key:value})
 def test_all_primary_declarations_use_exact_provenance_not_title(self):
  packet=json.loads((PACKETS/'inventory.json').read_text());joined=[q for q in packet['quest_log_declarations'] if q['verdict']=='EXACT_EXISTING_MANIFEST_JOIN'];self.assertEqual(len(joined),109)
  self.assertTrue(all(q['canonical_quest_key'] and q['provenance']['line']>0 and q['source_declaration_id'].endswith(':'+str(q['provenance']['line'])) for q in joined))
  demo=[q for q in packet['quest_log_declarations'] if q['verdict']=='ALTERNATIVE_PACK_DEMO_NOT_DEFAULT_GLOBAL_QUEST'];self.assertEqual(len(demo),1);self.assertIsNone(demo[0]['canonical_quest_key']);self.assertEqual(demo[0]['source_name'],'Example')
 def test_summer_does_not_create_quests_by_table_index_or_folder(self):
  j=json.loads((PACKETS/'correction-proposals.json').read_text());self.assertEqual(j['summer_journal_comparison']['new_declaration_ids'],[]);self.assertEqual(j['summer_journal_comparison']['unchanged_declaration_tables'],59)
  q=json.loads((PACKETS/'unjoined-components.json').read_text());self.assertEqual(len(q['records']),248);self.assertFalse(q['new_canonical_quest_identity_created']);self.assertTrue(all(r['candidate_join_limit'].startswith('Directory membership only') for r in q['records']))
 def test_schema_rejects_semantic_native_or_new_identity_promotion(self):
  schema=json.loads((SOURCES/'inventory.schema.json').read_text());j=json.loads((PACKETS/'inventory.json').read_text());v=Draft202012Validator(schema);v.validate(j)
  for key in ['native_admission','semantic_quest_completion','new_canonical_quest_identity_created']:
   with self.subTest(key=key):self.assertTrue(list(v.iter_errors({**j,key:True})))
  self.assertTrue(list(v.iter_errors({**j,'unexpected':True})))
if __name__=='__main__':unittest.main()
