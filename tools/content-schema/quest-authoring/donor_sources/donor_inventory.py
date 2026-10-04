import argparse,json,pathlib,sys,collections,hashlib,re,tempfile,tarfile
parser=argparse.ArgumentParser(description='Offline donor-only exact declaration and component inventory; no Quest identity by title.')
parser.add_argument('--root',required=True,type=pathlib.Path)
parser.add_argument('--archive',type=pathlib.Path)
parser.add_argument('--corpus-manifest',type=pathlib.Path)
parser.add_argument('--output',required=True,type=pathlib.Path)
args=parser.parse_args()
R=args.root.resolve();T=R/'tools/content-schema/quest-authoring';O=args.output;O.mkdir(parents=True,exist_ok=True)
sys.path.insert(0,str(T));import lua_tables
from donor_sources.validate_archive import verify_archive
from donor_sources.validate_corpus import safe_path,PINS
corpus_input=json.loads((T/'samples/donor-source/corpus-input.json').read_bytes())
tmp=None
if args.corpus_manifest:
 base=args.corpus_manifest.parent.resolve();raw=args.corpus_manifest.read_bytes()
 if hashlib.sha256(raw).hexdigest()!=corpus_input['manifest_sha256']:raise ValueError('portable manifest digest differs')
 manifest=json.loads(raw);verified={'archive_sha256':corpus_input['sha256'],'manifest_sha256':corpus_input['manifest_sha256']}
else:
 archive=args.archive or T/'samples/donor-source/corpus.tar.gz'
 verified=verify_archive(archive,corpus_input['sha256'])
 tmp=tempfile.TemporaryDirectory(prefix='donor-inventory-replay-');base=pathlib.Path(tmp.name)
 with tarfile.open(archive,'r:gz') as stream:
  manifest=json.load(stream.extractfile('corpus-manifest.json'))
  for member in stream:
   if member.name.startswith('blobs/'):
    dest=base/member.name;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(stream.extractfile(member).read())
for f in manifest['files']:
 if (f['repository'],f['revision'])!=PINS[f['source']]:raise ValueError('source pin differs')
 candidate=(base/str(safe_path(f['cache_path']))).resolve()
 if not candidate.is_relative_to(base):raise ValueError('source cache escapes extraction root')
 f['cache_path']=str(candidate)
verified_reads={}
def source_text(f):
 path=f['cache_path']
 if path not in verified_reads:
  raw=pathlib.Path(path).read_bytes()
  blob=hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
  if len(raw)!=f['byte_count'] or hashlib.sha256(raw).hexdigest()!=f['sha256'] or blob!=f['git_blob_sha1']:
   raise ValueError('referenced raw source bytes differ: '+f['path'])
  verified_reads[path]=raw.decode('utf-8')
 return verified_reads[path]
bundle=json.load(open(T/'samples/source_migration/bundle.json'));logmanifest=json.load(open(T/'samples/questlog/manifest.json'));im=json.load(open(T/'samples/interactions/manifest.json'));qsource=json.load(open(T/'samples/questlog/quests.json'));sourcequests={q['identity']['key']:q for q in bundle['quests']};defs={}
for shard in json.load(open(R/'content/quests/definitions/index.json'))['shards']:
 for row in json.load(open(R/shard))['records']:
  q=row['definition'];key=q.get('source_refs',{}).get('quest',{}).get('key')
  if key:defs[key]=q['identity']['key']
bylog=collections.defaultdict(set)
for e in logmanifest['entries']:
 for s in e['sources']:bylog[(s['source'],s['path'],s['quest_line'])].add(e['quest'])
