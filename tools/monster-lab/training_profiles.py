#!/usr/bin/env python3
"""Generate isolated, explicitly simplified arena bundles; never edit canonical inputs."""
from __future__ import annotations
import argparse
import copy
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/monster-authoring'))
import validate_monster as validator
sys.path.insert(0, str(ROOT / 'tools/content-migration'))
import creature_admission_stage as admission

FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')
REVISION = 'lab-training-r1'


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def refs(value):
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            yield value['family'], value['key'], value['revision']
        else:
            for child in value.values():
                yield from refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs(child)


def identity(family, value):
    return family, value['key'], value['revision']


def local_records(monster, dependencies):
    records = {}
    for section, family in validator.FAMILIES.items():
        values = [monster[section]] if section in monster else dependencies.get(section, [])
        for value in values:
            records[identity(family, value['identity'])] = value
    return records


def unsupported_defenses(monster, dependencies):
    """Native scheduler rejects melee only in defenses; attack use stays supported."""
    abilities = {identity('Ability', a['identity']): a for a in dependencies.get('abilities', [])}
    return [{'index': i, 'ability': copy.deepcopy(row['ability']),
             'action': copy.deepcopy(row), 'reason': 'native_defence_melee'}
            for i, row in enumerate(monster['behavior'].get('defenses', []))
            if abilities.get(identity('Ability', row['ability']), {}).get('kind') == 'melee']


