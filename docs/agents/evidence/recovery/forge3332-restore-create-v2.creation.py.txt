from pathlib import Path
import json,hashlib,subprocess,copy
R=Path('/workspace/pr1437-resistances');C=Path('/workspace/audit-continuation/document208-forge1-source-checkpoint-20261002');manifest=json.loads((C/'manifest.json').read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
for p,h in manifest['files'].items():assert sha((C/p).read_bytes())==h
files=[p for p in manifest['files'] if 'forge3332' in p];assert len(files)==5
for p in files:assert not (R/p).exists(),p
for p in files:(R/p).write_bytes((C/p).read_bytes())
oldpath='docs/agents/evidence/OTV2-20261001-item-forge3332-source-qualification-v1.json';old=json.loads((R/oldpath).read_text());proof=copy.deepcopy(old);head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=R,text=True).strip();assert head=='2ec1f34f05fd89abbb041f313b0852c331ae2b53'
proof.update(schema='FORGE_ONE_EXISTING_OWNER_CURRENT_CUT_PROOF/v2',mode='SOURCE_ONLY_AUTHORING_CURRENT_PARENT_NOT_NATIVE_APPLIED',native_baseline_head=head,historical_qualification={'path':oldpath,'sha256':sha((R/oldpath).read_bytes()),'native_baseline_head':old['native_baseline_head'],'immutable':True})
proof['historical_input_digests']=proof.pop('input_digests');proof['input_digests']={p:h for p,h in proof['historical_input_digests'].items() if p.startswith(('imports/','docs/agents/evidence/')) or p=='content/items/aliases.json'}
for p,h in proof['input_digests'].items():assert sha((R/p).read_bytes())==h,p
refpath='content/world/definitions/reference.json';refs=json.loads((R/refpath).read_text())['records'];ordinal,record=next((i,e) for i,e in enumerate(refs) if e.get('identity')==proof['target']);projection={'reference_path':refpath,'reference_sha256':sha((R/refpath).read_bytes()),'row_ordinal':ordinal,'retained_current_record':record,'row_canonical_sha256':sha(json.dumps(record,sort_keys=True,separators=(',',':')).encode())}
for path in json.loads((R/'content/items/index.json').read_text())['shards']:
 for i,row in enumerate(json.loads((R/path).read_text())['records']):
  if row['definition']['identity']==proof['target']:projection.update(item_shard_path=path,item_shard_sha256=sha((R/path).read_bytes()),item_shard_row_ordinal=i,item_shard_record=row)
proof['historical_current_native_identity']=proof['current_native_identity'];proof['current_native_identity']=projection
owners=json.loads((R/'content/world/definitions/declarations.json').read_text())['item_authoring'];existing=[o for o in owners if o['item']==proof['target']];assert not existing
proof['historical_existing_owner']=copy.deepcopy(proof['existing_owner']);proof['existing_owner'].update(canonical_owner_file_sha256=sha((R/'content/world/definitions/declarations.json').read_bytes()),current_authoring_count=len(owners),existing_forge_profiles=sum('forge' in o for o in owners));assert proof['existing_owner']['existing_forge_profiles']==146
proof['implementation_proposal'].update(expected_existing_forge_count_after=147,expected_item_authoring_count_after=len(owners)+1)
proof['evidence_classification']={'retained_structured_pair_bytes':'PROVEN_EXACT_RETAINED_ARTIFACT','br_article_body':'NOT_RETAINED; full_article_sha_is_declared_capture_coordinate','identity_bridge':'DERIVED_BY_EXISTING_EXACT_SOURCE_BINDING_AND_RETAINED_ALIAS; no_numeric_ID_from_name_or_suffix','current_parent':'ACTUAL2ec_RECORD_SHARD_AND_OWNER_PROJECTION; full-document hashes are parent observations, not dynamic leaf pins','sept27_selector':'DERIVED_CLOSED_SINGLETON_SUPPLEMENT; July28 source packet and rejection remain immutable'}
proof['hold']='SOURCE_AUTHORING_AUTHORIZED_BY_ROOT; RUST_AND_FINAL_GENERATION_AWAIT_USE_OBSERVATION_PREDECESSOR_CARRY'
newpath='docs/agents/evidence/OTV2-20261002-item-forge3332-source-qualification-v2.json';data=(json.dumps(proof,ensure_ascii=False,indent=2)+'\n').encode();(R/newpath).write_bytes(data);print('RESTORED ONLY5FORGE prototypefiles; original v1 retained byte-identical; newv2',sha(data),'actualparent',head,'existing146Forge')
