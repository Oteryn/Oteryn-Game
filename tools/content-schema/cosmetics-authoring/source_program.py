"""Draft source linker: no donor code executes, only reviewed finite records bind."""
from pathlib import Path
import json,re,hashlib,xml.etree.ElementTree as ET,sys,subprocess,os
ROOT=Path(__file__).resolve().parents[3];IN=ROOT/'tools/content-schema/cosmetics-authoring/npc-source-transitions.json';OUT=ROOT/'content/cosmetics'
SHA='201189f871d23597b82caf88fd611454a2ddefb8'
DONOR_GIT=os.environ.get('OTERYN_CRYSTAL_DONOR_GIT')
if not DONOR_GIT:raise SystemExit('Set OTERYN_CRYSTAL_DONOR_GIT to the retained Crystal Git repository containing pinned revision '+SHA)
sys.path.insert(0,str(ROOT/'tools/content-schema/quest-authoring'));import lua_tables
raw=IN.read_bytes();source=json.loads(raw);assert source['schema']=='OTERYN_NPC_COSMETIC_TRANSITIONS/v1' and source['revision']==SHA
cosmetics=json.loads((ROOT/'content/cosmetics/outfits/authoring.json').read_bytes())['document']
looks={r[s]['id']:{'look':r[s]['id'],'sex':s,'outfit_key':r['key'],'addon_count':r['addon_count']} for r in cosmetics['outfits'] for s in ['male','female']}
known={t['key']:{'quest':q['quest'],'key':t['key']} for q in json.loads((ROOT/'content/quests/missions/quest-state.json').read_bytes())['quests'] for t in q['tracks']}
# The source-qualified Rust quest overlays use these existing canonical declarations too.
for path in (ROOT/'content/quests').glob('*.json'):
 try:d=json.loads(path.read_bytes())
 except (ValueError,UnicodeDecodeError):continue
 if not isinstance(d,dict):continue
 for t in d.get('tracks',[]) if isinstance(d.get('tracks'),list) else []:
  if isinstance(t,dict) and isinstance(t.get('key'),str):known.setdefault(t['key'],{'quest':t.get('quest',d.get('quest','oteryn:quest.outfit_and_addon_quests')),'key':t['key']})
# Explicit new acquisition component: never append tracks to a preexisting Quest definition.
component=json.loads((ROOT/'tools/content-schema/cosmetics-authoring/source-acquisition-components.json').read_bytes())
if component['schema']!='OTERYN_NPC_ACQUISITION_COMPONENTS/v1' or component['source_revision']!=SHA:raise ValueError('source acquisition component identity')
component_bindings={row['storage']:row['key'] for row in component['bindings']}
if len(component_bindings)!=12 or len(component['bindings'])!=12:raise ValueError('source acquisition component coverage')
if any(value['quest']==component['quest'] for value in known.values()):raise ValueError('source acquisition component collides with existing Quest owner')
# Four foreign final flags have no preexisting native declaration. Their
# explicit acquisition owner does not append to an existing Quest definition.
final_component=json.loads((ROOT/'tools/content-schema/cosmetics-authoring/source-final-acquisition-components.json').read_bytes())
if final_component['schema']!='OTERYN_NPC_FINAL_ACQUISITION_COMPONENT/v1' or final_component['source_revision']!=SHA:
 raise ValueError('final acquisition component identity')
final_bindings={row['storage']:row['key'] for row in final_component['bindings']}
if len(final_bindings)!=4 or len(final_component['bindings'])!=4 or set(final_bindings)&set(component_bindings):
 raise ValueError('final acquisition component coverage')
if any(value['quest']==final_component['quest'] for value in known.values()):
 raise ValueError('final acquisition component collides with existing Quest owner')
def snake(s):return re.sub(r'([a-z0-9])([A-Z])',r'\1_\2',s).lower()
def storage_key(s,npc):
 if s.isdigit():return f'oteryn:quest-progress/npc-cosmetic/{npc}/storage_{s}'
 tail='/'.join(snake(p) for p in s.removeprefix('Storage.').split('.'));tail=re.sub(r'\s*\+\s*(\d+)',r'_plus_\1',tail)
 choices=[f'oteryn:quest-progress/{tail}',f'oteryn:quest-progress/crystalserver/{tail}']
 choices+= [c.replace('/quest_line','/questline') for c in choices]
 matched=[c for c in choices if c in known]
 return matched[0] if matched else choices[0]
# Offline source initialization capture, qualified by exact pinned NPC bytes.
# Donor callbacks are not executed by the linker or by the server.
greet_capture=json.loads((ROOT/'tools/content-schema/cosmetics-authoring/source-default-greetings.json').read_bytes())
if greet_capture['schema']!='OTERYN_SOURCE_TOPLEVEL_GREET_CAPTURE/v1' or greet_capture['source_revision']!=SHA:raise ValueError('source greeting capture identity')
greet_defaults={row['npc']:row for row in greet_capture['npcs']}
if len(greet_defaults)!=12:raise ValueError('source greeting capture closure')
def source_greeting(node,code):
 row=greet_defaults.get(node['npc'])
 if row is None:
  if 'setCallback(CALLBACK_GREET' in code:raise ValueError('unqualified registered Greet source')
  return literal_message(code,'GREET','Greetings, |PLAYERNAME|.')
 if row['source_path']!=node['source']['path'] or row['source_sha256']!=node['source']['sha256']:
  raise ValueError('source greeting capture identity')
 # Ajax always overrides MESSAGE_GREET, including the registered callback else.
 if node['npc']=='oteryn:npc.ajax':
  text="Whatcha do in my place?"
  if text not in code:raise ValueError('source Ajax ordinary greeting drift')
  return text
 return row['default_greeting']

