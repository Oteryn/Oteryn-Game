"""Build the complete, schema-qualified 67-profile native spell regression catalogue.

Coverage identities are transcribed from the r18 native-work worker queue, rather
than inferred from a count. Generated profiles are mechanics evidence, never live
cast admission or domain commit authority.
"""
import argparse
import copy
import json
import re
from pathlib import Path

from jsonschema import Draft202012Validator

import native_catalog
from validate_spell import validate, walk

REVISION = 'spell-p2-r20'
ITEM_REVISION = 'spell-p2-r21'
# Exact primary_spell_ids from implementation-handoff-r18/native-work/worker-queue.json.
EXPECTED_IDS = frozenset("""
instant-avatar_of_balance
instant-avatar_of_light
instant-avatar_of_nature
instant-avatar_of_steel
instant-avatar_of_storm
instant-balanced_brawl
instant-blood_rage
instant-challenge
instant-charge
instant-chivalrous_challenge
instant-creature_illusion
instant-death_echo
instant-divine_dazzle
instant-divine_empowerment
instant-divine_grenade
instant-druid_familiar
instant-energy_beam
instant-energy_wave
instant-executioner_s_throw
instant-find_fiend
instant-find_person
instant-flurry_of_blows
instant-focus_harmony
instant-focus_serenity
instant-food
instant-front_sweep
instant-great_energy_beam
instant-haste
instant-house_door_list
instant-house_guest_list
instant-house_kick
instant-house_subowner_list
instant-ice_burst
instant-knight_familiar
instant-levitate
instant-magic_rope
instant-magic_shield
instant-mass_healing
instant-mass_spirit_mend
instant-monk_familiar
instant-paladin_familiar
instant-protector
instant-sharpshooter
instant-shield_bash
instant-shield_slam
instant-sorcerer_familiar
instant-spiritual_outburst
instant-strong_haste
instant-strong_ice_wave
instant-summon_creature
instant-summon_druid_familiar
instant-summon_knight_familiar
instant-summon_paladin_familiar
instant-summon_sorcerer_familiar
instant-sweeping_takedown
instant-swift_foot
instant-terra_burst
instant-virtue_of_harmony
instant-virtue_of_justice
instant-virtue_of_sustain
rune-animate_dead_rune
rune-chameleon_rune
rune-convince_creature_rune
rune-desintegrate_rune
rune-destroy_field_rune
rune-magic_wall_rune
rune-wild_growth_rune
""".strip().splitlines())
BARRIER_IDS = frozenset({'rune-magic_wall_rune', 'rune-wild_growth_rune'})


def expected_operations():
    """Bind every coverage identity to its independently declared family owner."""
    import native_actor_states as actor
    import native_combat as combat
    import native_companions as companions
    import native_delayed as delayed
    import native_house_movement as house
    import native_world_items as items

    result = {}

    def register(carrier, name, operation):
        identity = carrier + '-' + re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')
        if identity in result:
            raise ValueError('duplicate native operation ownership: ' + identity)
        result[identity] = operation

    for models in (actor.MODELS, combat.TEMPLATES, house.BEHAVIOURS):
        for name, model in models.items():
            register('instant', name, model['key'])
    for name in delayed.FILES:
        register('instant', name, delayed._snapshot(name)['key'])
    for name, spec in companions.SPECS.items():
        source = 'canary' if 'canary' in spec['sources'] else 'crystal'
        register(spec['carrier'], name, companions._recipe(name, source)['key'])
    for name, (carrier, _) in items.PATHS.items():
        operation = ('create_item_barrier' if name in ('magic wall rune', 'wild growth rune')
                     else 'random_item_grant' if name == 'food' else 'tile_item_operation')
        register(carrier, name, operation)
    if set(result) != EXPECTED_IDS:
        raise ValueError('declared native operation owners do not match exact 67-record coverage')
    return result


def check_operation_owner(profile, owners):
    identity = profile_id(profile)
    if identity not in owners:
        raise ValueError('native coverage mismatch: unowned profile ' + identity)
    expected = owners[identity]
    native = profile['execution'].get('native_behavior')
    if expected != 'create_item_barrier':
        if not isinstance(native, dict) or native.get('key') != expected:
            raise ValueError(f'{identity}: native operation owner must be {expected}')
        return
    if native is not None or 'ability' not in profile['execution']:
        raise ValueError(identity + ': barrier must execute its declared Ability')
    effects = profile['dependencies']['effects']
    expected_ids = (2128, 10181) if identity == 'rune-magic_wall_rune' else (2130, 10182)
    duration = (16000, 24000) if identity == 'rune-magic_wall_rune' else (30000, 60000)
    if len(effects) != 1:
        raise ValueError(identity + ': barrier needs one exact create_item effect')
    effect = effects[0]
    for field, number in zip(('created_item', 'pvp_safe_item'), expected_ids):
        if effect.get(field) != {'family': 'Item', 'key': f'candidate:item/{number}', 'revision': ITEM_REVISION}:
            raise ValueError(identity + ': wrong barrier ' + field + ' binding')
    if (effect.get('operation') != 'create_item'
            or effect.get('duration_range_ms') != {'minimum': duration[0], 'maximum': duration[1]}):
        raise ValueError(identity + ': wrong barrier operation or duration binding')


