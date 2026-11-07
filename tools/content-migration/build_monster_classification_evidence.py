import argparse,json,re,pathlib,collections,hashlib,subprocess,os
parser=argparse.ArgumentParser(description='Extract source-backed creature roles and independent contexts without changing runtime data.')
for flag in ['population','canary','crystal','out']:parser.add_argument('--'+flag,required=True,type=pathlib.Path)
args=parser.parse_args()
ROOT=args.population;OUT=args.out;C=args.canary;X=args.crystal
if OUT.exists():raise ValueError('output must be new')
if any(OUT.resolve()==base.resolve() or base.resolve() in OUT.resolve().parents for base in (C,X,ROOT/'bundles',ROOT/'encounters')):raise ValueError('output would modify a source input')
CP='47dfd51f45280a59a1d3e50ba7edd573d7234446';XP='00ce02a57ca5a12e48f32a3476e37471167e4c3f'
source_trees={}
source_bytes_verified=set()
for repo,cache,pin in [('opentibiabr/canary',C,CP),('zimbadev/crystalserver',X,XP)]:
 raw=subprocess.check_output(['git','-C',str(cache),'ls-tree','-r','--format=%(objectname) %(path)',pin],text=True,env={**os.environ,'GIT_NO_LAZY_FETCH':'1'})
 source_trees[repo]={line.split(' ',1)[1]:line.split(' ',1)[0]for line in raw.splitlines()}
def verify_source_bytes(repository,path):
 cache=C if repository=='opentibiabr/canary'else X
 expected=source_trees[repository].get(path)
 if expected is None:raise ValueError(f'Source path absent from pinned Git tree: {repository}:{path}')
 data=(cache/path).read_bytes()if(cache/path).is_file()else subprocess.check_output(['git','-C',str(cache),'cat-file','blob',expected],env={**os.environ,'GIT_NO_LAZY_FETCH':'1'})
 blob=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
 if blob!=expected:raise ValueError(f'Source bytes differ from pinned Git blob: {repository}:{path}')
 source_bytes_verified.add((repository,path,blob))
 return blob
invalid_source_paths=[]
idx=json.loads((ROOT/'population-index.json').read_text());byidx={r['monster']:r for r in idx['monsters']};ann={};names=collections.defaultdict(list)
def ev(kind,source,pointer=None,value=None,**kw):
 d={'kind':kind,'source':source,**kw}
 if pointer is not None:d['pointer']=pointer
 if value is not None:d['value']=value
 return d
def add_role(a,role,conf,e):
 for r in a['roles']:
  if r['role']==role and r['confidence']==conf:
   if e not in r['evidence']:r['evidence'].append(e)
   return
 a['roles'].append({'role':role,'confidence':conf,'evidence':[e]})
def add_context(a,ctx,conf,e):
 for r in a['contexts']:
  if r['context']==ctx and r['confidence']==conf:
   if e not in r['evidence']:r['evidence'].append(e)
   return
 a['contexts'].append({'context':ctx,'confidence':conf,'evidence':[e]})
for b in sorted((ROOT/'bundles').iterdir()):
 slug=b.name;m=json.loads((b/'monster.json').read_text());c=m['creature'];man=json.loads((b/'manifest.json').read_text());paths=[]
 for en in man.get('entries',[]):
  sp=en.get('source_file');si=en.get('source_index')
  if sp and '/monster/'in sp and sp.endswith('.lua') and 0<=si<len(man['sources']):
   s=man['sources'][si];fact={'path':sp,'repository':s.get('repository'),'revision':s.get('revision')}
   if sp not in source_trees.get(s.get('repository'),set()):
    invalid_source_paths.append({'slug':slug,**fact});continue
   fact['blob_sha1']=verify_source_bytes(s['repository'],sp)
   if fact not in paths:paths.append(fact)
 a={'source_path':paths[0]['path'] if paths else None,'source_files':paths,'roles':[],'contexts':[],'encounter_memberships':[],'variant_relations':[]}
 ann[slug]=a;names[c['display_name'].casefold()].append(slug)
 bundle=f'bundles/{slug}/monster.json'
 if c.get('bestiary'):add_role(a,'creature','confirmed',ev('bundle_field',bundle,'/creature/bestiary/class',c['bestiary']['class']))
 if c.get('bosstiary'):add_role(a,'boss','confirmed',ev('bundle_field',bundle,'/creature/bosstiary/category',c['bosstiary']['category']))
 if c['system_eligibility'].get('reward_boss'):add_role(a,'boss','confirmed',ev('bundle_field',bundle,'/creature/system_eligibility/reward_boss',True))
 if c['summoning'].get('is_familiar'):add_role(a,'familiar','confirmed',ev('bundle_field',bundle,'/creature/summoning/is_familiar',True))
 for s in paths:
  pts=pathlib.PurePosixPath(s['path']).parts
  evidence=ev('source_folder',s['path'],repository=s['repository'],revision=s['revision'],qualification='Folder organization is a context hint, not exclusive gameplay membership.')
  for folder,ctx in [('quests','quest'),('raids','raid'),('events','event'),('event_creatures','event'),('dawnport','dawnport'),('nostalgia','historical')]:
   if folder in pts:add_context(a,ctx,'inferred',evidence)
  if 'bosses'in pts and not any(r['role']=='boss' and r['confidence']=='confirmed'for r in a['roles']):add_role(a,'boss','inferred',evidence)
  cache=C if s['repository']=='opentibiabr/canary' else X if s['repository']=='zimbadev/crystalserver'else None
  if cache and (cache/s['path']).exists():
   txt=(cache/s['path']).read_text();
   if slug=='training_machine':
    for n,line in enumerate(txt.splitlines(),1):
     if 'Please feel free to hit me' in line:add_role(a,'trainer','confirmed',ev('source_lua_line',s['path'],value=line.strip(),line=n,repository=s['repository'],revision=s['revision']))
   if slug=='salamander_trainer':add_role(a,'trainer','inferred',ev('source_definition',s['path'],value='Registered Salamander Trainer; not a training dummy classification.',repository=s['repository'],revision=s['revision']))
