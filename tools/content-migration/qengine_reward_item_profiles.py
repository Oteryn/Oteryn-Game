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
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import questl2_rewards

IDS = {133, 139, 875, 3244, 9041, 10198, 10199, 11700, 11701} | questl2_rewards.ITEMS
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
    admitted={r['production_definition']['production_key']:r
              for r in json.loads((root/PROFILE).read_bytes())['records']}
    records=[]
    for item_id in sorted(IDS):
        node=xml[item_id]
        timed = {key: value for key, value in source.attrs(node).items()
                 if key in {'charges', 'duration', 'decayto', 'stopduration'}}
        # This source quest item uses the existing absolute-deadline Item owner.
        # Preserve its duration in source.profile; the reward owner still refuses
        # persisted timed states until that composition is separately qualified.
        if timed and not (item_id == 4829 and timed == {'duration': '86400'}):
            raise ValueError('QENGINE_REWARD_ITEM_INSTANCE_OWNER_REQUIRED')
        flags=objects[item_id]['flags']
        if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
            raise ValueError('QENGINE_REWARD_ITEM_FLUID_OWNER_REQUIRED')
        record=source.profile(item_id,node,objects[item_id],physical[item_id],bindings,None)
        definition=definitions[f'oteryn:item.tibia.i{item_id}']
        # Accepted canonical weights outrank donor hypotheses. These two exact
        # conflicts are pinned and checked, rather than admitting arbitrary drift.
        weights = {23677: (2700, 200), 34210: (6660, 6659)}
        if item_id in weights:
            canonical_weight, source_weight = weights[item_id]
            accepted_weight = definition['semantics']['physical']['value']['weight']
            donor_weight = record['semantics']['physical']['value']['weight']
            if accepted_weight != {'state': 'KNOWN', 'value': canonical_weight} or donor_weight != {'state': 'KNOWN', 'value': source_weight}:
                raise ValueError('QENGINE_REWARD_ITEM_WEIGHT_PRECEDENCE_DRIFT')
            record['semantics']['physical']['value']['weight'] = accepted_weight
        for group in ('physical','stack','container'):
            if accepted.known_conflict(definition['semantics'][group],record['semantics'][group]):
                raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_CONFLICT:'+group)
        if definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QENGINE_REWARD_ITEM_STACK_CONFLICT')
        if record['admission']['legal_destinations']!=['CharacterInventory','Ground']:
            raise ValueError('QENGINE_REWARD_ITEM_NOT_PORTABLE')
        record['semantics']=accepted.accepted_first(definition['semantics'],record['semantics'])
        prior=admitted.get(f'oteryn:item.tibia.i{item_id}')
        if prior is not None:
            # The native shell carries accepted readability facts absent from
            # this XML importer. Physical and equipment facts still come from
            # their canonical definitions and source qualification above.
            record['semantics']['readable_writeable']=accepted.accepted_first(
                prior['semantics']['readable_writeable'],record['semantics']['readable_writeable'])
        records.append(record)
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,IDS)['records']
    return {'schema':'OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1',
        'producer':'tools/content-migration/qengine_reward_item_profiles.py',
        'sources':[{'path':XML,'revision':'47dfd51f45280a59a1d3e50ba7edd573d7234446','sha256':XML_SHA},
                   {'path':CLIENT,'sha256':CLIENT_SHA}],
        'scope':'Physical custody for the shared quest exchanges; no custom use or script activation.',
        'records':records}


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    document=json.loads(generated[profile_path])
    present={r['production_definition']['production_key']:r for r in document['records']}
    for record in packet['records']:
        key=record['production_definition']['production_key']
        if key in present:
            prior = present[key]
            if prior == record:
                continue
            # A source-qualified recipe can promote an inert canonical shell;
            # never replace a richer provider or contradict its known facts.
            inert_shell = prior['admission'] == {
                'materializable': False, 'stack_class': record['admission']['stack_class'],
                'legal_destinations': ['Ground']}
            portable_shell = (prior['admission'] == record['admission']
                and prior['attributes'] == {'speed_bonus': 0}
                and prior['authoring'] == {'item': {'family': 'Item', 'key': key,
                    'revision': record['production_definition']['revision_ref']}})
            if (not (inert_shell or portable_shell)
                    or prior['production_definition'] != record['production_definition']
                    or accepted.accepted_first(prior['semantics'], record['semantics'])
                       != record['semantics']):
                raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:'+key)
            document['records'][document['records'].index(prior)] = record
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
