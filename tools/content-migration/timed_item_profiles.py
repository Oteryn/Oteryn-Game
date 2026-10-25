"""Carry admitted timed definitions into the native server provider.

Values/transform pairs are already owned by TIMED-CONTENT-1. The small admission
completion uses Canary 47dfd51 items.xml: nonstackable wearable boots/amulet, no
container, extra slot, vocation or level restriction. Worn boots remain repair-only.
"""
import copy
import hashlib
import json

IDS = {3051, 3081, 3088, 3549, 6529, 6530}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    definitions = {}
    for path in (root / 'content/items/definitions').glob('items-*.json'):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            if definition['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}:
                definitions[definition['identity']['key']] = definition
    if len(definitions) != len(IDS):
        raise ValueError('TIMED_PROFILE_DEFINITIONS_MISSING')
    for key, source in sorted(definitions.items()):
        if any(row['production_definition']['production_key'] == key for row in document['records']):
            raise ValueError('TIMED_PROFILE_DUPLICATE')
        binding = next(row for row in bindings if row['target'] == source['identity'])
        if binding['disposition'] != 'EXACT':
            raise ValueError('TIMED_PROFILE_BINDING')
        semantics = copy.deepcopy(source['semantics'])
        # ENERGY-RING-1: the active form inherits the wiki's existing wearable facts,
        # including KNIGHT/MONK/PALADIN (never Canary's older vocation list).
        # Pinned Canary 04b83b51 data/items/items.xml: 3051 -> 3088, manashield=1,
        # duration=600, deequip=3051, decayTo=0. Timed promotion already carries
        # the duration and pair; only complete the active form's equipment facts.
        # Global's 2:1 rule is domain logic, not a new content/schema attribute:
        # https://www.tibia.com/news/?subtopic=newsarchive&id=8833
        if key == 'oteryn:item.tibia.i3088':
            base = definitions['oteryn:item.tibia.i3051']['semantics']
            for group in ('equipment', 'stack', 'physical'):
                semantics[group] = copy.deepcopy(base[group])
            semantics['skill_modifiers'] = {'state': 'KNOWN', 'value': {
                'modifiers': {'state': 'KNOWN', 'value': [{
                    'kind': 'MANA_SHIELD',
                    'parameter': {'state': 'KNOWN', 'value': {'kind': 'BOOLEAN', 'value': True}},
                    'evaluation_phase': {'state': 'UNKNOWN'},
                    'priority': {'state': 'UNKNOWN'},
                    'target_domain': {'state': 'UNKNOWN'},
                }]}}}
        semantics['container'] = {'state': 'NOT_APPLICABLE'}
        semantics['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': False}, 'stack_max': {'state': 'NOT_APPLICABLE'}}}
        if key != 'oteryn:item.tibia.i6530':
            for pattern in semantics['equipment']['value']['patterns']['value']:
                if key not in ('oteryn:item.tibia.i3051', 'oteryn:item.tibia.i3088'):
                    pattern['vocations'] = {'state': 'KNOWN', 'value': []}
                pattern['level'] = {'state': 'KNOWN', 'value': 0}
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics,
            'attributes': {'speed_bonus': 0},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key,
                                      'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None,
        })
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
