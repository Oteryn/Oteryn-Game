import argparse,pathlib,json,hashlib,sys,copy
ap=argparse.ArgumentParser();ap.add_argument('--corpus-manifest',type=pathlib.Path,required=True);ap.add_argument('--source-root',type=pathlib.Path,required=True);ap.add_argument('--authoring',type=pathlib.Path,required=True);ap.add_argument('--out',type=pathlib.Path,required=True);args=ap.parse_args();sys.path.insert(0,str(args.authoring));import ots_interactions as base;import donor_semantic_rewards as helper;from lua_writers import mask_code
read=lambda p:json.loads(p.read_text());sha=lambda b:hashlib.sha256(b).hexdigest();corpus=read(args.corpus_manifest);files={(x['source'],x['path']):x for x in corpus['files']};manifest=read(args.authoring/'samples/interactions/manifest.json');original=read(args.authoring/'samples/interactions/interactions.json');indices={g['identity']['key']:n for n,g in enumerate(original['interactions'])};records=[]
def source_file(row):
 p=pathlib.Path(row.get('cache_path',row.get('blob_path')))
 return p if p.is_absolute()else args.corpus_manifest.parent/p
for entry in manifest['entries']:
 key=entry.get('destination')
 if key not in indices:continue
 graph=original['interactions'][indices[key]]
 if not any(c.get('owner')=='Item'and c.get('request')=='hand_out'and 'value_source_line'in c for c in base.walk(graph['rules'])):continue
 chosen=key.split(':',1)[0]
 decision=base.CONFLICT_DECISIONS['interactions'].get(key)
 if decision and decision['decision']=='crystalserver':chosen='crystalserver'
 for src in entry['sources']:
  if src['source']!=chosen:continue
  row=files.get((src['source'],src['path']))
  if not row:continue
  blob=source_file(row).read_bytes();assert sha(blob)==row['sha256'];assert hashlib.sha1(b'blob '+str(len(blob)).encode()+b'\0'+blob).hexdigest()==row['git_blob_sha1'];assert src['blob_sha1']==row['git_blob_sha1']
  repo=args.source_root/row['source']/row['revision']
  if not (repo/row['path']).exists():repo=args.source_root/row['source']
  assert (repo/row['path']).read_bytes()==blob
  s=base.Script(row['source'],repo,row['path'],{},key.split(':interaction/')[1]);callback=src.get('callback_line')
  if callback is None:continue
  match=base.CALLBACK.match(s.lines[callback-1]);s.bind(callback,match.group(2));candidate,changes=helper.apply(s,graph)
  if not changes:continue
  for change in changes:
   text=blob.decode('utf-8','surrogateescape');lines=text.splitlines(keepends=True);start=sum(len(x.encode('utf-8','surrogateescape'))for x in lines[:change['table_line']-1]);table_text=''.join(lines[change['table_line']-1:]);visible=mask_code(table_text);opening=visible.index('{');depth=0;end=None
   for n,c in enumerate(visible[opening:],opening):
    if c=='{':depth+=1
    elif c=='}':
     depth-=1
     if depth==0:end=n+1;break
   table_raw=table_text[:end].encode('utf-8','surrogateescape');callstart=sum(len(x.encode('utf-8','surrogateescape'))for x in lines[:change['line']-1]);call_raw=lines[change['line']-1].encode('utf-8','surrogateescape');change['source_table_span']={'byte_start':start,'byte_end':start+len(table_raw),'sha256':sha(table_raw),'raw_source':table_raw.decode('utf-8','surrogateescape')};change['source_call_span']={'byte_start':callstart,'byte_end':callstart+len(call_raw),'sha256':sha(call_raw),'raw_source':call_raw.decode('utf-8','surrogateescape')};aliasstart=sum(len(x.encode('utf-8','surrogateescape'))for x in lines[:change['alias_line']-1]);alias_raw=lines[change['alias_line']-1].encode('utf-8','surrogateescape');change['source_alias_span']={'byte_start':aliasstart,'byte_end':aliasstart+len(alias_raw),'sha256':sha(alias_raw),'raw_source':alias_raw.decode('utf-8','surrogateescape')};change['baseline_json_pointer']='/interactions/'+str(indices[key])+change['baseline_child_pointer']
  # Validate graph against the accepted SOURCE interaction vocabulary, not a Native contract.
  import jsonschema
  jsonschema.Draft202012Validator(read(args.authoring/'interaction.schema.json')).validate({'interactions':[candidate]})
  records.append({'source':{k:row[k]for k in ('source','repository','revision','path','git_blob_sha1','sha256')},'interaction':graph['identity'],'baseline_graph_sha256':sha(json.dumps(graph,sort_keys=True,separators=(',',':')).encode()),'baseline_graph':graph,'candidate_graph':candidate,'changes':changes,'native_admission':False,'full_source_complete':False,'chosen_source':chosen})
packet={'schema':'OTERYN_DONOR_REWARD_RECORD_SEMANTIC_SUPPLEMENT/v1','scope':'CLOSED_LOCAL_UID_RECORD_ITEM_ID_COUNT_ONLY_OUTER_GUARDS_AND_UNKNOWN_FALLBACK_PRESERVED','source_manifest_sha256':sha(args.corpus_manifest.read_bytes()),'native_semantic_admission':False,'full_source_complete':False,'baseline_interactions_sha256':sha((args.authoring/'samples/interactions/interactions.json').read_bytes()),'records':records,'summary':{'source_records':len(records),'changed_call_sites':sum(len(r['changes'])for r in records),'known_guarded_reward_leaves':sum(len(c['rewards'])for r in records for c in r['changes']),'full_source_quests_completed':0,'native_admission':False}};args.out.write_text(json.dumps(packet,ensure_ascii=True,indent=2)+'\n');print(packet['summary'])
