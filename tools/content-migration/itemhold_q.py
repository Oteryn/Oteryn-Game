"""Reconcile the finite ITEMHOLD-Q catalogue cohort with pinned donor facts.

No quest charge/subtype disposition is superseded. Existing native fluid, timed,
starter and reward-bag owners keep authority; only wolf/collar need new profiles.
"""
import copy
import hashlib
import json
import sys
from pathlib import Path

import quest_reward_item_semantics as reward

ROOT = Path(__file__).resolve().parents[2]
IDS = (2881, 2882, 3054, 3056, 3083, 3572, 12369, 14156, 39546)
NEW_NATIVE = (12369, 14156)
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
EXPECTED = {
    2881: ('cup', 200, None), 2882: ('jug', 750, None),
    3054: ('silver amulet', 500, 200), 3056: ('bronze amulet', 500, 200),
    3083: ('garlic necklace', 380, 150), 3572: ('scarf', 200, None),
    12369: ('captured wolf', 5000, None), 14156: ('dog collar', 110, None),
    39546: ('primal bag', 16000, None),
}
NA = {'state': 'NOT_APPLICABLE'}


def source_facts(root):
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    raw = (root / XML).read_bytes()
    if hashlib.sha256(raw).hexdigest() != XML_SHA:
        raise ValueError('ITEMHOLD_Q_XML_PIN')
    client = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(client).hexdigest() != reward.CLIENT_SHA:
        raise ValueError('ITEMHOLD_Q_CLIENT_PIN')
    xml = source.xml_index(raw)
    objects = source.engine_items.load_appearance_objects(client)
    facts = {}
    for item, (name, weight, charges) in EXPECTED.items():
        node, obj = xml[item], objects[item]
        attrs, flags = source.attrs(node), obj['flags']
        if (node.get('name') != name or obj['name'] != name
                or attrs.get('weight') != str(weight)
                or attrs.get('charges') != (str(charges) if charges else None)
                or any(key in attrs for key in ('duration', 'decayto', 'stopduration', 'containersize'))
                or not flags.get('flags.take')
                or any(flags.get(key, False) for key in
                       ('flags.cumulative', 'flags.container', 'flags.unmove', 'flags.liquidpool'))
                or bool(flags.get('flags.liquidcontainer')) != (item in (2881, 2882))):
            raise ValueError('ITEMHOLD_Q_SOURCE_CONFLICT:' + str(item))
        facts[item] = (weight, charges)
    return facts


def apply_admissions(definitions, root=ROOT):
    facts = source_facts(root)
    for definition in definitions:
        key = definition['identity']['key']
        if key not in {f'oteryn:item.tibia.i{i}' for i in IDS}:
            continue
        item = int(key.rsplit('.i', 1)[1])
        if definition['identity'] != {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}',
                                      'revision': 'definition-r1'}:
            raise ValueError('ITEMHOLD_Q_IDENTITY')
        if definition['stack_class'] not in ('Unknown', 'NonStackable'):
            raise ValueError('ITEMHOLD_Q_EXISTING_STACK_CONFLICT')
        weight, charges = facts[item]
        # Existing known facts may never be overwritten by a conflicting donor.
        reward.put_group(definition, 'physical', {'movable': reward.known(True),
                         'pickupable': reward.known(True), 'weight': reward.known(weight)})
        reward.put_group(definition, 'stack', {'stackable': reward.known(False), 'stack_max': NA})
        for group, target in (('container', NA), ('temporal', NA),
                              ('charges', reward.known({'count': reward.known(charges)}) if charges else NA)):
            previous = definition['semantics'].get(group, reward.UNKNOWN)
            if previous not in (reward.UNKNOWN, target):
                raise ValueError('ITEMHOLD_Q_EXISTING_CONFLICT:' + group)
            definition['semantics'][group] = copy.deepcopy(target)
        if item in (2881, 2882):
            reward.put_group(definition, 'fluid', {'fluid_type': NA})
        definition['stack_class'] = 'NonStackable'
        definition['materializable'] = True
    import itemhold_q_remaining
    itemhold_q_remaining.apply_admissions(definitions, root)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    # Read only source-qualified canonical rows; no candidate key/name reminting.
    document = json.loads(generated[profile_path])
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    definitions = [row['definition']
                   for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for row in json.loads(path.read_bytes())['records']
                   if row['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in NEW_NATIVE}]
    if len(definitions) != len(NEW_NATIVE):
        raise ValueError('ITEMHOLD_Q_NATIVE_CLOSURE')
    for definition in definitions:
        identity = definition['identity']
        binding, = [b for b in bindings if b['target'] == identity]
        if (binding['disposition'] != 'EXACT' or not definition['materializable']
                or definition['stack_class'] != 'NonStackable'
                or any(r['production_definition']['production_key'] == identity['key'] for r in document['records'])):
            raise ValueError('ITEMHOLD_Q_NATIVE_ADMISSION')
        document['records'].append({
            'authoring': {'item': identity}, 'semantics': copy.deepcopy(definition['semantics']),
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': identity['key'],
                                      'revision_ref': identity['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    document['records'].sort(key=order)
    raw = canonical(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
    import itemhold_q_remaining
    itemhold_q_remaining.admit(root, generated, profile_path, manifest_path, order, canonical)
