#!/usr/bin/env python3
"""Add explicitly flagged melee facts to a NEW candidate; never rewrite preserved inputs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import math


def identity(ref):
    return ref['key'], ref['revision']



CANARY_PIN = '99902524e052f37574194466c2949c576e4ab269'
BUILTIN_SECONDARY_CONDITIONS = {'poison', 'fire', 'freezing'}


def qualify_melee_skill_engine(source):
    if subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD']).decode().strip() != CANARY_PIN:
        raise ValueError('melee skill source checkout differs from pinned revision')
    sources = []
    expected = {
        'src/creatures/monsters/monsters.hpp': [b'int32_t attack = 0;', b'int32_t skill = 0;'],
        'src/creatures/monsters/monsters.cpp': [
            b'if (spell->attack > 0 && spell->skill > 0)',
            b'sb.minCombatValue = 0;',
            b'sb.maxCombatValue = -Weapons::getMaxMeleeDamage(spell->skill, spell->attack);'],
        'src/items/weapons/weapons.cpp': [
            b'int32_t Weapons::getMaxMeleeDamage(int32_t attackSkill, int32_t attackValue)',
            b'return static_cast<int32_t>(std::ceil((attackSkill * (attackValue * 0.05)) + (attackValue * 0.5)));'],
    }
    for path, fragments in expected.items():
        raw = subprocess.check_output(['git', '-C', str(source), 'show', f'{CANARY_PIN}:{path}'])
        blob = subprocess.check_output(['git', '-C', str(source), 'rev-parse', f'{CANARY_PIN}:{path}']).decode().strip()
        if hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest() != blob or any(v not in raw for v in fragments):
            raise ValueError('melee skill engine bytes/formula do not qualify')
        sources.append({'repository': 'opentibiabr/canary', 'revision': CANARY_PIN, 'path': path,
                        'git_blob': blob, 'sha256': hashlib.sha256(raw).hexdigest(), 'bytes': len(raw)})
    return {'formula': 'maximum=ceil(skill*(attack*0.05)+attack*0.5); minimum=0',
            'arithmetic': 'source binary64 expression order; exact integer inputs before conversion', 'sources': sources,
            'distribution': 'approximate_owner_bounded_roll_not_Canary_normal_random',
            'mitigation': 'armor_shield_mitigation_not_applied'}


def melee_skill_range(formula):
    melee = formula.get('melee', {})
    attack, skill = melee.get('attack'), melee.get('skill')
    if type(attack) is not int or type(skill) is not int or not (0 < attack <= 0x7FFFFFFF and 0 < skill <= 0x7FFFFFFF):
        return None
    # Match Canary's actual binary64 expression order. Rational arithmetic
    # would change ceil at real source edges (attack61/skill30 ->123, not122).
    raw_maximum = skill * (attack * 0.05) + attack * 0.5
    if not math.isfinite(raw_maximum):
        return None
    maximum = math.ceil(raw_maximum)
    if maximum > 0x7FFFFFFF:
        return None
    return 0, maximum

def select_melee(monster, dependencies, behavior, allow_skill=False, allow_secondary_conditions=False):
    """Require exact source ordinal and schedule; disable ambiguous/advanced effects."""
    source_attacks = monster['behavior'].get('attacks', [])
    abilities = {identity(a['identity']): a for a in dependencies.get('abilities', [])}
    effects = {identity(e['identity']): e for e in dependencies.get('effects', [])}
    formulas = {identity(f['identity']): f for f in dependencies.get('formulas', [])}
    for index, schedule in enumerate(source_attacks):
        ability = abilities.get(identity(schedule['ability']))
        if not ability or ability.get('kind') != 'melee' or ability.get('range_tiles') != 1:
            continue
        refs = ability.get('effects', [])
        if not refs:
            continue
        omitted = []
        if len(refs) > 1:
            if not allow_secondary_conditions:
                continue
            omitted = [effects.get(identity(r)) for r in refs[1:]]
            if any(not e or e.get('operation') != 'condition' or e.get('condition', {}).get('type') not in BUILTIN_SECONDARY_CONDITIONS for e in omitted):
                continue
        effect = effects.get(identity(refs[0]))
        if not effect or effect.get('operation') != 'damage' or effect.get('damage_type') != 'physical':
            continue
        formula = formulas.get(identity(effect['formula']))
        if not formula:
            continue
        if formula.get('kind') == 'range':
            magnitude = formula['magnitude']
            minimum, maximum = magnitude['minimum'], magnitude['maximum']
        elif allow_skill and formula.get('kind') == 'melee_attack_skill':
            endpoints = melee_skill_range(formula)
            if endpoints is None:
                continue
            minimum, maximum = endpoints
        else:
            continue
        if index >= len(behavior['attacks']):
            continue
        selected = behavior['attacks'][index]
        chance = schedule.get('chance_percent', 100) * 10_000
        if selected['interval_ms'] != schedule['interval_ms'] or selected['chance_ppm'] != chance:
            continue
        # Preserve source slot ordering explicitly; never bind a different effect
        # simply because the species name or its translated label happens to match.
        donor_suffix = schedule['ability']['key'].rsplit('/', 1)[-1]
        target_suffix = selected['ability']['key'].rsplit('/', 1)[-1].rsplit('.', 1)[-1]
        if donor_suffix != target_suffix:
            continue
        if type(minimum) is not int or type(maximum) is not int or not (0 <= minimum <= maximum <= 0xFFFFFFFF and maximum > 0):
            continue
        return index, selected, schedule, ability, effect, formula, minimum, maximum, omitted
    return None


def build(creatures, bundle_root, source_root, pack, melee_skill_engine=None, enable_secondary_conditions=False):
    population = {}
    for path in sorted((bundle_root / pack).rglob('monster.json')):
        monster = json.loads(path.read_text())
        name = monster['creature']['display_name'].casefold()
        population.setdefault(name, []).append((path, monster))
    source_name = pack.split('/')[0]
    index_path = source_root.parent / f'{source_name}-monster-files.json'
    source_index = json.loads(index_path.read_text())
    indexed = {(r['provenance']['repository'], r['provenance']['revision'], r['provenance']['path']): r['provenance']
               for r in source_index}
    result = copy.deepcopy(creatures)
    report = {'schema': 'OTERYN_MONSTER_MELEE_DERIVATION/v1', 'pack': pack,
              'execution_status': 'approximate_nonlethal_physical_melee',
              'source_index_sha256': hashlib.sha256(index_path.read_bytes()).hexdigest(),
              'limitations': ['No armor/shield mitigation in accepted AI-4 bite owner',
                              'Player health remains at least 1 (D54)',
                              'One selected melee slot per creature; ranged/defenses/custom disabled',
                              'No spawn or chase qualification implied',
                              'Damage distribution remains approximate; owner bounded roll differs from Canary normal_random',
                              'Secondary builtin conditions may be explicitly omitted, never claimed applied'], 'records': []}
    if melee_skill_engine is not None:
        report['melee_skill_engine'] = melee_skill_engine
    for record in result['records']:
        record.pop('monster_melee', None)
        profile = record['profile']['data']['profile']
        name = profile['details']['display_name'].casefold()
        choices = population.get(name, [])
        observation = {'creature': record['profile']['target'], 'status': 'disabled_no_unique_source'}
        report['records'].append(observation)
        if len(choices) != 1 or not record.get('behavior'):
            continue
        path, monster = choices[0]
        if profile['health'] != monster['creature']['stats']['max_health']:
            observation['status'] = 'disabled_source_health_conflict'
            continue
        dependencies_path = path.with_name('dependencies.json')
        dependencies = json.loads(dependencies_path.read_text())
        chosen = select_melee(monster, dependencies, record['behavior']['data']['profile'],
                              allow_skill=melee_skill_engine is not None,
                              allow_secondary_conditions=enable_secondary_conditions)
        if chosen is None:
            observation['status'] = 'disabled_no_compatible_range_melee_closure'
            continue
        index, selected, donor, ability, effect, formula, minimum, maximum, omitted = chosen
        manifest_path = path.with_name('manifest.json')
        manifest = json.loads(manifest_path.read_text())
        provenance = [e for e in manifest['entries']
                      if e.get('destination') == f'/monster/behavior/attacks/{index}'
                      and e.get('status') == 'mapped']
        if len(provenance) != 1 or len(manifest['sources']) != 1:
            observation['status'] = 'disabled_source_provenance_ambiguous'
            continue
        source = manifest['sources'][0]
        source_file = provenance[0]['source_file']
        raw_source = source_root / pack.split('/')[0] / source_file
        raw = raw_source.read_bytes()
        source_sha = hashlib.sha256(raw).hexdigest()
        attested = indexed[(source['repository'], source['revision'], source_file)]
        git_blob = hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest()
        if source_sha != attested['sha256'] or len(raw) != attested['bytes'] or git_blob != attested['git_blob']:
            raise ValueError('source bytes differ from the frozen per-file provenance index')
        record['monster_melee'] = {
            'ability': selected['ability'], 'interval_ms': selected['interval_ms'],
            'chance_ppm': selected['chance_ppm'],
            'minimum': minimum, 'maximum': maximum,
            'source_repository': source['repository'], 'source_revision': source['revision'],
            'source_path': source_file, 'source_sha256': source_sha,
            'execution_status': 'primaryphysicalonly_omits_secondary_conditions' if omitted else report['execution_status'],
        }
        observation.update(status='enabled_approximate_melee', source=source,
            source_path=source_file, source_sha256=source_sha,
            source_slot=index + 1, target_ability=selected['ability'], donor_ability=donor['ability'],
            execution_status=record['monster_melee']['execution_status'],
            native_range={'minimum': minimum, 'maximum': maximum},
            disabled_secondary_conditions=omitted,
            dependencies={'ability': ability, 'effect': effect, 'formula': formula},
            bundle_hashes={p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                           for p in (path, dependencies_path, manifest_path)})
    report['enabled'] = sum(r['status'] == 'enabled_approximate_melee' for r in report['records'])
    report['disabled'] = len(report['records']) - report['enabled']
    return result, report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--creatures', required=True, type=Path)
    parser.add_argument('--bundle-root', required=True, type=Path)
    parser.add_argument('--source-root', required=True, type=Path)
    parser.add_argument('--pack', default='canary/data-otservbr-global/monster')
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--canary-source', type=Path)
    parser.add_argument('--enable-secondary-condition-omission', action='store_true')
    args = parser.parse_args()
    if args.out.exists():
        parser.error('output directory already exists; preserve earlier candidates')
    engine = qualify_melee_skill_engine(args.canary_source) if args.canary_source else None
    creatures, report = build(json.loads(args.creatures.read_text()), args.bundle_root, args.source_root, args.pack,
                              engine, args.enable_secondary_condition_omission)
    args.out.mkdir(parents=True)
    for name, document in [('creature-profiles.json', creatures), ('monster-melee-derivation.json', report)]:
        (args.out / name).write_text(json.dumps(document, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'enabled_approximate_melee': report['enabled'], 'disabled': report['disabled']}))


if __name__ == '__main__':
    main()
