"""Combine independently owned completion packets, preserving input bytes.

Packets bind the same baseline index and contain four-file replacement/addition
bundles. Conflicting writers fail closed. This prepares content, not a rollout.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path

import complete_creature_dependencies as population


def read(path):
    return json.loads(Path(path).read_text())


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def immutable_copy(source, destination):
    for path in sorted(source.rglob('*')):
        if path.is_symlink():
            raise ValueError('symlink in completion input')
        if path.is_file():
            target = destination / path.relative_to(source)
            target.parent.mkdir(parents=True, exist_ok=True)
            os.link(path, target)


def validate_final_bindings(stage):
    """A declared dependency is not proof of an admitted source identity."""
    bindings = stage['source_identity_bindings']
    sources = [(row['source_key'], row['source_revision'], row['identity_namespace'],
                row['external_id']) for row in bindings]
    if len(sources) != len(set(sources)):
        raise ValueError('duplicate source identity binding')
    identity = lambda value: tuple(value[k] for k in ('family', 'key', 'revision'))
    targets = [identity(row['target']) for row in bindings]
    if len(targets) != len(set(targets)):
        raise ValueError('multiple source bindings for one admitted actor')
    creatures = {identity(row['target']) for row in stage['authoring_profiles']
                 if row['data']['kind'] == 'Creature'}
    encounters = {identity(dict(row['identity'], family='Encounter'))
                  for row in stage['declarations'] if row['kind'] == 'Encounter'}
    if set(targets) != creatures | encounters:
        raise ValueError('source bindings do not cover exactly admitted actors and encounters')
    return {'unique_source_bindings': len(sources), 'creatures': len(creatures),
            'encounters': len(encounters)}


def merge(baseline, packets, output):
    baseline, output = Path(baseline).resolve(), Path(output).resolve()
    packets = [Path(p).resolve() for p in packets]
    if output.exists():
        raise ValueError('output must be new')
    for source in [baseline, population.ROOT, *packets]:
        if output == source or source in output.parents:
            raise ValueError('output would modify inputs')
    index_path = baseline / 'population-index.json'
    index = read(index_path)
    rows = {row['monster']: row for row in index['monsters']}
    supplements, writers, receipts, encounter_replacements = [], {}, [], {}
    for packet in packets:
        report = read(packet / 'completion.json')
        if report['baseline_index_sha256'] != sha(index_path):
            raise ValueError('packet belongs to another baseline')
        receipts.append({'packet': packet.name, 'receipt_sha256': sha(packet / 'completion.json'),
                         'report': report})
        for row in report['index_monsters']:
            slug = row['monster']
            if slug in writers:
                raise ValueError('conflicting completion writers: ' + slug)
            writers[slug] = packet.name
            original = rows.get(slug)
            combined = {**copy.deepcopy(original or {}), **copy.deepcopy(row)}
            inherited = set((original or {}).get('completion_flags', []))
            resolved = set(row.get('resolved_completion_flags', []))
            if not resolved <= inherited:
                raise ValueError('resolved flag was not present in baseline: ' + slug)
            if 'OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE' in resolved:
                raise ValueError('completion cannot erase estimate qualification')
            combined.pop('resolved_completion_flags', None)
            combined['completion_flags'] = sorted((inherited - resolved)
                                                  | set(row.get('completion_flags', [])))
            supplements.append((packet / 'bundles' / slug, combined))
        encounters = packet / 'encounters'
        if encounters.exists():
            for folder in sorted(encounters.iterdir()):
                if folder.name in encounter_replacements:
                    raise ValueError('conflicting encounter writers: ' + folder.name)
                if folder.is_symlink() or not folder.is_dir() or not (folder / 'encounter.json').is_file():
                    raise ValueError('invalid Encounter packet: ' + folder.name)
                encounter_replacements[folder.name] = folder
    output.mkdir(parents=True)
    merged = population.merge_population(index, baseline / 'bundles', output, supplements)
    merged['completion_generation'] = {'baseline_index_sha256': sha(index_path),
                                      'packet_receipts': [r['receipt_sha256'] for r in receipts]}
    write(output / 'population-index.json', merged)
    for folder in sorted((baseline / 'encounters').iterdir()):
        immutable_copy(encounter_replacements.get(folder.name, folder), output / 'encounters' / folder.name)
    for name, folder in sorted(encounter_replacements.items()):
        if not (baseline / 'encounters' / name).exists():
            immutable_copy(folder, output / 'encounters' / name)
    quality = {'schema': 'OTERYN_PARALLEL_MONSTER_COMPLETION/v1',
               'baseline_quality': read(baseline / 'completion-quality.json'),
               'baseline_index_sha256': sha(index_path), 'packets': receipts,
               'changed_existing': sum(slug in rows for slug in writers),
               'added': sum(slug not in rows for slug in writers),
               'prepared': len(merged['monsters']), 'production_activated': False,
               'live_gameplay_verified': False}
    write(output / 'completion-quality.json', quality)
    return {key: quality[key] for key in ('prepared', 'added', 'changed_existing')}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True, type=Path)
    parser.add_argument('--packet', required=True, action='append', type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    print(json.dumps(merge(args.baseline, args.packet, args.output)))


if __name__ == '__main__':
    main()
