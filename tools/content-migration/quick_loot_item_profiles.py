"""QUICKLOOT: admit existing bound pouch/quiver identities into native content.

Physical/container facts come from the existing World reference. Quiver slot and
level defaults come from the digest-pinned Canary XML; Extra is the task's Global
rule. No source asset or identity is introduced.
"""
import copy
import hashlib
import json
import xml.etree.ElementTree as ET

IDS = {23721, 35524, 35562, 35848, 35849, 36666, 39150, 39160, 45644}
SOURCE = 'imports/canary/items-xml/items.xml'
SOURCE_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'


def known(value):
    return {'state': 'KNOWN', 'value': value}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('QUICKLOOT_SOURCE_DIGEST')
    xml = {int(n.get('id')): n for n in ET.fromstring(raw).findall('item') if n.get('id')}
    world = json.loads((root / 'content/world/definitions/reference.json').read_bytes())
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    document = json.loads(generated[profile_path])
    for number in sorted(IDS):
        key = f'oteryn:item.tibia.i{number}'
        source, = [r for r in world['records'] if r['identity']['key'] == key]
        binding, = [b for b in bindings['bindings'] if b['target']['key'] == key]
        if binding['disposition'] != 'EXACT' or binding['target'] != source['identity']:
            raise ValueError('QUICKLOOT_IDENTITY')
        if any(a.get('key', '').lower() in {'charges', 'duration', 'decayto'} for a in xml[number].findall('attribute')):
            raise ValueError('QUICKLOOT_UNEXPECTED_TIMED_ITEM')
        semantics = copy.deepcopy(source['semantics'])
        for field in ('charges', 'temporal'):
            semantics[field] = {'state': 'NOT_APPLICABLE'}
        semantics['stack'] = known({'stackable': known(False), 'stack_max': {'state': 'NOT_APPLICABLE'}})
        if number != 23721:
            node = xml[number]
            attrs = {a.get('key').lower(): a.get('value') for a in node.findall('attribute')}
            if attrs.get('primarytype') != 'quivers' or attrs.get('slottype') != 'right-hand':
                raise ValueError('QUICKLOOT_QUIVER_FACTS')
            script = {a.get('key'): a.get('value') for a in node.findall("attribute[@key='script']/attribute")}
            patterns = copy.deepcopy(semantics['equipment']['value']['patterns']['value'])
            if len(patterns) != 1:
                raise ValueError('QUICKLOOT_EQUIPMENT_PATTERN')
            patterns[0]['level'] = known(int(script.get('level', 0)))
            patterns.append(copy.deepcopy(patterns[0]))
            patterns[1]['pattern_id'] = 2
            patterns[1]['primary_slot'] = known('EXTRA')
            semantics['equipment'] = known({'patterns': known(patterns)})
            semantics['weapon'] = {'state': 'NOT_APPLICABLE'}
        record = {
            'authoring': {'item': source['identity']}, 'semantics': semantics, 'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory'] if number == 23721 else ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key, 'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        }
        if any(r['production_definition']['production_key'] == key for r in document['records']):
            raise ValueError('QUICKLOOT_DUPLICATE_PROFILE')
        document['records'].append(record)
    taxonomy = {r['target']['key']: r['source_taxonomy'] for r in json.loads((root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    for record in document['records']:
        key = record['production_definition']['production_key']
        if key in taxonomy:
            record['authoring']['taxonomy'] = copy.deepcopy(taxonomy[key])
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
