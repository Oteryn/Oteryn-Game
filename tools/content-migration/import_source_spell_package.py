#!/usr/bin/env python3
"""Copy verified r28 source facts into an immutable, inactive reference import."""
import argparse
from collections import Counter
import gzip
import hashlib
import io
import json
from pathlib import Path
import shutil
import tarfile

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[2]
SOURCE = 'docs/reference/spells/r28-source-closure'
SCHEMAS = {
    'monster.schema.json': 'monster-authoring/monster.schema.json',
    'monster-dependencies.schema.json': 'monster-authoring/monster-dependencies.schema.json',
    'monster-import-readiness.schema.json': 'monster-authoring/monster-import-readiness.schema.json',
    'source-profile-facts.schema.json': 'monster-authoring/source-profile-facts.schema.json',
    'player-source-import.schema.json': 'spell-authoring/player-source-import.schema.json',
    'player-source-projection.schema.json': 'spell-authoring/player-source-projection.schema.json',
    'spell-source-registrar.schema.json': 'spell-authoring/spell-source-registrar.schema.json',
    'spell.schema.json': 'spell-authoring/spell.schema.json',
    'spell-dependencies.schema.json': 'spell-authoring/spell-dependencies.schema.json',
}
ARTIFACTS = {
    'monster-source-package.tar.gz': ('monster_candidate_bundle_archive', ['monster.schema.json', 'monster-dependencies.schema.json', 'monster-import-readiness.schema.json']),
    'monster-source-package-manifest.json': ('monster_package_receipt', []),
    'monster-import-summary.json': ('monster_population_summary', []),
    'monster-verification-proof.json': ('monster_conservation_proof', []),
    'source-profile-facts.jsonl.gz': ('monster_source_profile_facts', ['source-profile-facts.schema.json']),
    'monster-profile-field-coverage.json': ('monster_source_field_coverage', []),
    'source-mechanics-inventory.json.gz': ('source_mechanics_inventory', ['source-mechanics-evidence.schema.json']),
    'source-mechanics-coverage.json': ('source_mechanics_coverage', []),
    'source-mechanics-receipt.json': ('source_mechanics_proof', []),
    'source-population-audit.json': ('source_population_audit', []),
    'player-source-registrars.jsonl.gz': ('player_source_registrars', ['player-source-import.schema.json', 'spell-source-registrar.schema.json']),
    'player-source-authoring-projections.jsonl.gz': ('player_partial_authoring_projections', ['player-source-projection.schema.json', 'spell.schema.json']),
    'player-source-import-proof.json': ('player_registrar_proof', []),
    'player-source-projection-proof.json': ('player_projection_proof', []),
    'player-source-schema-coverage.json': ('player_source_schema_coverage', []),
}
SUPPLEMENTS = {
    'source-custom-mechanics.json.gz': ('source-custom-mechanics-qualification.json', 'source-custom-mechanics.schema.json', 'json'),
    'player-source-guards.jsonl.gz': ('player-source-guards-proof.json', 'player-source-guards.schema.json', 'jsonl'),
    'source-formula-evidence.jsonl.gz': ('source-formula-evidence-receipt.json', 'source-formula-evidence.schema.json', 'jsonl'),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pinned(data, name, expected):
    require(name in data and digest(data[name]) == expected, 'SOURCE_HASH_MISMATCH:' + name)


def snapshot_schemas(root, source):
    schemas = {name: (root / 'tools/content-schema' / path).read_bytes() for name, path in SCHEMAS.items()}
    schemas['source-mechanics-evidence.schema.json'] = (source / 'source-mechanics-evidence.schema.json').read_bytes()
    player_schema = source / 'player-source-bundles/receipt.schema.json'
    if player_schema.exists():
        schemas['player-bundle-receipt.schema.json'] = player_schema.read_bytes()
    for _, (proof, name, _) in SUPPLEMENTS.items():
        if (source / proof).exists():
            schemas[name] = (root / 'tools/content-schema/spell-authoring' / name).read_bytes()
    documents = {name: json.loads(data) for name, data in schemas.items()}
    uris = {name: doc.get('$id', 'schemas/' + name) for name, doc in documents.items()}
    registry = Registry().with_resources((uris[name], Resource.from_contents(doc)) for name, doc in documents.items())
    def references(value):
        if isinstance(value, dict):
            for key, child in value.items():
                if key == '$ref' and not child.startswith('#'):
                    yield child.split('#', 1)[0]
                else:
                    yield from references(child)
        elif isinstance(value, list):
            for child in value:
                yield from references(child)
    validators = {}
    for name, document in documents.items():
        Draft202012Validator.check_schema(document)
        require(set(references(document)) <= set(uris.values()), 'UNRESOLVED_SCHEMA_URI:' + name)
        validators[name] = Draft202012Validator(document, registry=registry)
    return schemas, uris, validators


def verify_monster_archive(data, receipt):
    pinned(data, 'monster-source-package.tar.gz', receipt['archive']['sha256'])
    require(receipt['runtime_activation'] is False and all(receipt['checks'].values()), 'MONSTER_PROOF_NOT_PASSED')
    seen, hashes = set(), {}
    with tarfile.open(fileobj=io.BytesIO(data['monster-source-package.tar.gz']), mode='r|gz') as archive:
        for member in archive:
            name = member.name
            require(member.isfile() and name not in seen and not Path(name).is_absolute() and '..' not in Path(name).parts,
                    'UNSAFE_ARCHIVE_MEMBER:' + name)
            require(name.endswith('.json') or name == 'SHA256SUMS', 'ORIGINAL_ASSET_REFUSED:' + name)
            seen.add(name)
            body = archive.extractfile(member).read()
            if name == 'SHA256SUMS':
                hashes = dict((n, h) for h, n in (line.split('  ', 1) for line in body.decode().splitlines()))
            else:
                require(digest(body) == hashes.get(name), 'ARCHIVE_HASH_MISMATCH:' + name)
            if name in ('summary.json', 'verification-proof.json'):
                destination = 'monster-import-summary.json' if name == 'summary.json' else 'monster-verification-proof.json'
                require(body == data[destination], 'ARCHIVE_PROOF_COPY_MISMATCH:' + name)
    require(seen == set(hashes) | {'SHA256SUMS'} and len(seen) == receipt['archive']['members'], 'ARCHIVE_MEMBERSHIP_MISMATCH')


def verified_inputs(source, root):
    data = {name: (source / name).read_bytes() for name in ARTIFACTS}
    schemas, uris, validators = snapshot_schemas(root, source)
    read = lambda name: json.loads(data[name])
    monster = read('monster-source-package-manifest.json')
    verify_monster_archive(data, monster)
    summary, proof = read('monster-import-summary.json'), read('monster-verification-proof.json')
    require(proof['status'] == 'PASS' and proof['checks'] > 0 and not proof['failures'], 'MONSTER_CONSERVATION_FAILED')
    require(summary['runtime_activation'] is False and summary['source_first'] is True, 'MONSTER_IMPORT_NOT_SOURCE_ONLY')
    pinned(data, 'monster-import-summary.json', proof['input_sha256']['summary.json'])
    coverage = read('monster-profile-field-coverage.json')
    pinned(data, coverage['facts_file'], coverage['facts_gzip_sha256'])
    facts = gzip.decompress(data[coverage['facts_file']])
    require(digest(facts) == coverage['facts_payload_sha256'], 'PROFILE_PAYLOAD_HASH_MISMATCH')
    require(digest(schemas['source-profile-facts.schema.json']) == coverage['facts_schema_sha256'], 'PROFILE_SCHEMA_MISMATCH')
    records = [json.loads(line) for line in facts.splitlines()]
    donors = {s['source']: s for s in summary['sources']}
    for record in records:
        validators['source-profile-facts.schema.json'].validate(record)
        require(record['runtime_activation'] is False, 'PROFILE_RUNTIME_ACTIVATION_REFUSED')
        donor = donors[record['candidate_id'].split('/', 1)[0]]
        require(all(record['provenance'][key] == donor[key] for key in ('repository', 'revision')), 'PROFILE_SOURCE_PIN_MISMATCH')
    require(len(records) == coverage['counts']['profiles'] == summary['total_monster_profiles'], 'PROFILE_COUNT_MISMATCH')
    require(coverage['source_profile_inventory_sha256'] == proof['input_sha256']['monster-profiles.json'], 'PROFILE_INVENTORY_MISMATCH')
    mechanics_coverage, mechanics_receipt = read('source-mechanics-coverage.json'), read('source-mechanics-receipt.json')
    for entry in mechanics_receipt['artifacts']:
        if not entry['path'].startswith(SOURCE + '/'):
            continue  # Generator code hashes describe the archived generation, not current execution authority.
        path = (root / entry['path']).resolve()
        require(path.is_relative_to(root.resolve()) and digest(path.read_bytes()) == entry['sha256'], 'MECHANICS_RECEIPT_HASH_MISMATCH:' + entry['path'])
    pinned(data, 'source-mechanics-inventory.json.gz', mechanics_coverage['inventory_gzip_sha256'])
    mechanics_bytes = gzip.decompress(data['source-mechanics-inventory.json.gz'])
    require(digest(mechanics_bytes) == mechanics_coverage['inventory_sha256'], 'MECHANICS_PAYLOAD_HASH_MISMATCH')
    mechanics = json.loads(mechanics_bytes)
    validators['source-mechanics-evidence.schema.json'].validate(mechanics)
    require(mechanics['runtime_activation'] is False and mechanics['counts'] == mechanics_coverage['counts'], 'MECHANICS_COUNT_OR_ACTIVATION_MISMATCH')
    require(len(mechanics['files']) == mechanics['counts']['files'] == summary['total_spell_registrations'], 'MECHANICS_POPULATION_MISMATCH')
    require(sum(len(f['calls']) for f in mechanics['files']) == mechanics['counts']['calls'], 'MECHANICS_CALL_COUNT_MISMATCH')
    require(all(f['revision'] == donors[f['source']]['revision'] for f in mechanics['files']), 'MECHANICS_SOURCE_PIN_MISMATCH')
    player_raw, player_projection = read('player-source-import-proof.json'), read('player-source-projection-proof.json')
    player_coverage = read('player-source-schema-coverage.json')
    for player_proof, schema in [(player_raw, 'player-source-import.schema.json'), (player_projection, 'player-source-projection.schema.json')]:
        pinned(data, player_proof['artifact'], player_proof['artifact_sha256'])
        pinned(data, 'player-source-schema-coverage.json', player_proof['coverage_report_sha256'])
        payload = gzip.decompress(data[player_proof['artifact']])
        require(digest(payload) == player_proof['payload_sha256'], 'PLAYER_PAYLOAD_HASH_MISMATCH')
        player_records = [json.loads(line) for line in payload.splitlines()]
        require(len(player_records) == player_proof['records'] == player_proof['unique_registration_keys'], 'PLAYER_COUNT_MISMATCH')
        require(len({x['registration_key'] for x in player_records}) == len(player_records), 'DUPLICATE_PLAYER_REGISTRATION')
        require(dict(Counter(x['snapshot'] for x in player_records)) == player_proof['counts_by_snapshot'], 'PLAYER_SNAPSHOT_COUNT_MISMATCH')
        for record in player_records:
            validators[schema].validate(record)
            require(record['execution_mapped'] is False, 'PLAYER_SOURCE_FACTS_NOT_EXECUTION')
            require(record['source_revision'] == player_raw['source_revisions'][record['snapshot']], 'PLAYER_SOURCE_PIN_MISMATCH')
    for schema, expected in [('player-source-import.schema.json', player_raw['record_schema_sha256']),
                             ('spell-source-registrar.schema.json', player_raw['registrar_schema_sha256']),
                             ('player-source-projection.schema.json', player_projection['projection_schema_sha256']),
                             ('spell.schema.json', player_projection['authoring_schema_sha256'])]:
        require(digest(schemas[schema]) == expected, 'PLAYER_SCHEMA_PIN_MISMATCH:' + schema)
    require(player_raw['records'] == player_projection['records'], 'PLAYER_COMPONENT_COUNT_MISMATCH')
    return data, schemas, uris, {
        'monster_profiles': summary['total_monster_profiles'], 'monster_spell_slots': summary['total_source_spell_slots'],
        'monster_registered_variants': summary['total_spell_registrations'], 'monster_slot_outcomes': proof['slot_conversion_counts'],
        'monster_declarative_fields': coverage['counts'], 'source_mechanics': mechanics['counts'], 'player_source_registrars': player_raw['records'],
        'player_source_authoring_projections': player_projection['records'],
    }, {'monster_donors': summary['sources'], 'player_snapshots': player_raw['source_revisions'],
        'monster_converter_fingerprints_at_generation': summary['converter_inputs_sha256']}


def bundle_member_path(folder, name):
    parts = Path(name).parts
    require(not Path(name).is_absolute() and '..' not in parts, 'PLAYER_BUNDLE_PATH_REFUSED')
    require(not folder.is_symlink() and all(not folder.joinpath(*parts[:i]).is_symlink() for i in range(1, len(parts) + 1)),
            'PLAYER_BUNDLE_SYMLINK_REFUSED')
    path = (folder / name).resolve()
    require(path.is_relative_to(folder.resolve()), 'PLAYER_BUNDLE_PATH_REFUSED')
    return path


def verified_player_bundles(source, data, schemas, root):
    folder = source / 'player-source-bundles'
    manifest_bytes = (folder / 'package-manifest.json').read_bytes()
    package = json.loads(manifest_bytes)
    selected = set(package['files']) | {'package-manifest.json'}
    require(selected == {x.relative_to(folder).as_posix() for x in folder.rglob('*') if x.is_file()}, 'PLAYER_BUNDLE_MEMBERSHIP_MISMATCH')
    receipts = []
    resources = [json.loads(body) for body in schemas.values()]
    registry = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in resources if '$id' in s)
    receipt_validator = Draft202012Validator(json.loads(schemas['player-bundle-receipt.schema.json']), registry=registry)
    for name in sorted(selected):
        path = bundle_member_path(folder, name)
        require(name.endswith('.json') or name == 'source-callback-facts.jsonl.gz', 'PLAYER_BUNDLE_PATH_REFUSED')
        body = manifest_bytes if name == 'package-manifest.json' else path.read_bytes()
        if name != 'package-manifest.json':
            require(digest(body) == package['files'][name], 'PLAYER_BUNDLE_HASH_MISMATCH:' + name)
        if name.endswith('/receipt.json'):
            receipt = json.loads(body)
            receipt_validator.validate(receipt)
            require(receipt['runtime_activation'] is False, 'PLAYER_BUNDLE_ACTIVATION_REFUSED')
            receipts.append(receipt)
        for basename, schema in [('spell.json', 'spell.schema.json'), ('dependencies.json', 'spell-dependencies.schema.json')]:
            if name.endswith('/' + basename):
                Draft202012Validator(json.loads(schemas[schema]), registry=registry).validate(json.loads(body))
        data['player-source-bundles/' + name] = body
    summary = json.loads(data['player-source-bundles/import-summary.json'])
    require(summary['runtime_activation'] is False and summary['external_sources_used'] is False, 'PLAYER_BUNDLE_NOT_SOURCE_ONLY')
    require(len(receipts) == summary['records'] == len({x['registration_key'] for x in receipts}), 'PLAYER_BUNDLE_COUNT_MISMATCH')
    require(dict(Counter(x['status'] for x in receipts)) == summary['status_counts'], 'PLAYER_BUNDLE_STATUS_COUNT_MISMATCH')
    pinned(data, 'player-source-registrars.jsonl.gz', summary['input_proofs']['player-source-registrars.jsonl.gz'])
    pinned(data, 'player-source-authoring-projections.jsonl.gz', summary['input_proofs']['player-source-authoring-projections.jsonl.gz'])
    monster_proof = json.loads(data['monster-verification-proof.json'])
    require(summary['input_proofs']['all-registered-spells.json'] == monster_proof['input_sha256']['all-registered-spells.json'], 'PLAYER_BUNDLE_MONSTER_CAPTURE_MISMATCH')
    callback_payload = gzip.decompress(data['player-source-bundles/source-callback-facts.jsonl.gz'])
    require(digest(callback_payload) == summary['source_callback_payload_sha256'], 'PLAYER_CALLBACK_PAYLOAD_MISMATCH')
    callback_rows = [json.loads(line) for line in callback_payload.splitlines()]
    require({x['registration_key'] for x in callback_rows} == {x['registration_key'] for x in receipts}, 'PLAYER_CALLBACK_POPULATION_MISMATCH')
    return {'status': 'candidate_population_imported_with_explicit_blockers', 'records': summary['records'], 'status_counts': summary['status_counts'],
            'package_manifest_sha256': digest(manifest_bytes), 'converter_fingerprints_at_generation': summary['converter_proofs'], 'execution_complete': False}


