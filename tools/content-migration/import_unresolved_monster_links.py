#!/usr/bin/env python3
"""Import exact unresolved source-slot evidence; never admit executable Abilities."""
import argparse
from collections import Counter
import gzip
import json
from pathlib import Path
import tarfile

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, encoded, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def prepare_import(root, source=None):
    source = source or root / 'docs/reference/spells/r37-source-closure'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name, data_name = 'unresolved-monster-spell-links-proof.json', 'unresolved-monster-spell-links.jsonl.gz'
    proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    compressed = bundle_member_path(source, data_name).read_bytes(); payload = gzip.decompress(compressed)
    schema_path = 'tools/content-schema/monster-authoring/unresolved-monster-spell-links.schema.json'
    schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(compressed) == proof['gzip_sha256'] and digest(payload) == proof['payload_sha256']
            and digest(schema_body) == proof['schema_sha256'], 'MONSTER_LINK_PACKET_HASH_MISMATCH')
    pins = {r['source']: r['revision'] for r in baseline['source_pins']['monster_donors']}
    require(proof['source_revisions'] == pins and proof['runtime_activation'] is False and proof['native_ability_admission'] is False,
            'MONSTER_LINK_SOURCE_SCOPE_MISMATCH')
    rows = [json.loads(line) for line in payload.splitlines()]
    validator = Draft202012Validator(schema)
    for row in rows: validator.validate(row)
    archive_receipt = (base / 'monster-source-package-manifest.json').read_bytes()
    archive = base / 'monster-source-package.tar.gz'
    require(digest(archive_receipt) == proof['source_receipt_sha256'] and digest(archive.read_bytes()) == proof['source_archive_sha256'],
            'MONSTER_LINK_ARCHIVE_PIN_MISMATCH')
    slots = []; total = 0
    with tarfile.open(archive) as tar:
        manifest_body = tar.extractfile('monster-package-manifest.json').read()
        require(digest(manifest_body) == proof['internal_manifest_sha256'], 'MONSTER_LINK_INTERNAL_MANIFEST_MISMATCH')
        internal = json.loads(manifest_body); entries = {e['path']: e for e in internal['entries']}
        require({s['source']: s['revision'] for s in internal['sources']} == pins, 'MONSTER_LINK_ARCHIVE_SOURCE_MISMATCH')
        require({p['member'] for p in proof['source_slot_members']} == {'canary-monster-spell-slots.json', 'crystal-monster-spell-slots.json'},
                'MONSTER_LINK_SLOT_MEMBER_COHORT_MISMATCH')
        for ref in proof['source_slot_members']:
            body = tar.extractfile(ref['member']).read(); entry = entries[ref['member']]
            require(digest(body) == ref['sha256'] == entry['sha256'] and len(body) == ref['bytes'] == entry['bytes'], 'MONSTER_LINK_SLOT_MEMBER_HASH_MISMATCH')
            all_rows = json.loads(body)
            require(len(all_rows) == ref['records'], 'MONSTER_LINK_SLOT_MEMBER_COUNT_MISMATCH')
            total += len(all_rows); slots.extend(s for s in all_rows if s['conversion_status'] == 'unresolved_semantics')
            del all_rows, body
    inventory_body = (base / 'source-mechanics-inventory.json.gz').read_bytes()
    custom_body = (base / 'source-custom-mechanics.json.gz').read_bytes()
    require(digest(inventory_body) == proof['inventory_sha256'] and digest(custom_body) == proof['custom_descriptors_sha256'], 'MONSTER_LINK_FACT_INPUT_HASH_MISMATCH')
    inventory = json.loads(gzip.decompress(inventory_body))['files']; custom = json.loads(gzip.decompress(custom_body))['records']
    def source_identity(donor, record): return (donor, *(record[k] for k in ('revision', 'path', 'sha256', 'git_blob')))
    facts = {source_identity(r['source'], r): (i, r) for i, r in enumerate(inventory)}
    descriptors = {source_identity(r['source_identity']['source'], r['source_identity']): (i, r) for i, r in enumerate(custom)}
    require(len(facts) == len(inventory) and len(descriptors) == len(custom), 'MONSTER_LINK_DUPLICATE_FACT_IDENTITY')
    actual = {(r['slot_identity']['candidate_id'], r['slot_identity']['group'], r['slot_identity']['source_slot_index']): r for r in rows}
    require(len(actual) == len(rows) == len(slots) == proof['record_count'] and total == proof['all_source_slots'], 'MONSTER_LINK_POPULATION_MISMATCH')
    seen = set()
    for slot in slots:
        key = (slot['candidate_id'], slot['group'], slot['source_slot_index']); row = actual.get(key)
        require(key not in seen and row is not None, 'MONSTER_LINK_SLOT_IDENTITY_MISMATCH'); seen.add(key)
        registered = slot['registered_source']; donor = slot['source']; params = slot['source_parameters']
        require(row['original_slot_sha256'] == digest(canonical(slot)) and all(row[k] == slot[k] for k in ('source', 'monster', 'source_parameters', 'conversion_status', 'resolution', 'script_tier', 'external_verification'))
                and row['monster_source'] == {k: slot['monster_source'][k] for k in ('revision', 'path', 'sha256', 'git_blob')}
                and row['reasons'] == [r.get('resolution', '') for r in slot['conversion_manifest_rows'] if r.get('status') == 'unresolved_semantics']
                and row['runtime_activation'] is False and row['native_ability_admission'] is False, 'MONSTER_LINK_ORIGINAL_SLOT_CHANGED')
        require(row['registered_source'] == (None if registered is None else {k: registered[k] for k in ('revision', 'path', 'sha256', 'git_blob')})
                and row['inline_type_present'] == (registered is None and 'type' in params)
                and row['inline_raw_type'] == (params.get('type') if registered is None else None), 'MONSTER_LINK_REGISTERED_SOURCE_CHANGED')
        expected_fact = expected_custom = None
        if registered is not None:
            identity = source_identity(donor, registered); require(identity in facts, 'MONSTER_LINK_MISSING_INVENTORY_SOURCE')
            index, fact = facts[identity]
            expected_fact = {'artifact': 'docs/reference/spells/r28-source-closure/source-mechanics-inventory.json.gz', 'record_index': index,
                             'record_sha256': digest(canonical(fact)), 'source_identity': {'source': donor, **{k: registered[k] for k in ('revision', 'path', 'sha256', 'git_blob')}},
                             'call_count': len(fact['calls']), 'scope': 'source_file_lexical_evidence_not_executable_cast'}
            if identity in descriptors:
                index, descriptor = descriptors[identity]
                expected_custom = {'artifact': 'docs/reference/spells/r28-source-closure/source-custom-mechanics.json.gz', 'record_index': index,
                                   'record_sha256': digest(canonical(descriptor)), 'mechanic_category': descriptor['mechanic_category'], 'scope': 'exact_source_file_descriptor_not_native_admission'}
        require(row['registered_fact_link'] == expected_fact and row['custom_descriptor_link'] == expected_custom, 'MONSTER_LINK_FACT_JOIN_MISMATCH')
    registered = sum(r['registered_fact_link'] is not None for r in rows); linked = sum(r['custom_descriptor_link'] is not None for r in rows)
    counts = {'source_records': len(rows), 'source_counts': dict(Counter(r['source'] for r in rows)), 'status_counts': dict(Counter(r['conversion_status'] for r in rows)),
              'registered_fact_links': registered, 'inline_raw_slots': len(rows) - registered, 'custom_descriptor_links': linked, 'full_spell_candidates': 0}
    require(all(counts[k] == proof[k] for k in counts if k != 'source_records' and k != 'full_spell_candidates'), 'MONSTER_LINK_PROOF_COUNT_MISMATCH')
    local_schema = 'schemas/unresolved-monster-spell-links.schema.json'; schemas[local_schema] = schema_body
    source_path = source.resolve().relative_to(root.resolve()).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body, 'evidence/' + data_name: compressed, 'evidence/' + proof_name: proof_body}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_UNRESOLVED_MONSTER_LINK_IMPORT/v1', 'revision': 37, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
                'input_provider_equivalence': False, 'native_ability_admission': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'counts': counts, 'source_metadata_path': 'evidence/' + data_name,
                'schemaRefs': baseline['schemaRefs'] + [{'uri': schema.get('$id', local_schema), 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'source_evidence' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'base_manifest',
                               'schemaRefs': [schema.get('$id', local_schema)] if p == 'evidence/' + data_name else []} for p, b in sorted(data.items())],
                'limits': ['All 175 unresolved source slots retain their original conversion status and parameters.', 'Registered and custom links identify lexical source evidence; no executable Ability admission or new Spell candidate is claimed.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 37 and manifest['full_spell_candidates'] == 0 and manifest['native_ability_admission'] is False, 'MONSTER_LINK_ACTIVATION_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r37', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source)
    print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r37', data, manifest), indent=2))


if __name__ == '__main__': main()
