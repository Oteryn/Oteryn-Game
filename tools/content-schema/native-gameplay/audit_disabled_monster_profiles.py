#!/usr/bin/env python3
"""Describe every disabled native melee profile against both frozen donor packs."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
from build_monster_melee_profiles import identity

PACKS = ('canary/data-otservbr-global/monster', 'crystal/data-global/monster')


def attest_source(path, source_root, indexed):
    manifest = json.loads(path.with_name('manifest.json').read_bytes())
    if len(manifest['sources']) != 1:
        raise ValueError('source provenance ambiguous')
    source = manifest['sources'][0]
    files = {entry['source_file'] for entry in manifest['entries'] if entry.get('source_file')}
    if len(files) != 1:
        raise ValueError('bundle must have one actual donor file')
    file = files.pop()
    attested = indexed[(source['repository'], source['revision'], file)]
    # Repository identity selects the already captured source root; no URLs or
    # candidate JSON are permitted to select arbitrary filesystem paths.
    directory = {'opentibiabr/canary': 'canary', 'zimbadev/crystalserver': 'crystal'}[source['repository']]
    relative = Path(file)
    if relative.is_absolute() or '..' in relative.parts:
        raise ValueError('unsafe donor path')
    raw = (source_root / directory / relative).read_bytes()
    if (hashlib.sha256(raw).hexdigest() != attested['sha256']
            or len(raw) != attested['bytes']
            or hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest() != attested['git_blob']):
        raise ValueError('donor bytes differ from frozen per-file index')
    return attested


def closure_facts(monster, dependencies):
    abilities = {identity(row['identity']): row for row in dependencies.get('abilities', [])}
    effects = {identity(row['identity']): row for row in dependencies.get('effects', [])}
    formulas = {identity(row['identity']): row for row in dependencies.get('formulas', [])}
    facts = []
    for slot, schedule in enumerate(monster['behavior'].get('attacks', []), 1):
        ability = abilities.get(identity(schedule['ability']))
        if not ability or ability.get('kind') != 'melee':
            continue
        effect = effects.get(identity(ability.get('effects', [{}])[0])) if ability.get('effects') else None
        formula = formulas.get(identity(effect['formula'])) if effect and effect.get('formula') else None
        facts.append({'source_slot': slot, 'ability': schedule['ability'], 'range_tiles': ability.get('range_tiles'),
                      'primary_effect': effect, 'formula': formula,
                      'secondary_effects': [effects.get(identity(ref)) for ref in ability.get('effects', [])[1:]]})
    return facts


def audit(creatures, report, bundle_root, source_root):
    native = {tuple(r['profile']['target'][k] for k in ('family', 'key', 'revision')): r for r in creatures['records']}
    packs = {}
    for pack in PACKS:
        population = defaultdict(list)
        for path in sorted((bundle_root / pack).rglob('monster.json')):
            row = json.loads(path.read_bytes())
            population[row['creature']['display_name'].casefold()].append((path, row))
        index_path = source_root.parent / f"{pack.split('/')[0]}-monster-files.json"
        index = json.loads(index_path.read_bytes())
        indexed = {(row['provenance']['repository'], row['provenance']['revision'], row['provenance']['path']): row['provenance'] for row in index}
        packs[pack] = population, indexed
    records = []
    for observation in report['records']:
        if observation['status'] == 'enabled_approximate_melee':
            continue
        key = tuple(observation['creature'][k] for k in ('family', 'key', 'revision'))
        profile = native[key]['profile']['data']['profile']
        row = {'creature': observation['creature'], 'display_name': profile['details']['display_name'],
               'native_health': profile['health'], 'primary_status': observation['status'],
               'fallback_status': observation.get('fallback_disabled_observation', {}).get('status'), 'donors': []}
        for pack, (population, indexed) in packs.items():
            choices = population.get(profile['details']['display_name'].casefold(), [])
            donor = {'pack': pack, 'matching_source_count': len(choices), 'sources': []}
            for path, monster in choices:
                dependencies = json.loads(path.with_name('dependencies.json').read_bytes())
                donor['sources'].append({'source': attest_source(path, source_root, indexed),
                                        'source_health': monster['creature']['stats']['max_health'],
                                        'source_hostile': monster['creature'].get('flags', {}).get('hostile'),
                                        'melee_closures': closure_facts(monster, dependencies)})
            row['donors'].append(donor)
        if row['primary_status'] == 'disabled_source_health_conflict':
            row['resolution'] = 'requires_owner_resolution_of_accepted_native_health_vs_source; preserve_existing_profile'
        elif row['primary_status'] == 'disabled_no_unique_source':
            row['resolution'] = 'no_unique_matching_donor_with_accepted_health_and_schedule_in_either_selected_pack'
        else:
            primary_closures = [c for source in row['donors'][0]['sources'] for c in source['melee_closures']]
            if not primary_closures:
                row['resolution'] = 'source_has_no_melee_slot; do_not_invent_melee_for_ranged_special_or_passive_profile'
            elif all(c['formula'] and c['formula'].get('kind') == 'range' and c['formula']['magnitude']['maximum'] == 0 for c in primary_closures):
                row['resolution'] = 'source_primary_melee_damage_is_zero; no_damage_is_not_a_missing_formula'
            else:
                row['resolution'] = 'unsupported_closure_requires_runtime_or_accepted_data_resolution'
        records.append(row)
    if len(records) != report['disabled']:
        raise ValueError('disabled report population differs')
    return {'schema': 'OTERYN_DISABLED_MONSTER_MELEE_RESOLUTION/v1',
            'read_method': 'local pinned Git capture; no fresh wiki claim',
            'runtime_activation': False, 'identity_allocation': False,
            'count': len(records), 'resolution_counts': dict(Counter(r['resolution'] for r in records)), 'records': records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('creatures', 'report', 'bundle-root', 'source-root', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    result = audit(json.loads(args.creatures.read_bytes()), json.loads(args.report.read_bytes()), args.bundle_root, args.source_root)
    with args.out.open('x') as stream:
        stream.write(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'count': result['count'], 'resolution_counts': result['resolution_counts']}))


if __name__ == '__main__':
    main()