def artifact_binding(name):
    if name in ARTIFACTS:
        return ARTIFACTS[name]
    if name == 'source-formula-population.json':
        return 'source_formula_selection_population', ['source-formula-evidence.schema.json#/$defs/population']
    for artifact, (proof, schema, _) in SUPPLEMENTS.items():
        if name in (artifact, proof):
            return 'source_evidence_' + name.replace('.', '_').replace('-', '_'), [schema] if name == artifact else []
    require(name.startswith('player-source-bundles/'), 'UNEXPECTED_IMPORT_ARTIFACT:' + name)
    basename = Path(name).name
    refs = {'receipt.json': ['player-bundle-receipt.schema.json'], 'spell.json': ['spell.schema.json'],
            'dependencies.json': ['spell-dependencies.schema.json']}.get(basename, [])
    return 'player_bundle_' + basename.replace('.', '_').replace('-', '_'), refs


def verified_supplements(source, data, schemas):
    counts = {}
    resources = [json.loads(body) for body in schemas.values()]
    registry = Registry().with_resources((s['$id'], Resource.from_contents(s)) for s in resources if '$id' in s)
    for name, (proof_name, schema, format_) in SUPPLEMENTS.items():
        if not (source / proof_name).exists():
            counts[name] = {'status': 'pending'}
            continue
        proof_bytes = (source / proof_name).read_bytes()
        proof = json.loads(proof_bytes)
        body = (source / name).read_bytes()
        require(digest(body) == proof['gzip_sha256'] and digest(schemas[schema]) == proof['schema_sha256'], 'SUPPLEMENT_PIN_MISMATCH:' + name)
        require(proof['runtime_activation'] is False, 'SUPPLEMENT_ACTIVATION_REFUSED')
        revisions = proof['source_revisions']
        donor_revisions = {s['revision'] for s in json.loads(data['monster-import-summary.json'])['sources']}
        require(set(revisions.values() if isinstance(revisions, dict) else revisions) == donor_revisions,
                'SUPPLEMENT_DONOR_PIN_MISMATCH:' + name)
        payload = gzip.decompress(body)
        require(digest(payload) == proof['payload_sha256'], 'SUPPLEMENT_PAYLOAD_MISMATCH:' + name)
        value = json.loads(payload) if format_ == 'json' else [json.loads(line) for line in payload.splitlines()]
        validator = Draft202012Validator(json.loads(schemas[schema]), registry=registry)
        records = value['records'] if format_ == 'json' else value
        if format_ == 'json':
            validator.validate(value)
        for record in records:
            if format_ == 'jsonl':
                validator.validate(record)
            if name == 'source-formula-evidence.jsonl.gz':
                require(all(p['revision'] in proof['source_revisions'] for p in record['source_proofs']), 'FORMULA_SOURCE_PIN_MISMATCH')
            else:
                identity = record.get('source_identity', record)
                snapshot = identity.get('source', identity.get('snapshot'))
                revision = identity.get('revision', identity.get('source_revision'))
                require(revision == proof['source_revisions'][snapshot], 'SUPPLEMENT_SOURCE_PIN_MISMATCH')
            require(record['runtime_activation'] is False, 'SUPPLEMENT_ACTIVATION_REFUSED')
        require(len(records) == proof['record_count'], 'SUPPLEMENT_COUNT_MISMATCH:' + name)
        if name == 'player-source-guards.jsonl.gz':
            require(sum(len(x['events']) for x in records) == proof['events'] and sum(x['guard_count'] for x in records) == proof['guards'], 'GUARD_FACT_COUNT_MISMATCH')
            pinned(data, 'player-source-registrars.jsonl.gz', proof['input_registrars_sha256'])
            pinned(data, 'player-source-bundles/source-callback-facts.jsonl.gz', proof['input_callback_facts_sha256'])
        if name == 'source-formula-evidence.jsonl.gz':
            cohort_name = 'source-formula-population.json'
            cohort_bytes = (source / cohort_name).read_bytes()
            require(digest(cohort_bytes) == proof['selection_population_sha256'], 'FORMULA_COHORT_HASH_MISMATCH')
            cohort = json.loads(cohort_bytes)
            Draft202012Validator({'$ref': json.loads(schemas[schema])['$id'] + '#/$defs/population'}, registry=registry).validate(cohort)
            identities = {x['registration_key']: x for x in cohort['records']}
            require(len(identities) == len(records) == cohort['record_count'] and set(identities) == {x['registration_key'] for x in records}, 'FORMULA_COHORT_CONSERVATION_MISMATCH')
            require(dict(Counter(x['status'] for x in records)) == proof['status_counts'], 'FORMULA_STATUS_COUNT_MISMATCH')
            for record in records:
                identity = identities[record['registration_key']]
                require(record['source_proofs'][0]['revision'] == identity['source_revision'] and record['source_proofs'][0]['sha256'] == identity['source_sha256'], 'FORMULA_COHORT_SOURCE_IDENTITY_MISMATCH')
            pinned(data, 'player-source-bundles/source-callback-facts.jsonl.gz', proof['input_callback_facts_sha256'])
            data[cohort_name] = cohort_bytes
        data.update({name: body, proof_name: proof_bytes})
        counts[name] = {'status': 'verified_source_only', 'records': len(records), 'proof_sha256': digest(proof_bytes), 'gzip_sha256': proof['gzip_sha256'], 'payload_sha256': proof['payload_sha256'],
                        'schema_sha256': proof['schema_sha256'], 'source_revisions': proof['source_revisions']}
        for key in ('status_counts', 'category_counts', 'guards', 'events', 'unique_source_files', 'selection_population_sha256'):
            if key in proof:
                counts[name][key] = proof[key]
    return counts


