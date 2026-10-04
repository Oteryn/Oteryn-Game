#!/usr/bin/env python3
"""Bounded pinned source Item importer for native spell and map candidates.

Loader defaults are explicit source facts. Unknown unsupported XML systems stay
UNKNOWN and their complete source records are retained in the evidence packet.
No asset membership, numeric key rewriting or name guess grants materialization.
"""
from __future__ import annotations
import argparse, copy, hashlib, json, subprocess, sys, xml.etree.ElementTree as ET
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/item-authoring'))
import engine_items
from build_spell_item_identities import binding_index, source_xml, bind_profiles, CANARY_PIN, BINDINGS
from canary_generic_item_identity import candidate
from build_source_world import policies, bool_xml
MAP_IDS={1949,2338,2429,2430,2433,2441,2454,2471,2472,2480,2487,2488,2493,2494,2983,2985,2988,3277,3287,3349,3350,3446,3447,3449,3450,3655,7363,7364,7365,7366,7368,7378,9552,9561,9573,9574,15320}
PATHS=('data/items/appearances.dat','data/items/items.xml','src/items/items.hpp','src/items/items.cpp','src/items/item.cpp','src/items/functions/item/item_parse.cpp','src/items/functions/item/item_parse.hpp','src/utils/const.hpp','src/creatures/combat/spells.cpp','src/creatures/combat/combat.cpp','src/items/tile.cpp','src/utils/utils_definitions.hpp','config.lua.dist')
K=lambda value:{'state':'KNOWN','value':value}
NA={'state':'NOT_APPLICABLE'};U={'state':'UNKNOWN'}
SECTIONS=('presentation','classification','physical','stack','equipment','weapon','protection','skill_modifiers','charges','temporal','container','imbuement','use_transform','trade_restrictions','fluid','readable_writeable')

def pinned(root,path):return subprocess.check_output(['git','-C',str(root),'show',f'{CANARY_PIN}:{path}'])
def attrs(node):
 result={}
 for a in node.findall('attribute'):
  if a.get('key','').lower()!='script':result[a.get('key','').lower()]=a.get('value')
 return result

def xml_index(raw):
 result={}
 for n in ET.fromstring(raw):
  lo=int(n.get('id',n.get('fromid','0')));hi=int(n.get('id',n.get('toid','0')))
  for i in range(lo,hi+1):
   if i in result:raise ValueError('duplicate effective XML Item')
   result[i]=n
 return result

def refs(value):
 if isinstance(value,dict):
  if value.get('family')=='Item' and value.get('key','').startswith('candidate:item/'):yield int(value['key'].split('/')[-1])
  for v in value.values():yield from refs(v)
 elif isinstance(value,list):
  for v in value:yield from refs(v)

