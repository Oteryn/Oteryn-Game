"""Import every pinned Canary/Crystal monster spell slot through existing typed converter.
Reference inputs stay local. Converted JSON and exact provenance are distributable candidates.
No unresolved callback is silently discarded or activated as production gameplay.
"""
import argparse,hashlib,json,sys,subprocess,re,time
from collections import Counter,defaultdict
from pathlib import Path
REPOSITORY=Path(__file__).resolve().parents[5]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--out',type=Path,required=True)
parser.add_argument('--source-inputs',type=Path,required=True)
args=parser.parse_args()
ROOT=args.out.resolve()
if ROOT.exists():raise ValueError('output directory must not exist')
SOURCE_INPUTS=args.source_inputs.resolve(strict=True)
TOOLS=REPOSITORY/'tools/content-schema/monster-authoring'
sys.path.insert(0,str(TOOLS))
import canary_batch as cb,spell_scripts as ss,spell_census as sc,validate_monster as vm
import partial_monster_loader as partial
cb.load_monster=partial.load
pipeline_paths=[Path(__file__).resolve(),Path(partial.__file__).resolve()]
pipeline_paths += [Path(module.__file__).resolve() for module in tuple(sys.modules.values())
                  if getattr(module,'__file__',None) and Path(module.__file__).resolve().parent==TOOLS]
pipeline_paths += [TOOLS/name for name in vm.SCHEMA_NAMES]
PIPELINE_SHA256={p.relative_to(REPOSITORY).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(set(pipeline_paths))}
OPEN={'unsupported_source_field','unresolved_semantics','unresolved_dependency','partial_text'}
SOURCES=[('canary','opentibiabr/canary',['data-otservbr-global/monster','data-canary/monster'],['data/scripts','data-otservbr-global/scripts','data-canary/scripts']),('crystal','zimbadev/crystalserver',['data-global/monster','data-crystal/monster'],['data/scripts','data-global/scripts','data-crystal/scripts'])]
def sha(b):return hashlib.sha256(b).hexdigest()
def dump(path,v):
 path.parent.mkdir(parents=True,exist_ok=True);path.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def git(repo,*args):return subprocess.check_output(['git','-C',str(repo),*args])
def sequence(v):
 if isinstance(v,list):return list(enumerate(v,1))
 if isinstance(v,dict):return sequence(v['_list']) if '_list' in v else [(k,v[k]) for k in sorted(k for k in v if isinstance(k,int))]
 return []
def evaluate_safe(scripts,name):
 try:return scripts.evaluate(name)
 except Exception as exc:return {'name':name,'tier':'P4','error':f'{type(exc).__name__}: {exc}','native_conversion':'unresolved_callback'}
def evaluate_registration_safe(scripts,name,kind,path):
 try:return scripts.evaluate_registration(name,kind,path)
 except Exception as exc:return {'name':name,'kind':kind,'tier':'P4','error':f'{type(exc).__name__}: {exc}','native_conversion':'unresolved_callback'}
def provenance(source,repo,rev,path,blob,data=None):
 b=path.read_bytes() if data is None else data;relative=path.relative_to(source).as_posix()
 if hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()!=blob:raise ValueError('source blob mismatch '+relative)
 return {'repository':repo,'revision':rev,'path':relative,'git_blob':blob,'sha256':sha(b),'bytes':len(b),'url':f'https://github.com/{repo}/blob/{rev}/{relative}','read_method':'existing local snapshot verified against immutable Git blob'}