source_file_witnesses={(s['repository'],s['revision'],s['path']):s['blob_sha1'] for s in logmanifest['sources'] if s.get('kind')=='git'}
records=[];errors=[]
for f in manifest['files']:
 path=f['path'];name=f['source'];canarycatalog=name=='canary' and '/lib/core/quests/catalog/' in path and re.match(r'\d',pathlib.PurePosixPath(path).name)
 crystalcatalog=name in ('crystalserver','crystal-summer') and path in ('data-global/lib/core/quests.lua','data-crystal/lib/core/quests.lua')
 if not canarycatalog and not crystalcatalog:continue
 if name in ('canary','crystalserver'):
  proof=source_file_witnesses.get((f['repository'],f['revision'],path))
  if proof is not None and proof!=f['git_blob_sha1']:raise ValueError('quest log provenance blob differs')
 try:
  parsed=lua_tables.assignments(source_text(f),{'quest' if canarycatalog else 'Quests'})
  tables=[{'key':{'value':pathlib.PurePosixPath(path).stem.split('_',1)[0]},'value':parsed['quest']}] if canarycatalog else parsed['Quests']['fields']
  for field in tables:
   table=field['value'];value=lua_tables.as_python(table);missions=value.get('missions') or [];missions=missions if isinstance(missions,list) else [missions[k] for k in sorted(missions)];line=table['line'];sourcekeys=bylog.get((name,path,line),set());
   if len(sourcekeys)>1:raise ValueError('ambiguous exact declaration join')
   sourcekey=next(iter(sourcekeys),None)
   if sourcekey and sourcekey not in sourcequests:raise ValueError('manifest source quest missing')
   identity={'source':name,'revision':f['revision'],'path':path,'line':line,'table_key':field['key'],'blob_sha1':f['git_blob_sha1'],'sha256':f['sha256']}
   records.append({'source_declaration_id':f'{name}:{f["revision"]}:{path}:{line}','provenance':identity,'source_name':value['name'],'raw_structural_value':value,'missions':[{'name':m.get('name'),'storageId':m.get('storageId'),'startValue':m.get('startValue'),'endValue':m.get('endValue'),'missionId':m.get('missionId'),'source_fields':sorted(m)} for m in missions],'mission_count':len(missions),'source_table_sha256':hashlib.sha256(json.dumps(value,sort_keys=True,ensure_ascii=False,separators=(',',':')).encode()).hexdigest(),'exact_manifest_source_quest':sourcekey,'canonical_quest_key':defs.get(sourcekey),'verdict':'EXACT_EXISTING_MANIFEST_JOIN' if sourcekey else 'SUMMER_COMPARISON_DECLARATION' if name=='crystal-summer' else 'UNJOINED_DECLARATION','dynamic_mission_generation_possible':'for ' in source_text(f) if path=='data-crystal/lib/core/quests.lua' else False})
 except Exception as e:errors.append({'source':name,'path':path,'error':repr(e)})
# Existing explicit alias/fallback is checked as provenance, never new quest guess.
for rec in records:
 if rec['verdict']=='UNJOINED_DECLARATION' and rec['provenance']['path']=='data-crystal/lib/core/quests.lua' and rec['source_name']=='The Ultimate Challenges':
  rec['verdict']='KNOWN_CONDITIONAL_FALLBACK_NOT_NEW_QUEST';rec['conditional_limit']='Exact Source fallback in existing questlog-primary policy; literal mission list is empty; do not infer generated missions.'
represented=collections.defaultdict(set)
for e in im['entries']:
 for s in e['sources']:represented[(s['source'],s['path'])].add(e['destination'])
writers=collections.defaultdict(set)
for p in bundle['progress']:
 for t in p['transitions']:
  for occ in t.get('source_occurrences',[]):writers[(occ['source'],occ['path'])].add(p['key'])
# Group source components solely by explicit source path namespace, not titles.
components=[];groups=collections.defaultdict(list)
for f in manifest['files']:
 if '/scripts/quests/' not in f['path'] or not f['path'].endswith('.lua'):continue
 pack,tail=f['path'].split('/scripts/quests/',1);group=tail.split('/',1)[0] if '/' in tail else '<quest-root>'
 graphs=sorted(represented.get((f['source'],f['path']),[]));tracks=sorted(writers.get((f['source'],f['path']),[]))
 entry={'source_component_id':f'{f["source"]}:{f["revision"]}:{f["path"]}','provenance':{k:f[k] for k in ('source','repository','revision','path','git_blob_sha1','sha256','byte_count')},'pack':pack,'source_directory_group':group,'existing_interaction_graphs':graphs,'existing_progress_tracks':tracks,'verdict':'SUMMER_COMPARISON_COMPONENT' if f['source']=='crystal-summer' else 'INTERACTION_OR_PROGRESS_EVIDENCE_PRESENT' if graphs or tracks else 'RAW_SOURCE_ONLY_NOT_SEMANTICALLY_JOINED'}
 components.append(entry);groups[(f['source'],pack,group)].append(entry)