def manifest_for(data, schemas, uris, counts, pins, player_full):
    schema_refs = [{'uri': uris[name], 'path': 'schemas/' + name, 'sha256': digest(body)} for name, body in sorted(schemas.items())]
    entries = [{'path': name, 'role': artifact_binding(name)[0], 'sha256': digest(body), 'bytes': len(body),
                'schemaRefs': [uris[s.split('#', 1)[0]] + ('#' + s.split('#', 1)[1] if '#' in s else '') for s in artifact_binding(name)[1]], 'sourcePath': SOURCE + '/' + name} for name, body in sorted(data.items())]
    return {'schema': 'OTERYN_SOURCE_SPELL_REFERENCE_IMPORT/v1', 'admission_status': 'source_only_not_active',
            'runtime_activation': False, 'native_identity_allocation': False, 'execution_complete': False,
            'schemaRefs': schema_refs, 'artifacts': entries, 'counts': counts, 'source_pins': pins,
            'components': {'monster_source_package': 'verified_imported', 'source_profile_facts': 'verified_imported',
                           'source_mechanics': 'verified_imported', 'player_registrars_and_projections': 'verified_imported', 'player_full_candidate_bundles': player_full},
            'limits': ['Source structural/declarative preservation does not prove execution parity or external gameplay agreement.',
                       'Unresolved source semantics and explicit capture errors remain preserved; no callback defaults invented.',
                       'No active content manifest, canonical catalog, native sources, node wiring or runtime registry changed.']}


