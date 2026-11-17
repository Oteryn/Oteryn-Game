"""Pinned M25 corpse policies; Crystal room mechanics do not select Canary Item facts.

Ahau uses the existing Crystal identities. Mitmah's canonical Creature is Canary47df,
so its four existing reference-only Item identities require that donor's exact chain.
No canonical identities, WorldObject rows, decay clocks or runtime authority are created.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import build_source_item_profiles as source
from build_spell_item_identities import binding_index

PATH = 'tools/content-schema/native-gameplay/m25-corpse-item-profiles.json'
PIN_PATH = 'tools/content-schema/native-gameplay/m25-corpse-item-profiles-pin.json'
PACKET = 'tools/content-schema/native-gameplay/candidate-bindings/m25-corpse-items-candidate.json'
RUST_PIN = 'crates/gameplay/content/src/content/native_gameplay/m25_corpse_items_pin.rs'
SCHEMA = 'OTERYN_M25_CORPSE_ITEM_PROFILES/v1'
REVISIONS = {'crystal': '201189f871d23597b82caf88fd611454a2ddefb8',
             'canary': '47dfd51f45280a59a1d3e50ba7edd573d7234446'}
CHAINS = [[42069, 42072, 42071, 42070], [44687, 44686, 44685, 44684]]
OWNED = frozenset(item for chain in CHAINS for item in chain)
CAPACITIES = {42069:30, 42072:30, 42071:30, 42070:None,
              44687:20, 44686:20, 44685:10, 44684:None}
SOURCE_HASHES = {
 'crystal': {
  'data/items/items.xml':'8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
  'data/items/appearances.dat':'17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
  'src/items/items.hpp':'06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
  'src/items/items.cpp':'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d'},
 'canary': {
  'data/items/items.xml':'1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2',
  'data/items/appearances.dat':'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50',
  'src/items/items.hpp':'b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6',
  'src/items/items.cpp':'f695ed647e111923cf8e4d487c4a0706f19e7edbfb7896655c20494d6d927fd7',
  'data-otservbr-global/monster/bosses/mitmah_vanguard.lua':'a5ad42956dadd61b9e4fef0b9d7c5afea1a41049bfad23f83eb21be8a6a7d7f4'},
}
REFERENCE_PATH = 'content/items/definitions/items-25500-25999.json'
ORDINALS = {44684:279, 44685:280, 44686:281, 44687:282}


def encode(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))+'\n').encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def reference(item):
    return {'client_projection':'ClientSafe', 'identity':{'family':'Item',
        'key':f'oteryn:item.tibia.i{item}', 'revision':'definition-r1'},
        'kind':'Item', 'materializable':False, 'stack_class':'Unknown'}


def checked_references(root):
    rows = json.loads((root/REFERENCE_PATH).read_bytes())['records']
    result=[]
    for item, ordinal in ORDINALS.items():
        if rows[ordinal]['definition'] != reference(item):
            raise ValueError('M25_CANONICAL_REFERENCE_CHANGED')
        if sum(row['definition']['identity']['key']==reference(item)['identity']['key'] for row in rows)!=1:
            raise ValueError('M25_CANONICAL_REFERENCE_DUPLICATE')
        result.append({'path':REFERENCE_PATH,'ordinal':ordinal,
                       'definition_sha256':digest(encode(reference(item)))})
    return result


def binding(item):
    return {'source_key':'oteryn:source.canary','source_revision':REVISIONS['canary'],
            'identity_namespace':'ots/item_server_id','external_id':str(item),
            'target':reference(item)['identity'],'disposition':'EXACT'}


def source_rows():
    return [{'repository':'zimbadev/crystalserver' if donor=='crystal' else 'opentibiabr/canary',
             'revision':REVISIONS[donor],'path':path,'sha256':sha}
            for donor, rows in SOURCE_HASHES.items() for path,sha in rows.items()]


def capture(crystal_cache, canary_root, root=ROOT):
    blobs={}
    for donor, hashes in SOURCE_HASHES.items():
        for path, sha in hashes.items():
            file=(crystal_cache/Path(path).name) if donor=='crystal' else canary_root/path
            raw=file.read_bytes()
            if digest(raw)!=sha: raise ValueError('M25_SOURCE_DIGEST:'+donor+'/'+path)
            blobs[donor,path]=raw
        defaults=blobs[donor,'src/items/items.hpp']
        if any(token not in defaults for token in [b'int32_t weight = 0;',b'uint16_t maxItems = 8;',b'bool stopTime = false;']):
            raise ValueError('M25_LOADER_DEFAULTS')
    if b'monster.corpse = 44687' not in blobs['canary','data-otservbr-global/monster/bosses/mitmah_vanguard.lua']:
        raise ValueError('M25_NATIVE_CORPSE_IDENTITY')
    candidate={'schema':'OTERYN_SOURCE_ITEM_IDENTITY_CANDIDATE/v1','family':'Item',
        'bindings':[binding(item) for item in sorted(ORDINALS)],
        'qualification':{'source_revision':REVISIONS['canary'],
            'sources':[row for row in source_rows() if row['revision']==REVISIONS['canary']],
            'existing_references':checked_references(root),
            'method':'Pinned native corpse identity, XML decay chain, protobuf flags and Item loader defaults; existing canonical references only'}}
    bindings=binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    for item in ORDINALS:
        if str(item) in bindings: raise ValueError('M25_MITMAH_ALREADY_BOUND')
        bindings[str(item)]=binding(item)
    records=[]
    for donor, chain in zip(['crystal','canary'],CHAINS):
        xml_raw=blobs[donor,'data/items/items.xml'];dat=blobs[donor,'data/items/appearances.dat']
        xml=source.xml_index(xml_raw);objects=source.engine_items.load_appearance_objects(dat)
        physics={row['server_item_id']:row for row in source.policies(dat,xml_raw,set(chain))}
        for item in chain:
            if item not in xml or not objects[item]['flags'].get('flags.corpse'):
                raise ValueError('M25_CORPSE_SOURCE_MISSING')
            record=source.profile(item,xml[item],objects[item],physics[item],bindings,None)
            # Both pinned loaders use clip/top/bottom for stack position. This
            # is the same source fact later composed by House registration,
            # derived here from the SELECTED donor rather than final output.
            top = next((order for flag, order in [('clip',1),('top',3),('bottom',2)]
                        if objects[item]['flags'].get('flags.'+flag)), 0)
            record['attributes']['always_on_top'] = top != 0
            record['attributes']['always_on_top_order'] = top
            record['authoring']['item']['revision']='m25-galthen-corpses-'+donor+'-'+REVISIONS[donor][:7]
            record['production_binding']=copy.deepcopy(bindings[str(item)])
            record['production_binding_qualification']=copy.deepcopy(candidate) if donor=='canary' else None
            target=bindings[str(item)]['target']
            record['production_definition']={'family':'Item','production_key':target['key'],'revision_ref':target['revision']}
            records.append(record)
    records.sort(key=lambda row:int(row['production_binding']['external_id']))
    packet={'schema':SCHEMA,'source_revisions':REVISIONS,'roots':[42069,44687],
            'decay_chains':CHAINS,'sources':source_rows(),'records':records}
    validate(packet,candidate,root)
    return packet,candidate


def validate(packet,candidate,root=ROOT):
    if (packet['schema']!=SCHEMA or packet['source_revisions']!=REVISIONS
        or packet['roots']!=[42069,44687] or packet['decay_chains']!=CHAINS
        or packet['sources']!=source_rows()): raise ValueError('M25_SOURCE_AUTHORITY')
    if (candidate['schema']!='OTERYN_SOURCE_ITEM_IDENTITY_CANDIDATE/v1'
        or candidate['family']!='Item' or candidate['bindings']!=[binding(i) for i in sorted(ORDINALS)]
        or candidate['qualification']['source_revision']!=REVISIONS['canary']
        or candidate['qualification']['existing_references']!=checked_references(root)
        or candidate['qualification']['sources']!=[r for r in source_rows() if r['revision']==REVISIONS['canary']]):
        raise ValueError('M25_IDENTITY_AUTHORITY')
    if [int(r['production_binding']['external_id']) for r in packet['records']]!=sorted(OWNED):
        raise ValueError('M25_EXACT_RECORD_SET')
    crystal_bindings=binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    for record in packet['records']:
        item=int(record['production_binding']['external_id']);canary=item in ORDINALS
        expected=binding(item) if canary else crystal_bindings[str(item)]
        if (record['production_binding']!=expected
            or record['production_binding_qualification']!=(candidate if canary else None)
            or record['production_definition']!={'family':'Item','production_key':expected['target']['key'],'revision_ref':expected['target']['revision']}):
            raise ValueError('M25_RECORD_BINDING')
        sem=record['semantics'];cap=CAPACITIES[item]
        if (record['attributes'].get('always_on_top') is not False
            or record['attributes'].get('always_on_top_order') != 0):
            raise ValueError('M25_SOURCE_TOP_ORDER')
        if sem['container'] != (source.K({'capacity':source.K(cap)}) if cap is not None else source.NA):
            raise ValueError('M25_SOURCE_CAPACITY')
        chain=CHAINS[1 if canary else 0];stage=chain.index(item)
        target=chain[stage+1] if stage<3 else None
        if sem['physical']!=source.K({'weight':source.K(0), 'movable':source.K(stage!=0), 'pickupable':source.K(False)}):
            raise ValueError('M25_SOURCE_PHYSICS')
        if sem['stack']!=source.K({'stackable':source.K(False),'stack_max':source.NA}):
            raise ValueError('M25_SOURCE_STACK')
        if sem['temporal']!=source.K({'consumption_mode':source.K('DURABLE_ABSOLUTE_DEADLINE'),
                'duration':source.K([10000,300000,300000,60000][stage]),'stop_duration':source.K(False),
                'decay_target':source.K({'key':f'oteryn:item.tibia.i{target}','revision':'definition-r1'}) if target else source.NA}):
            raise ValueError('M25_SOURCE_DECAY')
        if record['admission']!={'materializable':True,'stack_class':'NonStackable','legal_destinations':['Ground']}:
            raise ValueError('M25_GROUND_CUSTODY')


def checked_packet(root=ROOT):
    raw=(root/PATH).read_bytes();candidate_raw=(root/PACKET).read_bytes()
    if json.loads((root/PIN_PATH).read_bytes())!={'path':PATH,'sha256':digest(raw),'candidate_path':PACKET,'candidate_sha256':digest(candidate_raw)}:
        raise ValueError('M25_CAPTURE_DRIFT')
    data=json.loads(raw);candidate=json.loads(candidate_raw);validate(data,candidate,root)
    return data,candidate


def admit(root,generated,destination,manifest_path,order,canonical):
    data,_=checked_packet(root);document=json.loads(generated[destination])
    existing={row['production_definition']['production_key']:row for row in document['records']}
    for record in data['records']:
        key=record['production_definition']['production_key']
        if key in existing:
            if existing[key]!=record: raise ValueError('M25_EXISTING_POLICY_CONFLICT:'+key)
        else: document['records'].append(record)
    document['records'].sort(key=order);generated[destination]=canonical(document)
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=digest(generated[destination])
    generated[manifest_path]=canonical(manifest)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal-cache',type=Path,required=True)
    parser.add_argument('--canary-root',type=Path,required=True)
    parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data,candidate=capture(args.crystal_cache,args.canary_root)
    raw=encode(data);candidate_raw=encode(candidate)
    outputs={PATH:raw,PACKET:candidate_raw,
        PIN_PATH:encode({'path':PATH,'sha256':digest(raw),'candidate_path':PACKET,'candidate_sha256':digest(candidate_raw)}),
        RUST_PIN:('// Generated only by tools/content-schema/native-gameplay/m25_corpse_items.py.\n'
            'pub(super) const CAPTURE_SHA256: &str =\n    "'+digest(raw)+'";\n'
            'pub(super) const PACKET_SHA256: &str =\n    "'+digest(candidate_raw)+'";\n').encode()}
    for path,content in outputs.items():
        if args.check:
            if (ROOT/path).read_bytes()!=content: raise ValueError('M25_OUTPUT_DRIFT:'+path)
        else:
            (ROOT/path).parent.mkdir(parents=True,exist_ok=True);(ROOT/path).write_bytes(content)
    print('M25_CORPSE_SOURCE_RECORDS',len(data['records']),'CHECK' if args.check else 'GENERATED')


if __name__=='__main__': main()
