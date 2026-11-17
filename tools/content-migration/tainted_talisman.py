"""M22: the carried star-herb/antlers conversion's exact portable Item cohort.

Stack shape comes from the admitted 15.30 appearance snapshot; weights come from
pinned Canary XML. One nonstacking talisman replaces the captured antlers object.
"""
import copy
import hashlib
import json

import quest_reward_item_semantics as reward

# Star herb and antlers already have native Hireling/Grizzly owners.
IDS = {22008: ('antler talisman', 240, False)}


def apply_admissions(definitions, root):
    import sys
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    xml_raw = (root / 'imports/canary/items-xml/items.xml').read_bytes()
    if hashlib.sha256(xml_raw).hexdigest() != '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2':
        raise ValueError('M22_XML_PIN')
    raw = (root / reward.CLIENT).read_bytes()
    if hashlib.sha256(raw).hexdigest() != reward.CLIENT_SHA:
        raise ValueError('M22_CLIENT_PIN')
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(raw)
    found = set()
    for definition in definitions:
        key = definition['identity']['key']
        if key not in {f'oteryn:item.tibia.i{i}' for i in IDS}:
            continue
        item = int(key.rsplit('.i', 1)[1])
        name, weight, stackable = IDS[item]
        flags = objects[item]['flags']
        if (xml[item].get('name') != name or objects[item]['name'] != name
                or source.attrs(xml[item]).get('weight') != str(weight)
                or not flags.get('flags.take') or flags.get('flags.unmove', False)
                or flags.get('flags.cumulative', False) != stackable
                or flags.get('flags.container', False) or flags.get('flags.liquidcontainer', False)):
            raise ValueError('M22_ITEM_SOURCE_CONFLICT')
        reward.put_group(definition, 'physical', {'movable': reward.known(True),
                         'pickupable': reward.known(True), 'weight': reward.known(weight)})
        reward.put_group(definition, 'stack', {'stackable': reward.known(stackable),
                         'stack_max': reward.known(100) if stackable else {'state': 'NOT_APPLICABLE'}})
        definition['materializable'] = True
        definition['stack_class'] = 'StackCapable' if stackable else 'NonStackable'
        found.add(item)
    if found != set(IDS):
        raise ValueError('M22_ITEM_COHORT')


def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    definitions = [r['definition'] for p in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for r in json.loads(p.read_bytes())['records']
                   if r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}]
    apply_admissions(definitions, root)
    for definition in definitions:
        identity = definition['identity']
        if identity['key'] in existing:
            if existing[identity['key']]['admission']['stack_class'] != definition['stack_class']:
                raise ValueError('M22_NATIVE_STACK_CONFLICT')
            continue
        binding, = [b for b in bindings if b['target'] == identity]
        if binding['disposition'] != 'EXACT':
            raise ValueError('M22_NATIVE_BINDING')
        document['records'].append({
            'authoring': {'item': identity}, 'semantics': copy.deepcopy(definition['semantics']),
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': definition['stack_class'],
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