def write_import(data, schemas, manifest, destination, root=ROOT):
    destination = destination.resolve()
    allowed = (root / 'imports/spells/r28').resolve()
    require(destination == allowed or destination.is_relative_to(allowed), 'IMPORT_DESTINATION_OUTSIDE_R28')
    require(not destination.exists(), 'IMPORT_SET_ALREADY_EXISTS')
    require(manifest['admission_status'] == 'source_only_not_active' and manifest['runtime_activation'] is False, 'ACTIVE_IMPORT_REFUSED')
    require(set(manifest) == {'schema', 'admission_status', 'runtime_activation', 'native_identity_allocation', 'execution_complete', 'schemaRefs',
                              'artifacts', 'counts', 'source_pins', 'components', 'limits'}, 'IMPORT_MANIFEST_SHAPE_INVALID')
    require(manifest['native_identity_allocation'] is False and manifest['execution_complete'] is False, 'IMPORT_AUTHORITY_CLAIM_REFUSED')
    require({x['path'] for x in manifest['artifacts']} == set(data) and len(manifest['artifacts']) == len(data), 'IMPORT_MANIFEST_MEMBERSHIP_MISMATCH')
    for name in list(data) + ['schemas/' + name for name in schemas]:
        require(not Path(name).is_absolute() and '..' not in Path(name).parts, 'IMPORT_PATH_ESCAPE')
    require({x['path'] for x in manifest['schemaRefs']} == {'schemas/' + n for n in schemas}, 'IMPORT_SCHEMA_MEMBERSHIP_MISMATCH')
    schema_uris = {x['uri'] for x in manifest['schemaRefs']}
    for schema in manifest['schemaRefs']:
        require(set(schema) == {'uri', 'path', 'sha256'} and digest(schemas[schema['path'][8:]]) == schema['sha256'], 'PREWRITE_SCHEMA_MISMATCH')
        require(schema['uri'] == json.loads(schemas[schema['path'][8:]]).get('$id', schema['path']), 'IMPORT_SCHEMA_URI_MISMATCH')
    for entry in manifest['artifacts']:
        require(set(entry) == {'path', 'role', 'sha256', 'bytes', 'schemaRefs', 'sourcePath'} and entry['role'] == artifact_binding(entry['path'])[0]
                and {ref.split('#', 1)[0] for ref in entry['schemaRefs']} <= schema_uris and entry['sourcePath'] == SOURCE + '/' + entry['path'], 'IMPORT_ARTIFACT_BINDING_INVALID')
        require(entry['path'] in data and digest(data[entry['path']]) == entry['sha256'] and len(data[entry['path']]) == entry['bytes'], 'PREWRITE_ARTIFACT_MISMATCH')
    destination.mkdir(parents=True, exist_ok=False)
    try:
        outputs = dict(data, **{'schemas/' + name: body for name, body in schemas.items()})
        outputs['import-manifest.json'] = encoded(manifest)
        for name, body in outputs.items():
            path = destination / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        (destination / 'SHA256SUMS').write_text(''.join(f'{digest(body)}  {name}\n' for name, body in sorted(outputs.items())))
    except BaseException:
        shutil.rmtree(destination)
        raise
    return {'artifacts': len(data), 'schema_snapshots': len(schemas), 'manifest_sha256': digest(outputs['import-manifest.json']), 'runtime_activation': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / SOURCE)
    parser.add_argument('--out', type=Path, default=ROOT / 'imports/spells/r28')
    parser.add_argument('--allow-pending-player', action='store_true')
    args = parser.parse_args()
    data, schemas, uris, counts, pins = verified_inputs(args.source.resolve(strict=True), ROOT)
    if (args.source / 'player-source-bundles/package-manifest.json').exists():
        player_full = verified_player_bundles(args.source, data, schemas, ROOT)
    else:
        require(args.allow_pending_player, 'PLAYER_FULL_BUNDLES_PENDING')
        player_full = {'status': 'pending', 'reason': 'Player bundle producer still working; no full candidate bundle claim'}
    counts['source_evidence_additions'] = verified_supplements(args.source, data, schemas)
    manifest = manifest_for(data, schemas, uris, counts, pins, player_full)
    print(json.dumps(write_import(data, schemas, manifest, args.out), indent=2))


if __name__ == '__main__':
    main()
