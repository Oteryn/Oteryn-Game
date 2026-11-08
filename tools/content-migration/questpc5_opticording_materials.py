"""Exact physical Opticording materials; existing importers own all native semantics."""
import hashlib
import json
import qengine_reward_item_profiles as shared
import questpc5_opticording_head as head

SPECS = {19063: ('opticording sphere', 200), 23572: ('complete opticording sphere', 200),
         23573: ('broken opticording sphere', 180), 27522: ('blue spectacles', 250),
         33313: ('the supreme cube', 1000)}
PROOF = 'tools/content-schema/native-gameplay/questpc5-opticording-materials.json'


def capture(root=shared.ROOT):
    xml_raw, client_raw = [(root / path).read_bytes() for path in (shared.XML, shared.CLIENT)]
    if (hashlib.sha256(xml_raw).hexdigest() != shared.XML_SHA
            or hashlib.sha256(client_raw).hexdigest() != shared.CLIENT_SHA):
        raise ValueError('OPTICORDING_SOURCE_PIN')
    source = shared.source
    xml, objects = source.xml_index(xml_raw), source.engine_items.load_appearance_objects(client_raw)
    physical = {r['server_item_id']: r for r in source.policies(client_raw, xml_raw, set(SPECS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    definitions = {r['definition']['identity']['key']: r['definition']
                   for path in sorted((root / 'content/items/definitions').glob('items-*.json'))
                   for r in json.loads(path.read_bytes())['records']
                   if r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in SPECS}}
    rows = []
    for number, (name, weight) in sorted(SPECS.items()):
        flags = objects[number]['flags']
        if (objects[number]['name'] != name or flags.get('flags.take') is not True
                or any(flags.get(f, False) for f in ['flags.unmove', 'flags.container',
                    'flags.cumulative', 'flags.liquidcontainer', 'flags.liquidpool'])):
            raise ValueError('OPTICORDING_CLIENT_CUSTODY')
        if xml[number].get('name') != name or source.attrs(xml[number]).get('weight') != str(weight):
            raise ValueError('OPTICORDING_XML_IDENTITY')
        rows.append(head.qualify(number, xml[number], objects[number], physical[number],
                    bindings, definitions[f'oteryn:item.tibia.i{number}']))
    return rows


def admit(root, generated, profile_path, manifest_path, order, canonical):
    rows = capture(root)
    generated[PROOF] = (json.dumps(dict(schema=1, records=rows,
        sources=[dict(path=shared.XML, sha256=shared.XML_SHA), dict(path=shared.CLIENT, sha256=shared.CLIENT_SHA)],
        scope='Physical custody only; no vision, repair, charge, timer or scripted-use admission.'),
        ensure_ascii=False, indent=2) + '\n').encode()
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, rows)
