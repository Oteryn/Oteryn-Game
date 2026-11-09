"""Exact Ground-only Items reached by pinned boss movement and death callbacks."""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import boss_corpse_profiles as base
import boss_conditional_item_reference as reference

ROOT=base.ROOT
IDS=frozenset({20121,23726,23727,23728,33004,37000,42851,42853,43294,43295,43589,43590,43625})
NEW=(43589,43590,42851,42853,43625)
CAPTURE_FIRST=34112
FIRST=reference.canonical_ordinal(ROOT,CAPTURE_FIRST)
PATH='tools/content-schema/native-gameplay/boss-movement-item-profiles.json'
PIN_PATH='tools/content-schema/native-gameplay/boss-movement-item-profiles-pin.json'
PACKET='tools/content-schema/native-gameplay/candidate-bindings/boss-movement-items-candidate.json'
RUST_PIN='crates/gameplay/content/src/content/native_gameplay/boss_movement_bindings_pin.rs'
SOURCES={
 'data-global/scripts/quests/soul_war/moveevent-soul_war_entrances.lua':'532157fda2bf03109937b04ef3182294fe3512467e7d4d983db82c7f440aea4a',

 'data-global/scripts/creaturescripts/monster/minion_gaz_haragoth_vortex.lua':'140e7db53054ac626d49e0b411395fe7b5e0a3d4bdb951de42be0853c77d6159',
 'data-global/scripts/quests/forgotten_knowledge/creaturescripts_corrupted_soul.lua':'1e03253a5fdbe294e57b0a7066b7ec232dfac8191a7b62665be633d71421c0fa',

 'data-global/monster/quests/rotten_blood_quest/murcion.lua':'cdd4e39e5119d6c1952f3850ba4bfd19877f608716f8b166fe39a6f25ca1801a',
 'data-global/monster/quests/rotten_blood_quest/chagorz.lua':'297fc5a9fc5e58ade139dd3c3e60e10fdd2f0d6745c05dc9137c0f44d6a60732',
 'data-global/monster/quests/rotten_blood_quest/ichgahal.lua':'237c4b4a8743fc2f2868f0a0ce183fa9c1a0d66cc2037345419fe70cd3eb852d',
 'data-global/monster/quests/rotten_blood_quest/vemiath.lua':'d0b477325e4d3e058490023870075dd6e1b159cd614f82610d122510f7c7a8e7',
 'data-global/scripts/quests/rotten_blood_quest/rotten_blood_boss_mechanics.lua':'1743fdb5bc1ea75932af6dcfe8ebd13e0f6e4529d8af09dc34e47c289183773d',
 'data-global/monster/quests/rotten_blood_quest/bakragore.lua':'974df9389e734ca947111cd41af9adec44f531adf0cc53233d5a7a94ab190178',
 'data-global/scripts/quests/rotten_blood_quest/movements_fields.lua':'3ed5050ea68ba182e3bfed783ddce5198d154bf48537500ff96ac19de7c0b9e3',
 'src/lua/functions/core/game/game_functions.cpp':'e75c5a941e9ae67e10a768920b0c508c296c30824834afe90c84c640ef6293c0',
 'src/game/game.cpp':'5f7ba67a158af607bacb9d75451ca490817f189ca7d3dcf9e774d55dc1ccaab6',
 'src/items/tile.cpp':'a61778a590a2a7b4a23fe8875320d6724a8d9dbef8a76548ca45fcf89c834c9e',
}


def load(cache):
    blobs=base.load_source_blobs(cache)
    for path,expected in SOURCES.items():
        name='crystal_game.cpp' if path=='src/game/game.cpp' else Path(path).name
        raw=(cache/name).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=expected:raise ValueError('BOSS_MOVEMENT_SOURCE:'+path)
        blobs[path]=raw
    return blobs


def definitions():
    return [reference.definition(f'oteryn:item.tibia.i{i}') for i in NEW]