# Exact-name public source scripted raid spawns and quest createMonster references.
for cache,repository,pin,base in [(C,'opentibiabr/canary',CP,'data-otservbr-global/scripts'),(X,'zimbadev/crystalserver',XP,'data-global/scripts')]:
 scripts=cache/base
 if not scripts.exists():continue
 for sub,ctx in [('raids','raid'),('quests','quest')]:
  folder=scripts/sub
  if not folder.exists():continue
  for f in sorted(folder.rglob('*.lua')):
   source_path=str(f.relative_to(cache))
   if source_path not in source_trees[repository]:continue
   source_blob=verify_source_bytes(repository,source_path)
   txt=f.read_text();lines=txt.splitlines()
   for n,line in enumerate(lines,1):
    patterns=[r'Game\.createMonster\(\s*["\']([^"\']+)["\']']
    if ctx=='raid' and 'addSpawnMonsters'in txt:patterns.append(r'\bname\s*=\s*["\']([^"\']+)["\']')
    for pat in patterns:
     for match in re.finditer(pat,line):
      for slug in names.get(match.group(1).casefold(),[]):
       literal_slug=re.sub(r'[^a-z0-9]+','_',match.group(1).lower()).strip('_')
       ambiguous=len(names[match.group(1).casefold()])>1 or literal_slug!=slug
       add_context(ann[slug],ctx,'inferred'if ambiguous else'confirmed',ev('name_ambiguous_spawn_reference'if ambiguous else'source_script_spawn',str(f.relative_to(cache)),value=line.strip(),line=n,repository=repository,revision=pin,blob_sha1=source_blob,spawn_name=match.group(1),matching_definitions=names[match.group(1).casefold()],qualification='Shared display name does not identify this concrete variant; source context is inferred only.'if ambiguous else'Unique prepared display name and literal normalized source name match the exact definition slug; does not imply exclusive context.'))
# Encounter source membership and executable transform relations.
for d in sorted((ROOT/'encounters').iterdir()):
 ep=d/'encounter.json';e=json.loads(ep.read_text());mp=d/'manifest.json';manifest=json.loads(mp.read_text());roles={}
 for proof_source in manifest.get('sources',[]):
  if proof_source.get('repository')in source_trees and proof_source.get('path'):
   proof_source['verified_blob_sha1']=verify_source_bytes(proof_source['repository'],proof_source['path'])
 for i,p in enumerate(e['participants']):
  roles[p['role']]=p['creatures']
  for cr in p['creatures']:
   slug=cr['key'].split('/')[-1]
   if slug not in ann:continue
   a=ann[slug];a['encounter_memberships'].append({'encounter':e['identity'],'role':p['role'],'evidence':[ev('encounter_participant',f'encounters/{d.name}/encounter.json',f'/participants/{i}',sources=manifest.get('sources',[]))]})
 for ri,r in enumerate(e['rules']):
  for ai,act in enumerate(r['actions']):
   if act['kind']!='transform':continue
   into=act['into'];targets=into.get('random_of',[into]);source_role=act['role']
   for target in targets:
    for original in roles.get(source_role,[]):
     ss=original['key'].split('/')[-1];ts=target['key'].split('/')[-1]
     proof=ev('encounter_transform',f'encounters/{d.name}/encounter.json',f'/rules/{ri}/actions/{ai}',sources=manifest.get('sources',[]))
     if ss in ann:ann[ss]['variant_relations'].append({'relation':'transforms_into','related_identity':target,'evidence':[proof]})
     if ts in ann:ann[ts]['variant_relations'].append({'relation':'transforms_from','related_identity':original,'evidence':[proof]})
