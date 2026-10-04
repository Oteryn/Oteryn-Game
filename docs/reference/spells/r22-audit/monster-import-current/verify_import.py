"""Independent source conservation and candidate closure checks; no runtime activation."""
import argparse,hashlib,json,subprocess,sys
from collections import Counter
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--input',type=Path,required=True)
args=parser.parse_args()
R=args.input.resolve(strict=True)
T=Path(__file__).resolve().parents[5]/'tools/content-schema/monster-authoring'
sys.path.insert(0,str(T));import validate_monster as vm
checks=[]
def read(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def check(name,value):
 checks.append({'check':name,'passed':bool(value)})
 if not value:raise AssertionError(name)
s=read(R/'summary.json');profiles=read(R/'monster-profiles.json');slots=read(R/'monster-spell-slots.json');defs=read(R/'all-registered-spells.json')
for path,digest in s.get('converter_inputs_sha256',{}).items():
 check('converter code/schema exact '+path,sha(T.parents[2]/path)==digest)
check('reported profile count',len(profiles)==s['total_monster_profiles']);check('reported slot count',len(slots)==s['total_source_spell_slots']);check('activation stays false',all(not p['runtime_activation'] for p in profiles+slots+defs))
keys=[(x['source'],x['monster_source']['path'],x['group'],x['source_slot_index']) for x in slots];check('unique source slot keys',len(keys)==len(set(keys)))
byprofile={p['candidate_id']:p for p in profiles};provenances={}
for x in slots:
 check('slot original source bound '+str(keys[len(provenances)%len(keys)]) if False else 'slot profile source '+x['candidate_id']+'/'+x['group']+'/'+str(x['source_slot_index']),x['monster_source']==byprofile[x['candidate_id']]['provenance'])
 for pr in [x['monster_source'],x.get('registered_source')]:
  if pr:provenances[(x['source'],pr['path'])]=pr
for p in profiles:
 provenances[(p['source'],p['provenance']['path'])]=p['provenance']
 for dependency in p.get('partial_extraction',{}).get('source_literal_dependencies',[]):
  pr=dependency['provenance']
  check('dependency revision '+pr['path'],pr['revision']==p['provenance']['revision'])
  check('dependency SHA '+pr['path'],dependency.get('sha256')==pr['sha256'])
  provenances[(p['source'],pr['path'])]=pr
registration_keys=[(d['source'],d['provenance']['path'],d['kind'],d['name']) for d in defs]
check('distinct source registration identities',len(registration_keys)==len(set(registration_keys)))
for d in defs:
 provenances[(d['source'],d['provenance']['path'])]=d['provenance']
 conversion=d.get('conversion')
 check('each source variant captured or explicit error '+d['provenance']['path']+'/'+d['name'],isinstance(conversion,dict) and bool(conversion))
 check('source disabled references remain disabled '+d['provenance']['path'],not any(part.startswith('#') for part in Path(d['provenance']['path']).parts) or d.get('engine_enabled_path') is False)
 for dependency in conversion.get('reference_dependencies',[]):
  pr=dependency['provenance']
  check('registration dependency revision '+pr['path'],dependency['revision']==pr['revision']==d['provenance']['revision'])
  check('registration dependency SHA '+pr['path'],dependency['sha256']==pr['sha256'])
  provenances[(d['source'],pr['path'])]=pr
for (source,path),pr in provenances.items():
 checkout=Path('/workspace/spell-sources')/source
 b=subprocess.check_output(['git','-C',str(checkout),'show',pr['revision']+':'+path])
 staged=Path(s['source_input_root'])/source/path
 if staged.is_file():check('staged source equality '+source+'/'+path,staged.read_bytes()==b)
 check('source SHA '+source+'/'+path,hashlib.sha256(b).hexdigest()==pr['sha256']);check('source Git blob '+source+'/'+path,hashlib.sha1(b'blob '+str(len(b)).encode()+b'\0'+b).hexdigest()==pr['git_blob'])
for p in profiles:
 if 'bundle_path' not in p:continue
 root=R/p['bundle_path'];bundle=[read(root/f) for f in ('monster.json','dependencies.json','catalog.json','manifest.json')]
 for f,v in p['bundle_sha256'].items():check('bundle digest '+p['candidate_id']+'/'+f,sha(root/f)==v)
 errors=vm.validate(*bundle[:3],None);check('schema outcomes '+p['candidate_id'],errors==p['structure_errors'])
 manifest=bundle[3];ss=[x for x in slots if x['candidate_id']==p['candidate_id']]
 for slot in ss:
  if slot['conversion_status']=='mapped':
   field=f"{slot['group']}[{slot['source_slot_index']}]";rows=[r for r in manifest['entries'] if r['source_index']==0 and r['source_field']==field]
   check('actual mapped row '+p['candidate_id']+'/'+field,len(rows)==1 and rows[0]['status']=='mapped')
   deps=bundle[1];check('actual Ability '+p['candidate_id']+'/'+field,slot['typed_ability'] in deps['abilities']);check('actual Effects '+p['candidate_id']+'/'+field,all(e in deps['effects'] for e in slot['typed_effects']))
for source in s['sources']:
 checkout=Path('/workspace/spell-sources')/source['source'];actual=[]
 for directory in source['monster_directories']:
  actual+=subprocess.check_output(['git','-C',str(checkout),'ls-tree','-r','--name-only',source['revision'],'--',directory]).decode().splitlines()
 actual={p for p in actual if p.endswith('.lua')};reported=read(R/(source['source']+'-monster-files.json'))
 check('entire tracked monster population '+source['source'],actual=={x['provenance']['path'] for x in reported})
report={'status':'PASS','checks':len(checks),'failures':[],'source_files':len(provenances),'candidate_profiles':len(profiles),'source_spell_slots':len(slots),'slot_conversion_counts':dict(Counter(x['conversion_status'] for x in slots)),'input_sha256':{p.name:sha(p) for p in [R/'summary.json',R/'monster-profiles.json',R/'monster-spell-slots.json',R/'all-registered-spells.json']},'limits':['Checks establish exact source/candidate conservation and structure; do not establish external semantic agreement or runtime gameplay.']}
(R/'verification-proof.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
