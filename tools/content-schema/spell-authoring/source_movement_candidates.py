#!/usr/bin/env python3
"""R46: exact Canary Levitate/Magic Rope source-only movement candidates."""
import argparse
import copy
import gzip
import json
from pathlib import Path
import tempfile

import import_source_player_bundles as base
import native_house_movement as native
import source_movement_helper_closure as closure
import validate_spell

HERE = Path(__file__).resolve().parent
ORIGINAL = Path('docs/reference/spells/r28-source-closure/player-source-bundles')
OUTPUT = Path('docs/reference/spells/r46-source-closure/player-movement-candidates')
REVISION = closure.REVISION
CANDIDATE_REVISION = 'source-player-r46'
FACT_SHAS = {'levitate': '968fe8e45d422ac90ab8a47bedb8d8080effb8f585517bb9cca256d1e30fc84b',
             'magic rope': '71bbeba5a38c1b1822299c356b30e044fc1ff60c41c655d2c080c411978890a2'}
sha = closure.sha


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def qualify_fact(name, fact, source):
    path = native.FILES[name]
    registration = 'canary-main-current/' + path + '#1'
    if (fact['registration_key'] != registration or fact['source_revision'] != REVISION
            or fact['source_sha256'] != sha(source) or sha(canonical(fact)) != FACT_SHAS[name]):
        raise ValueError('exact movement source/capture identity mismatch')
    return registration


