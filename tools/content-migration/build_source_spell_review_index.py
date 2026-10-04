#!/usr/bin/env python3
"""Build an inactive review index; never write a server manifest or select a donor."""
from collections import Counter
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SETS = tuple(range(28, 63))
CANDIDATE_SETS = (28, 29, 30, 34, 42, 44, 46, 52, 53, 55, 56, 57, 58, 59, 60, 61, 62)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def member(folder, name):
    path = folder / name
    if Path(name).is_absolute() or '..' in Path(name).parts or path.is_symlink():
        raise ValueError('unsafe import member path')
    if not path.resolve().is_relative_to(folder.resolve()):
        raise ValueError('import member escapes its set')
    return path


def verified_artifact(folder, manifest, name):
    entry = next((a for a in manifest['artifacts'] if a['path'] == name), None)
    if entry is None:
        raise ValueError('review artifact missing from sealed manifest')
    body = member(folder, name).read_bytes()
    if sha(body) != entry['sha256'] or len(body) != entry['bytes']:
        raise ValueError('review artifact checksum mismatch')
    return body


def verify_reference_receipts(row, standard, projection):
    key = row['registration_key']
    if (projection.get('registration_key') != key or projection.get('status') != row['status']
            or standard.get('registration_key') != key or standard.get('status') != 'BLOCKED'):
        raise ValueError('reference receipt identity mismatch')


def private_qualification(row, receipt, projection):
    if (projection.get('registration_key') != row['registration_key']
            or receipt.get('registration_key') != row['registration_key']
            or receipt.get('status') != row['status']
            or projection.get('target_schema_family') != 'private_source_complete_v2'
            or projection.get('authoring_contract_extension_pending') is not True
            or projection.get('source_consumer_implemented') is not False
            or projection.get('input_provider_equivalence') is not False
            or projection.get('runtime_activation') is not False
            or projection.get('native_execution_qualified') is not False):
        raise ValueError('private source candidate qualification mismatch')
    result = {key: value for key, value in projection.items() if isinstance(value, bool)}
    result['target_schema_family'] = projection['target_schema_family']
    result.update({key: projection[key] for key in ('source_revision', 'source_sha256', 'source_header_sha256', 'native_consumer_identity_guard', 'normalization') if key in projection})
    if 'required_operations_unrepresented' in projection:
        result['required_operations_unrepresented'] = projection['required_operations_unrepresented']
    return result


