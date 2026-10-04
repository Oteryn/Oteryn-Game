import argparse,json,pathlib,hashlib,copy,sys,re
from jsonschema import Draft202012Validator
ap=argparse.ArgumentParser();ap.add_argument('--corpus-manifest',type=pathlib.Path,required=True);ap.add_argument('--authoring',type=pathlib.Path,required=True);ap.add_argument('--packet',type=pathlib.Path,required=True);ap.add_argument('--out',type=pathlib.Path,required=True);a=ap.parse_args();read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest();p=read(a.packet);sources={(r['source'],r['path']):r for r in read(a.corpus_manifest)['files']};baseline=read(a.authoring/'samples/interactions/interactions.json');assert p['native_semantic_admission']is False and p['full_source_complete']is False;assert p['source_manifest_sha256']==sha(a.corpus_manifest.read_bytes());assert p['baseline_interactions_sha256']==sha((a.authoring/'samples/interactions/interactions.json').read_bytes());checks=0
sys.path.insert(0,str(a.authoring));import ots_interactions as base;import lua_tables
Draft202012Validator(read(pathlib.Path(__file__).with_name('donor-reward-record-supplement.schema.json'))).validate(p)
bykey={g['identity']['key']:g for g in baseline['interactions']}
for r in p['records']:
 key=r['interaction']['key'];chosen=key.split(':',1)[0];decision=base.CONFLICT_DECISIONS['interactions'].get(key)
 if decision and decision['decision']=='crystalserver':chosen='crystalserver'
 assert r['chosen_source']==chosen==r['source']['source'];assert r['baseline_graph']==bykey[key]
 row=sources[(r['source']['source'],r['source']['path'])];assert all(row[k]==v for k,v in r['source'].items());path=pathlib.Path(row.get('cache_path',row.get('blob_path')));path=path if path.is_absolute()else a.corpus_manifest.parent/path;blob=path.read_bytes();assert sha(blob)==row['sha256'];assert hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()==row['git_blob_sha1'];assert r['native_admission']is False and r['full_source_complete']is False
 Draft202012Validator(read(a.authoring/'interaction.schema.json')).validate({'interactions':[r['candidate_graph']]})
 restored=copy.deepcopy(r['candidate_graph'])
 for c in r['changes']:
  for field in ('source_table_span','source_call_span','source_alias_span'):
   span=c[field];raw=blob[span['byte_start']:span['byte_end']];assert raw==span['raw_source'].encode('utf-8','surrogateescape');assert sha(raw)==span['sha256'];checks+=1
  ref=baseline
  for part in c['baseline_json_pointer'].split('/')[1:]:ref=ref[int(part)]if isinstance(ref,list)else ref[part]
  target=restored;parts=c['baseline_child_pointer'].split('/')[1:]
  for part in parts[:-1]:target=target[int(part)]if isinstance(target,list)else target[part]
  call=c['source_call_span']['raw_source'].strip();match=re.fullmatch(r'(\w+):addItem\((\w+)\.(\w+),\s*(.*?)\)',call);assert match
  actor,alias,idfield,countarg=match.groups();assert alias==c['record_alias']
  table=c['source_table_span']['raw_source'];values=lua_tables.as_python(base.LiteralParser(table[table.index('{'):]).table())
  countfield=re.fullmatch(re.escape(alias)+r'\.(\w+)',countarg)
  rewards={str(k):[v[idfield],v[countfield[1]]if countfield else int(countarg)]for k,v in values.items()};assert rewards==c['rewards']
  expected=[]
  for selector,(ident,count)in sorted(((int(k),v)for k,v in rewards.items())):
   item=lambda ident:{'family':'Item','key':chosen+':item/'+str(ident),'revision':r['baseline_graph']['identity']['revision']}
   condition={'object':{'role':'source','field':'unique_id','op':'==','value':selector}}if c['selector_kind']=='uid'else {'object':{'role':'source','field':'item_type','op':'==','item':item(selector)}}
   condition['negate']=False;expected.append({'when':condition,'then':[{'owner':'Item','request':'hand_out','item':item(ident),'count':count}]})
  branch=target[int(parts[-1])];assert branch=={'branch':expected,'otherwise':[ref]};target[int(parts[-1])]=copy.deepcopy(ref)
 assert restored==r['baseline_graph'];assert sha(json.dumps(restored,sort_keys=True,separators=(',',':')).encode())==r['baseline_graph_sha256']
receipt={'packet_sha256':sha(a.packet.read_bytes()),'valid':True,'source_variants':len(p['records']),'exact_raw_spans':checks,'guard_fallback_and_all_other_graph_fields_preserved':True,'interaction_schema':True,'native_semantic_admission':False,'full_source_complete':False};a.out.write_text(json.dumps(receipt,indent=2)+'\n');print(receipt)