def build(repo, source_root):
    repo, source_repo = Path(repo), Path(source_root) / 'canary'
    capture_bytes = (repo / ORIGINAL / 'source-callback-facts.jsonl.gz').read_bytes()
    facts = [json.loads(line) for line in gzip.decompress(capture_bytes).splitlines()]
    if len(facts) != 483:
        raise ValueError('immutable source capture population changed')
    index = json.loads((repo / ORIGINAL / 'import-summary.json').read_text())
    defaults, default_proofs = base.default_fields(source_repo, REVISION)
    values, records, proofs = {}, [], []
    for name in FACT_SHAS:
        helper_proof, current, old = closure.build(source_repo, name)
        path = native.FILES[name]
        registration = 'canary-main-current/' + path + '#1'
        selected = [fact for fact in facts if fact['registration_key'] == registration]
        if len(selected) != 1:
            raise ValueError('immutable movement capture registration not unique')
        fact = selected[0]
        qualify_fact(name, fact, current[path])
        row_id = sha(registration.encode())[:16]
        row_path = Path('canary-main-current') / row_id
        candidate_key = 'candidate:spell/source/canary-main-current/' + row_id
        matching = [row for row in index['records_index'] if row['registration_key'] == registration]
        if index['records'] != 483 or len(matching) != 1 or matching[0]['status'] != 'BLOCKED' or matching[0]['candidate_key'] != candidate_key:
            raise ValueError('movement overlay requires unique blocked original registration')
        old_receipt_path = repo / ORIGINAL / row_path / 'receipt.json'
        old_header_path = repo / ORIGINAL / row_path / 'source-header.json'
        receipt = json.loads(old_receipt_path.read_text())
        header = json.loads(old_header_path.read_text())
        if receipt['status'] != 'BLOCKED' or receipt['source_sha256'] != sha(current[path]) or receipt['source_header'] != header['spell']:
            raise ValueError('historical movement header/receipt/source mismatch')
        # Execute the existing authoring builder on its exact original reference bytes.
        # This temp tree is proof input, not a runtime environment or source payload.
        with tempfile.TemporaryDirectory(prefix='.movement-template-', dir=repo.parent) as temporary:
            reference_root = Path(temporary)
            for support in closure.FILES:
                target = reference_root / support
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(old[support])
            record = {**fact['source_callback_facts'], 'revision': closure.OLD_REVISION,
                      'source_root': str(reference_root)}
            descriptor = native.build(name, record['spell_type'], {'canary': record}, {('canary', path): old[path].decode()})
        spell = base.fill_header(header['spell'], defaults, {'key': candidate_key, 'revision': CANDIDATE_REVISION}, fact['source_callback_facts']['registrar'])
        spell['execution'] = {'native_behavior': descriptor}
        deps, catalog = {'abilities': [], 'effects': [], 'formulas': []}, {'definitions': []}
        errors = validate_spell.validate({'spell': spell}, deps, catalog)
        if errors:
            raise ValueError('movement existing schema/semantic validation: ' + ' | '.join(errors))
        receipt = copy.deepcopy(receipt)
        receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[], dependencies={key: 0 for key in deps},
                       schema_and_semantic_validation_errors=[], native_execution_qualified=False,
                       item_owner_bindings_required=[], remaining_mechanics=[
                           {'source_field': 'source.engine_and_transitive_API_execution', 'reason': 'Only the existing Lua authoring descriptor transfer is qualified. Native movement/teleport queryAdd, entry permissions, dynamic world state, transitive table.contains/startup overrides and engine flag providers remain unqualified.'},
                           {'source_field': 'source.assets_and_runtime_admission', 'reason': 'Exact source presentation declarations are retained; asset bytes, sound playback, native identities, canonical selection and runtime activation remain unqualified.'},
                           {'source_field': 'source.cached_callback_truncation', 'reason': 'Historical bounded callback bodies are truncated. They remain evidence only; full-file cast SHA and full support-byte proof qualify descriptor transfer.'}],
                       conversion_notes=['Exact current Canary cast bytes equal the previously qualified movement template.',
                                         'Exact global.lua difference restricted to the reviewed PvP function replacement; full tile.lua/position.lua bytes remain equal.',
                                         'Reuse existing vertical_move schema; source-player-r46 is inactive source data only.'])
        validate_spell.Draft202012Validator(base.receipt_schema(), registry=validate_spell.REGISTRY).validate(receipt)
        for filename, value in {'source-header.json': header, 'receipt.json': receipt, 'spell.json': {'spell': spell},
                                'dependencies.json': deps, 'catalog.json': catalog}.items():
            values[str(row_path / filename)] = value
        record_index = {'registration_key': registration, 'candidate_key': candidate_key,
                        'candidate_revision': CANDIDATE_REVISION, 'status': 'CANDIDATE_SCHEMA_VALID', 'blockers': []}
        records.append(record_index)
        proofs.append({**record_index, 'source_revision': REVISION, 'source_path': path,
                       'source_sha256': sha(current[path]), 'source_git_blob': fact['source_callback_facts']['blob'],
                       'selected_capture_fact_sha256': sha(canonical(fact)),
                       'historical_receipt_sha256': sha(old_receipt_path.read_bytes()),
                       'historical_header_sha256': sha(old_header_path.read_bytes()),
                       'native_behavior_sha256': sha(canonical(descriptor)), 'movement_helper_closure': helper_proof,
                       'native_execution_qualified': False})
    common = {'runtime_activation': False, 'external_sources_used': False, 'native_identity_allocation': False,
              'canonical_selection_changed': False, 'native_execution_qualified': False, 'legacy_matcher_modified': False}
    values['receipt.schema.json'] = base.receipt_schema()
    values['source-qualification-proof.json'] = {
        'schema': 'OTERYN_SOURCE_MOVEMENT_QUALIFICATION/v1', **common, 'records': proofs,
        'source_capture_gzip_sha256': sha(capture_bytes), 'source_capture_records': len(facts),
        'qualification_scope': 'existing_authoring_movement_descriptor_transfer_only',
        'engine_default_proofs': default_proofs,
        'producer_sha256': sha(Path(__file__).read_bytes()), 'helper_producer_sha256': sha(Path(closure.__file__).read_bytes()),
        'native_template_module_sha256': sha(Path(native.__file__).read_bytes()),
        'schema_proofs': {name: sha((HERE / name).read_bytes()) for name in ['spell.schema.json', 'spell-dependencies.schema.json']}}
    values['import-summary.json'] = {
        'schema': 'OTERYN_SOURCE_PLAYER_BUNDLE_IMPORT/v1', 'revision': CANDIDATE_REVISION, 'records': 2,
        **common, 'source_populations': {'canary-main-current': 2}, 'status_counts': {'CANDIDATE_SCHEMA_VALID': 2},
        'native_descriptor_count': 2, 'all_receipts_schema_valid': True, 'full_source_mechanics_1_to_1_complete': False,
        'captured_records': len(facts), 'source_callback_payload_sha256': sha(gzip.decompress(capture_bytes)),
        'input_proofs': {'r28/package-manifest.json': sha((repo / ORIGINAL / 'package-manifest.json').read_bytes()),
                         'r28/import-summary.json': sha((repo / ORIGINAL / 'import-summary.json').read_bytes())},
        'converter_proofs': {path.name: sha(path.read_bytes()) for path in [Path(__file__), Path(closure.__file__),
                            Path(native.__file__), Path(base.__file__), Path(validate_spell.__file__),
                            HERE / 'spell.schema.json', HERE / 'spell-dependencies.schema.json']},
        'records_index': records}
    return values, capture_bytes


def write(repo, source_root):
    repo = Path(repo).resolve()
    values, capture_bytes = build(repo, source_root)
    out = repo / OUTPUT
    out.mkdir(parents=True, exist_ok=True)
    for relative, value in values.items():
        path = out / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(value, sort_keys=True, ensure_ascii=False, indent=2) + '\n')
    (out / 'source-callback-facts.jsonl.gz').write_bytes(capture_bytes)
    manifest = {'schema': 'OTERYN_SOURCE_PLAYER_PACKAGE/v1',
                'files': {str(path.relative_to(out)): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'package-manifest.json'}}
    (out / 'package-manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo', default='.')
    parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args()
    print(json.dumps(write(args.repo, args.source_root), sort_keys=True))
