#!/usr/bin/env python3
"""Rebuild local planning views from sealed imports, never active content or selection."""
import argparse
from collections import Counter
import copy
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS = Path('docs/reference/spells')
FLAGS = ('runtime_activation', 'native_execution_qualified', 'native_identity_allocation', 'canonical_selection_changed')


def sha(body): return hashlib.sha256(body).hexdigest()


def read_json(path): return json.loads(path.read_bytes())


def member(folder, name):
    path = folder / name
    if Path(name).is_absolute() or '..' in Path(name).parts or path.is_symlink() or not path.resolve().is_relative_to(folder.resolve()):
        raise ValueError('unsafe import member')
    return path


def verified_manifest(root, revision):
    folder = root / 'imports/spells' / f'r{revision}'
    path = folder / 'import-manifest.json'
    manifest = read_json(path)
    if manifest['admission_status'] != 'source_only_not_active' or any(manifest[field] is not False for field in FLAGS):
        raise ValueError('active import cannot enter planning view')
    for artifact in manifest['artifacts']:
        body = member(folder, artifact['path']).read_bytes()
        if sha(body) != artifact['sha256'] or len(body) != artifact['bytes']:
            raise ValueError('sealed import member mismatch: ' + artifact['path'])
    return folder, manifest, {'manifest_path': path.relative_to(root).as_posix(), 'manifest_sha256': sha(path.read_bytes()),
                              'counts': copy.deepcopy(manifest['counts']),
                              'metadata_path': member(folder, manifest['source_metadata_path']).relative_to(root).as_posix()}


def monster_view(root, folder, manifest, pin):
    path = member(folder, 'target-projections/projected-monster-slot-candidates.json.gz')
    packet = json.loads(gzip.decompress(path.read_bytes()))
    rows = packet['slots']
    identities = [json.dumps(row['slot_identity'], sort_keys=True) for row in rows]
    if len(set(identities)) != len(rows): raise ValueError('duplicate monster slot')
    counts = Counter(row['status'] for row in rows)
    if set(counts) - {'SOURCE_SCHEMA_VALID', 'PARTIAL_TARGET_DEFINITIONS', 'BLOCKED'}:
        raise ValueError('unknown monster projection status')
    full = counts['SOURCE_SCHEMA_VALID']; partial = counts['PARTIAL_TARGET_DEFINITIONS']; blocked = counts['BLOCKED']
    expected = {'full_monster_slot_candidates': full, 'partial_monster_slots': partial,
                'blocked_monster_slots': blocked, 'target_definition_slots': full + partial}
    if any(manifest['counts'][key] != value for key, value in expected.items()):
        raise ValueError('monster manifest/status counts differ')
    records = []
    for row in rows:
        if row['runtime_activation'] is not False or row['native_execution_qualified'] is not False:
            raise ValueError('active monster projection refused')
        if row['full_slot_projection_complete'] != (row['status'] == 'SOURCE_SCHEMA_VALID'):
            raise ValueError('partial monster counted complete')
        records.append({key: copy.deepcopy(row[key]) for key in (
            'slot_identity', 'source', 'monster', 'original_slot_sha256', 'status',
            'full_slot_projection_complete', 'blockers', 'canonical_normalization')})
        records[-1].update({'target_fragment_count': len(row['target_fragments']),
                            'runtime_activation': False, 'native_execution_qualified': False})
    return {'schema': 'OTERYN_MONSTER_TARGET_PROJECTION_REVIEW/v1', **pin,
            'projection_packet_path': path.relative_to(root).as_posix(), 'projection_packet_sha256': sha(path.read_bytes()),
            'selected_source_slot_count': len(rows), 'status_counts': dict(counts),
            'complete_target_slot_candidates': full, 'partial_target_slots': partial,
            'blocked_without_target_slots': blocked, 'remaining_data_holds': partial + blocked,
            'runtime_unqualified_slot_count': len(rows), 'runtime_activation': False,
            'native_execution_qualified': False, 'canonical_selection_changed': False,
            'records': records, 'limits': ['Target projections do not rewrite original sealed source statuses.',
                'Complete target data do not qualify providers, native execution, activation or gameplay.',
                'This is a separate monster planning view, never a player spell index.']}


