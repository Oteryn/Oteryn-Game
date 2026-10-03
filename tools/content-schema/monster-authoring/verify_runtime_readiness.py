"""Inventory monster execution dependencies without equating admission with gameplay.

Reads a creature-admission stage or a qualified native WorldProject. Reference identity
includes family, key AND revision; a stage uses pre-closure Item keys, so do not combine
it with post-closure native definitions. Use --world-project for the complete native graph.
This is a dependency/coverage check, not a second content validator or interpreter.
An admitted Effect labelled Executable is a content classification, not an execution receipt.
No observed dependency can make runtime_qualified true without a separate E2E qualification.
"""
import argparse
import hashlib
import json
from collections import Counter
from pathlib import Path


EXECUTION_FAMILIES = {'Ability', 'Effect', 'Formula'}
PROFILE_FAMILIES = EXECUTION_FAMILIES | {'Creature', 'Behavior'}
BOUNDARIES = {
    'content_entry': ('apps/game-server/src/content/project/v2/creature.rs',
                      'pub struct ProjectV2AbilityDetails'),
    'player_reader': ('apps/game-server/src/spell/authoring.rs',
                      'pub(crate) fn spell_from_bundle'),
    'player_resolution': ('apps/game-server/src/spell/mod.rs', 'fn resolve_effect('),
    'player_plan': ('apps/game-server/src/spell/plan.rs', 'pub(crate) fn effect_plan('),
    'player_owner_cast': ('apps/game-server/src/spell/cast.rs', 'pub(crate) fn cast('),
    'ai_intent': ('apps/game-server/src/ai_think.rs', 'pub enum ThinkOutcome'),
    'bite_only_commit': ('apps/game-server/src/ability/creature_bite.rs',
                         'pub(crate) fn commit_ai_bite('),
    'composition': ('apps/game-server/src/lib.rs', 'mod ai_think;'),
}


def identity(ref):
    if not isinstance(ref, dict) or set(ref) != {'family', 'key', 'revision'}:
        raise ValueError(f'invalid definition reference: {ref!r}')
    if any(not isinstance(ref[k], str) or not ref[k] for k in ref):
        raise ValueError(f'invalid definition reference: {ref!r}')
    return ref['family'], ref['key'], ref['revision']


def references(value, path='$'):
    if isinstance(value, dict):
        if {'family', 'key', 'revision'} <= value.keys():
            yield path, identity(value)
        else:
            for key, child in sorted(value.items()):
                yield from references(child, f'{path}.{key}')
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from references(child, f'{path}[{index}]')


def index_rows(rows, field):
    result = {}
    for row in rows:
        key = identity(row[field])
        if key in result:
            raise ValueError(f'duplicate {field}: {key}')
        result[key] = row
    return result


def source_boundaries(repo):
    result = {}
    for name, (relative, needle) in BOUNDARIES.items():
        path = repo / relative
        content = path.read_bytes()
        lines = content.decode('utf-8').splitlines()
        matches = [i + 1 for i, line in enumerate(lines) if needle in line]
        if len(matches) != 1:
            raise ValueError(f'boundary {name}: expected one {needle!r} in {path}')
        result[name] = {'path': relative, 'line': matches[0],
                        'sha256': hashlib.sha256(content).hexdigest()}
    return result