def simplify(monster, dependencies, catalog, actor, creature_overrides=None, unavailable_refs=None):
    """Remove unsafe Encounter chains, retain exact reachable local dependency closure."""
    monster, dependencies, catalog = copy.deepcopy((monster, dependencies, catalog))
    records = local_records(monster, dependencies)
    unavailable_refs = set(unavailable_refs or ())
    dependency_families = {'Ability', 'Effect', 'Formula', 'Item', 'Document'}
    unsafe = {key for key, record in records.items() if key[0] in dependency_families and
              (key in unavailable_refs or any(r[0] == 'Encounter' or r in unavailable_refs for r in refs(record)))}
    while True:
        extra = {key for key, record in records.items() if key[0] in dependency_families and
                 any(r in unsafe for r in refs(record))}
        if extra <= unsafe:
            break
        unsafe |= extra
    disabled = [{'family': k[0], 'key': k[1], 'revision': k[2],
                 'reason': 'depends_on_disabled_encounter_or_unavailable_reference',
                 'original_ability': copy.deepcopy(records[k])} for k in sorted(unsafe) if k[0] == 'Ability']
    unsafe_abilities = {k for k in unsafe if k[0] == 'Ability'}
    # Creature and Behavior referencing a removed Ability are repaired explicitly below.
    creature, behavior = monster['creature'], monster['behavior']
    if 'summons' in behavior:
        summons = behavior['summons']
        kept = []
        for entry in summons['entries']:
            if identity('Creature', entry['creature']) in unavailable_refs:
                disabled.append({'scope': 'summon_action_only', 'reason': 'unavailable_summon_target',
                                 'action': copy.deepcopy(entry)})
            else:
                kept.append(entry)
        if kept:
            summons['entries'] = kept
        else:
            behavior.pop('summons')
    familiar = creature.get('summoning', {}).get('familiar')
    if familiar and identity('Ability', familiar['summon_ability']) in unavailable_refs:
        disabled.append({'scope': 'familiar_companion_contract', 'reason': 'unavailable_master_summon_ability',
                         'action': copy.deepcopy(creature['summoning'])})
        creature['summoning'].pop('familiar')
        creature['summoning']['is_familiar'] = False
    corpse = creature.get('corpse_item')
    if corpse and identity('Item', corpse) in unsafe:
        disabled.append({'scope': 'corpse_and_decay', 'reason': 'unregistered_decay_target',
                         'action': copy.deepcopy(corpse),
                         'original_items': [copy.deepcopy(v) for k, v in records.items() if k[0] == 'Item' and k in unsafe]})
        creature.pop('corpse_item')
    if 'loot' in monster:
        kept = []
        for entry in monster['loot']['entries']:
            if identity('Item', entry['item']) in unavailable_refs:
                disabled.append({'scope': 'loot_entry_only', 'reason': 'unregistered_loot_item',
                                 'action': copy.deepcopy(entry)})
            else:
                kept.append(entry)
        monster['loot']['entries'] = kept
    unsupported = unsupported_defenses(monster, dependencies)
    disabled.extend({'family': 'Ability', **row['ability'], 'reason': row['reason'],
                     'scope': 'defense_action_only', 'original_index': row['index'],
                     'action': row['action']} for row in unsupported)
    removed_indices = {row['index'] for row in unsupported}
    behavior['defenses'] = [row for i, row in enumerate(behavior.get('defenses', []))
                            if i not in removed_indices]
    creature['abilities'] = [r for r in creature.get('abilities', []) if identity('Ability', r) not in unsafe_abilities]
    if not creature['abilities']:
        creature.pop('abilities', None)
    for section in ('attacks', 'defenses'):
        behavior[section] = [row for row in behavior.get(section, [])
                             if identity('Ability', row['ability']) not in unsafe_abilities]
    reward = creature.pop('reward_encounter', None)
    reachable = set(refs(monster))
    while True:
        expanded = reachable | {r for key in reachable if key in records for r in refs(records[key])}
        if expanded == reachable:
            break
        reachable = expanded
    for section, values in dependencies.items():
        family = validator.FAMILIES[section]
        dependencies[section] = [v for v in values if identity(family, v['identity']) in reachable]
    remaining = local_records(monster, dependencies)
    used = set(refs([monster, dependencies]))
    catalog['definitions'] = [r for r in catalog.get('definitions', []) if identity(r['family'], r) in used]
    # Every local non-Item definition receives a private identity. Shared Item identities stay exact.
    mapping = dict(creature_overrides or {})
    for family, key, revision in remaining:
        if family != 'Item':
            prefix, _, path = key.partition('/')
            mapping[(family, key, revision)] = (family, f'{prefix}/lab/{actor}/{path}', REVISION)

    def rekey(value, family=None):
        if isinstance(value, dict):
            if set(value) == {'family', 'key', 'revision'}:
                target = mapping.get(identity(value['family'], value))
                if target:
                    value.update(family=target[0], key=target[1], revision=target[2])
            else:
                if family and 'identity' in value:
                    target = mapping.get(identity(family, value['identity']))
                    if target:
                        value['identity'].update(key=target[1], revision=target[2])
                for child in value.values():
                    rekey(child)
        elif isinstance(value, list):
            for child in value:
                rekey(child)
    for section, family in validator.FAMILIES.items():
        for value in ([monster[section]] if section in monster else dependencies.get(section, [])):
            rekey(value, family)
    rekey(catalog)
    if any(r[0] == 'Encounter' for r in refs([monster, dependencies])):
        raise ValueError('Encounter reference remains after simplification')
    return monster, dependencies, catalog, disabled, reward


