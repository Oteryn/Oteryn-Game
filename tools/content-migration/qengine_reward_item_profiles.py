"""Qualify the physical Items required by shared quest exchanges.

Uses the existing native source importer and exact canonical identity bindings.
Known canonical facts win. No use effect, subtype, timer or custom script is
inferred, and the canonical Item admission records remain unchanged.
"""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

# Reused from qengine2 5a6de28c1. Tereban's fixed eight source materials use
# the same physical qualifier; XML item type never activates a USE/teleport rule.
IDS = {133, 139, 875, 3244, 9041, 10198, 10199, 11700, 11701,
       12599, 12601, 12803, 19083, 19151, 21897, 22074, 22082, 22516, 22721, 23986, 27565, 28792, 28793, 3560, 5941, 5886} | set(range(11545, 11553))
STATIC_IDS = {8995, 8996, 21573, 23163, 32173, 32537}
IDS |= STATIC_IDS | {3114, 3456, 3661, 32197, 32225, 32226, 32227, 32228, 32582, 32583, 32584, 32585, 32602, 32757}
PROOF = 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CLIENT = f'content/assets/files/appearances-{CLIENT_SHA}.dat'


def reconcile_canonical_physical(item_id, definition, record):
    # Purified Soul: retain the canonical0.20oz value; raw Canary XML says2.50oz.
    # Admit only this exact reviewed pair, never a general weight override.
    if item_id != 32228:return
    known=lambda value:{'state':'KNOWN','value':value}
    canonical=definition['semantics']['physical']['value']['weight']
    donor=record['semantics']['physical']['value']['weight']
    if canonical!=known(20) or donor!=known(250):
        raise ValueError('QENGINE_PURIFIED_SOUL_WEIGHT_DECISION_DRIFT')
    record['semantics']['physical']['value']['weight']=canonical.copy()


def reconcile_active_physical(existing, record):
    if record['production_definition']['production_key']!='oteryn:item.tibia.i32228':return
    known=lambda value:{'state':'KNOWN','value':value}
    before=existing['semantics']['physical']['value']['weight']
    after=record['semantics']['physical']['value']['weight']
    if before not in (known(250),known(20)) or after!=known(20):
        raise ValueError('QENGINE_PURIFIED_SOUL_ACTIVE_WEIGHT_DRIFT')
    existing['semantics']['physical']['value']['weight']=after.copy()


def capture(root=ROOT):
    xml_raw=(root/XML).read_bytes()
    client_raw=(root/CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=XML_SHA or hashlib.sha256(client_raw).hexdigest()!=CLIENT_SHA:
        raise ValueError('QENGINE_REWARD_ITEM_SOURCE_PIN')
    xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client_raw)
    physical={r['server_item_id']:r for r in source.policies(client_raw,xml_raw,IDS)}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    keys={f'oteryn:item.tibia.i{i}' for i in IDS}
    definitions={r['definition']['identity']['key']:r['definition']
        for path in sorted((root/'content/items/definitions').glob('items-*.json'))
        for r in json.loads(path.read_bytes())['records']
        if r['definition']['identity']['key'] in keys}
    if definitions.keys()!=keys:
        raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_SET')
    records=[]
    for item_id in sorted(IDS):
        node=xml[item_id]
        if {'charges','duration','decayto','stopduration'} & source.attrs(node).keys():
            raise ValueError('QENGINE_REWARD_ITEM_INSTANCE_OWNER_REQUIRED')
        flags=objects[item_id]['flags']
        if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
            raise ValueError('QENGINE_REWARD_ITEM_FLUID_OWNER_REQUIRED')
        record=source.profile(item_id,node,objects[item_id],physical[item_id],bindings,None)
        definition=definitions[f'oteryn:item.tibia.i{item_id}']
        reconcile_canonical_physical(item_id,definition,record)
        semantics=definition.get('semantics') if item_id in STATIC_IDS else definition['semantics']
        if item_id in STATIC_IDS and (definition.get('kind')!='Item' or definition['identity']['family']!='Item'):
            raise ValueError('QENGINE_REWARD_STATIC_IDENTITY')
        for group in ('physical','stack','container'):
            if semantics is not None and accepted.known_conflict(semantics[group],record['semantics'][group]):
                raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_CONFLICT:'+group+':'+str(item_id))
        if definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QENGINE_REWARD_ITEM_STACK_CONFLICT')
        expected_destinations=['Ground'] if item_id in STATIC_IDS else ['CharacterInventory','Ground']
        if record['admission']['legal_destinations']!=expected_destinations:
            raise ValueError('QENGINE_REWARD_ITEM_DESTINATION_DRIFT')
        if item_id in STATIC_IDS and record['admission']['stack_class']!='NonStackable':
            raise ValueError('QENGINE_REWARD_STATIC_STACK_DRIFT')
        if semantics is not None:record['semantics']=accepted.accepted_first(semantics,record['semantics'])
        records.append(record)
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,IDS)['records']
    return {'schema':'OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1',
        'producer':'tools/content-migration/qengine_reward_item_profiles.py',
        'sources':[{'path':XML,'revision':'47dfd51f45280a59a1d3e50ba7edd573d7234446','sha256':XML_SHA},
                   {'path':CLIENT,'sha256':CLIENT_SHA}],
        'scope':'Source-qualified portable quest custody and fixed Ground-only authored props; no custom use or script activation.',
        'records':records}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    document=json.loads(generated[profile_path])
    present={r['production_definition']['production_key']:r for r in document['records']}
    for record in packet['records']:
        key=record['production_definition']['production_key']
        if key in present:
            existing=present[key]
            if any(existing[field]!=record[field] for field in ('production_definition','production_binding','admission')):
                raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:'+key)
            reconcile_active_physical(existing,record)
            if any(accepted.known_conflict(existing['semantics'][field],record['semantics'][field]) for field in ('physical','stack','container')):
                raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:'+key)
            # Only the source-qualified custody groups are completed here.
            # Existing use, charge, timer and equipment owners retain authority.
            for field in ('physical','stack','container'):
                existing['semantics'][field]=accepted.accepted_first(
                    existing['semantics'][field],record['semantics'][field])
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    raw=(json.dumps(document,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n').encode()
    generated[profile_path]=raw
    manifest=json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256']=hashlib.sha256(raw).hexdigest()
    # Ordinary native regeneration must retain the promoted Quest cosmetics
    # section instead of falling back to a manifest with no appearance owner.
    cosmetics='cosmetics/outfits/native-cosmetics.json'
    cosmetics_raw=generated.get('content/'+cosmetics)
    if cosmetics_raw is None and (root/'content'/cosmetics).exists():
        cosmetics_raw=(root/'content'/cosmetics).read_bytes()
    if cosmetics_raw is not None:
        if manifest['schema'] in {f'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v{i}' for i in range(1,6)}:
            manifest['schema']='OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6'
        manifest['cosmetics']={'path':cosmetics,
            'sha256':hashlib.sha256(cosmetics_raw).hexdigest()}
    generated[manifest_path]=canonical(manifest)
