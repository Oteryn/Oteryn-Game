"""M28: pinned OTServBR-derived Fugue body in the existing native provider."""
import hashlib
import json

CREATURES = 'content/creatures/definitions/spell-native-profiles.json'
PRESENTATIONS = 'content/presentations/definitions/spell-native-profiles.json'
SOURCE = 'docs/reference/content-prep/prey-content-1/creature-source-profiles/fugue.json'


def ref(family, suffix=''):
    stem = 'creature.fugue' if family not in ('Creature',) else 'fugue'
    return {'family': family, 'key': f'oteryn:{family.lower()}.{stem}{suffix}', 'revision': 'definition-r1'}


def profile(target, kind, data):
    return {'target': target, 'data': {'kind': kind, 'profile': data}}


def native(source):
    o = source['observations']
    records, profiles, attacks, defenses, abilities = [], [], [], [], []
    for i, attack in enumerate(o['attacks'] + [dict(o['healing'], kind='healing')]):
        suffix = f'.attack-{i + 1}' if i < 3 else '.defense-1'
        ability, effect, formula = ref('Ability', suffix), ref('Effect', suffix), ref('Formula', suffix + '-1')
        details = {'kind': 'Melee' if i == 0 else 'Spell', 'needs_direction': i == 2,
                   'needs_target': i == 0, 'range_tiles': attack.get('range', 1 if i == 0 else 0),
                   'effects': [{'kind': 'Executable', 'effect': effect}]}
        if i == 2:
            details['area'] = {'shape': 'Beam', 'length_tiles': attack['length'], 'spread_tiles': attack['spread']}
        presentation = {}
        if attack.get('effect'):
            presentation['impact_asset_binding'] = 'canary.appearance:effect/' + attack['effect']
        if attack.get('projectile'):
            presentation['projectile_asset_binding'] = 'canary.appearance:missile/' + attack['projectile']
        effect_profile = {'damage_type': 'physical' if i == 0 else attack['kind']}
        if i == 0:
            effect_profile['mitigated_by'] = ['Armor', 'Shield']
        if presentation:
            effect_profile['presentation'] = presentation
        records.extend([
            {'identity': ability, 'kind': 'Ability', 'effects': [effect]},
            {'identity': effect, 'kind': 'Effect', 'client_projection': 'ServerOnly',
             'effect_family': 'Heal' if i == 3 else 'Damage', 'formula': formula},
            {'identity': formula, 'kind': 'Formula'},
        ])
        profiles.extend([profile(ability, 'Ability', {'details': details}),
                         profile(effect, 'Effect', effect_profile),
                         profile(formula, 'Formula', {'formula': 'Range', 'minimum': attack['minimum'], 'maximum': attack['maximum']})])
        abilities.append(ability)
        (defenses if i == 3 else attacks).append({'ability': ability, 'interval_ms': attack['interval_ms'], 'chance_ppm': attack['chance_percent'] * 10000})
    creature = profile(ref('Creature'), 'Creature', {
        'health': o['health'], 'experience': o['experience'], 'speed': o['speed'], 'armor': o['armor'],
        'abilities': abilities,
        'resistances': [{'damage_type': 'physical', 'percent': {'numerator': o['physical_resistance_percent'], 'denominator': 1}}],
        'details': {'critical_chance_ppm': 0, 'defense': o['defense'], 'display_name': 'Fugue', 'inspection': 'Fugue', 'reference_level': None,
                    'condition_immunities': ['invisible', 'paralyze'],
                    'corpse_item': {'family': 'Item', 'key': f"oteryn:item.tibia.i{o['corpse']}", 'revision': 'definition-r1'},
                    'flags': {'attackable': True, 'health_hidden': False, 'illusionable': False},
                    'summoning': {'summonable': False, 'convinceable': False, 'is_familiar': False},
                    'system_eligibility': {'prey': False, 'forge': False, 'exclusive_prey': False, 'reward_boss': True},
                    'spawn_eligibility': {'period': 'All', 'blocked_by_nearby_players': False, 'ignore_period_underground': False}}})
    behavior = profile(ref('Behavior'), 'Behavior', {
        'attacks': attacks, 'defenses': defenses,
        'movement': {'can_walk': True, 'pass_through': False, 'push_creatures': True, 'push_items': True,
                     'pushable': False, 'walks_on_energy': True, 'walks_on_fire': True, 'walks_on_poison': True},
        'targeting': {'can_target': True, 'hostile': True, 'sense_invisible': True,
                      'change_target': {'interval_ms': 5000, 'chance_ppm': 80000}, 'flee_health': 0,
                      'static_attack_chance_ppm': 700000, 'strategy_weights': {'nearest': 100, 'damage': 0, 'health': 0, 'random': 0}, 'target_distance_tiles': 1}})
    records.extend([{'identity': ref('Creature'), 'kind': 'Creature', 'client_projection': 'ClientSafe', 'behavior': ref('Behavior'), 'presentation': ref('Presentation')},
                    {'identity': ref('Behavior'), 'kind': 'Generic', 'client_projection': 'ServerOnly'}])
    profiles.extend([creature, behavior])
    outfit = o['outfit']
    presentation = profile(ref('Presentation'), 'Presentation', {
        'asset_binding': f"canary.appearance:outfit/{outfit['lookType']}", 'light_level': 0,
        'attachment_bindings': [{'slot': 'Addon', 'asset_binding': f"canary.appearance:outfit/{outfit['lookType']}/addon-{i}"} for i in (1, 2)],
        'palette_bindings': [{'slot': slot, 'asset_binding': f"canary.appearance:palette/{outfit['look' + slot]}"} for slot in ('Head', 'Body', 'Legs', 'Feet')]})
    return {'profile': creature, 'behavior': behavior, 'presentation': ref('Presentation'),
            'attack_definitions': {'records': records, 'profiles': profiles},
            'monster_melee': {'ability': abilities[0], 'interval_ms': 2000, 'chance_ppm': 1000000,
                              'minimum': 150, 'maximum': 700, 'source_repository': 'Meth28/global',
                              'source_revision': source['sources'][0]['revision'],
                              'source_sha256': source['sources'][0]['response_sha256'],
                              'source_path': 'data/monster/quests/the_order_of_the_lion/fugue.lua',
                              'execution_status': 'approximate_nonlethal_physical_melee'}}, presentation


