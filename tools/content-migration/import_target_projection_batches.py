"""Archive reviewed target models with explicit source differences and no activation."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from import_source_program_batches import canonical, jsonl, unique_index, verify_input_proofs
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
FALSE_FLAGS = ('runtime_activation', 'native_execution_qualified', 'native_identity_allocation', 'canonical_selection_changed')
CONFIGS = {
    52: {'source': 'docs/reference/spells/r52-source-closure', 'proof': 'player-control-mapping-receipt.json',
         'proof_sha': '1a6be21d7d6a746c90b007040320cdc417342f3c439a1c39b5ad44500c955187'},
    53: {'source': 'docs/reference/spells/r53-source-closure', 'proof': 'projection-qualification.json',
         'packet_sha': '95cbd3042c2ab8b57f715741b081b39f0feee4988ca09504bddef4c9de9353b4',
         'proof_sha': 'e998c59181b4209cc683659e6eb89c26b68dd0fb8c415ad88f80567703eefb7a'},
    54: {'source': 'docs/reference/spells/r54-source-closure', 'proof': 'projected-monster-slot-candidates-receipt.json',
         'artifact': 'projected-monster-slot-candidates.json.gz',
         'schema': 'tools/content-schema/monster-authoring/project-monster-slot-candidates.schema.json',
         'packet_sha': '08e7f3852bf54e1b9b0084a907d0bd949f6b7a92c785eb5902fd509cdd422187',
         'proof_sha': '567ec9b2a8ae19dfbb887bf9e9594471b3c04e83f6257c1b93dbf831298d98e4'},
}


def inactive(value):
    require(all(value[key] is False for key in FALSE_FLAGS), 'TARGET_PROJECTION_ACTIVATION_REFUSED')


def folder(reg):
    return reg.split('/', 1)[0] + '/' + digest(reg.encode())[:16]


def bundle(packet, path, validators):
    spell, deps, catalog = (json.loads(packet[path + '/' + name]) for name in ['spell.json', 'dependencies.json', 'catalog.json'])
    validators['schemas/spell.schema.json'].validate(spell)
    validators['schemas/spell-dependencies.schema.json'].validate(deps)
    require(catalog == {'definitions': []}, 'TARGET_EXTERNAL_CATALOG_REFUSED')
    definitions = {(family, row['identity']['key'], row['identity']['revision']) for plural, family in [('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')] for row in deps[plural]}
    require(len(definitions) == sum(len(rows) for rows in deps.values()), 'TARGET_DEPENDENCY_DUPLICATE')
    references = []
    def visit(value):
        if isinstance(value, dict):
            if set(value) == {'family', 'key', 'revision'}:
                references.append((value['family'], value['key'], value['revision']))
            for child in value.values(): visit(child)
        elif isinstance(value, list):
            for child in value: visit(child)
    visit(spell); visit(deps)
    require(set(references) == definitions, 'TARGET_DEPENDENCY_CLOSURE_MISMATCH')
    return spell, deps


def verify_source_bundle(packet, path, reg, base, captures, validators):
    require(reg in captures, 'TARGET_REGISTRATION_UNKNOWN')
    old = base / 'player-source-bundles' / folder(reg)
    header = (old / 'source-header.json').read_bytes()
    require(packet[path + '/source-header.json'] == header, 'TARGET_SOURCE_HEADER_CHANGED')
    require(json.loads((old / 'receipt.json').read_bytes())['status'] == 'BLOCKED', 'TARGET_BASE_NOT_BLOCKED')
    receipt = json.loads(packet[path + '/receipt.json'])
    validators['schemas/player-bundle-receipt.schema.json'].validate(receipt)
    require(receipt['registration_key'] == reg and receipt['source_header'] == json.loads(header)['spell']
            and receipt['source_sha256'] == captures[reg]['source_sha256'] and receipt['source_revision'] == captures[reg]['source_revision']
            and receipt['runtime_activation'] is False and receipt['native_execution_qualified'] is False
            and receipt['status'] == 'CANDIDATE_SCHEMA_VALID' and not receipt['blockers'], 'TARGET_SOURCE_RECEIPT_MISMATCH')
    spell, deps = bundle(packet, path, validators)
    require(receipt['candidate_key'] == spell['spell']['identity']['key']
            and receipt['dependencies'] == {key: len(values) for key, values in deps.items()}, 'TARGET_BUNDLE_RECEIPT_MISMATCH')
    return spell, deps


def read_reviewed_files(source, config, proof_body, proof):
    if 'artifact' in config:
        body = bundle_member_path(source, config['artifact']).read_bytes()
        require(digest(body) == config['packet_sha'], 'TARGET_PACKET_REVIEW_PIN_MISMATCH')
        return {config['artifact']: body, config['proof']: proof_body}
    membership = proof['artifacts'] if config is CONFIGS[52] else json.loads(bundle_member_path(source, 'package-manifest.json').read_bytes())['files']
    packet = {name: bundle_member_path(source, name).read_bytes() for name in membership}
    for name, body in packet.items():
        require(digest(body) == membership[name], 'TARGET_MEMBER_PIN_MISMATCH:' + name)
        require(name.endswith('.json') or name == 'README.md', 'TARGET_ORIGINAL_ASSET_REFUSED')
    packet[config['proof']] = proof_body
    if config is CONFIGS[53]:
        body = bundle_member_path(source, 'package-manifest.json').read_bytes()
        require(digest(body) == config['packet_sha'], 'TARGET_PACKET_REVIEW_PIN_MISMATCH')
        packet['package-manifest.json'] = body
    return packet


def verify_player_projection(root, revision, packet, proof, base, validators):
    captures = unique_index(jsonl(base, 'player-source-bundles/source-callback-facts.jsonl.gz'), lambda row: row['registration_key'], 'BASE_CAPTURE_DUPLICATE')
    if revision == 53:
        summary = json.loads(packet['import-summary.json'])
        require(summary['records'] == proof['record_count'] == proof['standard_bundle_count'] == 6
                and summary['status_counts'] == {'CANDIDATE_SCHEMA_VALID': 6}
                and proof['canonical_normalization_used'] is True and proof['source_numeric_equivalence'] is False
                and proof['input_provider_equivalence'] is False and proof['executable_spell_count'] == 0, 'CANONICAL_PROJECTION_SCOPE_MISMATCH')
        reader = proof['reader_proof']
        require(digest(bundle_member_path(root, reader['path']).read_bytes()) == reader['sha256'], 'CANONICAL_READER_PROOF_MISMATCH')
        for name, expected in proof['consumer_proof']['files'].items():
            require(digest(bundle_member_path(root, 'apps/game-server/src/spell/' + name).read_bytes()) == expected, 'CANONICAL_CONSUMER_CACHE_PIN_MISMATCH')
        policy = proof['policy_proof']; body = bundle_member_path(root, policy['path']).read_bytes()
        require(digest(body) == policy['sha256'] and policy['exact_row'] in body.decode()
                and policy['additional_exact_row'] in body.decode() and policy['decision'] == 'S5' and policy['additional_decision'] == 'S16', 'CANONICAL_POLICY_PROOF_MISMATCH')
        library_body = (root / 'imports/spells/r50/source-programs/source-monk-formula-library.json.gz').read_bytes()
        require(digest(library_body) == proof['source_library_gzip_sha256']
                and digest((base / 'player-source-bundles/source-callback-facts.jsonl.gz').read_bytes()) == proof['source_capture_sha256'], 'CANONICAL_SOURCE_INPUT_PIN_MISMATCH')
        library = json.loads(gzip.decompress(library_body))
        source = unique_index(library['formula_definitions'], lambda row: row['registration_key'], 'CANONICAL_LIBRARY_DUPLICATE')
        rows = unique_index(proof['records'], lambda row: row['registration_key'], 'CANONICAL_PROOF_DUPLICATE')
        require(set(rows) == set(source), 'CANONICAL_PROJECTION_POPULATION_MISMATCH')
        for reg, row in rows.items():
            spell, deps = verify_source_bundle(packet, folder(reg), reg, base, captures, validators)
            require(row['source_capture_fact_sha256'] == digest(canonical(captures[reg])) and row['source_formula_library_key'] == source[reg]['key']
                    and spell['spell']['identity'] == {'key': row['candidate_key'], 'revision': 'canonical-player-r53'}
                    and {k: len(v) for k, v in deps.items()} == {'abilities': 1, 'effects': 1, 'formulas': 1}, 'CANONICAL_SOURCE_OR_MODEL_MISMATCH')
        return {'player_candidates': 6, 'native_template_profiles': 0, 'proposed_native_source_bindings': 0, 'source_equivalent_full_spells': 0}, sorted(rows)
    proposal = json.loads(packet['player-control-mapping-proposal.json'])
    require(proof['records'] == proposal['records'] == 21 and proof['reader_shape_candidates'] == 3
            and proof['native_template_profiles'] == 3 and proof['proposed_native_template_source_bindings'] == 4
            and proof['source_equivalent_full_spell_candidates'] == 0, 'CONTROL_PROJECTION_COUNTS_MISMATCH')
    rows = unique_index(proposal['rows'], lambda row: row['registration_key'], 'CONTROL_PROPOSAL_DUPLICATE')
    programs = unique_index(jsonl(root, 'imports/spells/r49/source-programs/source-cast-programs.jsonl.gz'), lambda row: row['registration_key'], 'CONTROL_PROGRAM_DUPLICATE')
    require(set(rows) == set(programs) and digest((root / 'imports/spells/r49/source-programs/source-cast-programs.jsonl.gz').read_bytes()) == proposal['input_programs_sha256'], 'CONTROL_SOURCE_PROGRAM_MISMATCH')
    for reg, row in rows.items():
        require(row['source_sha256'] == captures[reg]['source_sha256'] and row['source_revision'] == captures[reg]['source_revision']
                and row['original_r28_receipt_status'] == 'BLOCKED' and row['full_source_equivalent'] is False, 'CONTROL_SOURCE_ROW_MISMATCH')
    candidates = [name.rsplit('/', 1)[0] for name in packet if name.startswith('player-control-candidates/') and name.endswith('/receipt.json')]
    require(len(candidates) == 3, 'CONTROL_FULL_BUNDLE_COUNT_MISMATCH')
    for path in candidates:
        reg = json.loads(packet[path + '/receipt.json'])['registration_key']
        verify_source_bundle(packet, path, reg, base, captures, validators)
        inactive(json.loads(packet[path + '/projection-receipt.json']))
    profiles_path = 'tools/content-schema/spell-authoring/samples/native-spell-profiles.json'
    profiles_body = (root / profiles_path).read_bytes()
    profiles = json.loads(profiles_body)['profiles']
    bindings = [name.rsplit('/', 1)[0] for name in packet if name.startswith('existing-native-canonical-bindings/') and name.endswith('/projection-receipt.json')]
    require(len(bindings) == 3, 'CONTROL_NATIVE_PROFILE_COUNT_MISMATCH')
    total = 0
    for path in bindings:
        binding = json.loads(packet[path + '/projection-receipt.json']); inactive(binding)
        require(binding['current_source_receipts_promoted'] is False and binding['source_full_mechanics_1_to_1_complete'] is False
                and binding['existing_native_profile_file_sha256'] == digest(profiles_body), 'CONTROL_NATIVE_PROMOTION_REFUSED')
        profile = next(row for row in profiles if row['spell']['identity'] == binding['target_identity'])
        spell, deps = bundle(packet, path, validators)
        require(spell == {'spell': profile['spell']} and deps == profile['dependencies']
                and digest(canonical(profile)) == binding['existing_native_profile_sha256'], 'CONTROL_NATIVE_PROFILE_CHANGED')
        for row in binding['registrations']:
            reg = row['registration_key']; old = base / 'player-source-bundles' / folder(reg)
            require(row['source_header'] == json.loads((old / 'source-header.json').read_bytes()) and row['source_sha256'] == captures[reg]['source_sha256'], 'CONTROL_NATIVE_SOURCE_BINDING_MISMATCH')
            total += 1
    require(total == 4, 'CONTROL_NATIVE_BINDING_COUNT_MISMATCH')
    return {'player_candidates': 3, 'native_template_profiles': 3, 'proposed_native_source_bindings': 4, 'source_equivalent_full_spells': 0}, sorted(rows)


def verify_monster_projection(root, packet, proof, validators):
    artifact = CONFIGS[54]['artifact']; payload = gzip.decompress(packet[artifact]); document = json.loads(payload)
    require(digest(payload) == proof['payload_sha256'], 'MONSTER_PROJECTION_PAYLOAD_PIN_MISMATCH')
    source = json.loads(gzip.decompress((root / 'imports/spells/r51/source-programs/source-monster-slot-semantics.json.gz').read_bytes()))
    originals = unique_index(source['slots'], lambda row: canonical(row['slot_identity']), 'MONSTER_SOURCE_SLOT_DUPLICATE')
    slots = unique_index(document['slots'], lambda row: canonical(row['slot_identity']), 'MONSTER_TARGET_SLOT_DUPLICATE')
    require(set(slots) == set(originals) and len(slots) == proof['records'] == 175, 'MONSTER_PROJECTION_SLOT_POPULATION_MISMATCH')
    counts = Counter(); definitions = Counter()
    for key, row in slots.items():
        require(all(row[field] == originals[key][field] for field in ['source', 'monster', 'original_slot_sha256', 'source_parameters'])
                and row['runtime_activation'] is False and row['native_execution_qualified'] is False
                and row['source_type_equivalence'] is False and row['source_numeric_equivalence'] is False, 'MONSTER_PROJECTION_SOURCE_OR_SCOPE_MISMATCH')
        counts[row['status']] += 1
        require(row['full_slot_projection_complete'] == (row['status'] == 'SOURCE_SCHEMA_VALID'), 'MONSTER_PARTIAL_COUNTED_FULL')
        if row['full_slot_projection_complete']:
            schedule = row['target_schedule']
            abilities = {canonical({'family': 'Ability', **a['identity']}) for part in row['target_fragments'] for a in part['definitions']['abilities']}
            require(schedule is not None and canonical(schedule['ability']) in abilities
                    and schedule['interval_ms'] == row['source_parameters']['interval']
                    and schedule['chance_percent'] == row['source_parameters']['chance'], 'MONSTER_FULL_SLOT_SCHEDULE_MISMATCH')
        for fragment in row['target_fragments']:
            require(fragment['runtime_activation'] is False, 'MONSTER_FRAGMENT_ACTIVATION_REFUSED')
            deps = fragment['definitions']
            for plural, kind in [('abilities', 'Ability'), ('effects', 'Effect'), ('formulas', 'Formula')]:
                for definition in deps[plural]:
                    validators['schemas/monster.schema.json'].evolve(schema={'$ref': 'urn:oteryn:monster-authoring:candidate:1#/$defs/' + kind.lower()}).validate(definition)
                definitions[plural] += len(deps[plural])
    require(counts == {'SOURCE_SCHEMA_VALID': 10, 'PARTIAL_TARGET_DEFINITIONS': 62, 'BLOCKED': 103}
            and definitions == {'abilities': 130, 'effects': 141, 'formulas': 64}, 'MONSTER_PROJECTION_COUNTS_MISMATCH')
    return {'player_candidates': 0, 'full_monster_slot_candidates': 10, 'partial_monster_slots': 62,
            'blocked_monster_slots': 103, 'target_definition_slots': 72, **dict(definitions)}, []


def prepare_import(root, revision, source=None):
    require(revision in CONFIGS, 'UNSUPPORTED_TARGET_PROJECTION_FAMILY')
    config = CONFIGS[revision]
    require('proof_sha' in config and (revision == 52 or 'packet_sha' in config), 'TARGET_PROJECTION_REVIEW_PENDING')
    source = root / config['source'] if source is None else source
    require(source.resolve().is_relative_to(root.resolve()), 'SOURCE_OUTSIDE_REPOSITORY')
    base, base_body, baseline, schemas, validators = read_base(root, BASE_SHA)
    proof_body = bundle_member_path(source, config['proof']).read_bytes()
    require(digest(proof_body) == config['proof_sha'], 'TARGET_PROOF_REVIEW_PIN_MISMATCH')
    proof = json.loads(proof_body); inactive(proof)
    packet = read_reviewed_files(source, config, proof_body, proof)
    if revision == 54:
        verify_input_proofs(root, proof['input_proofs'])
        for path, expected in proof['normalization_proofs'].items():
            require(digest(bundle_member_path(root, path).read_bytes()) == expected, 'MONSTER_NORMALIZATION_PIN_MISMATCH')
        resources = [(ref['uri'], Resource.from_contents(json.loads(schemas[ref['path']]))) for ref in baseline['schemaRefs']]
        for path, expected in proof['schema_proofs'].items():
            body = bundle_member_path(root, path).read_bytes()
            require(digest(body) == expected, 'MONSTER_TARGET_SCHEMA_PIN_MISMATCH')
            require(Path(path).name != 'monster.schema.json' or body == schemas['schemas/monster.schema.json'], 'MONSTER_TARGET_SCHEMA_CHANGED')
            local = 'schemas/' + Path(path).name; schemas[local] = body
            definition = json.loads(body); resources.append((definition['$id'], Resource.from_contents(definition)))
        registry = Registry().with_resources(resources)
        target_schema = json.loads(schemas['schemas/' + Path(config['schema']).name])
        Draft202012Validator(target_schema, registry=registry).validate(json.loads(gzip.decompress(packet[config['artifact']])))
        counts, keys = verify_monster_projection(root, packet, proof, validators)
    else:
        counts, keys = verify_player_projection(root, revision, packet, proof, base, validators)
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    def schema_refs(name):
        path = {'spell.json': 'spell.schema.json', 'dependencies.json': 'spell-dependencies.schema.json', 'receipt.json': 'player-bundle-receipt.schema.json'}.get(Path(name).name)
        if revision == 54 and name.endswith(config['artifact']):
            path = Path(config['schema']).name
        return [json.loads(schemas['schemas/' + path])['$id']] if path else []
    data = {**schemas, 'base-r28/import-manifest.json': base_body, **{'target-projections/' + name: body for name, body in packet.items()}}
    def origin(name):
        if name.startswith('target-projections/'): return source_path + '/' + name.split('/', 1)[1]
        if name.startswith('base-r28/'): return 'imports/spells/r28/import-manifest.json'
        if name not in {ref['path'] for ref in baseline['schemaRefs']}: return 'tools/content-schema/monster-authoring/' + Path(name).name
        return 'imports/spells/r28/' + name
    manifest = {'schema': 'OTERYN_TARGET_PROJECTION_IMPORT/v1', 'revision': revision, 'admission_status': 'source_only_not_active',
                **{flag: False for flag in FALSE_FLAGS}, 'input_provider_equivalence': False, 'source_numeric_equivalence': False,
                'full_source_mechanics_1_to_1_complete': False, 'executable_spell_count': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'source_keys': keys, 'counts': counts, 'qualification_proof_sha256': config['proof_sha'],
                'source_metadata_path': 'target-projections/' + config['proof'],
                'schemaRefs': baseline['schemaRefs'] + [{'uri': json.loads(body)['$id'], 'path': name, 'sha256': digest(body)} for name, body in schemas.items() if name not in {ref['path'] for ref in baseline['schemaRefs']}],
                'artifacts': [{'path': name, 'sourcePath': origin(name), 'sha256': digest(body), 'bytes': len(body), 'schemaRefs': schema_refs(name), 'role': 'target_projection' if name.startswith('target-projections/') else 'reference_schema' if name.startswith('schemas/') else 'base_manifest'} for name, body in sorted(data.items())],
                'limits': ['Only reviewed target data are archived; engine execution, provider equivalence and activation remain unqualified.',
                           'Native bindings and partial monster definitions are counted separately from complete target models.',
                           'Source originals and historic blocked receipts remain immutable; accepted normalization is explicit.']}
    return data, manifest


def write_import(root, revision, destination, data, manifest):
    require(revision in CONFIGS and manifest['revision'] == revision, 'WRONG_TARGET_PROJECTION_FAMILY')
    config = CONFIGS[revision]; inactive(manifest)
    require(manifest['executable_spell_count'] == 0 and manifest['input_provider_equivalence'] is False
            and manifest['source_numeric_equivalence'] is False, 'TARGET_EXECUTION_OR_EQUIVALENCE_REFUSED')
    require(digest(data['target-projections/' + config['proof']]) == manifest['qualification_proof_sha256'] == config['proof_sha'], 'TARGET_OUTPUT_PROOF_PIN_MISMATCH')
    proof = json.loads(data['target-projections/' + config['proof']])
    if revision == 54:
        require(digest(data['target-projections/' + config['artifact']]) == config['packet_sha'], 'TARGET_OUTPUT_PACKET_PIN_MISMATCH')
        expected_counts = {'player_candidates': 0, 'full_monster_slot_candidates': 10, 'partial_monster_slots': 62, 'blocked_monster_slots': 103, 'target_definition_slots': 72, 'abilities': 130, 'effects': 141, 'formulas': 64}
    else:
        if revision == 53:
            packet_manifest = data['target-projections/package-manifest.json']
            require(digest(packet_manifest) == config['packet_sha'], 'TARGET_OUTPUT_PACKET_PIN_MISMATCH')
            members = json.loads(packet_manifest)['files']
        else:
            members = proof['artifacts']
        require(all(digest(data['target-projections/' + name]) == expected for name, expected in members.items()), 'TARGET_OUTPUT_MEMBER_PIN_MISMATCH')
        expected_counts = {'player_candidates': 3 if revision == 52 else 6, 'native_template_profiles': 3 if revision == 52 else 0, 'proposed_native_source_bindings': 4 if revision == 52 else 0, 'source_equivalent_full_spells': 0}
    require(manifest['counts'] == expected_counts, 'TARGET_OUTPUT_COUNTS_MISMATCH')
    return write_source_only_set(root, destination, 'imports/spells/r' + str(revision), data, manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--revision', type=int, choices=sorted(CONFIGS), required=True)
    parser.add_argument('--source', type=Path); parser.add_argument('--prepare-only', action='store_true'); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.revision, args.source)
    print(json.dumps({'artifacts': len(data), 'counts': manifest['counts']} if args.prepare_only else write_import(ROOT, args.revision, ROOT / 'imports/spells' / ('r' + str(args.revision)), data, manifest), sort_keys=True))