all_reports=[];all_profiles=[];all_slots=[];all_definitions=[];t0=time.monotonic()
# Source-first import: external captures are deliberately not loaded or applied.
wiki_index={}
dump(ROOT/'reference-inputs.json',[])
for key,repository,monster_dirs,script_dirs in SOURCES:
 source=SOURCE_INPUTS/key;checkout=Path('/workspace/spell-sources')/key
 rev={'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystal':'00ce02a57ca5a12e48f32a3476e37471167e4c3f'}[key]
 tree={}
 for line in git(checkout,'ls-tree','-r',rev).decode().splitlines():
  meta,path=line.split('\t');tree[path]=meta.split()[2]
 # Source-scoped candidate revisions; no source identity is admitted as native Game authority.
 cb.REVISION=rev;cb.REV=f'{key}-{rev[:8]}'
 ss.SCRIPT_DIRS=tuple(script_dirs)
 objs=cb.load_appearance_objects(source/'data/items/appearances.dat');items=cb.load_items_xml(source/'data/items/items.xml');names,index=cb.name_index(objs,items)
 conv=cb.Converter(source,objs,items,names,index);conv.source={'repository':repository,'revision':rev};conv.spell_scripts=ss.SpellScripts(source,source_git=checkout,source_revision=rev)
 registrations=ss.registrations(source,script_dirs)
 script_by_name={}; registration_by_path={}
 for name,entries in sorted(registrations.items()):
  choices=sorted(entries,key=lambda e:(e[0]!='rune',str(e[1])))
  rows=[]
  for kind,path in choices:
   pr=provenance(source,repository,rev,path,tree[path.relative_to(source).as_posix()]);text=path.read_text();objects,error=sc.run_script(path)
   tier,reasons=sc.classify(text,objects,error,path.relative_to(source).as_posix().startswith(sc.SHARED_DIRS))
   conversion=evaluate_registration_safe(conv.spell_scripts,name,kind,path)
   for dependency in conversion.get('reference_dependencies',[]):
    relative_dependency=dependency['path']
    data=git(checkout,'show',rev+':'+relative_dependency)
    if dependency['revision']!=rev or dependency['sha256']!=sha(data):raise ValueError('registration dependency provenance mismatch')
    dependency['provenance']=provenance(source,repository,rev,source/relative_dependency,tree[relative_dependency],data)
   row={'source':key,'name':name,'kind':kind,'provenance':pr,'tier':tier,'tier_reasons':reasons,'primitives':sorted(sc.primitives(objects)),'conversion':conversion,'resolution_selected':path==choices[0][1],'external_verification':'NOT_VERIFIED','runtime_activation':False}
   row['resolution_scope']='combined_inventory_only_not_active_pack_precedence'
   row['source_classification']='disabled_example_or_test_fixture' if any(part.startswith('#') for part in path.relative_to(source).parts) else 'source_registration_reference'
   row['engine_enabled_path']=row['source_classification']!='disabled_example_or_test_fixture'
   rows.append(row);all_definitions.append(row);registration_by_path[(name,str(path))]=row
  script_by_name[name]=rows[0]
 dump(ROOT/'registered-spells'/f'{key}.json',all_definitions[-sum(len(x) for x in registrations.values()):])
 resultcounts=Counter();conversion_errors=[];slotcounts=Counter();files=[]
 for directory in monster_dirs:
  conv.monster_dir=directory
  ss.SCRIPT_DIRS=('data/scripts',directory.split('/')[0]+'/scripts')
  conv.spell_scripts=ss.SpellScripts(source,source_git=checkout,source_revision=rev)
  paths=sorted(source/d for d in tree if d.startswith(directory+'/') and d.endswith('.lua'))
  for path in paths:
   pr=provenance(source,repository,rev,path,tree[path.relative_to(source).as_posix()]);relative=path.relative_to(source/directory).as_posix()[:-4]
   late=[]
   try:name,m,callbacks=cb.load_monster(path,late)
   except Exception as exc:
    err={'source':key,'provenance':pr,'status':'SOURCE_NOT_EVALUATED','error':str(exc).splitlines()[0],'external_verification':'NOT_VERIFIED'}
    files.append(err);conversion_errors.append(err);resultcounts['source_not_evaluated']+=1;continue
   if not isinstance(m,dict):
    err={'source':key,'provenance':pr,'status':'NOT_MONSTER_REGISTRATION','error':'No registered monster table'};files.append(err);resultcounts['not_monster_registration']+=1;continue
   slug=cb.slug(name);candidate_id=key+'/'+directory+'/'+relative
   profile={'candidate_id':candidate_id,'source':key,'name':name,'slug':slug,'provenance':pr,'late_load_errors':late,'external_references':[],'external_spell_verification':'NOT_VERIFIED','runtime_activation':False}
   rawslots=[]
   for group in ('attacks','defenses'):
    for n,entry in sequence(m.get(group)):
     if not isinstance(entry,dict):continue
     sname=str(entry.get('name','')).lower();selected=conv.spell_scripts.index.get(sname)
     registration=registration_by_path.get((sname,str(selected[1]))) if selected else None
     slot={'candidate_id':candidate_id,'source':key,'monster':name,'slug':slug,'group':group,'source_slot_index':n,'source_parameters':entry,'monster_source':pr,'resolution':'registered' if registration else ('inline' if sname in sc.INLINE else 'missing_registration'),'registered_source':registration['provenance'] if registration else None,'script_tier':registration['tier'] if registration else None,'external_verification':'NOT_VERIFIED','runtime_activation':False}
     rawslots.append(slot)
   try:
    conv.pending_definitions=set();s,monster,deps,catalog,manifest,_=conv.convert(relative)
    local={i['identity']['key'] for i in deps['items']}
    for family,k in sorted(conv.pending_definitions):
     if cb.ref(family,k) not in catalog['definitions'] and k!=monster['creature']['identity']['key'] and k not in local:catalog['definitions'].append(cb.ref(family,k))
    recovery=partial.RECOVERIES.get(str(path))
    if recovery:
     profile['partial_extraction']=recovery
     for omitted in recovery['omitted_fields']:
      manifest['entries'].append({'source_index':0,'source_file':pr['path'],'source_line':1,'source_field':omitted['field'],'kind':'field','status':'unresolved_semantics','resolution':'Source expression not evaluated; '+omitted['error']})
     for dependency in recovery['source_literal_dependencies']:
      dp=Path(dependency['path'])
      if not dp.is_absolute():dp=source/dp
      relative_dependency=dp.relative_to(source).as_posix()
      data=git(checkout,'show',rev+':'+relative_dependency)
      if 'sha256' in dependency and dependency['sha256']!=sha(data):raise ValueError('dependency digest mismatch')
      if 'revision' in dependency and dependency['revision']!=rev:raise ValueError('dependency revision mismatch')
      dependency['provenance']=provenance(source,repository,rev,dp,tree[relative_dependency],data)
    catalog['definitions']=[r for r in catalog['definitions'] if not (r['family']=='Item' and any(r['key']==i['identity']['key'] and r['revision']==i['identity']['revision'] for i in deps['items']))]
    errors=vm.validate(monster,deps,catalog,None)
    pending=[r for r in manifest['entries'] if r['status'] in OPEN]
    target=ROOT/'converted-bundles'/key/directory/relative
    for filename,obj in [('monster.json',monster),('dependencies.json',deps),('catalog.json',catalog),('manifest.json',manifest)]:dump(target/filename,obj)
    profile.update(conversion_status='STRUCTURE_INVALID' if errors else ('CANDIDATE_BLOCKED' if pending else 'CANDIDATE_STRUCTURE_RESOLVED'),structure_errors=errors,unresolved_rows=pending,bundle_path=str(target.relative_to(ROOT)),bundle_sha256={filename:sha((target/filename).read_bytes()) for filename in ['monster.json','dependencies.json','catalog.json','manifest.json']},dependency_counts={k:len(v) for k,v in deps.items()})
    for slot in rawslots:
     field=f"{slot['group']}[{slot['source_slot_index']}]"
     entries=[r for r in manifest['entries'] if r['source_index']==0 and (r['source_field']==field or r['source_field'].startswith(field+'.'))]
     slot['conversion_manifest_rows']=entries;slot['conversion_status']=entries[0]['status'] if entries else 'MISSING_SOURCE_SLOT_COVERAGE'
     if slot['conversion_status']=='mapped':
      dest=entries[0]['destination'];obj={'monster':monster}
      for part in dest.strip('/').split('/'):obj=obj[int(part)] if isinstance(obj,list) else obj[part]
      slot['typed_schedule']=obj
      aref=obj['ability'];ability=next(a for a in deps['abilities'] if a['identity']=={'key':aref['key'],'revision':aref['revision']})
      slot['typed_ability']=ability
      keys={r['key'] for r in ability.get('effects',[])}
      slot['typed_effects']=[e for e in deps['effects'] if e['identity']['key'] in keys]
      slot['typed_formulas']=deps['formulas']
    resultcounts[profile['conversion_status']]+=1
   except Exception as exc:
    profile.update(conversion_status='CONVERSION_ERROR',conversion_error=f'{type(exc).__name__}: {exc}');conversion_errors.append({'candidate_id':candidate_id,'provenance':pr,'error':profile['conversion_error']});resultcounts['CONVERSION_ERROR']+=1
    for slot in rawslots:slot.setdefault('conversion_status','CONVERSION_ERROR')
   for slot in rawslots:slotcounts[slot['conversion_status']]+=1
   all_slots.extend(rawslots);all_profiles.append(profile);files.append({'candidate_id':candidate_id,'provenance':pr,'conversion_status':profile['conversion_status'],'slots':len(rawslots)})
 dump(ROOT/f'{key}-monster-files.json',files)
 dump(ROOT/f'{key}-conversion-errors.json',conversion_errors)
 all_reports.append({'source':key,'repository':repository,'revision':rev,'monster_directories':monster_dirs,'script_directories':script_dirs,'source_monster_files':len(files),'registered_spell_names':len(registrations),'registered_spell_definitions':sum(len(x) for x in registrations.values()),'outcomes':dict(resultcounts),'slot_outcomes':dict(slotcounts)})
 print(json.dumps(all_reports[-1]),flush=True)
