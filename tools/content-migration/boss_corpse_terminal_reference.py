"""Declare the actual source43666 terminal body; preserve all unknown Item semantics."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
import boss_conditional_item_reference as reference

KEY='oteryn:item.tibia.i43666'
ORDINAL=25377
LEGACY_APPEND_ORDINAL=34117
REFERENCE_COUNT=34117
PATH='tools/content-schema/native-gameplay/elder-corpse-terminal-reference.json'
PIN_PATH='tools/content-schema/native-gameplay/elder-corpse-terminal-reference-pin.json'
PACKET='tools/content-schema/native-gameplay/candidate-bindings/elder-corpse-terminal-candidate.json'
RUST_PIN='crates/gameplay/content/src/content/native_gameplay/boss_corpse_terminal_pin.rs'
EXTRA_SOURCES={
 'src/items/item.cpp':'e27e622bf4fd9a94d7fffb3a818fd0c5f911e6fbf4b92844320154ded326950b',
 'data-global/monster/quests/rotten_blood_quest/elder_bloodjaw.lua':'483c813d4e44181a72325e2468307b3d8cd36611276703cf1f0e8668e3eed1c6',
}


def definition():
    # This exact reference-only body already exists at25377 on task base b2cfa798.
    # No semantics field is added: its original UNKNOWN projection remains exact.
    return {'identity':{'family':'Item','key':KEY,'revision':'definition-r1'},
            'kind':'Item','client_projection':'ClientSafe','materializable':False,'stack_class':'Unknown'}


def _retain_existing(records):
    matches=[i for i,row in enumerate(records) if row['identity']['key']==KEY]
    if matches==[ORDINAL,LEGACY_APPEND_ORDINAL]:
        if len(records)!=LEGACY_APPEND_ORDINAL+1 or records[-1]!=reference.definition(KEY):
            raise ValueError('ELDER_TERMINAL_OBSOLETE_APPEND_CHANGED')
        if records[ORDINAL]!=definition():raise ValueError('ELDER_TERMINAL_EXISTING_REFERENCE_CHANGED')
        records.pop() # Only this publisher's exact erroneous final append is removed.
        matches=[ORDINAL]
    if matches!=[ORDINAL] or records[ORDINAL]!=definition():
        raise ValueError('ELDER_TERMINAL_EXISTING_REFERENCE_CHANGED')


def evidence(cache,root):
    import boss_corpse_profiles as base
    blobs=base.load_source_blobs(cache)
    for path,expected in EXTRA_SOURCES.items():
        raw=(cache/Path(path).name).read_bytes()
        if hashlib.sha256(raw).hexdigest()!=expected:raise ValueError('ELDER_TERMINAL_SOURCE:'+path)
        blobs[path]=raw
    if (b'uint32_t decayTime = 0;' not in blobs['src/items/items.hpp']
            or b'int32_t decayTo = -1;' not in blobs['src/items/items.hpp']
            or b'it.decayTo < 0 || it.decayTime == 0' not in blobs['src/items/item.cpp']
            or b'monster.corpse = 43669' not in blobs['data-global/monster/quests/rotten_blood_quest/elder_bloodjaw.lua']):
        raise ValueError('ELDER_TERMINAL_DEFAULT_DECAY_SOURCE')
    xml=base.source.xml_index(blobs['data/items/items.xml'])
    for item,target,duration in [(43669,43668,10),(43668,43667,300),(43667,43666,300)]:
        attrs=base.source.attrs(xml[item])
        if attrs!={'containersize':'30','decayto':str(target),'duration':str(duration),'fluidsource':'blood'}:
            raise ValueError('ELDER_TERMINAL_XML_CHAIN')
    if 43666 in xml:raise ValueError('ELDER_TERMINAL_UNEXPECTED_XML_OVERRIDE')
    policy,=base.source.policies(blobs['data/items/appearances.dat'],blobs['data/items/items.xml'],{43666})
    if (not policy['movable'] or policy['pickupable'] or policy['container'] or policy['ground']
            or policy['dynamic_kind'] is not None or policy['magic_field']
            or policy['block_solid'] or policy['block_projectile'] or policy['block_pathfind']):
        raise ValueError('ELDER_TERMINAL_DAT_PHYSICS')
    official=(root/f'content/assets/files/appearances-{reference.OFFICIAL_SHA}.dat').read_bytes()
    if (hashlib.sha256(official).hexdigest()!=reference.OFFICIAL_SHA
            or 43666 not in base.source.engine_items.load_appearance_objects(official)):
        raise ValueError('ELDER_TERMINAL_OFFICIAL_APPEARANCE')
    admitted=json.loads((root/'imports/official/appearance-membership/admitted.json').read_bytes())
    membership=next(row for row in admitted['files'] if row['label']=='client-15.30')
    raw=(root/'imports/official/appearance-membership'/membership['manifest']).read_bytes()
    if (hashlib.sha256(raw).hexdigest()!=membership['manifest_sha256']
            or membership['appearances_sha256']!=reference.OFFICIAL_SHA
            or 43666 not in {row[0] for row in json.loads(raw)['entries']}):
        raise ValueError('ELDER_TERMINAL_OFFICIAL_MEMBERSHIP')
    return {'schema':'OTERYN_ELDER_CORPSE_TERMINAL_REFERENCE/v1','ordinal':ORDINAL,
            'definition':definition(),'reference_disposition':'EXISTING_CANONICAL_REFERENCE','source_revision':base.PIN,
            'sources':[{'path':path,'sha256':hashlib.sha256(raw).hexdigest()} for path,raw in blobs.items()],
            'source_chain':[43669,43668,43667,43666],'source_durations_seconds':[10,300,300,0],
            'terminal_zero_means':'No automatic decay: ItemType defaults and Item::canDecay. InstanceRetire remains authoritative.',
            'official_appearances_sha256':reference.OFFICIAL_SHA,
            'official_membership':{'manifest':membership['manifest'],'sha256':membership['manifest_sha256']}}


def packet(proof):
    return {'schema':'OTERYN_NATIVE_ELDER_CORPSE_TERMINAL_BINDINGS/v1',
            'source_revision':proof['source_revision'],'evidence':proof,
            'bindings':[{'source_key':'oteryn:source.crystalserver','source_revision':proof['source_revision'],
                'identity_namespace':'ots/item_server_id','external_id':'43666','target':definition()['identity'],
                'disposition':'EXACT'}]}


def checked_packet(root):
    raw=(root/PATH).read_bytes()
    if json.loads((root/PIN_PATH).read_bytes())!={'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()}:
        raise ValueError('ELDER_TERMINAL_REFERENCE_PIN')
    proof=json.loads(raw)
    if (proof['ordinal']!=ORDINAL or proof['definition']!=definition()
            or proof.get('reference_disposition')!='EXISTING_CANONICAL_REFERENCE'
            or proof['schema']!='OTERYN_ELDER_CORPSE_TERMINAL_REFERENCE/v1'
            or proof['source_revision']!=reference.SOURCE
            or proof['source_chain']!=[43669,43668,43667,43666]
            or proof['source_durations_seconds']!=[10,300,300,0]):
        raise ValueError('ELDER_TERMINAL_REFERENCE_AUTHORITY')
    import boss_corpse_profiles as base
    if proof['sources'] != [{'path':path,'sha256':sha} for path,sha in {**base.SOURCE_HASHES, **EXTRA_SOURCES}.items()]:
        raise ValueError('ELDER_TERMINAL_SOURCE_SET')
    expected=packet(proof)
    if json.loads((root/PACKET).read_bytes())!=expected:raise ValueError('ELDER_TERMINAL_BINDING_AUTHORITY')
    return expected


def extend_records(records,root):
    checked_packet(root)
    _retain_existing(records)


def publish(cache,root,check=False):
    import boss_corpse_profiles as base
    proof=evidence(cache,root);raw=base.encode(proof);bindings=base.encode(packet(proof))
    for path,content in [(PATH,raw),(PIN_PATH,base.encode({'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()})),(PACKET,bindings)]:
        if check:
            if (root/path).read_bytes()!=content:raise ValueError('ELDER_TERMINAL_GENERATED_DRIFT:'+path)
        else:(root/path).write_bytes(content)
    index_path=root/'content/items/index.json';index=json.loads(index_path.read_bytes())
    manifest_path=root/'content/manifest.json';manifest=json.loads(manifest_path.read_bytes())
    count=index['record_count'];old=index['shards'][-1];doc=json.loads((root/old).read_bytes())
    if (count not in (REFERENCE_COUNT,REFERENCE_COUNT+1) or manifest['families']['Item']['records']!=count
            or doc['shard']!={'count':count-34000,'end':count-1,'index':68,'start':34000}):
        raise ValueError('ELDER_TERMINAL_REFERENCE_NUMBER_GUARD')
    records=[row['definition'] for path in index['shards']
             for row in json.loads((root/path).read_bytes())['records']]
    if len(records)!=count:raise ValueError('ELDER_TERMINAL_REFERENCE_REGISTRY_COUNT')
    _retain_existing(records)
    if count==REFERENCE_COUNT+1:
        if check:raise ValueError('ELDER_TERMINAL_OBSOLETE_APPEND_STILL_REGISTERED')
        if doc['records'][-1]!={'definition':reference.definition(KEY)}:
            raise ValueError('ELDER_TERMINAL_OBSOLETE_APPEND_CHANGED')
        doc['records'].pop();doc['shard']['count']-=1;doc['shard']['end']-=1
        new=f'content/items/definitions/items-34000-{REFERENCE_COUNT-1}.json'
        index['record_count']-=1;index['shards'][-1]=new;manifest['families']['Item']['records']-=1
        matching=[row for row in manifest['managed_files'] if row['path']==old]
        if len(matching)!=1:raise ValueError('ELDER_TERMINAL_REFERENCE_MANIFEST')
        matching[0]['path']=new
        (root/new).write_bytes(base.encode(doc));index_path.write_bytes(base.encode(index))
        manifest_path.write_text(json.dumps(manifest,sort_keys=True,indent=2,ensure_ascii=False)+'\n');(root/old).unlink()
