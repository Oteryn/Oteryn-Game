"""Qualify the physical Items required by shared quest exchanges.

Uses the existing native source importer and exact canonical identity bindings.
Known canonical facts win. No use effect, subtype or custom script is
inferred; the selected corpse chain retains its exact source timers, and the canonical Item admission records remain unchanged.
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
sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
import qmech_m39
import m07_lion_flower_profile as lion_flower
import questl2_ready

IDS = {21897, 21898, 21899, 21901, 21902, 21905, 21906, 30168, 30169, 37002, 37003, 768, 769, 770, 3723, 15698, 16103, 16165, 16167, 16242, 12517, 12413, 16128, 16129, 20275, 20276, 5903, 5947, 133, 139, 875, 3244, 9041, 10198, 10199, 11700, 11701,
       4867, 5886, 5924, 5937, 5938, 5939, 5940, 5921, 6107, 5945, 7289, 6099, 6100, 6101, 6102, 6126, 12599, 12601, 12803, 22516, 22721, 28792, 28793}
# M51 arena trophy custody is qualified from the same pinned physical sources.
CUSTODY_PROMOTIONS = {5805, 5806}
# M12 selected daily harvest: existing Juicy Roots identity, exact portable facts.
IDS |= {21291}
# M13: source-pinned carried Guardcatcher and each Perfume Gatherer stage.
IDS |= {17669, 17670, 17672, 17673, 17674, 17675, 7253}
# M13: native Slug corpse closure; Ground-only source durations remain authoritative.
CORPSE_IDS = {12526, 12533, 12534, 12535}
IDS |= CORPSE_IDS
# Retain questpc4's qualified native offerings and source tool through this owner.
IDS |= {3537, 3543, 3587, 3589, 3590, 3593, 11544}
# M36: exact Rock Steady collection hand-ins from the pinned NPC source.
IDS |= {14021, 14022, 14023}
# Vascalir's source-authored mission 11 grants this ordinary flask with 3054.
IDS |= {12785}
# QN4 Cursed Crystal actions_MedusaOil.lua:2-4 uses these plain carried ingredients.
IDS |= {21504, 21505, 21506, 21507, 9106}
# M56: exact carried Molten Wax consumption through the existing quest input owner.
IDS |= {9106}
# QN3 Gnomus measurement report (Crystal 201189f gnomus.lua:88-97) grants this jar.
IDS |= {27654}
IDS |= qmech_m39.ITEMS
IDS |= {21504, 21505}
# Lion trials: exact holy-water vessel plus separately qualified one-hour flower duration.
IDS |= {21466, 21389}
# M07 retained Realm key and source bricklayers kit for the assigned repair stages.
IDS |= {14682, 7785}
# M07 deterministic siege burning uses the existing Fire Bug as a retained tool.
IDS |= {5467}
# M07 Too Hot cleaning, column and floor-repair tools from the same pinned source.
IDS |= {36543, 36544, 36545}
# ENGNPC2: the registered Goblin Merchant Slug and Rotworm exchanges consume
# a gatherer and mint its next variant. Only physical custody is qualified
# here; the exact corpse/live-target callbacks remain the use authority.
IDS |= {17670, 17672, 17673, 17674}
# Rafzan supplies this plain quest bow; capture policy remains a separate owner.
IDS |= {17669}
# Siflind grants one plain paint vial; the World owner controls its use.
IDS |= {7253}
# M22: Star Herb and Antlers already have native owners; qualify only the missing Talisman.
IDS |= {22008}
# M01c Fanfare: copper key is numbered by the existing key MINT owner.
IDS |= {2973}
# M01c Grimvale: harvested blossoms, distinct from either planted nightshade.
IDS |= {22082}
# M44: source flower harvest and the two remaining native pagoda offerings.
IDS |= {21389, 21466}
IDS |= questl2_ready.ITEMS
# Already present in main f5b584ef1's native provider; the existing cohort
# cardinality includes these six shells, even when a source fact is refined.
PREVIOUSLY_COUNTED_IDS = {3029, 3035, 3205, 3212, 3234, 3607}
# Reused from qengine2 5a6de28c1. Tereban's fixed eight source materials use
# the same physical qualifier; XML item type never activates a USE/teleport rule.
SIDE_IDS = {133, 139, 875, 3244, 9041, 10198, 10199, 11700, 11701,
       12599, 12601, 12803, 19083, 19151, 21897, 22074, 22082, 22516, 22721, 23986, 27565, 28792, 28793, 3560, 5941, 5886} | set(range(11545, 11553)) | {768, 769, 770, 16128, 16129, 133, 4867, 12803, 37002, 37003, 3723, 139, 5903, 5921, 5924, 16165, 16167, 3244, 5937, 5938, 20275, 20276, 11700, 11701, 5939, 5940, 5945, 12599, 5947, 12601, 22721, 9041, 15698, 6099, 6100, 6101, 10198, 10199, 30168, 30169, 6102, 6107, 28793, 12517, 16103, 875, 6126, 16242, 22516, 28792, 7289, 12413, 5886}
SIDE_IDS |= {17668, 17669, 17670, 17671, 17672, 17673, 17674, 17675, 17676, 3120}  # Rafzan's five source-priced portable tools.
STATIC_IDS = {8995, 8996, 9527, 17677, 17679, 17681, 21573, 23163, 32173, 32537}
SIDE_IDS |= STATIC_IDS | {3114, 3456, 3661, 32197, 32225, 32226, 32227, 32228, 32582, 32583, 32584, 32585, 32602, 32757}
SIDE_IDS |= {3537, 3543, 3587, 3589, 3590, 3593, 11544}

IDS |= SIDE_IDS
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


def capture(root=ROOT, ids=None, *, item_ids=None):
    if item_ids is not None:
        if ids is not None:
            raise ValueError("choose ids or item_ids")
        ids = item_ids
    selected_ids = IDS if ids is None else frozenset(ids)
    selected = selected_ids | CUSTODY_PROMOTIONS
    xml_raw=(root/XML).read_bytes()
    client_raw=(root/CLIENT).read_bytes()
    if hashlib.sha256(xml_raw).hexdigest()!=XML_SHA or hashlib.sha256(client_raw).hexdigest()!=CLIENT_SHA:
        raise ValueError('QENGINE_REWARD_ITEM_SOURCE_PIN')
    xml=source.xml_index(xml_raw)
    objects=source.engine_items.load_appearance_objects(client_raw)
    physical={r['server_item_id']:r for r in source.policies(client_raw,xml_raw,selected)}
    bindings=source.binding_index((root/'imports/crystalserver/bindings/items.json').read_bytes())
    keys={f'oteryn:item.tibia.i{i}' for i in selected}
    definitions={r['definition']['identity']['key']:r['definition']
        for path in sorted((root/'content/items/definitions').glob('items-*.json'))
        for r in json.loads(path.read_bytes())['records']
        if r['definition']['identity']['key'] in keys}
    if definitions.keys()!=keys:
        raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_SET')
    admitted={r['production_definition']['production_key']:r for r in
        json.loads((root/PROFILE).read_bytes())['records']}
    records=[]
    for item_id in sorted(selected):
        node=xml[item_id]
        flower = lion_flower.admitted(item_id, source.attrs(node))
        if not flower and item_id not in CORPSE_IDS and {'charges','duration','decayto','stopduration'} & source.attrs(node).keys():
            raise ValueError('QENGINE_REWARD_ITEM_INSTANCE_OWNER_REQUIRED')
        flags=objects[item_id]['flags']
        if flags.get('flags.liquidcontainer') or flags.get('flags.liquidpool'):
            raise ValueError('QENGINE_REWARD_ITEM_FLUID_OWNER_REQUIRED')
        record=source.profile(item_id,node,objects[item_id],physical[item_id],bindings,None)
        definition=definitions[f'oteryn:item.tibia.i{item_id}']
        reconcile_canonical_physical(item_id,definition,record)
        if flower:
            record=lion_flower.temporal(record, definition, accepted.known_conflict)
        # These exact Ground-only props and corpse stages have canonical
        # identity-only rows; their physical facts come from the pinned sources.
        semantics=definition.get('semantics') if item_id in (STATIC_IDS | CORPSE_IDS) else definition['semantics']
        if item_id in (STATIC_IDS | CORPSE_IDS) and (definition.get('kind')!='Item' or definition['identity']['family']!='Item'):
            raise ValueError('QENGINE_REWARD_STATIC_IDENTITY')
        for group in ('physical','stack','container','temporal'):
            if semantics is not None and accepted.known_conflict(semantics[group],record['semantics'][group]):
                raise ValueError('QENGINE_REWARD_ITEM_CANONICAL_CONFLICT:'+group+':'+str(item_id))
        if definition['stack_class'] not in ('Unknown',record['admission']['stack_class']):
            raise ValueError('QENGINE_REWARD_ITEM_STACK_CONFLICT')
        expected_destinations=['Ground'] if item_id in (STATIC_IDS | CORPSE_IDS) else ['CharacterInventory','Ground']
        if record['admission']['legal_destinations']!=expected_destinations:
            raise ValueError('QENGINE_REWARD_ITEM_DESTINATION_DRIFT')
        if item_id in STATIC_IDS and record['admission']['stack_class']!='NonStackable':
            raise ValueError('QENGINE_REWARD_STATIC_STACK_DRIFT')
        if semantics is not None:record['semantics']=accepted.accepted_first(semantics,record['semantics'])
        prior=admitted.get(f'oteryn:item.tibia.i{item_id}')
        if prior is not None:
            refined=accepted.accepted_first(prior['semantics'],record['semantics'])
            record['semantics']=accepted.accepted_first(semantics,refined) if semantics is not None else refined
            record['attributes']=accepted.accepted_first(prior['attributes'],record['attributes'])
            record['authoring']=copy.deepcopy(prior['authoring'])
            # The proof and native admission share the same custody boundary:
            # physical qualification cannot invent an owner's unknown mechanic.
            import questpc7_physical_custody_join
            record=questpc7_physical_custody_join.observation(prior,record,accepted.accepted_first)
        records.append(record)
    records=source.bind_profiles({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':records},bindings,selected)['records']
    return {'schema':'OTERYN_QENGINE_REWARD_ITEM_PROFILES/v1',
        'producer':'tools/content-migration/qengine_reward_item_profiles.py',
        'sources':[{'path':XML,'revision':'47dfd51f45280a59a1d3e50ba7edd573d7234446','sha256':XML_SHA},
                   {'path':CLIENT,'sha256':CLIENT_SHA}],
        'scope':'Source-qualified portable quest custody, fixed Ground-only props and exact source corpse timers; no custom use or script activation.',
        'records':[r for r in records if int(r['production_binding']['external_id']) in selected_ids],
        'custody_promotions':[r for r in records if int(r['production_binding']['external_id']) in CUSTODY_PROMOTIONS]}


def reconcile(existing, observed):
    """Join pinned physical facts without replacing an established owner's identity."""
    # Only these two formerly Ground-only trophies are promoted, and only to
    # the exact portable facts captured above. No identity, use or text owner changes.
    key = existing.get('production_definition', {}).get('production_key')
    old_custody = dict(materializable=False, stack_class='NonStackable', legal_destinations=['Ground'])
    portable = dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground'])
    if (key in {'oteryn:item.tibia.i5805', 'oteryn:item.tibia.i5806'}
            and existing.get('admission') == old_custody and observed.get('admission') == portable):
        existing = dict(existing, admission=portable)
    for field in ('production_definition', 'production_binding', 'admission'):
        if existing.get(field) != observed.get(field):
            raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:' + field)
    merged = dict(existing)
    semantics = accepted.accepted_first(existing['semantics'], observed['semantics'])
    if (accepted.known_conflict(existing['semantics'], semantics)
            or accepted.known_conflict(observed['semantics'], semantics)):
        raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:semantics')
    merged['semantics'] = semantics
    attributes = dict(existing.get('attributes', {}))
    for key, value in observed.get('attributes', {}).items():
        previous = attributes.get(key)
        if previous is not None and value is not None and previous != value:
            raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:attributes.' + key)
        if previous is None:
            attributes[key] = value
    merged['attributes'] = attributes
    if existing.keys() != observed.keys():
        raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:shape')
    for key in existing.keys() - {'authoring', 'attributes', 'semantics'}:
        if existing[key] != observed[key]:
            raise ValueError('QENGINE_REWARD_ITEM_NATIVE_CONFLICT:' + key)
    return merged