def build(root):
    imports, manifests, summaries = [], {}, {}
    for revision in SETS:
        folder = root / 'imports/spells' / ('r' + str(revision))
        body = member(folder, 'import-manifest.json').read_bytes()
        manifest = json.loads(body)
        if (manifest['admission_status'] != 'source_only_not_active'
                or manifest['runtime_activation'] is not False
                or manifest['native_identity_allocation'] is not False):
            raise ValueError('active import refused by review index')
        manifests[revision] = manifest
        entry = {'path': (folder / 'import-manifest.json').relative_to(root).as_posix(),
                 'sha256': sha(body), 'counts': manifest['counts'],
                 'admission_status': manifest['admission_status'],
                 'limits': manifest.get('limits', [])}
        if manifest.get('source_metadata_path'):
            metadata = member(folder, manifest['source_metadata_path'])
            if not metadata.is_file():
                raise ValueError('import source metadata missing')
            entry['source_metadata_path'] = metadata.relative_to(root).as_posix()
        imports.append(entry)
        if revision not in CANDIDATE_SETS:
            continue
        if revision == 52:
            names = [a for a in manifest['artifacts']
                     if a['path'].startswith('target-projections/player-control-candidates/')
                     and a['path'].endswith('/receipt.json')]
            if len(names) != 3:
                raise ValueError('expected three player control candidates')
            rows = []
            for entry in names:
                data = member(folder, entry['path']).read_bytes()
                if sha(data) != entry['sha256']:
                    raise ValueError('control candidate receipt checksum mismatch')
                row = json.loads(data)
                if row['status'] != 'CANDIDATE_SCHEMA_VALID':
                    raise ValueError('partial control candidate refused')
                rows.append(row)
            summaries[revision] = ('target-projections/player-control-candidates',
                                   {'records_index': rows})
            continue
        names = [a for a in manifest['artifacts'] if a['path'].endswith('/import-summary.json')]
        if len(names) != 1:
            raise ValueError('expected one player candidate summary')
        entry = names[0]
        data = member(folder, entry['path']).read_bytes()
        if sha(data) != entry['sha256']:
            raise ValueError('imported player summary checksum mismatch')
        summaries[revision] = (Path(entry['path']).parent.as_posix(), json.loads(data))
    records = {}
    for revision in CANDIDATE_SETS:
        candidate_parent, summary = summaries[revision]
        for row in summary['records_index']:
            key = row['registration_key']
            if revision != 28 and (key not in records or records[key]['structural_status'] != 'BLOCKED'):
                raise ValueError('overlay must address an existing blocked registration exactly once')
            if revision >= 55 and row['status'] != 'CANDIDATE_SCHEMA_VALID':
                # Reference-only overlays preserve the original BLOCKED receipt.
                if revision >= 59:
                    parts = Path(candidate_parent) / key.split('/')[0] / sha(key.encode())[:16]
                    folder = root / 'imports/spells' / ('r' + str(revision))
                    standard_path = (parts / 'receipt.json').as_posix()
                    standard = json.loads(verified_artifact(folder, manifests[revision], standard_path))
                    ref_path = (parts / 'projection-receipt.json').as_posix()
                    reference = json.loads(verified_artifact(folder, manifests[revision], ref_path))
                    verify_reference_receipts(row, standard, reference)
                    records[key].update({'reference_only_status': row['status'], 'reference_set': f'r{revision}',
                                         'reference_receipt_path': (folder / ref_path).relative_to(root).as_posix(),
                                         'reference_standard_receipt_path': (folder / standard_path).relative_to(root).as_posix()})
                continue
            parts = (Path(candidate_parent) / row['snapshot'] if revision == 57
                     else Path(candidate_parent) / key.split('/')[0] / sha(key.encode())[:16])
            folder = root / 'imports/spells' / ('r' + str(revision))
            path = member(folder, (parts / 'receipt.json').as_posix())
            if not path.is_file():
                raise ValueError('candidate receipt missing')
            candidate_key = row.get('candidate_key')
            qualification = {}
            if revision >= 59:
                receipt = json.loads(verified_artifact(folder, manifests[revision], (parts / 'receipt.json').as_posix()))
                projection_path = (parts / 'projection-receipt.json').as_posix()
                projection = json.loads(verified_artifact(folder, manifests[revision], projection_path))
                qualification = private_qualification(row, receipt, projection)
                qualification['projection_receipt_path'] = (folder / projection_path).relative_to(root).as_posix()
                body = verified_artifact(folder, manifests[revision], (parts / 'spell.json').as_posix())
                identity = json.loads(body)['spell']['identity']
                if candidate_key is not None and candidate_key != identity['key']:
                    raise ValueError('private candidate identity mismatch')
                candidate_key = identity['key']
            if revision == 57:
                spell_path = (parts / 'spell.json').as_posix()
                body = member(folder, spell_path).read_bytes()
                entry = next(a for a in manifests[revision]['artifacts'] if a['path'] == spell_path)
                if sha(body) != entry['sha256']:
                    raise ValueError('summon candidate checksum mismatch')
                candidate_key = json.loads(body)['spell']['identity']['key']
                if json.loads(path.read_bytes())['registration_key'] != key:
                    raise ValueError('summon candidate receipt identity mismatch')
            records[key] = {'registration_key': key, 'candidate_key': candidate_key,
                            'structural_status': row['status'], 'candidate_set': 'r' + str(revision),
                            'receipt_path': path.relative_to(root).as_posix(),
                            'source_header_path': (folder / parts / 'source-header.json').relative_to(root).as_posix(),
                            'runtime_activation': False, **qualification}
            if row['status'] == 'CANDIDATE_SCHEMA_VALID':
                records[key]['candidate_bundle_path'] = (folder / parts).relative_to(root).as_posix()
    if len(records) != 483:
        raise ValueError('current player population changed')
    return {'schema': 'OTERYN_SOURCE_SPELL_REVIEW_INDEX/v1',
            'admission_status': 'source_only_not_active', 'runtime_activation': False,
            'native_identity_allocation': False, 'canonical_selection_changed': False,
            'selection_purpose': 'local_review_only_no_runtime_consumer',
            'player_registration_count': len(records),
            'player_structural_status_counts': dict(Counter(r['structural_status'] for r in records.values())),
            'reference_only_status_counts': dict(Counter(r['reference_only_status'] for r in records.values() if 'reference_only_status' in r)),
            'private_source_complete_v2_candidates': sum(r.get('target_schema_family') == 'private_source_complete_v2' for r in records.values()),
            'source_pins': manifests[28]['source_pins'], 'import_sets': imports,
            'player_records': [records[key] for key in sorted(records)],
            'limits': ['Donor registrations are variants, not unique playable spell counts.',
                       'Structural validity does not qualify native execution, providers or renderer bindings.',
                       'Typed source evidence is retained in the listed import sets without upgrading blocked registrations.',
                       'Private source-complete v2 data preserve pending authoring contracts and absent runtime consumers.',
                       'This review view does not choose Canary versus Crystal or change server selection.']}


if __name__ == '__main__':
    value = build(ROOT)
    path = ROOT / 'docs/reference/spells/source-closure-review-index.json'
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')
    print(json.dumps({'path': str(path), 'counts': value['player_structural_status_counts'],
                      'runtime_activation': False}))