def slot_key(row):
    return json.dumps(row['slot_identity'], sort_keys=True)


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def private_monster_overlay(root, historical, additions):
    """Join sealed DATA by original slot identity; preserve the earlier complete ten."""
    original_path = root / historical['projection_packet_path']
    originals = {slot_key(row): row for row in json.loads(gzip.decompress(original_path.read_bytes()))['slots']}
    current = {slot_key(row): copy.deepcopy(row) for row in historical['records']}
    holds = {key for key, row in current.items() if not row['full_slot_projection_complete']}
    seen, consumers, groups = set(), [], []
    for revision, folder, manifest, pin in additions:
        if (manifest.get('target_schema_family') != 'private_monster_slot_v2'
                or manifest.get('authoring_contract_extension_pending') is not True
                or manifest.get('source_consumer_implemented') is not False
                or manifest.get('input_provider_equivalence') is not False
                or any(manifest.get(name) is not False for name in FLAGS)):
            raise ValueError('monster data cannot qualify a contract or consumer')
        path = member(folder, 'monster-source-candidates/models.json')
        packet = read_json(path)
        if packet['schema'] != 'OTERYN_PRIVATE_MONSTER_SLOT_MODELS/v2':
            raise ValueError('wrong private monster model family')
        rows = packet['records']
        if any(manifest['counts'][name] != len(rows) for name in ('records', 'source_schema_valid', 'full_source_data_slots')):
            raise ValueError('private monster model/manifest counts differ')
        receipts = {slot_key(row): row for row in read_json(member(folder, 'monster-source-candidates/slot-receipts.json'))['records']}
        if len(receipts) != len(rows):
            raise ValueError('private monster receipt population differs')
        for row_index, row in enumerate(rows):
            key = slot_key(row)
            if key in seen or key not in holds:
                raise ValueError('monster overlay must address each original hold exactly once')
            old = originals[key]
            if (row['source'] != old['source'] or row['monster'] != old['monster']
                    or row['original_slot_sha256'] != old['original_slot_sha256']
                    or canonical(row['source_parameters']) != canonical(old['source_parameters'])):
                raise ValueError('monster overlay changed original source facts')
            if (row['status'] != 'SOURCE_SCHEMA_VALID' or row['full_slot_projection_complete'] is not True
                    or row['target_schema_family'] != 'private_monster_slot_v2'
                    or row['authoring_contract_extension_pending'] is not True
                    or row['required_operations_unrepresented'] != []
                    or any(row[name] is not False for name in (*FLAGS, 'input_provider_equivalence', 'source_consumer_implemented', 'source_alias_to_existing_native_profile'))):
                raise ValueError('private monster projection cannot receive runtime qualification')
            receipt = receipts.get(key)
            if (receipt is None or receipt['target_model_sha256'] != sha(canonical(row))
                    or receipt['original_slot_sha256'] != row['original_slot_sha256']
                    or receipt.get('status') != 'SOURCE_SCHEMA_VALID'
                    or receipt.get('full_slot_projection_complete') is not True
                    or receipt.get('target_schema_family') != 'private_monster_slot_v2'
                    or receipt.get('authoring_contract_extension_pending') is not True
                    or any(receipt.get(name) is not False for name in (*FLAGS, 'input_provider_equivalence', 'source_consumer_implemented'))):
                raise ValueError('private monster model/receipt join mismatch')
            details = {name: value for name, value in row.items() if isinstance(value, bool)}
            details.update({'slot_identity': copy.deepcopy(row['slot_identity']), 'source': row['source'], 'monster': row['monster'],
                'original_slot_sha256': row['original_slot_sha256'], 'status': row['status'],
                'target_schema_family': row['target_schema_family'], 'controller_kind': row['controller']['kind'],
                'source_model_path': path.relative_to(root).as_posix(), 'source_model_row_index': row_index,
                'source_model_sha256': sha(canonical(row)), 'import_set': f'r{revision}',
                'source_parameters_sha256': sha(canonical(row['source_parameters'])),
                'original_r54_status': old['status']})
            consumers.append(copy.deepcopy(details))
            current[key].update(details)
            current[key]['blockers'] = []
            current[key]['original_r54_blockers'] = copy.deepcopy(old['blockers'])
            seen.add(key)
        if set(receipts) != {slot_key(row) for row in rows}:
            raise ValueError('private monster model/receipt identity cohort differs')
        groups.append(copy.deepcopy(pin))
    if seen != holds:
        raise ValueError('complete all four monster lanes before rebuilding progress')
    result = copy.deepcopy(historical)
    result.update({'status_counts': {'SOURCE_SCHEMA_VALID': len(current)}, 'complete_target_slot_candidates': len(current),
        'partial_target_slots': 0, 'blocked_without_target_slots': 0, 'remaining_data_holds': 0,
        'private_monster_slot_v2_candidates': len(seen), 'authoring_contract_extension_pending_candidates': len(seen),
        'source_consumer_unimplemented_candidates': len(seen), 'runtime_unqualified_slot_count': len(current),
        'historical_r54_status_counts': copy.deepcopy(historical['status_counts']),
        'historical_r54_complete_target_slot_candidates': historical['complete_target_slot_candidates'],
        'private_monster_imports': groups, 'records': [current[key] for key in sorted(current)],
        'limits': historical['limits'] + ['New private MonsterSlot2 data require authoring contract extensions and actual consumers.',
            'Original source error flags and literal normalizations are retained in the pinned models, not silently rewritten.',
            'All selected slots remain runtime-unqualified; source data completeness is not donor behavior equivalence.']})
    worklist = {'schema': 'OTERYN_PRIVATE_MONSTER_SOURCE_CONSUMER_WORKLIST/v1',
        'purpose': 'runtime_owner_handoff_not_execution_authority', 'records_count': len(consumers),
        'selected_source_slot_count': len(current), 'runtime_unqualified_slot_count': len(current),
        'historical_r54_complete_data_slots': historical['complete_target_slot_candidates'],
        'authoring_contract_extension_pending': True, 'source_consumer_implemented': False,
        'input_provider_equivalence': False, **{name: False for name in FLAGS},
        'import_groups': groups, 'records': sorted(consumers, key=slot_key),
        'limits': ['Only the new private MonsterSlot2 cohort is in this worklist; the earlier ten retain their earlier provider limitations.',
                   'No native executor, runtime provider, active selection or donor 1:1 behavior is qualified by these DATA views.']}
    return result, worklist


