import gzip,json,unittest
from pathlib import Path
from jsonschema import Draft202012Validator
HERE=Path(__file__).parent
class AllControls(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  fixture=HERE/'all-source-dialogue-links.json.gz'
  if not fixture.exists():fixture=HERE.parents[2]/'samples/donor-source/refinements/dialogue.json.gz'
  cls.packet=json.loads(gzip.decompress(fixture.read_bytes()));cls.schema=Draft202012Validator(json.loads((HERE/'quest_dialogue_links_all.schema.json').read_text()))
 def test_schema(self):self.assertEqual([list(e.path) for e in self.schema.iter_errors(self.packet)],[])
 def test_exact_and_held_counts_not_complete_quests(self):
  s=self.packet['summary'];self.assertEqual(s['npc_progress_occurrences'],4379);self.assertEqual(s['exact_ast_write_joins']+s['unresolved_ast_write_joins'],4379);self.assertEqual(len(s['quest_keys']),80);self.assertFalse(self.packet['native_admission']);self.assertFalse(self.packet['quests_complete'])
 def test_context_ref_bounds_and_source_identity(self):
  for l in self.packet['quest_dialogue_links']:
   cid=l['callback_context_ref']
   if cid is None:continue
   ctx=self.packet['callback_contexts'][cid];r=self.packet['source_records'][ctx['source_record_index']];o=l['occurrence'];self.assertEqual((r['source'],r['revision'],r['path']),(o['source'],o['revision'],o['path']))
   self.assertTrue(l['write_event_ref']['ast_path'].startswith(ctx['function_scope']['witness']['ast_path']+'.fields.body'))
 def test_context_events_are_exact_function_siblings(self):
  for c in self.packet['callback_contexts']:
   r=self.packet['source_records'][c['source_record_index']];prefix=c['function_scope']['witness']['ast_path']+'.fields.body'
   for i in c['dialogue_event_indices']+c['effect_event_indices']:self.assertTrue(r['events'][i]['witness']['ast_path'].startswith(prefix))
 def test_no_full_guard_or_argument_ast_duplication(self):
  for r in self.packet['source_records']:
   for e in r['events']:
    self.assertNotIn('arguments',e);self.assertNotIn('structure',e)
    for g in e['guards']:self.assertIn('expression_ref',g['test']);self.assertNotIn('structure',g['test'])
 def test_every_joined_write_ref_is_in_retained_events(self):
  events={(r['source'],r['revision'],r['path'],e['witness']['ast_path']):e for r in self.packet['source_records'] for e in r['events']}
  for l in self.packet['quest_dialogue_links']:
   if l['write_event_ref']:
    w=l['write_event_ref'];e=events[w['source'],w['revision'],w['path'],w['ast_path']];self.assertEqual(e['resolved_source_target'],l['occurrence']['target']);self.assertEqual(e['witness'],w)
 def test_holds_keep_real_source_target_evidence(self):
  held=[l for l in self.packet['quest_dialogue_links'] if l['join_status']!='EXACT_AST_STORAGE_WRITE'];self.assertEqual(len(held),0)
  for l in held:self.assertIn('hold_reason',l);self.assertIn('source_line_storage_write_candidates',l)
 def test_semantic_hashes_exclude_recipe_and_full_shards(self):
  for i in self.packet['quest_inputs']:self.assertEqual(set(i),{'quest','source_data_sha256'})
  self.assertIn('helper_semantic_links_sha256',self.packet['inputs'])
 def test_source_hold_candidates_do_not_fake_success(self):
  for l in self.packet['quest_dialogue_links']:
   if l['join_status']!='EXACT_AST_STORAGE_WRITE':self.assertIsNone(l['write_event_ref']);self.assertIsNone(l['callback_context_ref']);self.assertFalse(l['quest_complete'])
 def test_legacy_source_helper_argument_position_preserved(self):
  records={(r['source'],r['revision'],r['path']):r for r in self.packet['source_records']}
  count=0
  for l in self.packet['quest_dialogue_links']:
   w=l['write_event_ref']
   if not w:continue
   e=next(e for e in records[w['source'],w['revision'],w['path']]['events'] if e['witness']==w)
   if e.get('source_call_kind')=='LEGACY_GLOBAL_HELPER_CALL_SOURCE_ONLY':
    count+=1;self.assertEqual(e['storage_target_argument_index'],1);self.assertEqual(len(e['argument_ast_types']),3);self.assertEqual(e['resolved_source_target'],l['occurrence']['target'])
  self.assertEqual(count,33)
 def test_legacy_helper_execution_never_promoted(self):
  calls=[e for r in self.packet['source_records'] for e in r['events'] if e.get('source_call_kind')=='LEGACY_GLOBAL_HELPER_CALL_SOURCE_ONLY'];self.assertTrue(calls)
  self.assertTrue(all(e['helper_execution']=='NOT_PROVEN' for e in calls))
 def test_remaining_holds_are_source_target_proofs_not_missing_files(self):
  keys={(r['source'],r['revision'],r['path']) for r in self.packet['source_records']}
  for l in self.packet['quest_dialogue_links']:
   if l['join_status']!='EXACT_AST_STORAGE_WRITE':
    o=l['occurrence'];self.assertIn((o['source'],o['revision'],o['path']),keys);self.assertTrue(l['source_line_storage_write_candidates'])
 def test_89_function_alias_joins_have_literal_values_and_declaration_before_use(self):
  scoped=[l for l in self.packet['quest_dialogue_links'] if any(p['scope']=='SOLE_DIRECT_FUNCTION_BODY_LOCAL_NO_COMPETING_BINDING_OR_SHADOW' for p in l.get('storage_alias_witnesses',[]))];self.assertEqual(len(scoped),89)
  for l in scoped:
   self.assertEqual(l['literal_to_value_checked'],l['occurrence']['write']['to'])
   ps=[p for p in l['storage_alias_witnesses'] if p['scope']=='SOLE_DIRECT_FUNCTION_BODY_LOCAL_NO_COMPETING_BINDING_OR_SHADOW']
   for p in ps:
    self.assertLessEqual(p['witness']['span']['end_char_exclusive'],l['write_event_ref']['span']['start_char']);self.assertTrue(l['write_event_ref']['ast_path'].startswith(p['function_witness']['ast_path']+'.fields.body'))
 def test_before_after_counts(self):
  s=self.packet['summary'];self.assertEqual(s['exact_ast_write_joins'],4379);self.assertEqual(s['unresolved_ast_write_joins'],0);self.assertEqual(s['function_scoped_alias_joins'],89)
if __name__=='__main__':unittest.main()
