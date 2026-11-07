#!/usr/bin/env python3
"""Reference import: genuine bounded Canary OTBM cells and item-loader facts.

No Lua is executed. No native neighbours, ground, flags or current mutable items
are invented. The source Temple and source rectangle are explicit candidate inputs.
"""
import argparse, hashlib, json, struct, subprocess, sys
from pathlib import Path
import xml.etree.ElementTree as ET
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'world-authoring'))
import otbm_reader as otbm
PIN = '99902524e052f37574194466c2949c576e4ab269'
PATHS = ['data-canary/world/canary.otbm', 'data/items/appearances.dat',
         'src/protobuf/appearances.proto', 'data/items/items.xml',
         'src/items/items.cpp', 'src/items/items.hpp',
         'src/items/functions/item/item_parse.cpp', 'src/items/functions/item/item_parse.hpp',
         'src/items/item.cpp', 'src/items/tile.cpp', 'src/io/iomap.cpp', 'src/utils/const.hpp',
         'src/game/game.cpp', 'src/io/io_definitions.hpp']
BOUND = [5840,5283,5871,5314,5,8]
TEMPLE = [5854,5298,5]

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',',':'), ensure_ascii=False).encode()

def pinned(root, path):
    raw = subprocess.check_output(['git','-C',str(root),'show',f'{PIN}:{path}'])
    blob = subprocess.check_output(['git','-C',str(root),'rev-parse',f'{PIN}:{path}'],text=True).strip()
    if hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest() != blob:
        raise ValueError('git blob substitution')
    return raw, {'path':path, 'git_blob':blob, 'sha256':hashlib.sha256(raw).hexdigest()}

def var(raw, i):
    value=shift=0
    while i<len(raw) and shift<=63:
        byte=raw[i]; i+=1; value|=(byte&127)<<shift
        if byte<128: return value,i
        shift+=7
    raise ValueError('invalid protobuf varint')

def fields(raw):
    result={}; i=0
    while i<len(raw):
        key,i=var(raw,i); number=key>>3; wire=key&7
        if not number: raise ValueError('protobuf field zero')
        if wire==0: value,i=var(raw,i)
        elif wire==2:
            size,i=var(raw,i)
            if size>len(raw)-i: raise ValueError('truncated protobuf length')
            value=raw[i:i+size]; i+=size
        elif wire in [1,5]:
            size=8 if wire==1 else 4
            if size>len(raw)-i: raise ValueError('truncated fixed field')
            value=raw[i:i+size]; i+=size
        else: raise ValueError('unsupported protobuf wire')
        result.setdefault(number,[]).append(value)
    return result

def bool_xml(value):
    # pugi::xml_attribute::as_bool accepts these true-leading characters.
    return bool(value) and value[0] in '1tTyY'

def policies(raw_bank, raw_xml, ids):
    bank={}
    for raw in fields(raw_bank).get(1,[]):
        obj=fields(raw); item_id=obj.get(1,[0])[0]
        if item_id not in ids: continue
        if item_id in bank: raise ValueError('duplicate source item object')
        fl=fields(obj.get(3,[b''])[0])
        bank[item_id]={'server_item_id':item_id,
            'ground':1 in fl and not bool(fl.get(5,[0])[0]),
            'ground_speed':fields(fl[1][0]).get(1,[0])[0]&65535 if 1 in fl else 0,
            'block_solid':bool(fl.get(13,[0])[0]), 'block_projectile':bool(fl.get(15,[0])[0]),
            'block_pathfind':bool(fl.get(16,[0])[0]), 'movable':not bool(fl.get(14,[0])[0]),
            'pickupable':bool(fl.get(18,[0])[0]), 'container':bool(fl.get(5,[0])[0]),
            'floor_change':False, 'floor_change_kind':None, 'has_height':27 in fl,
            'dynamic_kind':None, 'xml_attributes':[]}
    if set(bank)!=ids: raise ValueError(f'missing source item definitions: {ids-set(bank)}')
    seen=set()
    for item in ET.fromstring(raw_xml):
        low=int(item.get('id',item.get('fromid','-1'))); high=int(item.get('id',item.get('toid','-1')))
        for item_id in sorted(ids):
            if not low<=item_id<=high: continue
            if item_id in seen: raise ValueError('duplicate XML item overrides')
            seen.add(item_id); p=bank[item_id]
            p['xml_attributes']=[dict(a.attrib) for a in item]
            for attr in item:
                key=attr.get('key','').lower(); value=attr.get('value','')
                if key in ['movable','pickupable','allowpickupable','blockprojectile']:
                    target={'allowpickupable':'pickupable','blockprojectile':'block_projectile'}.get(key,key)
                    p[target]=bool_xml(value)
                elif key=='speed': p['ground_speed']=int(value)&65535
                elif key=='floorchange':
                    if value.lower() not in ['down','north','south','southalt','west','east','eastalt']:
                        raise ValueError('unsupported source floorchange')
                    p['floor_change']=True
                    p['floor_change_kind']=value.lower()
                elif key=='type' and value.lower() in ['door','magicfield','teleport','container','bed','mailbox','trashholder']:
                    p['dynamic_kind']=value.lower()
                elif key=='field': p['dynamic_kind']='magicfield'
            p['magic_field']=p['dynamic_kind']=='magicfield'
    for p in bank.values(): p.setdefault('magic_field',False)
    return [bank[i] for i in sorted(bank)]

