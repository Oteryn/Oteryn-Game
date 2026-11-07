"""Qualify the physical Items required by shared quest exchanges.

Uses the existing native source importer and exact canonical identity bindings.
Known canonical facts win. No use effect, subtype, timer or custom script is
inferred, and the canonical Item admission records remain unchanged.
"""
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

CORPSE_IDS = {18150, 18151, 18152, 18153}
IDS = {133, 139, 875, 3244, 9041, 10198, 10199, 11700, 11701, 13429} | CORPSE_IDS
PROOF = 'tools/content-schema/native-gameplay/qengine-reward-item-profiles.json'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CLIENT = f'content/assets/files/appearances-{CLIENT_SHA}.dat'


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
        if item_id not in CORPSE_IDS and {'charges','duration','decayto','stopduration'} & source.attrs(node).keys():
            raise ValueError('QENGINE_REWARD_ITEM_INSTANCE_OWNER_REQUIRED')
        flags=objects[item_id]['flags']
        if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
            raise ValueError('QENGINE_REWARD_ITEM_FLUID_OWNER_REQUIRED')
        record=source.profile(item_id,node,objects[item_id],physical[item_id],bindings,None)
        definition=definitions[f'oteryn:item.tibia.i{item_id}']
        # Owner WEVENTS: retained Crystal items.xml:31463-31466 agrees
        # with pinned Canary on weight 500. Historical wiki metadata says 5000.
        # Correct only the active native profile; keep the historical tree intact.
        if item_id == 13429:
            definition = copy.deepcopy(definition)
            weight = definition['semantics']['physical']['value']['weight']
            if weight not in (source.K(5000), source.K(500)):
                raise ValueError('WEVENTS_BAG_WEIGHT_SOURCE_CONFLICT')
            if record['semantics']['physical']['value']['weight'] != source.K(500):
                raise ValueError('WEVENTS_BAG_DONOR_WEIGHT_DRIFT')
            definition['semantics']['physical']['value']['weight'] = source.K(500)
        for group in ('physical','stack','container'):
            if accepted.known_conflict(definition.get('semantics', {}).get(group, {'state':'UNKNOWN'}),record['semantics'][group]):
                raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_CONFLICT:'+group)
        if definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QENGINE_REWARD_ITEM_STACK_CONFLICT')
        if item_id not in CORPSE_IDS and record['admission']['legal_destinations']!=['CharacterInventory','Ground']:
            raise ValueError('QENGINE_REWARD_ITEM_NOT_PORTABLE')
        record['semantics']=accepted.accepted_first(definition.get('semantics', {}),record['semantics'])
        records.append(record)
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,IDS)['records']
    return {'schema':'OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1',
        'producer':'tools/content-migration/qengine_reward_item_profiles.py',
        'sources':[{'path':XML,'revision':'47dfd51f45280a59a1d3e50ba7edd573d7234446','sha256':XML_SHA},
                   {'path':CLIENT,'sha256':CLIENT_SHA}],
        'scope':'Physical custody for the shared quest exchanges; no custom use or script activation.',
        'native_source_overrides':[{'item':'oteryn:item.tibia.i13429',
            'field':'physical.weight','historical_canonical':5000,'native':500,
            'source':'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data/items/items.xml:31463-31466',
            'source_sha256':'c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb',
            'decision':'docs/reference/WPKG-CONTRACT-NOTES.md#bank-robbery-implementation-policy'}],
        'records':records}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    document=json.loads(generated[profile_path])
    present={r['production_definition']['production_key']:r for r in document['records']}
    for record in packet['records']:
        key=record['production_definition']['production_key']
        if key in present:
            if present[key]!=record:
                raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:'+key)
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
    if cosmetics_raw is None:
        cosmetics_raw=(root/'content'/cosmetics).read_bytes()
    manifest['schema']='OTERYN_NATIVE_GAMEPLAY_MANIFEST/v6'
    manifest['cosmetics']={'path':cosmetics,
        'sha256':hashlib.sha256(cosmetics_raw).hexdigest()}
    generated[manifest_path]=canonical(manifest)
