"""Build a local view of actual r55-r58 lane audits; no runtime admission."""
from collections import Counter
import json
from pathlib import Path
from build_source_projection_progress import ROOT, DOCS, verified_manifest, sha


def build(root):
    index_path = root / DOCS / 'source-closure-review-index.json'
    index = json.loads(index_path.read_bytes())
    player = {row['registration_key']: row for row in index['player_records']}
    known = {row['path']: row['sha256'] for row in index['import_sets']}
    records, groups = {}, []
    for revision in range(55, 59):
        folder, manifest, pin = verified_manifest(root, revision)
        if known.get(pin['manifest_path']) != pin['manifest_sha256']:
            raise ValueError('rebuild player index from actual new imports first')
        audit_path = folder / 'player-source-candidates/lane-audit.json'
        audit = json.loads(audit_path.read_bytes())
        rows = audit if isinstance(audit, list) else audit.get('rows', audit.get('records'))
        if not isinstance(rows, list):
            raise ValueError('audit must contain an actual row list')
        inherited = audit if isinstance(audit, dict) else {}
        groups.append({**pin, 'audit_path': audit_path.relative_to(root).as_posix(),
                       'audit_sha256': sha(audit_path.read_bytes()), 'audited_records': len(rows)})
        full = 0
        for row in rows:
            key = row['registration_key']
            if key in records or key not in player:
                raise ValueError('duplicate or unknown audited registration')
            if (row.get('runtime_activation', inherited.get('runtime_activation')) is not False
                    or row.get('native_execution_qualified', inherited.get('native_execution_qualified')) is not False):
                raise ValueError('runtime admission cannot enter data review')
            complete = row['status'] == 'CANDIDATE_SCHEMA_VALID'
            if complete:
                if player[key]['structural_status'] != row['status'] or player[key]['candidate_set'] != f'r{revision}':
                    raise ValueError('full data candidate missing from effective player index')
                full += 1
            elif player[key]['structural_status'] != 'BLOCKED' and player[key]['candidate_set'] not in ('r59', 'r60', 'r61', 'r62'):
                raise ValueError('partial audit changed outside the later completion batch')
            records[key] = {**row, 'audit_import_set': f'r{revision}',
                            'full_target_data_candidate': complete,
                            'effective_structural_status': 'CANDIDATE_SCHEMA_VALID' if complete else 'BLOCKED'}
        if manifest['counts']['full_player_candidates'] != full:
            raise ValueError('audit/manifest candidate count mismatch')
    baseline = json.loads((root / DOCS / 'blocked-173-worklist.json').read_bytes())
    baseline_keys = {row['registration_key'] for row in baseline['records']}
    prior_completed = {row['registration_key'] for row in player.values()
                       if row['candidate_set'] in ('r46', 'r52', 'r53')}
    if set(records) != baseline_keys - prior_completed or len(records) != 162:
        raise ValueError('expected exact original 162-record audit population')
    full_count = sum(row['full_target_data_candidate'] for row in records.values())
    reference_counts = Counter(row.get('disposition') for row in records.values()
                               if row.get('disposition') in
                               ('DISABLED_REFERENCE_EXAMPLE', 'RETIRED_REFERENCE_ONLY_S24'))
    if reference_counts.get('DISABLED_REFERENCE_EXAMPLE', 0) != 4:
        raise ValueError('the four disabled examples must remain explicit references')
    historical_path = root / DOCS / 'unblocking-162-review-index.json'
    historical = json.loads(historical_path.read_bytes())
    historical_sha = historical.get('historical_source_index_sha256', historical['source_index_sha256'])
    return {'schema': 'OTERYN_SOURCE_UNBLOCKING_REVIEW/v1', 'audited_player_variants': len(records),
            'full_target_data_candidate_additions': full_count,
            'remaining_structural_holds_from_this_population': len(records) - full_count,
            'reference_only_variant_counts': dict(reference_counts),
            'remaining_gameplay_data_holds': len(records) - full_count - sum(reference_counts.values()),
            'player_structural_status_counts': {'CANDIDATE_SCHEMA_VALID': 354, 'BLOCKED': 129},
            'audit_status_counts': dict(Counter(row['status'] for row in records.values())),
            'source_index_path': index_path.relative_to(root).as_posix(),
            'source_index_sha256': historical_sha, 'import_groups': groups,
            'runtime_activation': False, 'native_execution_qualified': False,
            'canonical_selection_changed': False,
            'records': [records[key] for key in sorted(records)],
            'limits': ['Structural candidate status does not admit execution.',
                       'Four disabled source examples are separately excluded in lane audits.',
                       'Original blocked receipts and earlier sealed imports remain unchanged.',
                       'Partial target definitions and profile aliases do not receive full candidate status.']}


if __name__ == '__main__':
    result = build(ROOT)
    path = ROOT / DOCS / 'unblocking-162-review-index.json'
    path.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(json.dumps({key: value for key, value in result.items() if key not in ('records', 'import_groups', 'limits')}))