def profile(i,node,obj,physical,bindings,field):
 a=attrs(node) if node is not None else {}; fl=obj['flags'];sem={s:copy.deepcopy(U) for s in SECTIONS}
 stack=bool(fl.get('flags.cumulative',False));container=bool(fl.get('flags.container',False)) or 'containersize' in a
 movable=physical['movable'];pickup=physical['pickupable'];weight=int(a.get('weight','0'))
 if not 0<=weight<=0xffffffff:raise ValueError('unsupported source signed weight')
 sem['physical']=K({'weight':K(weight),'movable':K(movable),'pickupable':K(pickup)})
 sem['stack']=K({'stackable':K(stack),'stack_max':K(int(a.get('stacksize','100'))) if stack else copy.deepcopy(NA)})
 sem['container']=K({'capacity':K(int(a.get('containersize','8')))}) if container else copy.deepcopy(NA)
 sem['temporal']=copy.deepcopy(NA)
 if 'duration' in a:
  duration=int(a['duration'])*1000;target=int(a.get('decayto','0'))
  if duration<=0 or duration>0xffffffff:raise ValueError('source duration bounds')
  if target and str(target) not in bindings:raise ValueError('unbound temporal target')
  sem['temporal']=K({'consumption_mode':K('DURABLE_ABSOLUTE_DEADLINE'),'duration':K(duration),'stop_duration':K(a.get('stopduration','0') in ['1','true']),
   'decay_target':K({'key':bindings[str(target)]['target']['key'],'revision':bindings[str(target)]['target']['revision']}) if target else copy.deepcopy(NA)})
 typ=a.get('type'); itemtypes={'bed':'BED','door':'DOOR','magicfield':'MAGIC_FIELD','teleport':'TELEPORT','container':'CONTAINER','rune':'RUNE','key':'KEY','mailbox':'MAILBOX','trashholder':'TRASH_HOLDER','depot':'DEPOT'}
 if 'field' in a:typ='magicfield'
 if container and typ is None:typ='container'
 sem['classification']=K({'item_type':K(itemtypes[typ]) if typ in itemtypes else copy.deepcopy(NA),'capabilities':copy.deepcopy(U)})
 sem['presentation']=K({'name':K(node.get('name',obj.get('name',''))) if node is not None else K(obj.get('name','')),
  'description':K(a.get('description',obj.get('description','')))})
 sem['charges']=K({'count':K(int(a['charges']))}) if 'charges' in a else copy.deepcopy(NA)
 weapon=a.get('weapontype');script=node.find("attribute[@key='script']") if node is not None else None
 scripts={c.get('key','').lower():c.get('value','') for c in script} if script is not None else {}
 source_weapon=scripts.get('weapontype',weapon)
 wt={'sword':'SWORD','club':'CLUB','axe':'AXE','fist':'FIST','shield':'SHIELD','distance':'DISTANCE','missile':'DISTANCE','ammo':'AMMUNITION','ammunition':'AMMUNITION','wand':'WAND'}
 if source_weapon in wt:
  sem['weapon']=K({'weapon_type':K(wt[source_weapon]),'attack':K(int(a.get('attack','0'))),'defense':K(int(a.get('defense','0'))),'extra_defense':K(int(a.get('extradef','0'))),
    'range':K(int(a.get('range','1'))),'hit_chance':K({'numerator':int(a['hitchance']),'denominator':1}) if 'hitchance' in a else copy.deepcopy(NA),
    'max_hit_chance':K({'numerator':int(a['maxhitchance']),'denominator':1}) if 'maxhitchance' in a else copy.deepcopy(NA),
    'ammunition':K(a['ammotype'].upper()) if a.get('ammotype') in ['arrow','bolt'] else copy.deepcopy(NA),'elemental':copy.deepcopy(U) if any(k.startswith('element') for k in a) else K([])})
 # Equipment scripts contain required level/vocations; an untyped custom rule
 # remains unknown, never bypassed by an invented unconstrained pattern.
 slots={'head':'HEAD','armor':'TORSO','legs':'LEGS','feet':'FEET','shield':'SHIELD','hand':'WEAPON','ammo':'EXTRA','necklace':'AMULET','ring':'RING','backpack':'CONTAINER'}
 slot=scripts.get('slot')
 if slot in slots:
  vocations=[]
  if scripts.get('vocation'):
   for v in ['Druid','Knight','Monk','Paladin','Sorcerer']:
    if v.lower() in scripts['vocation'].lower():vocations.append(v.upper())
   if not vocations:raise ValueError('unknown source vocation constraint')
  sem['equipment']=K({'patterns':K([{'pattern_id':1,'primary_slot':K(slots[slot]),
   'additional_reserved_slots':K(['SHIELD'] if a.get('slottype')=='two-handed' and slot=='hand' else []),
   'mutually_exclusive_groups':K([]),'vocations':K(vocations),'level':K(int(scripts.get('level','0'))),'compatibility_rule':copy.deepcopy(NA)}])})
 else:sem['equipment']=copy.deepcopy(NA) if not weapon and not fl.get('flags.clothes') else copy.deepcopy(U)
 # Explicit Abilities constructor defaults speed0. Other modifiers are not
 # asserted absent when source XML declares any modifier family.
 modifier_tokens=('skill','magic','critical','leech','health','mana','reflect','absorb','suppress')
 sem['skill_modifiers']=copy.deepcopy(U) if any(any(t in k for t in modifier_tokens) for k in a) else K({'modifiers':K([])})
 return {'authoring':{'item':{'family':'Item','key':f'candidate:item/{i}','revision':'spell-p2-r21'}},'semantics':sem,
   'attributes':{'speed_bonus':int(a.get('speed','0')),'blocks_movement':physical['block_solid'],'blocks_projectile':physical['block_projectile'],
       'immovable_block_solid':physical['block_solid'] and not movable,'has_height':physical['has_height'],'field_replaceable':bool_xml(a['replaceable']) if 'replaceable' in a else True if typ=='magicfield' else None,'field_condition':field,'rune_consumption':'item_count_one' if typ=='rune' else None},
   'admission':{'materializable':True,'stack_class':'StackCapable' if stack else 'NonStackable','legal_destinations':['CharacterInventory','Ground'] if movable and pickup else ['Ground']}}

