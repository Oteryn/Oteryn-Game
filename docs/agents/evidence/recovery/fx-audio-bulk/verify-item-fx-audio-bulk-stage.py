from pathlib import Path
import hashlib,json,xml.etree.ElementTree as E
O=Path('/workspace/audit-continuation');p=O/'item-fx-audio-bulk-raw-source-staging-20261002.json';j=json.loads(p.read_bytes());cache={}
sha=lambda b:hashlib.sha256(b).hexdigest()
def pin(p):
 b=p.read_bytes();return {'path':str(p),'sha256':sha(b),'bytes':len(b)}
for f in j['source_files']:
 b=Path(f['path']).read_bytes();assert len(b)==f['bytes'] and sha(b)==f['sha256'];cache[f['path']]=b
for r in j['xml_media_declarations']:
 raw=cache[r['source_file']['path']];span=r['literal_utf8_byte_span'];literal=raw[span['start']:span['end_exclusive']]
 assert literal.decode()==r['raw_xml'] and sha(literal)==r['raw_xml_sha256'];n=E.fromstring(literal);assert n.attrib==r['item_xml_attributes']
 ids=[int(n.attrib['id'])] if 'id' in n.attrib else list(range(int(n.attrib['fromid']),int(n.attrib['toid'])+1));assert ids==r['source_item_ids']
 assert all(a['attributes']['key'].lower() in ('effect','shoottype','meleeattackeffect') or 'sound' in a['attributes']['key'].lower() for a in r['media_fields'])
 for a in r['media_fields']:
  child=n
  for idx in a['child_index_path']:child=child[idx]
  assert child.tag=='attribute' and child.attrib==a['attributes']
for f in j['full_selected_context_sources']:
 assert f['raw_full_source'].encode()==cache[f['source_file']['path']]
 for r in f['all_sound_references']:assert f['raw_full_source'].splitlines()[r['line']-1]==r['raw_line']
xmlids={i for r in j['xml_media_declarations'] for i in r['source_item_ids']};scriptids={r['source_item_id'] for s in j['weapon_script_observations'] for r in s['literal_item_id_calls']};ids={r['source_item_id'] for r in j['per_numeric_source_item_rows']}
assert len(xmlids)==295 and len(scriptids)==12 and scriptids<=xmlids and ids==xmlids
spell=[s for s in j['weapon_script_observations'] if s['script_domain']=='SPELL_CONTEXT'];assert len(spell)==1 and not spell[0]['literal_item_id_calls'] and spell[0]['literal_non_item_id_calls'][0]['source_item_id']==107
for r in j['per_numeric_source_item_rows']:
 assert r['native_qualification']=='NOT_ATTEMPTED_OTS_HYPOTHESIS_ONLY' and r['asset_resolution']=='UNKNOWN_NO_ASSET_KEYS_INVENTED'
 if r['guard_results']['EXACT_FULL_TARGET_CURRENT_ITEM']:
  b=r['existing_binding_candidates'][0];assert b['target']==r['current_native_identity'] and b['external_id']==str(r['source_item_id']) and b['identity_namespace']=='ots/item_server_id'
part=j['overlap_lists']['identity_partition_ids'];assert sum(map(len,part.values()))==295 and set.union(*(set(v) for v in part.values()))==ids
assert len(part['CURRENT_BOUND_ITEM_STATIC_JOIN'])==149 and len(part['CURRENT_WORLD_OWNER'])==141 and len(part['IDENTITY_OR_MEMBERSHIP_HOLD'])==5
checks={'source_files64_fullbytes':'PASS','xml552_literal_byte_spans_and_fields_and_ids':'PASS','full_source_text_and_sound_line_coordinates':'PASS','295closed_ID_union12scripts_all_overlap':'PASS','spell107_never_mapped_as_Item_ID':'PASS','full_existing_bindings_namespace_and_targets':'PASS','partition149_141_5':'PASS','no_native_asset_runtime_qualification_claim':'PASS'}
report={'schema':'OTERYN_ITEM_FX_AUDIO_EXTERNAL_LITERAL_REPLAY_VERIFICATION/v1','status':'PASS_RAW_STAGING_ONLY','report':pin(p),'checks':checks,'unrun':['Native qualification','Asset import/admission','Runtime execution','Cargo']}
v=O/'item-fx-audio-bulk-raw-source-staging-verification.json';v.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
m=O/'item-fx-audio-bulk-raw-source-staging-checkpoint-manifest.json';manifest=json.loads(m.read_bytes());manifest['owned_files']=[r for r in manifest['owned_files'] if r['path'] not in (str(v),str(Path(__file__)))] + [pin(v),pin(Path(__file__))];manifest['checks']['independent_literal_replay']=checks;m.write_text(json.dumps(manifest,sort_keys=True,ensure_ascii=False,indent=2)+'\n')
for r in manifest['owned_files']:
 b=Path(r['path']).read_bytes();assert len(b)==r['bytes'] and sha(b)==r['sha256']
print(json.dumps({'manifest':pin(m),'verification':pin(v),'counts':{k:v for k,v in j['counts'].items() if k in ('unique_numeric_source_ids','xml_media_declarations_all_three_snapshots','static_identity_applicability_partition','items_with_explicit_cross_snapshot_media_disagreement','items_with_source_presence_differences')},'owned_files':manifest['owned_files']},indent=2))
