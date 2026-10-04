#!/usr/bin/env python3
"""Review the exact former 129 data holds; never qualify runtime or contracts."""
from collections import Counter
import json
from pathlib import Path

from build_source_projection_progress import ROOT, DOCS, verified_manifest, sha

FULL = 'CANDIDATE_SCHEMA_VALID'
REFERENCES = {'REFERENCE_ONLY_REMOVED_S24', 'REFERENCE_ONLY_RETIRED_S24', 'REFERENCE_ONLY_DISABLED'}


def check_record(row, current, revision):
    if row['registration_key'] != current['registration_key']:
        raise ValueError('completion registration identity mismatch')
    if row['status'] == FULL:
        if (current.get('candidate_set') != f'r{revision}'
                or current['structural_status'] != FULL
                or current.get('target_schema_family') != 'private_source_complete_v2'
                or current.get('authoring_contract_extension_pending') is not True
                or current.get('source_consumer_implemented') is not False
                or current.get('input_provider_equivalence') is not False
                or current.get('runtime_activation') is not False
                or current.get('native_execution_qualified') is not False):
            raise ValueError('private completion cannot qualify a consumer or contract')
    elif row['status'] in REFERENCES:
        if (current.get('reference_set') != f'r{revision}' or current['structural_status'] != 'BLOCKED'
                or current.get('reference_only_status') != row['status']):
            raise ValueError('reference-only registration promoted to candidate')
    else:
        raise ValueError('unknown final data disposition')


def build(root):
    index_path = root / DOCS / 'source-closure-review-index.json'
    index_body = index_path.read_bytes(); index = json.loads(index_body)
    if index['runtime_activation'] is not False or index['canonical_selection_changed'] is not False:
        raise ValueError('active index cannot enter data completion review')
    players = {row['registration_key']: row for row in index['player_records']}
    historical_path = root / DOCS / 'unblocking-162-review-index.json'
    historical_body = historical_path.read_bytes(); historical = json.loads(historical_body)
    previous = {row['registration_key']: row for row in historical['records'] if not row['full_target_data_candidate']}
    if len(previous) != 129 or historical['full_target_data_candidate_additions'] != 33:
        raise ValueError('historical 162 audit changed meaning')
    known = {row['path']: row['sha256'] for row in index['import_sets']}
    records, groups = {}, []
    for revision in range(59, 63):
        folder, manifest, pin = verified_manifest(root, revision)
        if known.get(pin['manifest_path']) != pin['manifest_sha256']:
            raise ValueError('rebuild index from sealed completion imports first')
        audit_path = folder / manifest['source_metadata_path']; audit = json.loads(audit_path.read_bytes())
        rows = audit if isinstance(audit, list) else audit['records']
        group_full = group_references = 0
        for row in rows:
            key = row['registration_key']
            if key not in previous or key in records:
                raise ValueError('completion must cover former data holds exactly once')
            check_record(row, players[key], revision)
            if row.get('runtime_activation', audit.get('runtime_activation') if isinstance(audit, dict) else None) is not False:
                raise ValueError('active lane audit refused')
            complete = row['status'] == FULL
            group_full += complete; group_references += not complete
            records[key] = {'registration_key': key, 'baseline_lane': previous[key].get('baseline_lane', previous[key].get('lane')),
                'historical_audit_status': previous[key]['status'], 'current_data_status': row['status'],
                'candidate_set': f'r{revision}' if complete else None, 'reference_set': None if complete else f'r{revision}',
                'full_private_source_data_candidate': complete,
                'receipt_path': players[key]['receipt_path'] if complete else players[key]['reference_receipt_path'],
                'source_header_path': players[key]['source_header_path'], 'runtime_activation': False,
                'native_execution_qualified': False, 'canonical_selection_changed': False}
            if complete:
                records[key].update({name: players[key][name] for name in (
                    'target_schema_family', 'authoring_contract_extension_pending', 'source_consumer_implemented', 'projection_receipt_path')})
        if (manifest['counts']['full_source_data_candidates'] != group_full
                or manifest['counts']['reference_records'] != group_references):
            raise ValueError('completion manifest/lane count mismatch')
        groups.append({**pin, 'full_private_source_data_candidates': group_full, 'reference_only_records': group_references})
    if set(records) != set(previous):
        raise ValueError('completion cohort lost former data holds')
    reference_counts = dict(Counter(row['current_data_status'] for row in records.values() if not row['full_private_source_data_candidate']))
    full = sum(row['full_private_source_data_candidate'] for row in records.values())
    return {'schema': 'OTERYN_FINAL_129_SOURCE_DATA_COMPLETION_REVIEW/v1',
            'view_scope': 'source_data_only_pending_private_contracts_and_consumers',
            'former_data_hold_variants': len(records), 'new_private_source_complete_v2_candidates': full,
            'reference_only_variants': len(records) - full, 'reference_only_status_counts': reference_counts,
            'remaining_gameplay_data_holds_in_this_cohort': 0,
            'authoring_contract_extension_pending_candidates': full, 'source_consumer_unimplemented_candidates': full,
            'runtime_candidates_qualified_by_this_view': 0, 'runtime_activation': False,
            'native_execution_qualified': False, 'native_identity_allocation': False, 'canonical_selection_changed': False,
            'player_structural_status_counts': index['player_structural_status_counts'],
            'source_index_path': index_path.relative_to(root).as_posix(), 'source_index_sha256': sha(index_body),
            'historical_162_audit': {'path': historical_path.relative_to(root).as_posix(), 'sha256': sha(historical_body),
                'original_target_data_additions': 33, 'original_remaining_data_holds': 129},
            'import_groups': groups, 'records': [records[key] for key in sorted(records)],
            'limits': ['475 candidates are donor registration variants with data schemas, not a playable spell count.',
                '121 new private source-complete v2 candidates retain pending authoring contracts and absent runtime consumers.',
                'Two Sap Strength, two Expose Weakness and four disabled examples remain references with original BLOCKED receipts.',
                'This view closes the selected player data cohort; it does not qualify engine providers, assets, monster behavior or runtime activation.']}


if __name__ == '__main__':
    value = build(ROOT); path = ROOT / DOCS / 'final-129-completion-review-index.json'
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: value[key] for key in ('new_private_source_complete_v2_candidates', 'reference_only_variants', 'player_structural_status_counts', 'runtime_activation')}))
