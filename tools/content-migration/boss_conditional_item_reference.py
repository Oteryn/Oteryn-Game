"""One real source Item reference; no name, use, type, loot or container inferred."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path

KEY='oteryn:item.tibia.i43860'
CAPTURE_ORDINAL=34111
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/"tools/content-schema/native-gameplay"))
from event_item_identity import canonical_ordinal
ORDINAL=canonical_ordinal(Path(__file__).resolve().parents[2],CAPTURE_ORDINAL)
PATH='tools/content-schema/native-gameplay/bakragore-conditional-item-reference.json'
PIN_PATH='tools/content-schema/native-gameplay/bakragore-conditional-item-reference-pin.json'
OFFICIAL_SHA='2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CALLBACK='data-global/scripts/quests/rotten_blood_quest/bakragore_mechanics_death.lua'
CALLBACK_SHA='21ba33eb58d1b5a4a88ca0c950d49a8369b34f3a11fe39a7e549a8b3ab154b46'
SOURCE='201189f871d23597b82caf88fd611454a2ddefb8'
FIELDS=('charges','classification','container','equipment','fluid','imbuement','physical',
        'presentation','protection','readable_writeable','skill_modifiers','stack','temporal',
        'trade_restrictions','use_transform','weapon')


def encode(value):
    return (json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False)+'\n').encode()


def definition(key=KEY):
    return {'identity':{'family':'Item','key':key,'revision':'definition-r1'},
            'kind':'Item','client_projection':'ClientSafe','materializable':False,
            'stack_class':'Unknown','semantics':{key:{'state':'UNKNOWN'} for key in FIELDS}}


def evidence(cache,root):
    import boss_corpse_profiles as corpses
    blobs=corpses.load_source_blobs(cache)
    callback=(cache/'bakragore_mechanics_death.lua').read_bytes()
    if hashlib.sha256(callback).hexdigest()!=CALLBACK_SHA or b'{ 43895, 43860 }' not in callback:
        raise ValueError('CONDITIONAL_REFERENCE_SOURCE_CALLBACK')
    from build_source_item_profiles import engine_items,policies
    donor=engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    if 43860 not in donor:raise ValueError('CONDITIONAL_REFERENCE_NO_DONOR_ITEM')
    policy,=policies(blobs['data/items/appearances.dat'],blobs['data/items/items.xml'],{43860})
    if (not policy['pickupable'] or not policy['movable'] or policy['ground']
            or policy['container'] or policy['dynamic_kind'] is not None):
        raise ValueError('CONDITIONAL_REFERENCE_DONOR_CUSTODY')
    official=(root/f'content/assets/files/appearances-{OFFICIAL_SHA}.dat').read_bytes()
    if hashlib.sha256(official).hexdigest()!=OFFICIAL_SHA:
        raise ValueError('CONDITIONAL_REFERENCE_OFFICIAL_APPEARANCES')
    if 43860 not in engine_items.load_appearance_objects(official):
        raise ValueError('CONDITIONAL_REFERENCE_OFFICIAL_ITEM_MISSING')
    admitted=json.loads((root/'imports/official/appearance-membership/admitted.json').read_bytes())
    membership=next(row for row in admitted['files'] if row['label']=='client-15.30')
    raw=(root/'imports/official/appearance-membership'/membership['manifest']).read_bytes()
    if (hashlib.sha256(raw).hexdigest()!=membership['manifest_sha256']
            or membership['appearances_sha256']!=OFFICIAL_SHA
            or 43860 not in {row[0] for row in json.loads(raw)['entries']}):
        raise ValueError('CONDITIONAL_REFERENCE_OFFICIAL_MEMBERSHIP')
    return {'schema':'OTERYN_BAKRAGORE_CONDITIONAL_ITEM_REFERENCE/v1','ordinal':CAPTURE_ORDINAL,
            'definition':definition(),'source_revision':SOURCE,
            'callback':{'path':CALLBACK,'sha256':CALLBACK_SHA,'lines':[21,43]},
            'source_files':[{'path':path,'sha256':hashlib.sha256(blob).hexdigest()} for path,blob in blobs.items()],
            'official_appearances_sha256':OFFICIAL_SHA,
            'official_membership':{'manifest':membership['manifest'],'sha256':membership['manifest_sha256']},
            'scope':'Reference only. UNKNOWN name/type/use/loot. No invented bag container. Native custody requires its independent exact source profile.'}


def extend_records(records,root):
    raw=(root/PATH).read_bytes()
    if json.loads((root/PIN_PATH).read_bytes())!={'path':PATH,'sha256':hashlib.sha256(raw).hexdigest()}:
        raise ValueError('CONDITIONAL_REFERENCE_CAPTURE_PIN')
    packet=json.loads(raw)
    if (packet['schema']!='OTERYN_BAKRAGORE_CONDITIONAL_ITEM_REFERENCE/v1'
            or packet['ordinal']!=CAPTURE_ORDINAL or packet['definition']!=definition()
            or packet['source_revision']!=SOURCE
            or packet['callback']!={'path':CALLBACK,'sha256':CALLBACK_SHA,'lines':[21,43]}):
        raise ValueError('CONDITIONAL_REFERENCE_CAPTURE_AUTHORITY')
    matching=[i for i,row in enumerate(records) if row['identity']['key']==KEY]
    if matching:
        if matching!=[ORDINAL] or len(records)<ORDINAL+1 or records[ORDINAL]!=definition():
            raise ValueError('CONDITIONAL_REFERENCE_EXISTING_IDENTITY_CHANGED')
    else:
        if len(records)!=ORDINAL:raise ValueError('CONDITIONAL_REFERENCE_NEXT_FREE_ORDINAL_CHANGED')
        records.append(definition())


def publish(cache,root,check=False):
    proof=encode(evidence(cache,root)); pin=encode({'path':PATH,'sha256':hashlib.sha256(proof).hexdigest()})
    for path,raw in ((PATH,proof),(PIN_PATH,pin)):
        if check:
            if (root/path).read_bytes()!=raw:raise ValueError('CONDITIONAL_REFERENCE_SOURCE_DRIFT')
        else:(root/path).write_bytes(raw)
    index_path=root/'content/items/index.json'; index=json.loads(index_path.read_bytes())
    manifest_path=root/'content/manifest.json';manifest=json.loads(manifest_path.read_bytes())
    count=index['record_count']
    if count not in (ORDINAL,ORDINAL+1,ORDINAL+3,ORDINAL+6,ORDINAL+7) or manifest['families']['Item']['records']!=count:
        raise ValueError('CONDITIONAL_REFERENCE_NUMBER_GUARD')
    old=index['shards'][-1];doc=json.loads((root/old).read_bytes())
    if doc['shard']!={'count':count-34000,'end':count-1,'index':68,'start':34000}:
        raise ValueError('CONDITIONAL_REFERENCE_LAST_SHARD_GUARD')
    if count==ORDINAL:
        if check:raise ValueError('CONDITIONAL_REFERENCE_NOT_REGISTERED')
        if any(row['definition']['identity']['key']==KEY for row in doc['records']):
            raise ValueError('CONDITIONAL_REFERENCE_DUPLICATE')
        doc['records'].append({'definition':definition()})
        doc['shard']['count']+=1;doc['shard']['end']+=1
        new=f'content/items/definitions/items-34000-{ORDINAL:05d}.json'
        index['shards'][-1]=new;index['record_count']+=1
        manifest['families']['Item']['records']+=1
        matches=[row for row in manifest['managed_files'] if row['path']==old]
        if len(matches)!=1:raise ValueError('CONDITIONAL_REFERENCE_MANIFEST_GUARD')
        matches[0]['path']=new
        (root/new).write_bytes(encode(doc));index_path.write_bytes(encode(index))
        manifest_path.write_text(json.dumps(manifest,sort_keys=True,indent=2,ensure_ascii=False)+'\n')
        (root/old).unlink()
    elif doc['records'][ORDINAL-34000]!={'definition':definition()}:
        raise ValueError('CONDITIONAL_REFERENCE_DECLARATION_CHANGED')