def generate(index_path, stage_path, bundle_root, item_map_path, output):
    bundle_root, output = Path(bundle_root).resolve(), Path(output).resolve()
    if output == bundle_root or bundle_root in output.parents or output in bundle_root.parents:
        raise ValueError('Training output must be separate from canonical bundle tree')
    if output.exists() and any(output.iterdir()):
        raise ValueError('Training output must be empty (no stale artifacts)')
    index, stage = read(index_path), read(stage_path)
    encounter_selected = set(stage['deferred']['encounter'])
    rows = {r['monster']: r for r in index['monsters']}
    defense_selected = set()
    for name, indexed in rows.items():
        directory = bundle_root / name
        if directory.parent != bundle_root or admission.bundle_digest(directory) != indexed['sha256']:
            raise ValueError(f'Input bundle identity/digest mismatch: {name}')
        if unsupported_defenses(read(directory / 'monster.json'), read(directory / 'dependencies.json')):
            defense_selected.add(name)
    reference_selected = {r['monster'] for r in stage['deferred']['unresolved_reference']}
    item_selected = {r['monster'] for r in stage['deferred']['unregistered_items']}
    selected = sorted(encounter_selected | defense_selected | reference_selected | item_selected)
    original = {}
    global_mapping = {}
    for name in selected:
        directory = bundle_root / name
        if directory.parent != bundle_root or admission.bundle_digest(directory) != rows[name]['sha256']:
            raise ValueError(f'Input bundle identity/digest mismatch: {name}')
        original[name] = [read(directory / f) for f in FILES]
        ident = original[name][0]['creature']['identity']
        prefix, _, path = ident['key'].partition('/')
        global_mapping[identity('Creature', ident)] = ('Creature', f'{prefix}/lab/{name}/{path}', REVISION)
    output.mkdir(parents=True, exist_ok=True)
    report = {'schema': 'OTERYN_MONSTER_LAB_TRAINING/v1', 'runtime_activated': False,
              'dataset': 'isolated-arena-training', 'canonical_inputs_unchanged': True,
              'encounter_mechanics_enabled': False,
              'provenance_manifest_scope': 'unchanged_max_health_anchor_only; original digest retained in report',
              'inputs_sha256': {str(Path(p)): hashlib.sha256(Path(p).read_bytes()).hexdigest()
                                for p in (index_path, stage_path, item_map_path)}, 'monsters': []}
    passed = []
    for name in selected:
        row = {'monster': name, 'source_bundle_sha256': rows[name]['sha256'],
               'flags': ['SIMPLIFIED_TEST_PROFILE', 'GAMEPLAY_NOT_VERIFIED'] +
                        (['ENCOUNTER_MECHANICS_DISABLED'] if name in encounter_selected else []) +
                        (['DISABLED_MECHANIC:native_defence_melee'] if name in defense_selected else [])}
        try:
            missing_native = {ref for entry in stage['deferred']['unresolved_reference']
                              if entry['monster'] == name for ref in entry['references']}
            missing_item_ids = {item for entry in stage['deferred']['unregistered_items']
                                if entry['monster'] == name for item in entry['items']}
            unavailable = {ref for ref in refs(original[name][:2]) if
                           (ref[0] != 'Item' and ref not in global_mapping and
                            admission.Mapper({}).key(ref[0], ref[1]) in missing_native) or
                           (ref[0] == 'Item' and int(ref[1].rsplit('/', 1)[1]) in missing_item_ids)}
            row['unavailable_references'] = [dict(family=f, key=k, revision=r) for f, k, r in sorted(unavailable)]
            m, d, c, disabled, reward = simplify(*original[name][:3], name, global_mapping, unavailable)
            row['flags'].extend(sorted({'DISABLED_MECHANIC:' + entry['reason'] for entry in disabled}))
            manifest = {'sources': original[name][3]['sources'], 'entries': [
                copy.deepcopy(entry) for entry in original[name][3]['entries']
                if entry.get('destination') == '/monster/creature/stats/max_health']}
            if not manifest['entries']:
                raise ValueError('Missing unchanged max-health provenance anchor')
            errors = validator.validate(m, d, c, manifest)
            row.update(disabled_abilities=[entry for entry in disabled if entry.get('family') == 'Ability' and entry.get('scope') != 'defense_action_only'],
                       disabled_other_actions=[entry for entry in disabled if entry.get('scope') not in (None, 'defense_action_only')],
                       disabled_defense_actions=[entry for entry in disabled if entry.get('scope') == 'defense_action_only'],
                       disabled_reward_encounter=reward,
                       excluded_encounter_covers=name in encounter_selected, validation_errors=errors)
            if errors:
                row['status'] = 'BLOCKED_SCHEMA_REFERENCE'
            else:
                directory = output / name
                directory.mkdir()
                for f, value in zip(FILES, (m, d, c, manifest)):
                    write(directory / f, value)
                row['status'] = 'SCHEMA_PASS_AWAITING_NATIVE_CLOSURE'
                row['bundle_sha256'] = admission.bundle_digest(directory)
                passed.append({'monster': name, 'file': rows[name]['file'], 'sha256': row['bundle_sha256']})
        except (ValueError, KeyError) as exc:
            row.update(status='BLOCKED', error=str(exc))
        report['monsters'].append(row)
    # Recheck originals after output generation; artifacts are never production admission.
    for name in selected:
        if admission.bundle_digest(bundle_root / name) != rows[name]['sha256']:
            raise ValueError(f'Canonical bundle changed during run: {name}')
    # Use the real stage mapper/profile converter, then greatest-fixed-point exact closure.
    export = read(item_map_path)
    item_map = {r['source_item_id']: r['native_key'] for r in export['records']}
    for path in admission.ITEM_REKEYS:
        evidence = read(path)['source_identity']
        item_map[evidence['source_item_id']] = evidence['target_native_key']
    appearance_index, appearances = admission.load_admitted()
    item_map = admission.resolve_admitted_item_map(
        item_map, read(admission.REFERENCE)['records'], read(admission.ITEM_ALIASES)['entries'],
        read(admission.ITEM_BINDINGS)['bindings'],
        {entry[0] for entry in appearances[appearance_index['newest']]['entries']})
    mapper = admission.Mapper(item_map)
    probes = {}
    for row in passed:
        name = row['monster']
        try:
            probe = admission.Stage(mapper)
            probe.stage_dependencies(read(output / name / 'dependencies.json'), name)
            probe.stage_monster(read(output / name / 'monster.json'), row['file'])
            probes[name] = probe
        except (ValueError, KeyError, admission.StageError) as exc:
            next(r for r in report['monsters'] if r['monster'] == name).update(
                status='BLOCKED_NATIVE_CONVERSION', error=str(exc))
    baseline = {(r['identity']['family'], r['identity']['key'], r['identity']['revision']) for r in stage['records']}
    active = set(probes)
    while True:
        available = baseline | {identity(r['identity']['family'], r['identity'])
                                for name in active for r in probes[name].records.values()}
        rejected = {name: sorted(set(admission.exact_definition_refs(
            [probes[name].records, probes[name].profiles])) - available) for name in active}
        rejected = {name: missing for name, missing in rejected.items() if missing}
        if not rejected:
            break
        for name, missing in rejected.items():
            next(r for r in report['monsters'] if r['monster'] == name).update(
                status='BLOCKED_NATIVE_REFERENCE', missing_references=missing)
        active -= set(rejected)
    native_records, native_profiles = {}, {}
    for name in sorted(active):
        for target, incoming in ((native_records, probes[name].records),
                                 (native_profiles, probes[name].profiles)):
            for key, value in incoming.items():
                if key in target and target[key] != value:
                    raise ValueError(f'Conflicting training definition/profile: {key}')
                target[key] = value
        next(r for r in report['monsters'] if r['monster'] == name)['status'] = 'NATIVE_CLOSURE_PASS_GAMEPLAY_UNVERIFIED'
    write(output / 'training-native-profiles.json', {
        'dataset': report['dataset'], 'production_admission': False, 'runtime_activated': False,
        'records': list(native_records.values()), 'authoring_profiles': list(native_profiles.values()),
        'requires_baseline_stage_sha256': hashlib.sha256(Path(stage_path).read_bytes()).hexdigest(),
        'admitted_training_monsters': sorted(active), 'encounter_declarations': []})
    report['counts'] = {'selected': len(selected), 'schema_pass': len(passed),
                        'native_closure_pass': len(active), 'blocked': len(selected) - len(active),
                        'encounter_profiles': len(encounter_selected), 'native_defence_melee_profiles': len(defense_selected),
                        'scanned_bundles': len(rows), 'reference_profiles': len(reference_selected),
                        'unregistered_item_profiles': len(item_selected)}
    write(output / 'training-index.json', {'dataset': report['dataset'], 'production_admission': False,
                                         'bundle_files': list(FILES), 'bundles': len(passed), 'monsters': passed})
    write(output / 'training-report.json', report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('index', 'stage', 'bundles', 'item-map', 'out'):
        parser.add_argument('--' + arg, type=Path, required=True)
    args = parser.parse_args()
    result = generate(args.index, args.stage, args.bundles, args.item_map, args.out)
    print(json.dumps(result['counts']))


if __name__ == '__main__':
    main()
