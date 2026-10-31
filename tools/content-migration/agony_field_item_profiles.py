"""C08: admit Crystal's agony field 43297 into the native Item provider as a source-pinned profile.

The Canary pin has no 43297, so `build_source_item_profiles.py` (which reads the Canary field
profiles only) never carries it. The record is Crystal's:

- Crystal 201189f8 `data/items/items.xml:75219` (ff7ede59 `items.xml:75193`, the same record):
  `agony field`, type magicfield, duration 248, decayTo 0, field `agony` with ticks 5000,
  start 260, damage 100, a stepin moveevent. `data-global/scripts/spells/monster/agony_field.lua:6`
  creates it (`COMBAT_PARAM_CREATEITEM`).
- The damage recipe is the pinned Crystal ff7ede59 profile in `native-field-profiles.json`
  (`build_field_profiles.py`); this script only carries it.
- The physical facts come from the official client 15.30 `appearances.dat` the repository already
  holds (`content/assets/files/`): object 43297 is unmovable, not pickupable, blocks neither
  movement nor projectiles, has no height and is not cumulative. Its id is in every admitted
  appearance membership manifest.
- The binding is the EXACT Crystal binding of `imports/crystalserver/bindings/items.json`.
"""
import copy
import hashlib
import json
import sys

ITEM = 43297
KEY = f'oteryn:item.tibia.i{ITEM}'
FIELD_PROFILES = 'tools/content-schema/spell-authoring/samples/native-field-profiles.json'
APPEARANCES = ('content/assets/files/'
               'appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat')
APPEARANCES_SHA256 = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
MEMBERSHIP = 'imports/official/appearance-membership'
CRYSTAL_PIN = 'ff7ede593c69d4c658b382c97443e8155926924a'
# Crystal items.xml:75219: duration 248 s and decayTo 0.
DURATION_MS = 248_000
K = lambda value: {'state': 'KNOWN', 'value': value}
NA = {'state': 'NOT_APPLICABLE'}
U = {'state': 'UNKNOWN'}


def physical(root):
    raw = (root / APPEARANCES).read_bytes()
    if hashlib.sha256(raw).hexdigest() != APPEARANCES_SHA256:
        raise ValueError('AGONY_FIELD_APPEARANCES_DIGEST')
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    from build_source_world import fields, policies
    # The XML carries no physical override for 43297 (items.xml:75219).
    [facts] = policies(raw, b'<items><item id="43297"/></items>', {ITEM})
    for record in fields(raw).get(1, []):
        obj = fields(record)
        if obj.get(1, [0])[0] == ITEM:
            flags = fields(obj.get(3, [b''])[0])
            facts['stackable'] = bool(flags.get(10, [0])[0])
    return facts


def admit(root, generated, profile_path, manifest_path, order, canonical):
    document = json.loads(generated[profile_path])
    if any(row['production_definition']['production_key'] == KEY for row in document['records']):
        raise ValueError('AGONY_FIELD_ALREADY_ADMITTED')
    profiles = json.loads((root / FIELD_PROFILES).read_bytes())['profiles']
    [field] = [p for p in profiles
               if p['source']['server'] == 'crystal' and p['source']['server_item_id'] == ITEM]
    condition = field['condition']
    if (field['source']['revision'] != CRYSTAL_PIN or condition['kind'] != 'damage'
            or condition['element'] != 'agony'
            or [(e['amount'], e['interval_ms'], e['repetitions']) for e in condition['entries']]
            != [(amount, 5000, 1) for amount in range(260, 0, -1)]):
        raise ValueError('AGONY_FIELD_PROFILE')
    for path in sorted((root / MEMBERSHIP).glob('appearances-*.json')):
        if ITEM not in {row[0] for row in json.loads(path.read_bytes())['entries']}:
            raise ValueError('AGONY_FIELD_APPEARANCE_MEMBERSHIP')
    facts = physical(root)
    if facts['movable'] or facts['pickupable'] or facts['stackable'] or facts['ground']:
        raise ValueError('AGONY_FIELD_APPEARANCE_FLAGS')
    bindings = json.loads((root / 'imports/crystalserver/bindings/items.json').read_bytes())['bindings']
    matches = [b for b in bindings if b['external_id'] == str(ITEM)]
    if (len(matches) != 1 or matches[0]['disposition'] != 'EXACT'
            or matches[0]['source_revision'] != CRYSTAL_PIN or matches[0]['target']['key'] != KEY):
        raise ValueError('AGONY_FIELD_BINDING')
    identity = matches[0]['target']
    semantics = {
        'presentation': K({'name': K('agony field'), 'description': K('')}),
        'classification': K({'item_type': K('MAGIC_FIELD'), 'capabilities': copy.deepcopy(U)}),
        'physical': K({'weight': K(0), 'movable': K(False), 'pickupable': K(False)}),
        'stack': K({'stackable': K(False), 'stack_max': copy.deepcopy(NA)}),
        'equipment': copy.deepcopy(NA), 'weapon': copy.deepcopy(U), 'protection': copy.deepcopy(U),
        'skill_modifiers': K({'modifiers': K([])}), 'charges': copy.deepcopy(NA),
        'temporal': K({'consumption_mode': K('DURABLE_ABSOLUTE_DEADLINE'), 'duration': K(DURATION_MS),
                       'stop_duration': K(False), 'decay_target': copy.deepcopy(NA)}),
        'container': copy.deepcopy(NA), 'imbuement': copy.deepcopy(U),
        'use_transform': copy.deepcopy(U), 'trade_restrictions': copy.deepcopy(U),
        'fluid': copy.deepcopy(U), 'readable_writeable': copy.deepcopy(U),
    }
    document['records'].append({
        'authoring': {'item': identity},
        'semantics': semantics,
        'attributes': {'speed_bonus': 0, 'blocks_movement': facts['block_solid'],
                       'blocks_projectile': facts['block_projectile'],
                       'immovable_block_solid': facts['block_solid'] and not facts['movable'],
                       'has_height': facts['has_height'], 'field_replaceable': True,
                       'field_condition': field, 'rune_consumption': None},
        'admission': {'materializable': True, 'stack_class': 'NonStackable',
                      'legal_destinations': ['Ground']},
        'production_binding': matches[0],
        'production_definition': {'family': identity['family'], 'production_key': KEY,
                                  'revision_ref': identity['revision']},
        'production_binding_qualification': None,
    })
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)