def literal_message(text,kind,default):
 pattern=r'npcHandler:setMessage\(MESSAGE_'+kind+r',\s*("(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\')'
 m=re.search(pattern,text)
 if not m:return default
 return lua_tables._string(lua_tables.tokenize(m.group(1))[0])
program={'schema':source['schema'],'revision':SHA,'bindings':[],'items':[],'looks':list(looks.values()),'mounts':[],'npcs':[]}
bindings={};item_ids=set();mappings={};qualifications=[]
def visit(v,fn):
 if isinstance(v,dict):
  fn(v)
  for a in v.values():visit(a,fn)
 elif isinstance(v,list):
  for a in v:visit(a,fn)
for n in source['npcs']:
 name=n['npc'].split('.')[-1]
 if name in ['appaloosa','palomino','walter_jaeger','flickering_soul']:continue
 code=subprocess.check_output(['git','--git-dir='+DONOR_GIT,'show',SHA+':'+n['source']['path']]);assert hashlib.sha256(code).hexdigest()==n['source']['sha256']
 owned=set(n.get('cosmetic_owned_writable_tracks',[]));symbols=set()
 def collect(v):
  if isinstance(v.get('storage'),str):symbols.add(v['storage'])
  if v.get('kind') in ['consume_item','mint_item','inventory'] and isinstance(v.get('item'),int):item_ids.add(v['item'])
 for t in n['transitions']:visit(t,collect)
 for symbol in symbols:
  key=storage_key(symbol,name);binding={'storage':symbol,'key':key,'quest':known.get(key,{}).get('quest','oteryn:quest.outfit_and_addon_quests')}
  if symbol in component_bindings:
   if key!=component_bindings[symbol] or key in known:raise ValueError('source acquisition component would rebind an existing track')
   binding['quest']=component['quest']
  if symbol in final_bindings:
   if key!=final_bindings[symbol] or key in known:raise ValueError('final acquisition component would rebind an existing track')
   binding['quest']=final_component['quest']
  if symbol in bindings and bindings[symbol]!=binding:raise ValueError('ambiguous symbolic binding '+symbol)
  bindings[symbol]=binding
 node={'npc':n['npc'],'source':n['source'],'greeting':source_greeting(n,code.decode()),'farewell':literal_message(code.decode(),'FAREWELL','Good bye, |PLAYERNAME|.'),'owned_tracks':sorted(owned),'transitions':[]}
 for t0 in n['transitions']:
  t=dict(t0);t['source_key']=f"oteryn:npc-cosmetic/{name}/{t['id']}";t['grants']=[]
  grants=[e for e in t['effects'] if e['kind']=='grant_look_addon'];pairs={}
  for g in grants:
   definition=looks[g['look']];pairs.setdefault((definition['outfit_key'],g['addon']),[]).append(definition)
  for (outfit,addon),members in pairs.items():
   tail=outfit.rsplit('.',1)[-1].removesuffix('_outfits')
   if len(members)==2 and {m['sex'] for m in members}=={'male','female'}:
    key=f'oteryn:appearance-grant/npc/{name}/{tail}/'+('base' if addon==0 else f'addon{addon}')
    target={'kind':'Outfit','outfit_key':outfit} if addon==0 else {'kind':'Addon','outfit_key':outfit,'addon':addon}
    t['grants'].append({'key':key,'outfit_key':outfit,'look':members[0]['look'],'sex':members[0]['sex'],'addon':addon,'paired':True})
    mappings[key]={'grant_key':key,'target':target,'compatible_provenance':['crystal-201189f-npc-cosmetics-v1']}
   else:
    for member in members:
     key=f"oteryn:appearance-grant/npc/{name}/{tail}/{member['sex']}/"+('base' if addon==0 else f'addon{addon}')
     target={'kind':'LookAddon','outfit_key':outfit,'sex':member['sex'],'look_type':member['look'],'addon':addon}
     t['grants'].append({'key':key,'outfit_key':outfit,'look':member['look'],'sex':member['sex'],'addon':addon,'paired':False})
     mappings[key]={'grant_key':key,'target':target,'compatible_provenance':['crystal-201189f-npc-cosmetics-v1']}
  assert len(t['grants'])<=2
  node['transitions'].append(t)
 program['npcs'].append(node)
from source_program_support import append_support
append_support(ROOT,program,bindings)
program['bindings']=[bindings[k] for k in sorted(bindings)]
program['items']=[{'id':i,'definition':{'family':'Item','production_key':f'oteryn:item.tibia.i{i}','revision_ref':'definition-r1'}} for i in sorted(item_ids)]
mount_xml=ET.ElementTree(ET.fromstring(subprocess.check_output(['git','--git-dir='+DONOR_GIT,'show',SHA+':data/XML/mounts.xml'])))
for mount in mount_xml.getroot():
 if int(mount.get('id')) in [11,202]:program['mounts'].append({'id':int(mount.get('id')),'look':int(mount.get('clientid')),'key':'oteryn:content.mount.'+('stampor' if mount.get('id')=='11' else 'noxious_ripptor')})
OUT.mkdir(exist_ok=True)
(OUT/'npc-acquisitions.json').write_text(json.dumps(program,ensure_ascii=True,indent=2)+'\n')
(ROOT/'tools/content-schema/cosmetics-authoring/srv7-source-program-grants.json').write_text(json.dumps({'schema':'OTERYN_NPC_SOURCE_GRANT_MAPPINGS/v1','records':[mappings[k] for k in sorted(mappings)]},indent=2)+'\n')
print('Linked',len(program['npcs']),'NPCs',sum(len(n['transitions']) for n in program['npcs']),'transitions',len(bindings),'symbolicbindings',len(item_ids),'items',len(mappings),'grantmappings')