def profile_id(profile):
    return profile['carrier'] + '-' + re.sub(r'[^a-z0-9]+', '_', profile['name'].lower()).strip('_')


def check_revisions(value, label):
    for path, node in walk(value):
        if isinstance(node, dict) and 'revision' in node and ('key' in node or 'family' in node):
            expected = ITEM_REVISION if node.get('family') == 'Item' else REVISION
            if node['revision'] != expected:
                raise ValueError(f'{label}/{"/".join(map(str, path))}: expected exact {expected} reference')


def profile_from_bundle(directory):
    directory = Path(directory)
    documents = {name: json.loads((directory / (name + '.json')).read_text(encoding='utf-8'))
                 for name in ('spell', 'dependencies', 'catalog', 'manifest')}
    spell = documents['spell']['spell']
    execution = spell['execution']
    native = execution.get('native_behavior')
    schemas = native_catalog.schemas()
    if native is not None:
        key = native.get('key')
        if key in ('party_buff', 'unresolved'):
            return None
        if key not in schemas:
            raise ValueError(f'{directory.name}: unknown native behavior {key!r}')
    barrier = any(effect.get('operation') == 'create_item' and 'pvp_safe_item' in effect
                  for effect in documents['dependencies']['effects'])
    if native is None and not barrier:
        return None
    check_revisions(documents['spell'], directory.name + '/spell')
    check_revisions(documents['dependencies'], directory.name + '/dependencies')
    check_revisions(documents['catalog'], directory.name + '/catalog')
    errors = validate(documents['spell'], documents['dependencies'], documents['catalog'], documents['manifest'])
    if errors:
        raise ValueError(directory.name + ': ' + '; '.join(errors))
    profile = {'name': spell['name'], 'carrier': spell['carrier'], 'spell': copy.deepcopy(spell),
               'execution': copy.deepcopy(execution), 'dependencies': copy.deepcopy(documents['dependencies'])}
    identity = profile_id(profile)
    if barrier != (identity in BARRIER_IDS):
        raise ValueError(f'{identity}: wrong barrier coverage classification')
    check_operation_owner(profile, expected_operations())
    if directory.name != identity:
        raise ValueError(f'{directory.name}: profile identity disagrees with directory ({identity})')
    return profile


def assemble(profiles):
    seen = set()
    schemas = native_catalog.schemas()
    owners = expected_operations()
    native_count = barrier_count = 0
    for profile in profiles:
        if set(profile) != {'name', 'carrier', 'spell', 'execution', 'dependencies'}:
            raise ValueError('native profile envelope fields differ')
        if (profile['name'] != profile['spell']['name'] or profile['carrier'] != profile['spell']['carrier']
                or profile['execution'] != profile['spell']['execution']):
            raise ValueError('native profile name/carrier/execution disagrees with spell')
        check_revisions(profile['spell'], profile_id(profile) + '/spell')
        check_revisions(profile['dependencies'], profile_id(profile) + '/dependencies')
        identity = profile_id(profile)
        check_operation_owner(profile, owners)
        execution = profile['execution']
        native = execution.get('native_behavior')
        barrier = any(effect.get('operation') == 'create_item' and 'pvp_safe_item' in effect
                      for effect in profile['dependencies']['effects'])
        if barrier != (identity in BARRIER_IDS):
            raise ValueError(identity + ': wrong barrier coverage classification')
        if native is not None:
            if set(native) != {'key', 'parameters'} or native.get('key') not in schemas:
                raise ValueError(identity + ': unknown or malformed native behavior')
            Draft202012Validator(schemas[native['key']]).validate(native['parameters'])
            native_count += 1
        elif barrier:
            barrier_count += 1
        else:
            raise ValueError(identity + ': profile lacks a native or barrier operation')
        if identity in seen:
            raise ValueError('duplicate native profile: ' + identity)
        seen.add(identity)
    if seen != EXPECTED_IDS:
        raise ValueError(f'native coverage mismatch: missing={sorted(EXPECTED_IDS - seen)}, extra={sorted(seen - EXPECTED_IDS)}')
    # The exact identity set ensures 65 native operations plus two barrier profiles.
    if len(seen) != 67 or native_count != 65 or barrier_count != 2:
        raise ValueError('native catalogue must contain 65 native plus two barrier profiles')
    return {'revision': REVISION, 'profiles': sorted(profiles, key=lambda profile: (profile['carrier'], profile['name']))}


def build(bundles):
    profiles = []
    for directory in sorted(Path(bundles).iterdir()):
        if directory.is_dir():
            profile = profile_from_bundle(directory)
            if profile is not None:
                profiles.append(profile)
    return assemble(profiles)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bundles', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    document = build(args.bundles)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(f'wrote {len(document["profiles"])} qualified native profiles to {args.out}')


if __name__ == '__main__':
    main()