def extend_records(records,root):
    # Source refs follow the separately guarded43860 declaration; no ordinal can
    # be borrowed from another task's identity or silently reassigned.
    if json.loads((root/PACKET).read_bytes()) != packet():
        raise ValueError('BOSS_MOVEMENT_REFERENCE_SOURCE_PACKET')
    for offset,expected in enumerate(definitions()):
        ordinal=FIRST+offset
        matches=[n for n,row in enumerate(records) if row['identity']['key']==expected['identity']['key']]
        if matches:
            if matches!=[ordinal] or records[ordinal]!=expected:raise ValueError('BOSS_MOVEMENT_REFERENCE_CHANGED')
        else:
            if len(records)!=ordinal:raise ValueError('BOSS_MOVEMENT_NEXT_FREE_ORDINAL_CHANGED')
            records.append(expected)


def publish_references(cache,root,check):
    reference.evidence(cache,root) # actual DAT + official membership + C++ factory proof
    blobs=load(cache)
    official=json.loads((root/'imports/official/appearance-membership/admitted.json').read_bytes())
    membership=next(row for row in official['files'] if row['label']=='client-15.30')
    raw=(root/'imports/official/appearance-membership'/membership['manifest']).read_bytes()
    members={row[0] for row in json.loads(raw)['entries']}
    if not set(NEW)<=members:raise ValueError('BOSS_MOVEMENT_OFFICIAL_MEMBERSHIP')
    if not all(any(str(i).encode() in raw for path,raw in blobs.items() if path.endswith('.lua')) for i in NEW):
        raise ValueError('BOSS_MOVEMENT_NO_LITERAL_SOURCE_ITEM')
    index_path=root/'content/items/index.json';index=json.loads(index_path.read_bytes())
    manifest_path=root/'content/manifest.json';manifest=json.loads(manifest_path.read_bytes())
    count=index['record_count'];old=index['shards'][-1];doc=json.loads((root/old).read_bytes())
    if (count not in (FIRST,FIRST+2,FIRST+len(NEW),FIRST+len(NEW)+1) or manifest['families']['Item']['records']!=count
            or doc['shard']!={'count':count-34000,'end':count-1,'index':68,'start':34000}):
        raise ValueError('BOSS_MOVEMENT_REFERENCE_NUMBER_GUARD')
    already=min(count-FIRST,len(NEW))
    expected=[{'definition':value} for value in definitions()]
    if already and doc['records'][FIRST-34000:FIRST-34000+already]!=expected[:already]:
        raise ValueError('BOSS_MOVEMENT_REFERENCE_DECLARATIONS')
    if already<len(NEW):
        if check:raise ValueError('BOSS_MOVEMENT_REFERENCE_NOT_PUBLISHED')
        added=len(NEW)-already
        doc['records'] += expected[already:]
        doc['shard']['count']+=added;doc['shard']['end']+=added
        new=f'content/items/definitions/items-34000-{FIRST+len(NEW)-1}.json'
        index['record_count']+=added;index['shards'][-1]=new;manifest['families']['Item']['records']+=added
        matching=[row for row in manifest['managed_files'] if row['path']==old]
        if len(matching)!=1:raise ValueError('BOSS_MOVEMENT_REFERENCE_MANIFEST')
        matching[0]['path']=new
        (root/new).write_bytes(base.encode(doc));index_path.write_bytes(base.encode(index))
        manifest_path.write_text(json.dumps(manifest,sort_keys=True,indent=2,ensure_ascii=False)+'\n');(root/old).unlink()



def packet():
    return {'schema':'OTERYN_NATIVE_BOSS_MOVEMENT_BINDINGS/v1','source_revision':base.PIN,
        'sources':[{'path':path,'sha256':sha} for path,sha in {**base.SOURCE_HASHES,**SOURCES}.items()],
        'declarations':[{'ordinal':CAPTURE_FIRST+n,'definition':value} for n,value in enumerate(definitions())],
        'bindings':[{'source_key':'oteryn:source.crystalserver','source_revision':base.PIN,
            'identity_namespace':'ots/item_server_id','external_id':str(i),'target':value['identity'],
            'disposition':'EXACT'} for i,value in zip(NEW,definitions())]}


