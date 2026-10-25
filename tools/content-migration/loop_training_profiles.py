"""EXERCISE0 native item overlay from existing exact identities and declared charge facts.

Only item structure is admitted here. Gain rates live in the separate, disabled-by-default
provisional training configuration; no Reference rate reconciliation is changed.
"""
import copy
import hashlib
import json


def admit(root, generated, profile_path, manifest_path, order, canonical):
    config = json.loads((root / 'rulesets/training/provisional-v1.json').read_bytes())
    wanted = set(config['weapons'])
    bindings = {row['target']['key']: row for row in json.loads(
        (root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']}
    sources = {}
    for path in sorted((root / 'content/items/definitions').glob('*.json')):
        if path.name == 'spell-native-profiles.json':
            continue
        for row in json.loads(path.read_bytes()).get('records', []):
            definition = row.get('definition', {})
            key = definition.get('identity', {}).get('key')
            if key in wanted:
                if key in sources:
                    raise ValueError('TRAINING_ITEM_DUPLICATE')
                sources[key] = definition
    if set(sources) != wanted or len(wanted) != 23:
        raise ValueError('TRAINING_ITEM_RECORD_SET')
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key'] for row in document['records']}
    if present & wanted:
        raise ValueError('TRAINING_ITEM_PROFILE_CONFLICT')
    for key in sorted(wanted):
        source = sources[key]
        binding = bindings[key]
        semantics = copy.deepcopy(source['semantics'])
        physical = semantics['physical']['value']
        charges = semantics['charges']['value']['count']
        if (binding['disposition'] != 'EXACT' or source['identity'] != binding['target']
                or charges['state'] != 'KNOWN' or charges['value'] not in (500, 1800, 14400)
                or any(physical[field] != {'state': 'KNOWN', 'value': True}
                       for field in ('movable', 'pickupable'))
                or physical['weight']['state'] != 'KNOWN'):
            raise ValueError('TRAINING_ITEM_FACTS_UNAVAILABLE')
        # Accepted EXERCISE0: ordinary nonstackable charge items, never combat weapons.
        semantics['stack'] = {'state': 'KNOWN', 'value': {
            'stackable': {'state': 'KNOWN', 'value': False},
            'stack_max': {'state': 'NOT_APPLICABLE'}}}
        for section in ('weapon', 'equipment', 'temporal', 'container'):
            semantics[section] = {'state': 'NOT_APPLICABLE'}
        document['records'].append({
            'authoring': {'item': source['identity']}, 'semantics': semantics,
            'attributes': {'speed_bonus': 0, 'blocks_movement': False,
                           'blocks_projectile': False, 'immovable_block_solid': False,
                           'has_height': False},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
            'production_definition': {'family': 'Item', 'production_key': key,
                                      'revision_ref': source['identity']['revision']},
            'production_binding': binding, 'production_binding_qualification': None})
    document['records'].sort(key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