# Curated non-living encounter objects, requiring existing source-backed encounter membership.
object_slugs={'dragon_egg','egg','wine_cask','makeshift_home','pillar_of_summoning','pillar_of_death','pillar_of_protection','pillar_of_healing','pillar_of_draining','containment_crystal','containment_machine','glooth_generator'}
for slug,a in ann.items():
 if (slug in object_slugs or slug.startswith('cosmic_energy_prism_')) and a['encounter_memberships']:
  for membership in a['encounter_memberships']:
   source=membership['evidence'][0]['source'];enc=json.loads((ROOT/source).read_text());role=membership['role']
   for ri,rule in enumerate(enc['rules']):
    if f'\"role\": \"{role}\"'in json.dumps(rule) and rule.get('actions'):
     add_role(a,'mechanic_actor','confirmed',ev('curated_source_backed_encounter_object',source,f'/rules/{ri}',value={'source_object_name':slug,'role':role,'rule_key':rule['key'],'trigger':rule['trigger'],'action_kinds':[action['kind']for action in rule['actions']]},qualification='Curated object role is corroborated by this exact encoded source-backed mechanic rule, not the name alone; creature runtime family remains unchanged.'))
     break
# Strong containment source conversion removes item and creates crystal; controls boss damage.
sp='data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_machine.lua'
containment_source_blob=verify_source_bytes('opentibiabr/canary',sp)
for slug in ['containment_crystal','containment_machine']:
 if slug in ann:add_role(ann[slug],'mechanic_actor','confirmed',ev('source_script_mechanic',sp,value='Containment machine death removes barrier items and replaces item 7805 with Containment Crystal; crystal callback controls The Armored Voidborn health.',lines=[23,25,40,45,51,64],blob_sha1=containment_source_blob,repository='opentibiabr/canary',revision=CP))
# Positive summon references are non-exclusive: ordinary creatures can also be summoned.
for owner in sorted((ROOT/'bundles').iterdir()):
 m=json.loads((owner/'monster.json').read_text())
 for i,row in enumerate(m['behavior'].get('summons',{}).get('entries',[])):
  target=row['creature']['key'].split('/')[-1]
  if target in ann:add_role(ann[target],'summon','confirmed',ev('bundle_summon_reference',f'bundles/{owner.name}/monster.json',f'/behavior/summons/entries/{i}/creature',value=row['creature'],owner_identity=m['creature']['identity'],qualification='Referenced as summon in prepared source-backed behavior; does not imply exclusive summon role or verified cast execution.'))
for a in ann.values():
 if not a['roles']:a['roles']=[{'role':'unknown','confidence':'unknown','evidence':[]}]
# Validation and summaries do not use no-Bestiary as classification evidence.
summary={'source_bytes_verified':len(source_bytes_verified),'source_files_verified_in_pinned_git_trees':sum(len(a['source_files'])for a in ann.values()),'invalid_source_paths':invalid_source_paths,'population':len(ann),'roles':dict(collections.Counter(r['role']+':'+r['confidence']for a in ann.values()for r in a['roles'])),'contexts':dict(collections.Counter(r['context']+':'+r['confidence']for a in ann.values()for r in a['contexts'])),'encounter_members':sum(bool(a['encounter_memberships'])for a in ann.values()),'transform_related':sum(bool(a['variant_relations'])for a in ann.values())}
noency=[]
for slug in ann:
 c=json.loads((ROOT/'bundles'/slug/'monster.json').read_text())['creature']
 if not c.get('bestiary')and not c.get('bosstiary')and 'mitigation_percent'not in c['stats']:noency.append(slug)
summary['unknown_mitigation_no_encyclopedia']=len(noency);summary['unknown_group_roles']=dict(collections.Counter(r['role']+':'+r['confidence']for slug in noency for r in ann[slug]['roles']))
OUT.parent.mkdir(parents=True,exist_ok=True)
OUT.write_text(json.dumps({'schema_version':1,'source':'Read-only pinned donor and current prepared bundles/83 source-backed Encounters','population_index_sha256':hashlib.sha256((ROOT/'population-index.json').read_bytes()).hexdigest(),'limitations':['No classification derives from absent Bestiary/Bosstiary.','Source folders are inferred context; context labels may coexist.','No category implies missing mitigation equals zero or not-applicable.','Confirmed means supported in current pinned authoring/source evidence, not Global live verification.','Unclassified entries retain unknown; positive classification is incomplete, not fabricated.'],'summary':summary,'source_bytes_receipt':[{'repository':repo,'path':path,'blob_sha1':blob}for repo,path,blob in sorted(source_bytes_verified)],'annotations':ann},indent=2)+'\n')
print(json.dumps(summary,indent=2));print(OUT)