def inventory(stage, external_records=()):
    records = index_rows(stage['records'], 'identity')
    for key, row in index_rows(external_records, 'identity').items():
        if key in records and row != records[key]:
            raise ValueError(f'conflicting external definition: {key}')
        records[key] = row
    profiles = index_rows(stage['authoring_profiles'], 'target')
    declared = {identity({'family': row['kind'], **row['identity']})
                for row in stage.get('declarations', [])}
    if not records or not profiles:
        raise ValueError('empty stage cannot qualify')
    for key, row in profiles.items():
        if row['data']['kind'] != key[0]:
            raise ValueError(f'profile family mismatch: {key}')
    cache = {}

    def inspect(key, visiting=()):
        if key in visiting:
            return {'problems': [{'kind': 'ABILITY_REFERENCE_CYCLE', 'identity': key}],
                    'dependencies': [], 'features': []}
        if key in cache:
            return cache[key]
        problems, dependencies, features = [], {key}, []
        record, authored = records.get(key), profiles.get(key)
        if record is None:
            problems.append({'kind': 'MISSING_DEFINITION', 'identity': key})
        if key[0] in PROFILE_FAMILIES and authored is None:
            problems.append({'kind': 'MISSING_AUTHORING_PROFILE', 'identity': key})
        payload = authored['data']['profile'] if authored else {}
        if key[0] == 'Ability' and authored:
            details = payload.get('details')
            if not isinstance(details, dict):
                problems.append({'kind': 'MISSING_ABILITY_DETAILS', 'identity': key})
            else:
                features.extend(f'ability.{name}' for name in
                                ('area', 'variants', 'chain', 'windup', 'encounter') if name in details)
                for effect in details.get('effects', []):
                    if effect.get('kind') == 'Inline':
                        operation = effect['effect']['operation']
                        features.append(f"inline.{operation['operation']}")
                        if operation['operation'] == 'Condition':
                            condition = operation['condition']
                            features.append(f"condition.{condition['condition_type']}")
                            features.extend(f'condition.{name}' for name in
                                            ('damage_over_time', 'speed_formula', 'attribute_modifiers',
                                             'light', 'regeneration', 'buff_spell') if name in condition)
        if key[0] == 'Effect' and record:
            features.append(f"effect.{record['effect_family']}")
        if key[0] == 'Formula' and authored:
            features.append(f"formula.{payload['formula']}")
        for node in (record, payload):
            if node is None:
                continue
            for path, target in references(node):
                if node is record and path == '$.identity':
                    continue
                if target not in records and target not in declared:
                    problems.append({'kind': 'MISSING_REFERENCE', 'from': key,
                                     'path': path, 'identity': target})
                if target[0] in EXECUTION_FAMILIES:
                    child = inspect(target, visiting + (key,))
                    dependencies.update(child['dependencies'])
                    problems.extend(child['problems'])
                    features.extend(child['features'])
        # Record and authored references describe the same dependency in two dialects.
        result = {'problems': list({json.dumps(p, sort_keys=True): p for p in problems}.values()),
                  'dependencies': sorted(dependencies), 'features': sorted(set(features))}
        cache[key] = result
        return result

    creatures, ability_rows = [], {}
    for key in sorted(profiles):
        if key[0] != 'Creature':
            continue
        record = records.get(key)
        profile = profiles[key]['data']['profile']
        problems = []
        abilities = {identity(ref) for ref in profile.get('abilities', [])}
        if record is None:
            problems.append({'kind': 'MISSING_CREATURE_DEFINITION', 'identity': key})
        elif 'behavior' in record:
            behavior_key = identity(record['behavior'])
            behavior = profiles.get(behavior_key)
            if behavior_key not in records or behavior is None:
                problems.append({'kind': 'MISSING_BEHAVIOR', 'identity': behavior_key})
            else:
                for family in ('attacks', 'defenses'):
                    for scheduled in behavior['data']['profile'].get(family, []):
                        target = identity(scheduled['ability'])
                        if target not in abilities:
                            problems.append({'kind': 'SCHEDULE_NOT_IN_CREATURE_ABILITIES',
                                             'identity': target, 'schedule': family})
                        abilities.add(target)
        for target in sorted(abilities):
            result = inspect(target)
            problems.extend(result['problems'])
            ability_rows[target] = {'identity': target, **result,
                                   'runtime_status': 'UNQUALIFIED_CREATURE_DISPATCH'}
        creatures.append({'identity': key, 'ability_count': len(abilities),
                          'abilities': sorted(abilities), 'problems': problems,
                          'runtime_qualified': False,
                          'runtime_status': 'UNQUALIFIED_CREATURE_DISPATCH' if abilities
                          else 'NO_AUTHORED_ABILITIES_NOT_A_RUNTIME_RECEIPT'})
    if not creatures:
        raise ValueError('stage contains no creature profiles')
    scheduled_count = len(ability_rows)
    for key, result in cache.items():
        if key[0] == 'Ability':
            ability_rows.setdefault(key, {'identity': key, **result,
                                         'runtime_status': 'UNQUALIFIED_CREATURE_DISPATCH'})
    rows = [ability_rows[key] for key in sorted(ability_rows)]
    return {'schema': 'monster-runtime-readiness.v1',
            'qualification_scope': 'native dependency closure; no execution/parity qualification',
            'runtime_qualified': False,
            'blocking_boundary': 'Creature/Behavior schedules -> native Ability profile consumer '
                                 '-> live Channel owner dispatch has no qualification receipt.',
            'existing_partial_paths': {
                'player': 'spell_from_bundle -> resolve_effect -> effect_plan; separate player dialect',
                'player_owner': 'cast rejects side effects other than RemoveCondition; '
                                'V1 book loads only three authored player spells',
                'ai': 'ThinkOutcome::AttackIntent -> manually supplied CreatureBiteDefinition '
                      '-> commit_ai_bite; fixed-one-bite proof, HP floor 1',
                'inline': 'ProjectV2AbilityEffect::Inline is candidate content; not executable gameplay'},
            'counts': {'creatures': len(creatures), 'unique_reachable_abilities': len(rows),
                       'unique_direct_creature_abilities': scheduled_count,
                       'creatures_with_dependency_problems': sum(bool(c['problems']) for c in creatures),
                       'creatures_without_abilities': sum(c['ability_count'] == 0 for c in creatures),
                       'runtime_qualified_creatures': 0,
                       'features': dict(sorted(Counter(f for row in rows for f in row['features']).items()))},
            'creatures': creatures, 'abilities': rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--stage', type=Path)
    source.add_argument('--world-project', type=Path)
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--external-records', type=Path,
                        help='Qualified native definitions/reference.json (includes referenced Items).')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    inputs = {}
    if args.world_project:
        if args.external_records:
            parser.error('--external-records applies only to --stage')
        reference = args.world_project / 'definitions/reference.json'
        declarations = args.world_project / 'definitions/declarations.json'
        raw, declaration_bytes = reference.read_bytes(), declarations.read_bytes()
        declared = json.loads(declaration_bytes)
        stage = {'records': json.loads(raw)['records'],
                 'declarations': declared['records'],
                 'authoring_profiles': declared['authoring_profiles']}
        inputs['reference_sha256'] = hashlib.sha256(raw).hexdigest()
        inputs['declarations_sha256'] = hashlib.sha256(declaration_bytes).hexdigest()
    else:
        raw = args.stage.read_bytes()
        stage = json.loads(raw)
        inputs['stage_sha256'] = hashlib.sha256(raw).hexdigest()
    external = args.external_records.read_bytes() if args.external_records else None
    report = inventory(stage, json.loads(external)['records'] if external else ())
    report['inputs'] = {**inputs, 'source_boundaries': source_boundaries(args.repo)}
    if external is not None:
        report['inputs']['external_records_sha256'] = hashlib.sha256(external).hexdigest()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report['counts'], ensure_ascii=False, sort_keys=True))
    return int(bool(report['counts']['creatures_with_dependency_problems']))


if __name__ == '__main__':
    raise SystemExit(main())
