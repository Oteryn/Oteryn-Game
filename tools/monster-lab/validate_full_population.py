#!/usr/bin/env python3
"""Bind and validate a complete prepared population before snapshot promotion.

This read-only packet does not change the production materializer or qualify live
combat. The native arena remains a separate, hash-bound execution check.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

from lab import LabError, inventory, read_json
from run import output_directory


def identity(value):
    try:
        result = tuple(value[k] for k in ('family', 'key', 'revision'))
    except (KeyError, TypeError) as exc:
        raise LabError('Malformed native reference identity') from exc
    if any(not isinstance(part, str) or not part for part in result):
        raise LabError('Native reference identity must contain nonempty strings')
    return result


def references(value):
    if isinstance(value, dict):
        if {'family', 'key', 'revision'} <= value.keys():
            yield identity(value)
        for child in value.values():
            yield from references(child)
    elif isinstance(value, list):
        for child in value:
            yield from references(child)


def validate_stage(stage, index, index_sha256, item_export, admitted_items=()):
    if stage.get('source', {}).get('census_index_sha256') != index_sha256:
        raise LabError('Stage census does not bind the exact population index')
    records = stage['records']
    known = {identity(row['identity']) for row in records}
    if len(known) != len(records):
        raise LabError('Duplicate native record identity')
    declarations = stage['declarations']
    declared = {identity(dict(row['identity'], family=row['kind'])) for row in declarations}
    if len(declared) != len(declarations):
        raise LabError('Duplicate declaration identity')
    if known & declared:
        raise LabError('Declaration conflicts with native record')
    targets = [identity(row['target']) for row in stage['authoring_profiles']]
    if len(targets) != len(set(targets)) or set(targets) - known - declared:
        raise LabError('Profiles have duplicate or unresolved targets')
    if item_export.get('schema') != 'OTERYN_PROTECTED_ITEM_IDENTITY_MAP_EXPORT/v1':
        raise LabError('Expected protected Item identity export')
    item_keys = {row['native_key'] for row in item_export['records']}
    for rekey in stage.get('source', {}).get('item_rekeys', []):
        if rekey['from'] not in item_keys:
            raise LabError('Item rekey has no protected source identity')
        item_keys.discard(rekey['from'])
        item_keys.add(rekey['to'])
    external = {('Item', key, 'definition-r1') for key in item_keys | set(admitted_items)}
    missing = set(references(stage)) - known - declared - external
    if missing:
        raise LabError('Unresolved native references: ' + repr(sorted(missing)[:10]))
    profiles = stage['authoring_profiles']
    creatures = [row for row in profiles if row['data']['kind'] == 'Creature']
    for row in creatures:
        profile = row['data']['profile']
        for field in ('health', 'speed', 'armor', 'experience'):
            value = profile.get(field)
            if type(value) is not int or value < (1 if field == 'health' else 0):
                raise LabError(f'Invalid Creature {field}: {row["target"]["key"]}')
        defense = profile.get('details', {}).get('defense')
        if type(defense) is not int or defense < 0:
            raise LabError('Creature defense is missing or invalid')
        ratio = profile.get('mitigation', {})
        numerator, denominator = ratio.get('numerator'), ratio.get('denominator')
        if (type(numerator) is not int or type(denominator) is not int
                or denominator <= 0 or not 0 <= numerator <= 100 * denominator):
            raise LabError('Creature mitigation must be a closed 0..100 percentage')
    counts = {'records': len(records), 'profiles': len(profiles),
              'creatures': len(creatures), 'encounters': len(declarations)}
    if any(stage.get('counts', {}).get(key) != value for key, value in counts.items()):
        raise LabError('Stage counts disagree with actual records/profiles/declarations')
    expected_flags = {'oteryn:creature.' + row['monster']: row['completion_flags']
                      for row in index['monsters'] if row.get('completion_flags')}
    actual_flags = stage.get('completion_flags', {})
    if actual_flags != expected_flags:
        raise LabError('Native stage did not preserve exact population quality flags')
    return counts


def promotion_packet(config):
    report = inventory(config)  # Checks every four-file bundle digest and actor admission.
    inputs = config['read_only_inputs']
    stage, index = read_json(inputs['stage']), read_json(inputs['index'])
    # Reuse the admission mapper's accepted appearance/binding verification,
    # rather than treating unregistered loot identities as external by fiat.
    migration = Path(config['repository']) / 'tools/content-migration'
    sys.path.insert(0, str(migration))
    import creature_admission_stage as admission
    item_export = read_json(inputs['item_map'])
    if item_export.get('allocation_digest_sha256') != admission.ITEM_ALLOCATION_SHA256:
        raise LabError('Protected Item allocation digest drifted')
    appearance_index, appearances = admission.load_admitted()
    current_ids = {entry[0] for entry in appearances[appearance_index['newest']]['entries']}
    mapped_items = {row['source_item_id']: row['native_key'] for row in item_export['records']}
    for rekey in stage['source'].get('item_rekeys', []):
        mapped_items[rekey['source_item_id']] = rekey['to']
    try:
        admitted_items = admission.resolve_admitted_item_map(mapped_items,
            read_json(admission.REFERENCE)['records'], read_json(admission.ITEM_ALIASES)['entries'],
            read_json(admission.ITEM_BINDINGS)['bindings'], current_ids)
    except admission.StageError as exc:
        raise LabError(str(exc)) from exc
    counts = validate_stage(stage, index, report['inputs']['index_sha256'], item_export, admitted_items.values())
    materializer = Path(config['repository']) / 'apps/game-server/examples/materialize_content_world_project_v2.rs'
    code = materializer.read_text(encoding='utf-8')
    pinned = {}
    for label, constant in (('creatures', 'CREATURE_COUNT'), ('encounters', 'ENCOUNTER_COUNT')):
        match = re.search(rf'const {constant}: usize = (\d+);', code)
        if not match:
            raise LabError('Cannot identify production materializer count fence: ' + constant)
        pinned[label] = int(match.group(1))
    flags = Counter(flag for row in index['monsters'] for flag in row.get('completion_flags', []))
    return {'schema': 'monster-population-promotion-packet.v1',
            'scope': 'Read-only source/native reference validation; no live gameplay qualification',
            'inputs': dict(report['inputs'], item_map_sha256=hashlib.sha256(Path(inputs['item_map']).read_bytes()).hexdigest()),
            'counts': counts, 'inventory': report['counts'], 'quality_flags': dict(sorted(flags.items())),
            'materializer': {'sha256': hashlib.sha256(materializer.read_bytes()).hexdigest(),
                             'pinned_counts': pinned, 'counts_match': all(pinned[key] == counts[key] for key in pinned)},
            'content_validation_passed': True,
            'all_indexed_actors_admitted': report['counts']['native_held'] == 0,
            'promotion_executed': False, 'live_server_started': False,
            'gameplay_checks': {name: 'NOT_EXECUTED_BY_THIS_PACKET' for name in
                                ('ability_effect_commit', 'summon_owner_lifecycle', 'loot_drop_commit', 'respawn', 'client_e2e')},
            'promotion_requirements': ['Reconcile source revision/count/source-binding fences in the owning materializer change.',
                'Promote the exact stage SHA as reviewed evidence; keep quality metadata with the population index.',
                'Run the hash-bound native arena on this stage and existing runtime effect/loot/respawn checks.',
                'Complete protected-main integration and separately authorized deployment before claiming live activation.']}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', required=True)
    parser.add_argument('--output-dir', required=True)
    args = parser.parse_args(argv)
    try:
        config = read_json(args.config)
        output = output_directory(config, args.output_dir)
        packet = promotion_packet(config)
        path = output / 'promotion-packet.json'
        path.write_text(json.dumps(packet, sort_keys=True, indent=2) + '\n', encoding='utf-8')
        print(json.dumps({'packet': str(path), 'counts': packet['counts'], 'promotion_executed': False}))
        return 0
    except (LabError, KeyError, TypeError, OSError) as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