def capture(cache):
    blobs=load(cache);source=base.source;xml=source.xml_index(blobs['data/items/items.xml'])
    objects=source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    policies={p['server_item_id']:p for p in source.policies(blobs['data/items/appearances.dat'],blobs['data/items/items.xml'],IDS)}
    bindings=base.binding_index((ROOT/'imports/crystalserver/bindings/items.json').read_bytes())
    proof=packet()
    for binding in proof['bindings']:
        if binding['external_id'] in bindings:raise ValueError('BOSS_MOVEMENT_PROTECTED_BINDING_CONFLICT')
        bindings[binding['external_id']]=binding
    rows=[]
    for item in sorted(IDS):
        row=source.profile(item,xml.get(item),objects[item],policies[item],bindings,None)
        base.complete_source_top_order(row, objects[item])
        if row['admission']!={'materializable':True,'stack_class':'NonStackable','legal_destinations':['Ground']}:
            raise ValueError('BOSS_MOVEMENT_GROUND_CUSTODY')
        row['authoring']['item']['revision']='srx3-rotten-movement-crystal-201189f'
        row['production_binding']=copy.deepcopy(bindings[str(item)])
        row['production_binding_qualification']=copy.deepcopy(proof) if item in NEW else None
        row['production_definition']={'family':'Item','production_key':bindings[str(item)]['target']['key'],'revision_ref':'definition-r1'}
        if item in NEW:row['semantics']['presentation']={'state':'UNKNOWN'}
        rows.append(row)
    return {'schema':'OTERYN_NATIVE_BOSS_MOVEMENT_ITEMS/v1','source_revision':base.PIN,
            'sources':proof['sources'],'ids':sorted(IDS),'records':rows}


def admit(root,generated,destination,manifest_path,order,canonical):
    raw=(root/PATH).read_bytes()
    if json.loads((root/PIN_PATH).read_bytes())!={'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()}:
        raise ValueError('BOSS_MOVEMENT_CAPTURE_PIN')
    data=json.loads(raw);proof=packet()
    if (data['schema']!='OTERYN_NATIVE_BOSS_MOVEMENT_ITEMS/v1' or data['source_revision']!=base.PIN
            or data['ids']!=sorted(IDS) or len(data['records'])!=len(IDS) or data['sources']!=proof['sources']
            or json.loads((root/PACKET).read_bytes())!=proof):raise ValueError('BOSS_MOVEMENT_CAPTURE_AUTHORITY')
    document=json.loads(generated[destination]);existing={r['production_definition']['production_key']:r for r in document['records']}
    bindings=base.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    for binding in proof['bindings']:
        if binding['external_id'] in bindings:raise ValueError('BOSS_MOVEMENT_PROTECTED_BINDING_CONFLICT')
        bindings[binding['external_id']]=binding
    seen=set()
    for row in data['records']:
        key=row['production_definition']['production_key'];binding=row['production_binding'];item=int(binding['external_id'])
        if (item not in IDS or item in seen or binding!=bindings.get(str(item))
                or row['production_definition']!={'family':'Item','production_key':binding['target']['key'],'revision_ref':'definition-r1'}
                or row['production_binding_qualification']!=(proof if item in NEW else None)
                or row['admission']!={'materializable':True,'stack_class':'NonStackable','legal_destinations':['Ground']}):
            raise ValueError('BOSS_MOVEMENT_CUSTODY_CHANGED')
        seen.add(item)
        if key in existing:raise ValueError('BOSS_MOVEMENT_DUPLICATE')
        document['records'].append(row)
    document['records'].sort(key=order);generated[destination]=canonical(document)
    manifest=json.loads(generated[manifest_path]);manifest['item_profiles']['sha256']=hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path]=canonical(manifest)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source-cache',type=Path,required=True);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    publish_references(args.source_cache,ROOT,args.check)
    raw=base.encode(capture(args.source_cache));proof=base.encode(packet())
    rust=('// Generated only by tools/content-migration/boss_movement_item_profiles.py.\n'
        'pub(super) const CAPTURE_SHA256: &str =\n    "'+hashlib.sha256(raw).hexdigest()+'";\n'
        'pub(super) const PACKET_SHA256: &str =\n    "'+hashlib.sha256(proof).hexdigest()+'";\n').encode()
    for path,content in ((PATH,raw),(PIN_PATH,base.encode({'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()})),(PACKET,proof),(RUST_PIN,rust)):
        if args.check:
            if (ROOT/path).read_bytes()!=content:raise ValueError('BOSS_MOVEMENT_GENERATED_DRIFT:'+path)
        else:(ROOT/path).write_bytes(content)
    print('BOSS_MOVEMENT_SOURCE_PROFILES'+str(len(IDS)),'CHECK' if args.check else 'GENERATED')
if __name__=='__main__':main()