def admit(root, generated, canonical):
    source = json.loads((root / SOURCE).read_bytes())
    row, presentation = native(source)
    for path, new, key in [(CREATURES, row, lambda r: r['profile']['target']),
                           (PRESENTATIONS, presentation, lambda r: r['target'])]:
        document = json.loads(generated[path])
        if any(key(r) == key(new) for r in document['records']):
            raise ValueError('FUGUE_DUPLICATE_NATIVE_IDENTITY')
        document['records'].append(new)
        document['records'].sort(key=lambda r: tuple(key(r)[k] for k in ('family', 'key', 'revision')))
        generated[path] = canonical(document)
    manifest = json.loads(generated['content/spells.manifest.json'])
    for field, path in [('creature_profiles', CREATURES), ('presentation_profiles', PRESENTATIONS)]:
        manifest[field]['sha256'] = hashlib.sha256(generated[path]).hexdigest()
    generated['content/spells.manifest.json'] = canonical(manifest)


def inventory_extension(root, rows):
    """Review the exact native-only source addition before extending strict pins."""
    selected = [r for r in rows if r['profile']['target']['key'] == 'oteryn:creature.fugue']
    if not selected:
        return 0, 0
    expected, _ = native(json.loads((root / SOURCE).read_bytes()))
    if selected != [expected]:
        raise ValueError('FUGUE_INVENTORY_SOURCE_MISMATCH')
    return 1, 4
