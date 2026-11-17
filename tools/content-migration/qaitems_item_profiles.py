"""Physical custody for the source-qualified quest items, using the native importer.

Reward identity selection lives in reward-claim-authoring/qaitems_rewards.py.
Boots 9019 and the text-bearing lottery ticket 5956 already have native custody.
Their equipped/unused counterparts require no quest reward promotion.
"""
import copy
import hashlib
import json
import qengine_reward_item_profiles as shared
from import_cyclopedia_global import apply_profile_weights
from refresh_item_overlay_inputs import qualify_native_core

# Global Ghostsilver Lantern rev 1148076 selects lit 23738. Crystal items.xml:
# 47292-47296 supplies its name, description and weight, with no explicit decay
# or numeric light attributes. Preserve accepted client facts; invent no timer.
IDS = {347, 3208, 23738}
PROOF = 'tools/content-schema/native-gameplay/qaitems-item-profiles.json'


def capture(root):
    source = shared.source
    xml_raw = (root / shared.XML).read_bytes()
    client = (root / shared.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != shared.XML_SHA
            or hashlib.sha256(client).hexdigest() != shared.CLIENT_SHA):
        raise ValueError('QAITEMS_SOURCE_PIN')
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client)
    physical = {r['server_item_id']: r for r in source.policies(client, xml_raw, IDS)}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    records = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS)['records']
    apply_profile_weights(records, root)
    definitions = {r['definition']['identity']['key']: r['definition']
                   for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for r in json.loads(path.read_bytes())['records']
                   if r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}}
    if len(definitions) != len(IDS):
        raise ValueError('QAITEMS_CANONICAL_SET')
    for record in records:
        key = record['production_definition']['production_key']
        definition = definitions[key]
        for group in ('physical', 'stack', 'container', 'charges', 'temporal'):
            if shared.accepted.known_conflict(definition['semantics'][group], record['semantics'][group]):
                raise ValueError('QAITEMS_CANONICAL_CONFLICT:' + key + ':' + group)
        record['semantics'] = shared.accepted.accepted_first(definition['semantics'], record['semantics'])
        if record['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                   'legal_destinations': ['CharacterInventory', 'Ground']}:
            raise ValueError('QAITEMS_PORTABLE_CUSTODY')
    return {'schema': 'OTERYN_QAITEMS_ITEM_PROFILES/v1',
            'sources': [{'path': shared.XML, 'sha256': shared.XML_SHA},
                        {'path': shared.CLIENT, 'sha256': shared.CLIENT_SHA}],
            'records': records}


def apply_admissions(definitions, root):
    profiles = {r['production_definition']['production_key']: r for r in capture(root)['records']}
    for definition in definitions:
        if definition['identity']['key'] in profiles:
            profile = profiles[definition['identity']['key']]
            qualified = qualify_native_core(definition, profile)
            if definition['identity']['key'] == 'oteryn:item.tibia.i3208':
                # Ready claims construct from the canonical pattern before later
                # physical owners compose it. Carry the same source-qualified
                # pattern, preserving every already known canonical leaf.
                qualified['semantics']['equipment'] = copy.deepcopy(profile['semantics']['equipment'])
            definition.clear()
            definition.update(qualified)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    generated[PROOF] = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    document = json.loads(generated[profile_path])
    for prior in document['records']:
        if prior['production_definition']['production_key'] == 'oteryn:item.tibia.i347':
            if prior['admission'] not in (
                    {'materializable': False, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']},
                    {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['CharacterInventory', 'Ground']}):
                raise ValueError('QAITEMS_SCROLL_CUSTODY_DRIFT')
            prior['admission'] = {'materializable': True, 'stack_class': 'NonStackable',
                                  'legal_destinations': ['CharacterInventory', 'Ground']}
    generated[profile_path] = canonical(document)
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