def admit(root,generated,profile_path,manifest_path,order,canonical):
    packet=capture(root)
    generated[PROOF]=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode()
    admit_records(root,generated,profile_path,manifest_path,order,canonical,packet['records'] + packet['custody_promotions'])


def admit_records(root,generated,profile_path,manifest_path,order,canonical,records):
    """Merge qualified physical profiles while preserving existing native owners."""
    document=json.loads(generated[profile_path])
    present={r['production_definition']['production_key']:r for r in document['records']}
    for record in records:
        key=record['production_definition']['production_key']
        if key in present:
            prior = present[key]
            inert = prior['admission'] == {
                'materializable': False,
                'stack_class': record['admission']['stack_class'],
                'legal_destinations': ['Ground']}
            if (inert
                    and prior['production_definition'] == record['production_definition']
                    and accepted.accepted_first(prior['semantics'], record['semantics'])
                        == record['semantics']):
                document['records'][document['records'].index(prior)] = record
                continue
            reconcile_active_physical(prior, record)
            import questpc7_quest_item_reconcile
            questpc7_quest_item_reconcile.preserve(prior, record)
            import questpc7_physical_custody_join
            physical = questpc7_physical_custody_join.observation(prior, record, accepted.accepted_first)
            merged = reconcile(prior, physical)
            present[key].clear()
            present[key].update(merged)
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


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    expected = json.dumps(capture(), ensure_ascii=False, indent=2)+'\n'
    path = ROOT/PROOF
    if args.check:
        if path.read_text() != expected:
            raise SystemExit('QENGINE reward source capture is stale')
    else:
        path.write_text(expected)
