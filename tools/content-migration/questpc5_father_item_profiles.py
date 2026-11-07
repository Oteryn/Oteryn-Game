"""Physical custody for Tereban's eight fixed source materials.

The source magic crystal has type=teleport but no destination. This finite
overlay admits its portable physical instance, not a teleport action. Unknown
action semantics stay unknown; the existing NPC owner only consumes the item.
Canonical known facts and all other native cohorts retain authority.
"""
import hashlib
import json

import grizzly_shop_profiles as accepted
import quest_callback_profiles as callbacks
import quest_reward_item_semantics as reward

IDS = (*range(11545, 11553), 11701)
WEIGHTS = (10, 500, 500, 35, 222, 80, 60, 140, 3500)


def admit(root, generated, profile_path, manifest_path, order, canonical):
    xml_raw = (root / callbacks.SOURCE).read_bytes()
    client_raw = (root / reward.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != callbacks.SOURCE_SHA256
            or hashlib.sha256(client_raw).hexdigest() != reward.CLIENT_SHA):
        raise ValueError('FATHER_MATERIAL_SOURCE_PIN')
    source = accepted.source
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client_raw)
    physical = {row['server_item_id']: row
                for row in source.policies(client_raw, xml_raw, set(IDS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    wanted = {f'oteryn:item.tibia.i{number}' for number in IDS}
    definitions = {}
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key']
            if key in wanted:
                if key in definitions:
                    raise ValueError('FATHER_MATERIAL_DUPLICATE')
                definitions[key] = definition
    if set(definitions) != wanted:
        raise ValueError('FATHER_MATERIAL_CANONICAL_MISSING')
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key']: row for row in document['records']}
    if len(present) != len(document['records']):
        raise ValueError('FATHER_MATERIAL_DUPLICATE_NATIVE')
    for number, weight in zip(IDS, WEIGHTS):
        key = f'oteryn:item.tibia.i{number}'
        definition = definitions[key]
        attributes = source.attrs(xml[number])
        expected = {'primarytype': 'quest items', 'weight': str(weight)}
        if number == 11551:
            expected['description'] = 'It is able to store memories and thoughts.'
        if number == 11552:
            expected.update(type='teleport', description='It is able to store memories and thoughts')
        if number == 11701:
            expected.update(primarytype='armors', armor='1',
                            description='This old cape might be worth more than what it looks like')
        if attributes != expected:
            raise ValueError('FATHER_MATERIAL_XML_CHANGED:' + key)
        binding = bindings.get(str(number))
        if binding is None or binding['target'] != definition['identity'] or binding['disposition'] != 'EXACT':
            raise ValueError('FATHER_MATERIAL_BINDING:' + key)
        row = source.profile(number, xml[number], objects[number], physical[number], bindings, None)
        if (row['admission'] != {'materializable': True, 'stack_class': 'NonStackable',
                                 'legal_destinations': ['CharacterInventory', 'Ground']}
                or any(row['semantics'][group] != source.NA
                       for group in ('container', 'charges', 'temporal'))):
            raise ValueError('FATHER_MATERIAL_CUSTODY_CHANGED:' + key)
        if any(accepted.known_conflict(definition['semantics'][group], row['semantics'][group])
               for group in ('physical', 'stack', 'container')):
            raise ValueError('FATHER_MATERIAL_CANONICAL_CONFLICT:' + key)
        row['semantics'] = accepted.accepted_first(definition['semantics'], row['semantics'])
        row = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': [row]},
                                   bindings, {number})['records'][0]
        if key in present:
            if present[key] != row:
                raise ValueError('FATHER_MATERIAL_NATIVE_CONFLICT:' + key)
        else:
            document['records'].append(row)
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