def tile_semantics(record, policy):
    placed=[]; ground=None; dynamic=[]
    for item in record['items']:
        if item['depth']!=0: continue
        p=policy[item['server_item_id']]; attrs=item['attributes']
        movable=p['movable'] and '5' not in attrs and attrs.get('4')!=100
        if p['ground']: ground=item['server_item_id']
        elif movable or p['pickupable'] or p['container'] or p['dynamic_kind'] is not None or 'dest' in attrs:
            dynamic.append(item['server_item_id'])
        placed.append((p,movable))
    return {'ground':ground, 'ground_speed':policy[ground]['ground_speed'] if ground is not None else None,
        'top_ids':[i['server_item_id'] for i in record['items'] if i['depth']==0 and not policy[i['server_item_id']]['ground']],
        'unmaterialized_dynamic_items':dynamic,
        'height_count':sum(1 for p,_ in placed if p['has_height']),
        'floor_changes':sorted(set(p['floor_change_kind'] for p,_ in placed if p['floor_change_kind'] is not None)),
        'flags':{'block_solid':any(p['block_solid'] for p,_ in placed),
                 'block_projectile':any(p['block_projectile'] for p,_ in placed),
                 'immovable_block_solid':any(p['block_solid'] and not move for p,move in placed),
                 'immovable_block_item':any(p['block_pathfind'] and not move for p,move in placed),
                 'immovable_nonfield_block_item':any(p['block_pathfind'] and not p['magic_field'] and not move for p,move in placed),
                 'floor_change':any(p['floor_change'] for p,_ in placed),
                 'protection_zone':bool(record['otbm_flags']&1)}}

def extract(raw):
    rows=[]; towns=[]; seen=set()
    def handle(node):
        if node[0]==otbm.NODE_TOWNS:
            facts=otbm.MapFacts(); otbm._towns(node,facts); towns.extend(facts.towns)
        if node[0]!=otbm.NODE_TILE_AREA: return
        bx,by,z=struct.unpack_from('<HHB',node[1],0)
        for tile in node[2]:
            if tile[0] not in [otbm.NODE_TILE,otbm.NODE_HOUSE_TILE]: raise ValueError('unsupported source tile kind')
            x,y=bx+tile[1][0],by+tile[1][1]
            if not BOUND[0]<=x<=BOUND[2] or not BOUND[1]<=y<=BOUND[3] or not BOUND[4]<=z<=BOUND[5]: continue
            facts=otbm.MapFacts(); house,flags,zones,items=otbm._tile_content(tile,facts)
            if facts.unknown_item_attrs or facts.unknown_tile_attrs: raise ValueError('selected source tile has unknown attributes')
            pos=(x,y,z)
            if pos in seen: raise ValueError('duplicate source tile')
            seen.add(pos)
            rows.append({'source_position':[x,y,z], 'native_position':[x-TEMPLE[0],y-TEMPLE[1],-z],
                         'otbm_flags':flags or 0,'house_id':house,'zones':list(zones),
                         'items':[{'server_item_id':i,'depth':depth,'attributes':{str(k):v for k,v in attrs.items()}}
                                  for i,depth,attrs in items]})
    otbm._scan(otbm.load_bytes(raw),handle)
    town=[t for t in towns if list(t['temple'])==TEMPLE and t['name']=='Thalom']
    if len(town)!=1 or tuple(TEMPLE) not in seen: raise ValueError('source Temple missing or ambiguous')
    if not rows or len(rows)>4096: raise ValueError('source region capacity')
    return sorted(rows,key=lambda r:r['source_position']), town[0]

