"""W05d exact Crystal Cake Golem native profile; canonical identity stays source owned."""
import copy
import hashlib
import json

INPUT = 'tools/content-schema/native-gameplay/seasonal-cake-creature.json'
CREATURE = 'oteryn:creature.cake_golem'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'


def admit(root, generated, canonical):
    data = json.loads((root / INPUT).read_bytes())
    if data['schema'] != 'OTERYN_SEASONAL_CAKE_CREATURE/v1' or data['source']['revision'] != PIN:
        raise ValueError('SEASONAL_CAKE_CREATURE_SOURCE')
    stage = data['stage']
    profiles = {p['target']['family']: p for p in stage['profiles'] if p['target']['family'] in {'Creature', 'Behavior', 'Presentation'}}
    record = {'profile': copy.deepcopy(profiles['Creature']),
              'behavior': copy.deepcopy(profiles['Behavior']),
              'presentation': copy.deepcopy(profiles['Presentation']['target']),
              'source_qualification': 'Pinned Crystal world-event actor; current-Tibia wiki checks pending. Missing combat type preserves engine undefined health damage, not inferred poison.',
              'attack_definitions': {
                  'records': [r for r in stage['records'] if r['identity']['family'] in {'Creature', 'Behavior', 'Ability', 'Effect', 'Formula'}],
                  'profiles': [p for p in stage['profiles'] if p['target']['family'] in {'Creature', 'Behavior', 'Ability', 'Effect', 'Formula'}]}}
    if record['profile']['target']['key'] != CREATURE:
        raise ValueError('SEASONAL_CAKE_CREATURE_IDENTITY')
    from bind_monster_source_definitions import explicit_reference_levels
    explicit_reference_levels(record)
    source = data['source']
    record['monster_melee'] = {'ability': {'family': 'Ability', 'key': 'oteryn:ability.creature.cake_golem.attack-1', 'revision': 'definition-r1'},
        'interval_ms': 2000, 'chance_ppm': 1000000, 'minimum': 0, 'maximum': 45,
        'source_repository': 'zimbadev/crystalserver', 'source_revision': PIN,
        'source_path': source['path'], 'source_sha256': source['sha256'],
        'execution_status': 'primaryphysicalonly_omits_secondary_conditions'}
    paths = [('content/creatures/definitions/spell-native-profiles.json', record, lambda r: r['profile']['target']['key']),
             ('content/presentations/definitions/spell-native-profiles.json', profiles['Presentation'], lambda r: r['target']['key'])]
    for path, row, key in paths:
        document = json.loads(generated[path]); existing = [r for r in document['records'] if key(r) == key(row)]
        if existing and existing != [row]:
            raise ValueError('SEASONAL_CAKE_CREATURE_CONFLICT')
        if not existing: document['records'].append(row)
        generated[path] = canonical(document)
    # Every modified provider must remain independently bound by the manifest;
    # later overlays may refresh Creature, but do not own Presentation.
    manifest_path = 'content/spells.manifest.json'
    manifest = json.loads(generated[manifest_path])
    for field, path in (
        ('creature_profiles', paths[0][0]),
        ('presentation_profiles', paths[1][0]),
    ):
        manifest[field]['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated[manifest_path] = canonical(manifest)
