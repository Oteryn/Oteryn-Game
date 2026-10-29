"""GOLDCONV: admit the four coin converter identities into native content.

Physical facts come from the existing World reference; the temporary converter has no
World weight or movability there, so those two facts come from the digest-pinned Canary
XML (weight 300, no unmovable flag). Charges stay the World's KNOWN 500. The converters
are nonstackable carried tools; no source asset or identity is introduced.
"""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

IDS = {23722, 25719, 28525, 28526}
SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'


def known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('GOLDCONV_SOURCE_DIGEST')
    xml = {int(n.get('id')): n for n in ET.fromstring(raw).findall('item') if n.get('id')}
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for number in sorted(IDS):
        key = f'oteryn:item.tibia.i{number}'
        source, = [r for r in world['records'] if r['identity']['key'] == key]
        binding, = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if binding['disposition'] != 'EXACT' or binding['target'] != source['identity']:
            raise ValueError('GOLDCONV_IDENTITY')
        attrs = {a.get('key').lower(): a.get('value') for a in xml[number].findall('attribute')}
        if attrs.get('charges') != '500' or 'stackable' in attrs or 'containersize' in attrs:
            raise ValueError('GOLDCONV_SOURCE_FACTS')
        semantics = copy.deepcopy(source['semantics'])
        if semantics['charges'] != known({'count': known(500)}):
            raise ValueError('GOLDCONV_CHARGES')
        physical = semantics['physical']['value']
        if physical['weight']['state'] != 'KNOWN':
            physical['weight'] = known(int(attrs['weight']))
        if physical['movable']['state'] != 'KNOWN':
            physical['movable'] = known(True)
        semantics['temporal'] = {'state': 'NOT_APPLICABLE'}
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('GOLDCONV_DUPLICATE_PROFILE')
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics, 'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    taxonomy = {r['target']['key']: r['source_taxonomy'] for r in json.loads((root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    for record in document['records']:
        key = record['production_definition']['production_key']
        if key in taxonomy and key in {f'oteryn:item.tibia.i{n}' for n in IDS}:
            record['authoring']['taxonomy'] = copy.deepcopy(taxonomy[key])
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
