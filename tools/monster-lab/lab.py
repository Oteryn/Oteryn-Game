#!/usr/bin/env python3
"""Read-only bulk monster inventory; quality metadata stays outside runtime data."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

BUNDLE_FILES = ('monster.json', 'dependencies.json', 'catalog.json', 'manifest.json')


class LabError(ValueError):
    pass


def read_json(path):
    try:
        return json.loads(Path(path).read_text(encoding='utf-8'))
    except (OSError, ValueError) as exc:
        raise LabError(f'Cannot read JSON {path}: {exc}') from exc


def bundle_digest(directory):
    digest = hashlib.sha256()
    for name in BUNDLE_FILES:
        try:
            data = (directory / name).read_bytes()
        except OSError as exc:
            raise LabError(f'Cannot read bundle file {directory / name}: {exc}') from exc
        digest.update(f'{name}\0{len(data)}\0'.encode('ascii') + data)
    return digest.hexdigest()


def inventory(config):
    inputs = config.get('read_only_inputs', {})
    try:
        index_path, stage_path, root = (Path(inputs[k]).resolve() for k in ('index', 'stage', 'bundles'))
    except (KeyError, TypeError) as exc:
        raise LabError('Config requires read_only_inputs: index, stage, bundles') from exc
    index, stage = read_json(index_path), read_json(stage_path)
    indexed = index.get('monsters')
    if not isinstance(indexed, list) or type(index.get('bundles')) is not int or index['bundles'] != len(indexed):
        raise LabError('Malformed index: monsters list and bundles count must agree')
    if index.get('bundle_files') != list(BUNDLE_FILES):
        raise LabError('Unsupported bundle digest file order')
    profiles = stage.get('authoring_profiles')
    if not isinstance(profiles, list) or not isinstance(stage.get('deferred'), dict):
        raise LabError('Malformed native stage')
    native_keys = set()
    for profile in profiles:
        if profile.get('data', {}).get('kind') != 'Creature':
            continue
        key = profile.get('target', {}).get('key', '')
        if not key.startswith('oteryn:creature.') or key in native_keys:
            raise LabError(f'Invalid or duplicate native Creature key: {key}')
        native_keys.add(key)
    holds = {}
    for reason, entries in stage['deferred'].items():
        if reason == 'encounters':
            continue  # Encounter definitions are not additional Creature actors.
        if not isinstance(entries, list):
            raise LabError(f'Malformed deferred category: {reason}')
        for entry in entries:
            name = entry if isinstance(entry, str) else entry.get('monster') if isinstance(entry, dict) else None
            if not isinstance(name, str):
                raise LabError(f'Malformed deferred actor: {reason}')
            holds.setdefault(name, []).append({'reason': reason, 'details': entry})
    seen, rows = set(), []
    for entry in indexed:
        if not isinstance(entry, dict):
            raise LabError('Malformed index row')
        name = entry.get('monster')
        if not isinstance(name, str) or not name or '/' in name or '\\' in name or name in ('.', '..') or name in seen:
            raise LabError(f'Invalid or duplicate index actor: {name}')
        seen.add(name)
        directory = (root / name).resolve()
        if directory.parent != root:
            raise LabError(f'Bundle escapes configured input root: {name}')
        digest = bundle_digest(directory)
        if digest != entry.get('sha256'):
            raise LabError(f'Bundle digest mismatch: {name}')
        monster, manifest = read_json(directory / 'monster.json'), read_json(directory / 'manifest.json')
        creature = monster.get('creature')
        if not isinstance(creature, dict) or not isinstance(creature.get('stats'), dict):
            raise LabError(f'Malformed creature payload: {name}')
        key = f'oteryn:creature.{name}'
        admitted, deferred = key in native_keys, holds.get(name, [])
        if admitted == bool(deferred):
            raise LabError(f'Actor must be native or deferred, exclusively: {name}')
        flags = ['DONOR_OR_WIKI_PREPARED', 'GAMEPLAY_UNVERIFIED']
        if 'mitigation_percent' not in creature['stats']:
            flags.append('MITIGATION_UNKNOWN')
        if 'bestiary' not in creature:
            flags.append('BESTIARY_ABSENT_IN_PREPARED_DATA')
        if 'bosstiary' in creature:
            flags.append('BOSSTIARY_PRESENT')
        if deferred:
            flags.append('NATIVE_IMPORT_HELD')
        rows.append({'monster': name, 'display_name': creature.get('display_name'),
                     'source_identity': creature.get('identity'), 'source_file': entry.get('file'),
                     'sources': manifest.get('sources', []), 'bundle_sha256': digest,
                     'native_import': 'ADMITTED' if admitted else 'HELD',
                     'runtime_status': 'GAMEPLAY_UNVERIFIED', 'holds': deferred, 'quality_flags': sorted(flags),
                     'loot_entries': len(monster.get('loot', {}).get('entries', []))})
    if holds.keys() - seen or native_keys - {f'oteryn:creature.{name}' for name in seen}:
        raise LabError('Stage contains actors absent from bundle index')
    if stage.get('counts', {}).get('creatures') != len(native_keys):
        raise LabError('Native Creature count differs from profiles')
    rows.sort(key=lambda row: row['monster'])
    hold_counts = {reason: sum(any(h['reason'] == reason for h in row['holds']) for row in rows)
                   for reason in sorted(stage['deferred']) if reason != 'encounters'}
    return {'schema': 'monster-lab-inventory.v1', 'scope': 'Read-only prepared/native inventory; no gameplay qualification',
            'inputs': {'index_sha256': hashlib.sha256(index_path.read_bytes()).hexdigest(),
                       'stage_sha256': hashlib.sha256(stage_path.read_bytes()).hexdigest()},
            'counts': {'prepared': len(rows), 'native_admitted': len(native_keys), 'native_held': sum(bool(r['holds']) for r in rows),
                       'mitigation_unknown': sum('MITIGATION_UNKNOWN' in r['quality_flags'] for r in rows),
                       'bestiary_absent': sum('BESTIARY_ABSENT_IN_PREPARED_DATA' in r['quality_flags'] for r in rows),
                       'loot_entries': sum(r['loot_entries'] for r in rows), 'holds': hold_counts,
                       'encounter_definitions_held': len(stage['deferred'].get('encounters', []))}, 'monsters': rows}


def preparation_plan(report):
    return {'schema': 'monster-lab-preparation-plan.v1', 'status': 'PLAN_ONLY_NO_RUNTIME_PROFILE_CREATED',
            'canonical_inputs_mutated': False,
            'policy': 'Keep original data. Isolated profiles require closed executable references and explicit disabled-mechanic metadata.',
            'actors': [{'monster': row['monster'], 'holds': row['holds'],
                        'action': 'Resolve references or build separately reviewed training profile; flags alone do not admit actor.'}
                       for row in report['monsters'] if row['holds']]}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('inventory', 'report', 'prepare'))
    parser.add_argument('--config', required=True)
    parser.add_argument('--output', help='New report path outside canonical input directories')
    args = parser.parse_args(argv)
    try:
        config = read_json(args.config)
        result = inventory(config)
        if args.command == 'prepare':
            result = preparation_plan(result)
        rendered = json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + '\n'
        if args.output:
            output = Path(args.output).resolve()
            inputs = config['read_only_inputs']
            protected = [Path(inputs[k]).resolve() for k in ('index', 'stage', 'bundles')]
            if any(output == p or p in output.parents for p in protected):
                raise LabError('Output would overwrite canonical inputs')
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(rendered, encoding='utf-8')
            print(json.dumps({'output': str(output), 'counts': result.get('counts'), 'status': result.get('status', 'READ_ONLY_INVENTORY_READY')}, sort_keys=True))
        else:
            print(rendered, end='')
        return 0
    except LabError as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