grouprecords=[]
for (source,pack,group),fs in sorted(groups.items()):
 grouprecords.append({'source':source,'pack':pack,'source_directory_group':group,'files':len(fs),'graph_joined_files':sum(bool(f['existing_interaction_graphs']) for f in fs),'progress_joined_files':sum(bool(f['existing_progress_tracks']) for f in fs),'raw_only_files':[f['provenance']['path'] for f in fs if not f['existing_interaction_graphs'] and not f['existing_progress_tracks']],'quest_identity_admission':'NOT_ASSESSED_COMPONENT_NAMESPACE_NOT_QUEST_TITLE'})
summary={'quest_log_declarations':dict(collections.Counter(r['provenance']['source'] for r in records)),'quest_log_missions':dict(sum((collections.Counter({r['provenance']['source']:r['mission_count']}) for r in records),collections.Counter())),'log_declaration_verdicts':dict(collections.Counter(r['verdict'] for r in records)),'quest_script_files':dict(collections.Counter(c['provenance']['source'] for c in components)),'quest_script_component_verdicts':dict(collections.Counter(c['verdict'] for c in components)),'source_directory_groups':dict(collections.Counter(g['source'] for g in grouprecords)),'parse_errors':len(errors),'canonical_donor_definitions':len(defs),'source_inventory_is_quest_count':False}
if errors:raise ValueError('source declaration parse errors: '+json.dumps(errors))
if len(defs)!=284:raise ValueError('canonical donor scope differs')
result={'schema':'OTERYN_DONOR_FIRST_INVENTORY_AUDIT/v1','native_admission':False,'semantic_quest_completion':False,'new_canonical_quest_identity_created':False,'scope':'Exact source declaration/provenance and component-path joins only; no new title normalization, missing quest count or Native claim','inputs':{str(p.relative_to(R)):hashlib.sha256(p.read_bytes()).hexdigest() for p in [T/'samples/questlog/manifest.json',T/'samples/interactions/manifest.json',T/'samples/source_migration/bundle.json']},'summary':summary,'quest_log_declarations':records,'quest_script_components':components,'source_directory_groups':grouprecords,'parse_errors':errors}
result['portable_corpus']={'archive_sha256':verified['archive_sha256'],'manifest_sha256':verified['manifest_sha256'],'portable_manifest_verified':True,'verified_manifest_and_referenced_bytes':True}
(O/'inventory.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
from lua_writers import mask_code
j=json.load(open(O/'inventory.json'));corpus=manifest;fm={(f['source'],f['path']):f for f in corpus['files']};bundle=json.load(open(T/'samples/source_migration/bundle.json'));idx=json.load(open(R/'content/quests/definitions/index.json'));owners=collections.defaultdict(set);trackowners=collections.defaultdict(set)
for shard in idx['shards']:
 for row in json.load(open(R/shard))['records']:
  q=row['definition'];key=q['identity']['key']
  for graph in q.get('source_data',{}).get('interactions',[]):owners[graph['identity']['key']].add(key)
  for p in q.get('source_data',{}).get('progress',[]):trackowners[p['key']].add(key)
packgroups=collections.defaultdict(set)
for c in j['quest_script_components']:
 ks=set()
 for g in c['existing_interaction_graphs']:ks|=owners[g]
 for t in c['existing_progress_tracks']:ks|=trackowners[t]
 c['exact_existing_canonical_owners']=sorted(ks);packgroups[(c['source_component_id'].split(':')[0],c['pack'],c['source_directory_group'])]|=ks
unjoined=[];mechanisms=collections.defaultdict(list)
for c in j['quest_script_components']:
 if c['verdict']!='RAW_SOURCE_ONLY_NOT_SEMANTICALLY_JOINED':continue
 f=fm[(c['provenance']['source'],c['provenance']['path'])];raw=source_text(f);code=mask_code(raw);calls=[]
 for n,line in enumerate(code.splitlines(),1):
  for m in re.finditer(r'(?<![\w.:])([A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*)\s*\(',line):
   if m[1] not in ('if','for','while'):calls.append({'line':n,'callee_lexical':m[1]})
 markers=[]
 for marker,pattern in {'boss_lever_shared_constructor':r'\bBossLever\s*\(', 'move_remove_item_event':r'\bonRemoveItem\s*\(', 'think_event':r'\bonThink\s*\(', 'global_time_event':r'\bonTime\s*\(', 'global_startup_event':r'\bonStartup\s*\(', 'shared_registration_only':r'\bregister\s*\('}.items():
  if re.search(pattern,code):markers.append(marker);mechanisms[marker].append(c['source_component_id'])
 callback_declarations=[]; registrations=[]; constructors=[]
 for n,line in enumerate(code.splitlines(),1):
  rawline=raw.splitlines()[n-1]
  for m in re.finditer(r'\bfunction\s+([A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*)\s*\(',line):
   callback_declarations.append({'line':n,'source_function_name':m[1],'line_sha256':hashlib.sha256(rawline.encode()).hexdigest()})
  for m in re.finditer(r'([A-Za-z_]\w*)\s*:\s*(register|aid|uid|id|position|type)\s*\(',line):
   registrations.append({'line':n,'receiver':m[1],'method':m[2],'source_line':rawline,'line_sha256':hashlib.sha256(rawline.encode()).hexdigest(),'runtime_activation':'NOT_ASSESSED'})
  for m in re.finditer(r'(?<![\w.:])(Action|MoveEvent|CreatureEvent|GlobalEvent|BossLever)\s*\(',line):
   constructors.append({'line':n,'source_constructor_name':m[1],'source_line':rawline,'line_sha256':hashlib.sha256(rawline.encode()).hexdigest(),'builtin_identity':'NOT_ASSESSED_LEXICAL_REFERENCE_ONLY'})
 c['source_function_declarations']=callback_declarations;c['source_registrations']=registrations;c['source_constructors']=constructors
 if 'boss_lever_shared_constructor' in markers:
  helper=fm.get((f['source'],'data/libs/functions/boss_lever.lua'))
  c['shared_controller_source_candidate']={k:helper[k] for k in ('source','repository','revision','path','git_blob_sha1','sha256')} if helper else None
 c['why_unjoined']='No exact source-path entry in current interaction manifest or progress write occurrence index. Source constructors/callbacks below are lexical evidence, not proof that the file is an independent Quest.'
 c['component_coverage_limit']='Whole-file presence in another index does not prove every callback or helper behavior has been semantically decoded.'
 c['lexical_calls']=calls;c['source_only_mechanisms']=markers;c['same_source_directory_owner_candidates']=sorted(packgroups[(f['source'],c['pack'],c['source_directory_group'])]);c['candidate_join_limit']='Directory membership only; not authoritative Quest identity or proof of executable semantic coverage.';unjoined.append(c)
rs=j['quest_log_declarations'];baseline={(r['provenance']['path'],str(r['provenance']['table_key'])):r for r in rs if r['provenance']['source']=='crystalserver'};summer={(r['provenance']['path'],str(r['provenance']['table_key'])):r for r in rs if r['provenance']['source']=='crystal-summer'}
comparison={'new_declaration_ids':[summer[k]['source_declaration_id'] for k in summer.keys()-baseline.keys()],'removed_declaration_ids':[baseline[k]['source_declaration_id'] for k in baseline.keys()-summer.keys()],'changed_table_indices':[{'baseline':baseline[k]['source_declaration_id'],'summer':summer[k]['source_declaration_id']} for k in baseline.keys()&summer.keys() if baseline[k]['source_table_sha256']!=summer[k]['source_table_sha256']],'unchanged_declaration_tables':sum(baseline[k]['source_table_sha256']==summer[k]['source_table_sha256'] for k in baseline.keys()&summer.keys()),'join_basis':'Exact donor Quests table index + pack/file, not title; comparison only'}
configuration_evidence=[]
for source in ('canary','crystalserver','crystal-summer'):
 config=fm[(source,'config.lua.dist')];text=source_text(config);line=next((n,l) for n,l in enumerate(text.splitlines(),1) if l.startswith('dataPackDirectory'))
 configuration_evidence.append({'source':source,'provenance':{k:config[k] for k in ('repository','revision','path','git_blob_sha1','sha256')},'line':line[0],'source_line':line[1],'scope':'SOURCE_DEFAULT_CONFIGURATION_ONLY_NOT_PRODUCTION_DEPLOYMENT'})
loader=fm[('canary','data/lib/core/quests/loader.lua')]
configuration_evidence.append({'source':'canary','provenance':{k:loader[k] for k in ('repository','revision','path','git_blob_sha1','sha256')},'source_scope':'load(dataDirectory) constructs dataDirectory.lib.core.quests.catalog namespace; no implicit merge of alternative catalog'})
for rec in rs:
 if rec['verdict']=='UNJOINED_DECLARATION' and rec['provenance']['source']=='canary' and rec['provenance']['path']=='data-canary/lib/core/quests/catalog/001_example.lua':
  rec['verdict']='ALTERNATIVE_PACK_DEMO_NOT_DEFAULT_GLOBAL_QUEST';rec['activation_evidence']=configuration_evidence
primary_pack={'canary':'data-otservbr-global','crystalserver':'data-global'}
missing_primary=[r['source_declaration_id'] for r in rs if r['provenance']['source'] in primary_pack and r['provenance']['path'].split('/',1)[0]==primary_pack[r['provenance']['source']] and not r['canonical_quest_key']]
proposals={'schema':'OTERYN_DONOR_COMPONENT_FOLLOWUP/v1','native_admission':False,'semantic_quest_completion':False,'new_canonical_quest_identity_created':False,'formal_journal_declarations_missing_primary':len(missing_primary),'missing_primary_declaration_ids':missing_primary,'default_pack_configuration_evidence':configuration_evidence,'unjoined_alternative_declarations':[r for r in rs if r['verdict']=='ALTERNATIVE_PACK_DEMO_NOT_DEFAULT_GLOBAL_QUEST'],'known_alternative_fallbacks':[r for r in rs if r['verdict']=='KNOWN_CONDITIONAL_FALLBACK_NOT_NEW_QUEST'],'raw_only_baseline_script_files':len(unjoined),'raw_only_by_source':dict(collections.Counter(c['provenance']['source'] for c in unjoined)),'raw_only_by_pack':dict(collections.Counter(c['pack'] for c in unjoined)),'mechanism_candidates':{k:{'file_count':len(v),'source_component_ids':v} for k,v in mechanisms.items()},'summer_journal_comparison':comparison,'followup_policy':'No new quest inferred from folder or quest-like file; join shared controller/function declaration and source registration first. Example alternative pack is not global missingQuest.'}
j['summer_journal_comparison']=comparison;j['summary']['log_declaration_verdicts']=dict(collections.Counter(r['verdict'] for r in rs));j['summary']['formal_primary_declarations_missing']=len(missing_primary);j['summary']['baseline_raw_only_script_files']=len(unjoined);(O/'inventory.json').write_text(json.dumps(j,ensure_ascii=False,indent=2)+'\n');(O/'unjoined-components.json').write_text(json.dumps({'schema':'OTERYN_RAW_ONLY_QUEST_COMPONENTS/v1','native_admission':False,'semantic_quest_completion':False,'new_canonical_quest_identity_created':False,'records':unjoined},ensure_ascii=False,indent=2)+'\n');(O/'correction-proposals.json').write_text(json.dumps(proposals,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'schema':'OTERYN_DONOR_INVENTORY_REPLAY/v1','valid':True,'primary_quest_declarations':109,'primary_declarations_missing':len(missing_primary),'raw_only_source_components':len(unjoined),'native_admission':False},sort_keys=True))

if tmp:tmp.cleanup()
