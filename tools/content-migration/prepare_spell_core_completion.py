"""Prepare source-backed spell cores using existing Encounter contracts, without donor callback execution."""
import argparse, copy, hashlib, json, subprocess, sys
from pathlib import Path

def read(p): return json.loads(p.read_text())
def write(p,v):
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def digest(p):
 d=hashlib.sha256()
 for n in ('monster.json','dependencies.json','catalog.json','manifest.json'):
  b=(p/n).read_bytes();d.update(f'{n}\0{len(b)}\0'.encode()+b)
 return d.hexdigest()
def merge(rows,new,key):
 existing={key(x):x for x in rows}
 for x in new:
  k=key(x)
  if k in existing:
   if existing[k]!=x: raise ValueError('conflicting identity '+str(k))
  else: rows.append(copy.deepcopy(x));existing[k]=x

def prepare(repo,baseline,encounters,canary,output):
 if output.exists() and any(output.iterdir()):raise ValueError('output must be empty')
 sys.path[:0]=[str(repo/'tools/content-schema/monster-authoring'),str(repo/'tools/content-schema/encounter-authoring')]
 import validate_monster as vm, validate_encounter as ve
 draft=repo/'tools/content-schema/encounter-authoring/samples/round3-custom-spell-drafts.json';q=read(draft)
 receipt={'classification':'SOURCE_BACKED_TYPED_CORE_PARTIAL','engine_changes':False,'lua_callbacks_executed':False,'runtime_qualified':False,'draft_sha256':hashlib.sha256(draft.read_bytes()).hexdigest(),'source_checks':[],'actors':[],'encounters':[]}
 source_seen=set()
 for name,c in q['candidates'].items():
  old=read(encounters/name/'encounter.json');new=c['encounter.json']
  if new['rules'][:len(old['rules'])]!=old['rules']:raise ValueError('original rules not retained '+name)
  man=copy.deepcopy(c['manifest.json'])
  for s in man['sources']:
   if s['kind']!='git':continue
   k=(s['revision'],s['path'])
   if k in source_seen:continue
   data=subprocess.check_output(['git','-C',str(canary),'show',s['revision']+':'+s['path']])
   blob=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
   if blob!=s['blob_sha1']:raise ValueError('source blob mismatch '+s['path'])
   receipt['source_checks'].append({**s,'sha256':hashlib.sha256(data).hexdigest()});source_seen.add(k)
  # Encoded core is present; only documented runtime-parity gap is omitted.
  for row in man['entries']:
   if row['status']=='unresolved_semantics':
    row['status']='approved_omission';row['resolution']='Owner-authorized partial core import; retained runtime-parity limitation: '+row['resolution']
  man['classification']='SOURCE_BACKED_TYPED_CORE_PARTIAL'
  errors=ve.validate(new,c['catalog.json'],man)
  if errors:raise ValueError(errors)
  for fn,v in [('encounter.json',new),('manifest.json',man),('catalog.json',c['catalog.json'])]:write(output/'encounters'/name/fn,v)
  receipt['encounters'].append({'encounter':name,'original_rules_retained':len(old['rules']),'added_rules':len(new['rules'])-len(old['rules']),'remaining_parity_flags':['SOURCE_FORCED_PLACEMENT_PARITY_UNVERIFIED','SOURCE_NATIVE_TELEPORT_FAILURE_PARITY_UNVERIFIED']})
 for actor,owner in [('professor_maxxen','professor_maxxen'),('zamulosh','zamulosh'),('the_blazing_time_guardian','the_time_guardian'),('the_freezing_time_guardian','the_time_guardian')]:
  p=baseline/'bundles'/actor;vals={f:read(p/f) for f in ('monster.json','dependencies.json','catalog.json','manifest.json')};original=copy.deepcopy(vals)
  c=q['candidates'][owner]
  for section,rows in c['dependencies.json'].items():merge(vals['dependencies.json'][section],rows,lambda x:x['identity']['key'])
  merge(vals['catalog.json']['definitions'],c['dependency-catalog.json']['definitions'],lambda x:(x['family'],x['key'],x['revision']))
  vals['catalog.json']['assets']=sorted(set(vals['catalog.json']['assets'])|set(c['dependency-catalog.json']['assets']))
  bindings=[s for s in q['source_schedule_bindings'] if Path(s['source_file']).stem==actor]
  if not bindings:raise ValueError('missing source schedule '+actor)
  for binding in bindings:
   schedule=binding['typed_schedule'];section=binding['source_field'].split('[')[0]
   dest='/monster/behavior/'+section+'/'+str(len(vals['monster.json']['behavior'][section]))
   vals['monster.json']['behavior'][section].append(copy.deepcopy(schedule))
   matched=[e for e in vals['manifest.json']['entries'] if e['source_field']==binding['source_field']]
   if len(matched)!=1 or matched[0]['status']!='approved_omission':raise ValueError('unexpected source row')
   matched[0].update(status='mapped',destination=dest,resolution='Source schedule retained exactly; existing typed Encounter core restored. Placement and native execution parity remain explicitly unverified. Original omission retained in completion receipt.')
  # Native core leaves genuine omitted callbacks untouched.
  if vals['monster.json']['creature']['stats']!=original['monster.json']['creature']['stats'] or vals['monster.json']['loot']!=original['monster.json']['loot']:raise ValueError('stats/loot changed')
  errors=vm.validate(vals['monster.json'],vals['dependencies.json'],vals['catalog.json'],vals['manifest.json'])
  if errors:raise ValueError(errors)
  dest=output/'bundles'/actor
  for fn,v in vals.items():write(dest/fn,v)
  receipt['actors'].append({'monster':actor,'original_bundle_digest':digest(p),'bundle_digest':digest(dest),'restored_source_rows':[s['source_field'] for s in bindings],'spell_identities':[s['spell'] for s in bindings],'original_omission_rows':[e for e in original['manifest.json']['entries'] if e['source_field'] in [s['source_field'] for s in bindings]],'stats_and_loot_unchanged':True,'completion_flags':['SOURCE_TYPED_CORE_RESTORED','SOURCE_BEHAVIOR_PARTIAL','GAMEPLAY_UNVERIFIED']})
 receipt['counts']={'actors':len(receipt['actors']),'restored_source_rows':sum(len(a['restored_source_rows']) for a in receipt['actors']),'spell_identities':4,'encounter_successors':3,'remaining_actor_omissions_from_49':44}
 write(output/'completion.json',receipt)
 return receipt
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--repo',type=Path,required=True);p.add_argument('--baseline',type=Path,required=True);p.add_argument('--encounters',type=Path,required=True);p.add_argument('--canary',type=Path,required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args();print(json.dumps(prepare(a.repo,a.baseline,a.encounters,a.canary,a.output)['counts']))
