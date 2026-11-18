"""Qualify the thieving squirrel's items and Gamel's letter through native Item custody."""
import hashlib
import json

import qengine_reward_item_profiles as items

# Crystal 201189f871d23597b82caf88fd611454a2ddefb8 data/items/items.xml
# lines 475–480, 810–813 and 26267–26270. The pinned local XML and appearance bank
# agree on both entries and their appearance flags.
ENTRIES = {
    276: ('dead squirrel', {'containersize': '15', 'decayto': '277',
                            'duration': '10', 'fluidsource': 'blood'}),
    402: ('secret letter', {'primarytype': 'documents and papers', 'weight': '50'}),
    9843: ('flask with beaver bait', {
        'description': 'It smells like a mixture of wood and fish. Phew',
        'weight': '180',
    }),
}


def capture(root):
    xml_raw = (root / items.XML).read_bytes()
    client = (root / items.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != items.XML_SHA
            or hashlib.sha256(client).hexdigest() != items.CLIENT_SHA):
        raise ValueError('M02_ITEM_SOURCE_PIN')
    source = items.source
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client)
    ids = set(ENTRIES)
    physical = {row['server_item_id']: row for row in source.policies(client, xml_raw, ids)}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    records = []
    for item, (name, attributes) in ENTRIES.items():
        node, obj = xml[item], objects[item]
        if (node.attrib != {'id': str(item), 'article': 'a', 'name': name}
                or source.attrs(node) != attributes
                or bool(obj['flags'].get('flags.corpse')) != (item == 276)):
            raise ValueError(f'M02_ITEM_SOURCE_ENTRY:{item}')
        records.append(source.profile(item, node, obj, physical[item], bindings, None))
    qualified = source.bind_profiles(
        {'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
        bindings, ids,
    )['records']
    keys = {row['production_definition']['production_key'] for row in qualified}
    definitions = {
        row['definition']['identity']['key']: row['definition']
        for path in (root / 'content/items/definitions').glob('items-*.json')
        for row in json.loads(path.read_bytes())['records']
        if row['definition']['identity']['key'] in keys
    }
    for record in qualified:
        definition = definitions[record['production_definition']['production_key']]
        if (definition['identity'] != record['production_binding']['target']
                or definition['stack_class'] not in ('Unknown', record['admission']['stack_class'])):
            raise ValueError('M02_ITEM_CANONICAL_IDENTITY')
        semantics = definition.get('semantics', {})
        # Canonical presentation and market facts already have their own owners.
        # Only the native physical and lifecycle groups must agree with the source.
        if any(items.accepted.known_conflict(semantics.get(group, source.U), record['semantics'][group])
               for group in ('physical', 'stack', 'container', 'charges', 'temporal')):
            raise ValueError('M02_ITEM_CANONICAL_FACT_CONFLICT')
        record['semantics'] = items.accepted.accepted_first(semantics, record['semantics'])
    return qualified


def admit(root, generated, profile_path, manifest_path, order, canonical):
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, capture(root))
