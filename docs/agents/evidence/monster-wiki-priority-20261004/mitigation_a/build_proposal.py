import json,hashlib,pathlib,collections
out=pathlib.Path('/tmp/monster-wiki-priority-20261004/mitigation_a')
inp=out.parent/'mitigation_a.json'
root=pathlib.Path('/workspace/monster-main-reconciliation-20261004')
cache=pathlib.Path('/tmp/monster-final-fill-20261004/population/bundles')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
x=json.loads(inp.read_text());native=root/'content/creatures/definitions/spell-native-profiles.json'
assert sha(native)==x['native_sha256']
idx={tuple(r['profile']['target'][k] for k in ('family','key','revision')):r for r in json.loads(native.read_text())['records']}
counts=collections.Counter();rows=[];conflicts=[]
for r in x['rows']:
 t=r['target'];key=tuple(t[k]for k in ('family','key','revision'));slug=t['key'].split('.',1)[1]
 current=idx[key]['profile']['data']['profile'];assert ('mitigation'in current)==r['old_present'];assert current.get('mitigation')==r['old']
 mp=cache/slug/'manifest.json';bp=cache/slug/'monster.json';m=json.loads(mp.read_text());b=json.loads(bp.read_text())
 assert b['creature']['stats'].get('mitigation_percent')==r['canonical']
 active=[e for e in m['entries'] if e.get('status')=='mapped' and e.get('destination')=='/monster/creature/stats/mitigation_percent']
 assert len(active)==1,(slug,len(active))
 e=active[0];s=m['sources'][e['source_index']]
 if s.get('kind')=='mediawiki':
  assert s.get('revision_id') and s.get('content_sha256');classification='WIKI_CONFIRMED';action='APPLY_ACTIVE_WIKI_VALUE'
 elif s.get('kind')=='oteryn_balance_estimate' and s.get('qualification')=='OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE':
  assert s.get('global_parity') is False;classification='ACCEPTED_PROJECT_ESTIMATE';action='RESTORE_PRESERVE_ACCEPTED_PROJECT_ESTIMATE_NO_ACTIVE_WIKI_REPLACEMENT'
 else:
  classification='SOURCE_ONLY/UNKNOWN';action='RETAIN_CURRENT_PENDING_QUALIFICATION';conflicts.append(slug)
 counts[classification]+=1
 row=dict(r);row.update({'new_present':r['canonical_present'] if classification!='SOURCE_ONLY/UNKNOWN' else r['old_present'],'new':r['canonical'] if classification!='SOURCE_ONLY/UNKNOWN' else r['old'],'provenance_classification':classification,'action':action,'global_parity_claimed':False,'evidence':{'access_method':'LOCAL_CACHED_SOURCE_MANIFEST_AND_PREPARED_BUNDLE','manifest_path':str(mp),'manifest_sha256':sha(mp),'bundle_path':str(bp),'bundle_sha256':sha(bp),'active_mapping':e,'source':s},'historical_superseded_mitigation_entries':[e for e in m['entries']if e.get('status')!='mapped' and e.get('destination')=='/monster/creature/stats/mitigation_percent']})
 rows.append(row)
assert len(rows)==304 and len({tuple(r['target'][k]for k in ('family','key','revision'))for r in rows})==304
result={'schema':'OTERYN_WIKI_PRIORITY_FIELD_PROPOSAL/v1','batch':'mitigation_a','native_path':str(native),'native_sha256':x['native_sha256'],'input_path':str(inp),'input_sha256':sha(inp),'counts':dict(counts),'apply_count':sum(r['action']!='RETAIN_CURRENT_PENDING_QUALIFICATION'for r in rows),'genuine_conflicts':conflicts,'policy':'Active field manifest wins over historical estimate CSV. Wiki-confirmed fields use exact accepted wiki rational values; active owner-accepted non-Global estimates remain accepted when no active wiki replacement is recorded. Cached field lineage is proof for this reconciliation, not a new live Global verification.','rows':rows,'validation':{'unique_targets':304,'current_old_presence_and_value': 'PASS','prepared_bundle_matches_canonical_exact_rational':'PASS','single_active_mitigation_mapping':'PASS','native_whole_file_sha256':'PASS','shared_product_mutated':False}}
(out/'proposal.json').write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
(out/'validation.json').write_text(json.dumps({'proposal_sha256':sha(out/'proposal.json'),'counts':dict(counts),'rows':304,'conflicts':conflicts,'validation':result['validation']},indent=2)+'\n')
print(json.dumps({'counts':dict(counts),'apply_count':result['apply_count'],'proposal_sha256':sha(out/'proposal.json'),'conflicts':conflicts}))