def build(canary,crystal,bundles,out):
 _,canary_source=source_xml(canary,'canary');crystal_ids,crystal_source=source_xml(crystal,'crystal')
 captured={p:pinned(canary,p) for p in PATHS};xml=xml_index(captured['data/items/items.xml']);objects=engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
 hpp=captured['src/items/items.hpp'].decode();cpp=captured['src/items/items.cpp'].decode()
 for text in ['int32_t weight = 0','uint8_t stackSize = 100','uint16_t maxItems = 8','uint32_t decayTime = 0']:
  if text not in hpp:raise ValueError('loader defaults drifted')
 if 'bool replaceable = true;' not in hpp or 'itemType.replaceable = valueAttribute.as_bool();' not in captured['src/items/functions/item/item_parse.cpp'].decode():raise ValueError('source field replacement loader drifted')
 if 'iType.stackable = object.flags().cumulative()' not in cpp:raise ValueError('source stack loader drifted')
 if 'removeChargesFromRunes = true' not in captured['config.lua.dist'].decode() or 'item->getItemCount() - 1' not in captured['src/creatures/combat/spells.cpp'].decode():raise ValueError('source Rune consumption policy changed')
 wanted=set(MAP_IDS)|{2854,3264,3412,3147,3289,3321,25760,40450}
 excluded=[]
 for path in bundles.glob('*/spell.json'):
  doc=json.loads(path.read_bytes());found=set(refs(doc))
  if 0 in found:
   manifest=json.loads((path.parent/'manifest.json').read_bytes())
   removed_proof=[r for r in manifest['entries'] if r.get('status')=='approved_omission' and 'wiki amount 0 supersedes' in r.get('resolution','')]
   if path.parent.name not in ['instant-lightest_missile_rune','instant-light_stone_shower_rune'] or not removed_proof:
    raise ValueError('active Item0 reference')
   excluded.append({'bundle':path.parent.name,'spell_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'reason':'Explicit excluded removed spell; wiki amount0 source precedence proof','removed_source_proof':removed_proof})
   continue
  wanted.update(found);wanted.update(refs(json.loads((path.parent/'dependencies.json').read_bytes())))
 bindings=binding_index(BINDINGS.read_bytes());generic=candidate(canary)
 fields_doc=json.loads((ROOT/'tools/content-schema/spell-authoring/samples/native-field-profiles.json').read_bytes())
 field_index={p['source']['server_item_id']:p for p in fields_doc['profiles'] if p['source']['server']=='canary'}
 ordinary=wanted-{40450}
 if ordinary-set(xml):raise ValueError(f'Item XML source missing:{sorted(ordinary-set(xml))}')
 if ordinary-crystal_ids:raise ValueError(f'Crystal item membership missing:{sorted(ordinary-crystal_ids)}')
 physical={r['server_item_id']:r for r in policies(captured['data/items/appearances.dat'],captured['data/items/items.xml'],ordinary)}
 records=[profile(i,xml[i],objects[i],physical[i],bindings,field_index.get(i)) for i in sorted(ordinary)]
 p=generic['qualification']['policy'];sem={s:copy.deepcopy(U) for s in SECTIONS};sem['physical']=K({'weight':K(0),'movable':K(False),'pickupable':K(False)});sem['stack']=K({'stackable':K(False),'stack_max':copy.deepcopy(NA)});sem['container']=copy.deepcopy(NA);sem['temporal']=copy.deepcopy(NA);sem['classification']=K({'item_type':copy.deepcopy(NA),'capabilities':copy.deepcopy(U)})
 records.append({'authoring':{'item':{'family':'Item','key':'candidate:item/40450','revision':'spell-p2-r21'}},'semantics':sem,
  'attributes':{'speed_bonus':0,'blocks_movement':False,'blocks_projectile':False,'immovable_block_solid':False,'has_height':False,'field_replaceable':None,'field_condition':None,'rune_consumption':None},'admission':{k:p[k] for k in ['materializable','stack_class','legal_destinations']}})
 result=bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,ordinary,generic)
 out.mkdir(parents=True,exist_ok=True);raw=(json.dumps(result,indent=2,ensure_ascii=False)+'\n').encode();(out/'item-profiles.json').write_bytes(raw)
 evidence={'schema':'OTERYN_NATIVE_SOURCE_ITEM_PROFILE_IMPORT/v1','classification':'Explicit local source candidate; not production activation','sources':[canary_source,crystal_source],
  'loader_sources':[{'path':p,'revision':CANARY_PIN,'sha256':hashlib.sha256(b).hexdigest()} for p,b in captured.items()],
  'output_sha256':hashlib.sha256(raw).hexdigest(),'records':len(records),'excluded_zero_count_bundles':excluded,'map_ids':sorted(MAP_IDS),'starter_ids':{'backpack':2854,'sword':3264,'wooden_shield':3412},
  'rune_consumption_policy':'Explicit activated ItemCountOne; pinned config REMOVE_RUNE_CHARGES=true and RuneSpell::executeUse same-ID ItemCount decrement, no separate charge counter.',
  'temporal_policy':'Explicit candidate DurableAbsoluteDeadline for known non-stopping source decay; source duration/target unchanged. Offline overdue entries retain their deadline until an eligible current owner pass.',
  'source_rows':[{'id':i,'xml':ET.tostring(xml[i],encoding='unicode'),'appearance_flags':objects[i]['flags'],'projected_physical':physical[i]} for i in sorted(ordinary)],
  'limitations':'Unsupported XML systems remain UNKNOWN; no ItemDocument authoring operations fabricated. Equipment modifiers requiring source-specialized parser remain unavailable.'}
 (out/'item-profile-source-proof.json').write_text(json.dumps(evidence,indent=2,ensure_ascii=False)+'\n');return evidence
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--canary',type=Path,default=Path('/workspace/spell-sources/canary'));p.add_argument('--crystal',type=Path,default=Path('/workspace/spell-sources/crystal'));p.add_argument('--bundles',type=Path,default=Path('/workspace/spells-r21-implemented/bundles'));p.add_argument('--out',type=Path,required=True);a=p.parse_args();print(json.dumps({k:v for k,v in build(a.canary,a.crystal,a.bundles,a.out).items() if k in ['records','output_sha256','starter_ids']}))
