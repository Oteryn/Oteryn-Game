#!/usr/bin/env python3
"""Fail when a native creature profile `attacks[]` reference lacks attack authoring.

The authoring file (`monster-attack-authoring.jsonl`) must hold exactly one record per distinct
attack ability reference of `spell-native-profiles.json`: either `authored` with complete damage
facts and donor evidence, or `unavailable` with an explicit reason. The pin file binds the
authoring to the exact profile artifact digest, so changing the artifact without regenerating fails.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import build_monster_attack_authoring as gen

HEX = set('0123456789abcdef')
REPOSITORIES = {'opentibiabr/canary', 'zimbadev/crystalserver'}
AREA_KEYS = ({'radius_tiles'}, {'length_tiles', 'spread_tiles'}, {'matrix'})


def is_hex(text, length):
    return isinstance(text, str) and len(text) == length and set(text) <= HEX


def attack_references(profiles):
    """{ability key: {'ref': ref, 'creatures': set, 'schedules': set((interval_ms, chance_ppm))}}"""
    found = {}
    for record in profiles['records']:
        behavior = record.get('behavior')
        for attack in (behavior['data']['profile'].get('attacks', []) if behavior else []):
            entry = found.setdefault(attack['ability']['key'],
                                     {'ref': attack['ability'], 'creatures': set(), 'schedules': set()})
            entry['creatures'].add(record['profile']['target']['key'])
            entry['schedules'].add((attack['interval_ms'], attack['chance_ppm']))
    return found


def check_authored(row, errors, where):
    for field in ('kind', 'range_tiles', 'needs_target', 'needs_direction', 'effects', 'evidence', 'schedules'):
        if field not in row:
            errors.append(f'{where}: authored record lacks {field}')
            return
    if row['kind'] not in ('melee', 'spell') or type(row['range_tiles']) is not int or row['range_tiles'] < 0:
        errors.append(f'{where}: invalid kind or range')
    if 'area' in row and (not isinstance(row['area'], dict) or set(row['area']) not in AREA_KEYS):
        errors.append(f'{where}: invalid area')
    if row.get('qualification') != gen.QUALIFICATION:
        errors.append(f'{where}: qualification must stay {gen.QUALIFICATION}')
    if not row['effects']:
        errors.append(f'{where}: authored record has no effects')
    for effect in row['effects']:
        if effect.get('operation') == 'damage':
            low, high = effect.get('minimum'), effect.get('maximum')
            if (type(low) is not int or type(high) is not int or not 0 <= low <= high <= 0xFFFFFFFF
                    or not effect.get('damage_type') or not effect.get('derivation')):
                errors.append(f'{where}: damage effect lacks element or a valid minimum/maximum')
    evidence = row['evidence']
    if (evidence.get('source_repository') not in REPOSITORIES or not is_hex(evidence.get('source_revision'), 40)
            or not evidence.get('source_path') or str(evidence['source_path']).startswith('/')
            or '..' in str(evidence['source_path']).split('/')
            or set(evidence.get('bundle_sha256', {})) != set(gen.BUNDLE_FILES)
            or not all(is_hex(v, 64) for v in evidence['bundle_sha256'].values())):
        errors.append(f'{where}: invalid donor evidence')


def validate(profiles_raw: bytes, rows_raw: bytes, pin: dict) -> list[str]:
    errors = []
    profiles = json.loads(profiles_raw)
    if pin.get('schema') != gen.PIN_SCHEMA:
        errors.append('pin schema')
    if pin.get('profile_artifact', {}).get('sha256') != gen.sha256(profiles_raw):
        errors.append('pin does not match the creature profile artifact digest; regenerate the authoring')
    if pin.get('records', {}).get('sha256') != gen.sha256(rows_raw):
        errors.append('pin does not match the authoring records digest')
    rows = [json.loads(line) for line in rows_raw.decode().splitlines()]
    keys = [r['ability']['key'] for r in rows]
    if len(set(keys)) != len(keys) or keys != sorted(keys):
        errors.append('authoring records must be unique and sorted by ability key')
    by_key = {r['ability']['key']: r for r in rows}
    references = attack_references(profiles)
    for key in sorted(references.keys() - by_key.keys()):
        errors.append(f'attack reference lacks authoring: {key}')
    for key in sorted(by_key.keys() - references.keys()):
        errors.append(f'authoring for a reference no attacks[] uses: {key}')
    counts = {'authored': 0, 'unavailable': 0}
    for key, row in sorted(by_key.items()):
        reference = references.get(key)
        if reference is None:
            continue
        where = key
        if row.get('schema') != gen.SCHEMA or row['ability'] != reference['ref']:
            errors.append(f'{where}: schema or ability reference differs')
        if set(row.get('creatures', [])) != reference['creatures']:
            errors.append(f'{where}: creature bindings differ from attacks[] users')
        status = row.get('status')
        if status not in counts:
            errors.append(f'{where}: status must be authored or unavailable')
            continue
        counts[status] += 1
        if status == 'unavailable':
            if not row.get('reason'):
                errors.append(f'{where}: unavailable record lacks a reason')
            if any(f in row for f in ('effects', 'evidence', 'kind')):
                errors.append(f'{where}: unavailable record must not carry authoring')
            continue
        check_authored(row, errors, where)
        if 'schedules' in row and {(s.get('interval_ms'), s.get('chance_ppm')) for s in row['schedules']} \
                != reference['schedules']:
            errors.append(f'{where}: schedule differs from attacks[]')
    for name in ('authored', 'unavailable'):
        if pin.get('records', {}).get(name) != counts[name]:
            errors.append(f'pin {name} count differs')
    if pin.get('records', {}).get('count') != len(rows):
        errors.append('pin record count differs')
    return errors


def validate_repository(root: Path = gen.ROOT) -> list[str]:
    return validate((root / gen.PROFILES).read_bytes(), (root / gen.RECORDS).read_bytes(),
                    json.loads((root / gen.PIN).read_bytes()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=gen.ROOT)
    args = parser.parse_args()
    errors = validate_repository(args.root)
    for error in errors[:50]:
        print(error)
    if errors:
        raise SystemExit(f'{len(errors)} monster attack authoring errors')
    print('monster attack authoring covers every attacks[] reference')


if __name__ == '__main__':
    main()
