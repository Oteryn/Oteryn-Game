"""W18: admit the house podiums into native content as placeable furniture.

The five identities and their transform/wrap facts come from the digest-pinned Canary XML
(the same facts as Crystal 201189f8 `data/items/items.xml:67245-67252,71414-71416,
75107-75114`). Neither source carries a weight, so the Canary/Crystal ItemType default 0
applies. TibiaWiki (global, it wins where the donors are silent) gives the physical facts
for all three: `immobile = no`, `pickupable = no`, `stackable = no` (Podium_of_Renown rev
1020375, Podium_of_Vigour rev 1057934, Podium_of_Tenacity rev 1041421). Podiums are
furniture: they are admitted for the ground only and reach a House through its interior
owner, never a character inventory.
"""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

# Item id -> (podium kind, use-transform partner, wrap kit). Kind 1 renown, 2 vigour, 3 tenacity.
PODIUMS = {
    35973: (1, 35974, 23398),
    35974: (1, 35973, 23398),
    38707: (2, None, 23398),
    42367: (3, 42368, 23398),
    42368: (3, 42367, 23398),
}
IDS = set(PODIUMS)
SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'


def known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('PODIUM_SOURCE_DIGEST')
    xml = {int(n.get('id')): n for n in ET.fromstring(raw).findall('item') if n.get('id')}
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for number, (_, partner, kit) in sorted(PODIUMS.items()):
        key = f'oteryn:item.tibia.i{number}'
        source, = [r for r in world['records'] if r['identity']['key'] == key]
        binding, = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if binding['disposition'] != 'EXACT' or binding['target'] != source['identity']:
            raise ValueError('PODIUM_IDENTITY')
        node = xml[number]
        attrs = {a.get('key').lower(): a.get('value') for a in node.findall('attribute')}
        expected = {'wrapableto': str(kit)}
        if partner is not None:
            expected['transformonuse'] = str(partner)
        if not node.get('name', '').startswith('podium of ') or attrs != expected:
            raise ValueError('PODIUM_SOURCE_FACTS')
        semantics = copy.deepcopy(source['semantics'])
        semantics['physical'] = known({'weight': known(0), 'movable': known(True), 'pickupable': known(False)})
        semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        semantics['charges'] = {'state': 'NOT_APPLICABLE'}
        semantics['temporal'] = {'state': 'NOT_APPLICABLE'}
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('PODIUM_DUPLICATE_PROFILE')
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics, 'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    taxonomy = {r['target']['key']: r['source_taxonomy'] for r in json.loads((root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    keys = {f'oteryn:item.tibia.i{n}' for n in IDS}
    for record in document['records']:
        key = record['production_definition']['production_key']
        if key in taxonomy and key in keys:
            record['authoring']['taxonomy'] = copy.deepcopy(taxonomy[key])
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