def build(root):
    loaded = {revision: verified_manifest(root, revision) for revision in (52, 53, 54)}
    new_revisions = tuple(range(55, 59))
    present = [revision for revision in new_revisions
               if (root / 'imports/spells' / f'r{revision}/import-manifest.json').exists()]
    if present and len(present) != len(new_revisions):
        raise ValueError('finish the complete unblocking batch before rebuilding progress')
    for revision in present:
        loaded[revision] = verified_manifest(root, revision)
    completion_revisions = tuple(range(59, 63))
    completion_present = [revision for revision in completion_revisions
                          if (root / 'imports/spells' / f'r{revision}/import-manifest.json').exists()]
    if completion_present and len(completion_present) != len(completion_revisions):
        raise ValueError('finish the complete private source data batch before rebuilding progress')
    for revision in completion_present:
        loaded[revision] = verified_manifest(root, revision)
    index_path = root / DOCS / 'source-closure-review-index.json'
    index_body = index_path.read_bytes(); index = json.loads(index_body)
    known = {entry['path']: entry['sha256'] for entry in index['import_sets']}
    for folder, manifest, pin in loaded.values():
        if known.get(pin['manifest_path']) != pin['manifest_sha256']:
            raise ValueError('player review index must be rebuilt from these sealed imports first')
    players = {row['registration_key']: row for row in index['player_records']}
    player_counts = dict(Counter(row['structural_status'] for row in players.values()))
    if player_counts != index['player_structural_status_counts']: raise ValueError('player index counts differ')
    worklist = read_json(root / DOCS / 'current-source-gap-worklist.json')
    baseline = read_json(root / DOCS / worklist['baseline_worklist_path'].split('/')[-1])
    prior = {row['registration_key']: row for row in worklist['records']}
    records = []
    for old in baseline['records']:
        current = players[old['registration_key']]
        if current['structural_status'] != 'BLOCKED': continue
        row = copy.deepcopy(prior.get(old['registration_key'], {
            'registration_key': old['registration_key'], 'name': old['name'],
            'baseline_lane': old['lane'], 'phase': old['phase'], 'planned_owner': old['planned_owner'], 'source_evidence': []}))
        row.update({'current_receipt_path': current['receipt_path'], 'structural_status': 'BLOCKED', 'runtime_activation': False})
        row.update({key: current[key] for key in ('reference_only_status', 'reference_set', 'reference_receipt_path', 'reference_standard_receipt_path') if key in current})
        records.append(row)
    worklist.update({'source_index_sha256': sha(index_body), 'blocked_variants': len(records),
                     'structural_candidates': player_counts.get('CANDIDATE_SCHEMA_VALID', 0),
                     'lane_counts_using_historical_classification': dict(Counter(row['baseline_lane'] for row in records)), 'records': records})
    worklist['disabled_examples'] = sum(row['baseline_lane'] == 'disabled_examples' for row in records)
    worklist['target_projection_imports'] = [loaded[revision][2] for revision in loaded]
    worklist['completed_target_data_in_r52_r53'] = [copy.deepcopy(row) for row in players.values() if row['candidate_set'] in ('r52','r53')]
    consumer = None
    if completion_present:
        consumer_records = [copy.deepcopy(row) for row in players.values() if row.get('target_schema_family') == 'private_source_complete_v2']
        if any(row['authoring_contract_extension_pending'] is not True or row['source_consumer_implemented'] is not False
               or row['runtime_activation'] is not False or row['native_execution_qualified'] is not False for row in consumer_records):
            raise ValueError('consumer worklist must preserve pending qualification')
        consumer = {'schema': 'OTERYN_PRIVATE_SOURCE_CONSUMER_WORKLIST/v1', 'purpose': 'runtime_owner_handoff_not_execution_authority',
            'source_index_path': (DOCS / 'source-closure-review-index.json').as_posix(), 'source_index_sha256': sha(index_body),
            'records_count': len(consumer_records), 'authoring_contract_extension_pending': True,
            'source_consumer_implemented': False, 'runtime_activation': False, 'native_execution_qualified': False,
            'native_identity_allocation': False, 'canonical_selection_changed': False,
            'records': sorted(consumer_records, key=lambda row: row['registration_key']),
            'limits': ['Private source-complete data still need accepted authoring contracts and actual runtime consumers.',
                       'Earlier 354 candidates retain their earlier provider and execution limits; this worklist covers only r59-r62.']}
        worklist['source_private_consumer_worklist_path'] = (DOCS / 'source-private-consumer-worklist.json').as_posix()
        worklist['completed_private_source_data_in_r59_r62'] = [copy.deepcopy(row) for row in players.values()
            if row['candidate_set'] in {f'r{r}' for r in completion_present}]
        worklist['final_completion_audit_path'] = (DOCS / 'final-129-completion-review-index.json').as_posix()
        worklist['reference_only_status_counts'] = index['reference_only_status_counts']
        worklist['remaining_gameplay_data_holds'] = sum('reference_only_status' not in players[row['registration_key']] for row in records)
    if present:
        worklist['completed_target_data_in_r55_r58'] = [copy.deepcopy(row) for row in players.values()
                                                       if row['candidate_set'] in {f'r{r}' for r in present}]
        worklist['unblocking_audit_path'] = (DOCS / 'unblocking-162-review-index.json').as_posix()
    monster = monster_view(root, *loaded[54])
    monster_revisions = tuple(range(63, 67))
    monster_present = [revision for revision in monster_revisions
                       if (root / 'imports/spells' / f'r{revision}/import-manifest.json').exists()]
    if monster_present and len(monster_present) != len(monster_revisions):
        raise ValueError('finish all four private monster imports before rebuilding progress')
    monster_consumer = None
    monster_loaded = {revision: verified_manifest(root, revision) for revision in monster_present}
    if monster_present:
        monster, monster_consumer = private_monster_overlay(root, monster,
            [(revision, *monster_loaded[revision]) for revision in monster_present])
        worklist['monster_source_consumer_worklist_path'] = (DOCS / 'monster-source-consumer-worklist.json').as_posix()
    worklist['monster_target_projection_view'] = {'path': (DOCS / 'monster-target-projection-review-index.json').as_posix(),
        'selected_source_slots': monster['selected_source_slot_count'], 'complete_target_slot_candidates': monster['complete_target_slot_candidates'],
        'partial_target_slots': monster['partial_target_slots'], 'blocked_without_target_slots': monster['blocked_without_target_slots'],
        'remaining_data_holds': monster['remaining_data_holds'], 'runtime_unqualified_slots': monster['runtime_unqualified_slot_count']}
    progress = read_json(root / DOCS / 'source-schema-progress.json')
    progress['source_index_sha256'] = sha(index_body); progress['player_structural_status_counts'] = player_counts
    new_names = ({f'player_unblocking_r{revision}' for revision in present}
                 | {f'private_source_completion_r{revision}' for revision in completion_present}
                 | {f'private_monster_completion_r{revision}' for revision in monster_present})
    old_groups = [group for group in progress['groups'] if group['name'] not in
                  {'player_control_target_projections', 'canonical_monk_target_projections', 'monster_target_projections'} | new_names]
    for revision, name in [(52,'player_control_target_projections'), (53,'canonical_monk_target_projections'), (54,'monster_target_projections')]:
        old_groups.append({'name': name, **loaded[revision][2], 'scope': 'target_data_only_explicit_source_differences_runtime_unqualified'})
    for revision in present:
        old_groups.append({'name': f'player_unblocking_r{revision}', **loaded[revision][2],
                           'scope': 'complete_target_data_partial_models_and_exclusions_separately_counted_runtime_unqualified'})
    for revision in completion_present:
        old_groups.append({'name': f'private_source_completion_r{revision}', **loaded[revision][2],
                          'scope': 'private_source_complete_v2_data_pending_contract_and_consumer_not_runtime_qualified'})
    for revision in monster_present:
        old_groups.append({'name': f'private_monster_completion_r{revision}', **monster_loaded[revision][2],
                           'scope': 'private_monster_slot_v2_data_pending_contract_and_consumer_not_runtime_qualified'})
    progress['groups'] = old_groups
    progress['full_spell_promotions'] = 0
    progress['target_player_candidate_additions'] = sum(loaded[r][1]['counts']['player_candidates'] for r in (52,53))
    if present:
        progress['r55_r58_target_player_candidate_additions'] = sum(loaded[r][1]['counts']['full_player_candidates'] for r in present)
        progress['unblocking_audit_path'] = (DOCS / 'unblocking-162-review-index.json').as_posix()
    if completion_present:
        progress['r59_r62_private_source_candidate_additions'] = index['private_source_complete_v2_candidates']
        progress['reference_only_status_counts'] = index['reference_only_status_counts']
        progress['remaining_gameplay_data_holds'] = worklist['remaining_gameplay_data_holds']
        progress['final_completion_audit_path'] = worklist['final_completion_audit_path']
    progress['monster_target_projection_status'] = worklist['monster_target_projection_view']
    progress['qualification_note'] = 'Source baselines remain immutable. Existing structural candidates and private source-complete v2 data are counted separately. Private authoring contracts and runtime consumers remain pending; no execution promotion is claimed.'
    result = {'monster-target-projection-review-index.json': monster,
              'current-source-gap-worklist.json': worklist, 'source-schema-progress.json': progress}
    if monster_consumer is not None:
        result['monster-source-consumer-worklist.json'] = monster_consumer
        progress['r63_r66_private_monster_data_additions'] = monster['private_monster_slot_v2_candidates']
        progress['monster_source_consumer_worklist_path'] = worklist['monster_source_consumer_worklist_path']
    if consumer is not None:
        progress['source_private_consumer_worklist_path'] = worklist['source_private_consumer_worklist_path']
        result['source-private-consumer-worklist.json'] = consumer
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check-only', action='store_true'); args = parser.parse_args()
    result = build(args.root)
    if not args.check_only:
        for name, value in result.items():
            (args.root / DOCS / name).write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'player_counts': result['source-schema-progress.json']['player_structural_status_counts'],
                      'monster_counts': result['monster-target-projection-review-index.json']['status_counts'],
                      'written': not args.check_only, 'runtime_activation': False}))