dump(ROOT/'monster-profiles.json',all_profiles);dump(ROOT/'monster-spell-slots.json',all_slots);dump(ROOT/'all-registered-spells.json',all_definitions)
for source_key,*_ in SOURCES:
 dump(ROOT/f'{source_key}-monster-spell-slots.json',[x for x in all_slots if x['source']==source_key])
 dump(ROOT/f'{source_key}-monster-profiles.json',[x for x in all_profiles if x['source']==source_key])
if any(sha((REPOSITORY/path).read_bytes())!=digest for path,digest in PIPELINE_SHA256.items()):raise ValueError('converter code/schema changed during regeneration')
report={'schema_version':1,'runtime_activation':False,'sources':all_reports,'converter_inputs_sha256':PIPELINE_SHA256,'source_input_root':str(SOURCE_INPUTS),'source_first':True,'external_reference_inputs':0,'elapsed_seconds':round(time.monotonic()-t0,3),'total_monster_profiles':len(all_profiles),'total_source_spell_slots':len(all_slots),'total_spell_registrations':len(all_definitions),'external_spell_verification_counts':dict(Counter(s['external_verification'] for s in all_slots)),'limits':['Every candidate retains source-scoped identities; conversion does not allocate native Game/protocol identities.','Converter historical engine rules/approved encounter omissions are reference assumptions; latest source engine parity and external gameplay verification remain explicit requirements.','External wiki/official/calculator inputs were not loaded or applied; source/schema closure comes first.','Both donor source populations remain distinct; duplicate monster/spell names do not authorize source precedence.','Full wild monster AI and encounter callback consumers are not activated by this import.']}
dump(ROOT/'summary.json',report);print(json.dumps(report,ensure_ascii=False),flush=True)