def build(root):
    if subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD'],text=True).strip()!=PIN:
        raise ValueError('source checkout revision mismatch')
    blobs={}; closure=[]
    for path in PATHS:
        raw,proof=pinned(root,path); blobs[path]=raw; closure.append(proof)
    rows,town=extract(blobs[PATHS[0]])
    ids={i['server_item_id'] for r in rows for i in r['items']}
    ps=policies(blobs[PATHS[1]],blobs[PATHS[3]],ids); by_id={p['server_item_id']:p for p in ps}
    for r in rows: r['semantics']=tile_semantics(r,by_id)
    native_projection=json.loads((Path(__file__).parent/'source-map-native-item-bindings.json').read_bytes())
    native_bindings=native_projection['bindings']
    dynamic_ids={id for row in rows for id in row['semantics']['unmaterialized_dynamic_items']}
    native_by_id={int(binding['external_id']):binding for binding in native_bindings}
    if native_projection['schema']!='OTERYN_NATIVE_SOURCE_MAP_ITEM_BINDINGS/v1' or set(native_by_id)!=dynamic_ids or len(native_by_id)!=len(native_bindings):
        raise ValueError('explicit native map Item binding coverage')
    bindings=[]
    for p in ps:
        family='Terrain' if p['ground'] else 'Item'
        prefix='terrain' if p['ground'] else 'item'
        bindings.append({'disposition':'ACCEPTED_ALIAS','source_key':'oteryn:source/canary-spell-world-r3','source_revision':PIN,
            'identity_namespace':'ots/item_server_id','external_id':str(p['server_item_id']),
            'target':native_by_id[p['server_item_id']]['target'] if p['server_item_id'] in native_by_id else {'family':family,'key':f'oteryn:{prefix}.source.canary.id{p["server_item_id"]}','revision':'source-map-r3'}})
    result={'schema':'OTERYN_NATIVE_SPELL_SOURCE_WORLD/v1','candidate_profile':'native-source-spell-world-qualification-3',
        'source_repository':'https://github.com/opentibiabr/canary','source_revision':PIN,'source_closure':closure,
        'source_bounds':BOUND,'source_town':town,
        'imports':[{'batch_id':'native-canary-thalom-map-r3',
            'source_repository':'https://github.com/opentibiabr/canary','source_revision':PIN,
            'source_artifact_sha256':closure[0]['sha256'],'access_disposition':'REFERENCE_ONLY',
            'source_generation_profile':'native-source-spell-world-qualification-3',
            'importer':'source-otbm-item-loader-r3',
            'mapper':'tools/content-schema/native-gameplay/build_source_world.py','mapper_revision':'r3',
            'mapper_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'candidates':[],'reimport_states':[]}],
        'sources':[{'key':'oteryn:source/canary-spell-world-r3','import_batch_id':'native-canary-thalom-map-r3',
            'revision':PIN,'sha256':closure[0]['sha256'],'evidence':'Proven'}],
        'native_frame':{'key':'oteryn:frame/canary-thalom-spell-r3','map_revision':'oteryn:map/canary-thalom-spell-r3',
                        'origin_x':TEMPLE[0],'origin_y':TEMPLE[1],'floor_transform':'native_floor=-source_z'},
        'source_identity_bindings':bindings,'native_item_bindings':native_bindings,'item_policies':ps,'tiles':rows}
    return result

def main():
    p=argparse.ArgumentParser(); p.add_argument('--source-root',type=Path,required=True); p.add_argument('--out',type=Path,required=True)
    args=p.parse_args(); document=build(args.source_root); args.out.parent.mkdir(parents=True,exist_ok=True)
    args.out.write_bytes(canonical(document)+b'\n')
    print(f'{len(document["tiles"])} actual source tiles, {len(document["item_policies"])} item policies, sha256={hashlib.sha256(args.out.read_bytes()).hexdigest()}')
if __name__=='__main__': main()
