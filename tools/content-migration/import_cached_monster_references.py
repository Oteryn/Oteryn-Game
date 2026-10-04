#!/usr/bin/env python3
"""Import qualified cached comparisons as supplemental evidence, never source overrides."""
import argparse
from collections import Counter, defaultdict
import gzip
import json
from pathlib import Path
import tarfile

from jsonschema import Draft202012Validator

from import_source_spell_package import ROOT, bundle_member_path, digest, require
from source_spell_import_guards import read_base, write_source_only_set

BASE_SHA = '1c8ed40b00c5457ad408cbfd918147e0e1c197cd5877f7f8e8f18305e9cb7b5f'
PROOF_SHA = '38f94b0f6103acf75a20f2b43822ec5e1012a42d18c03af666810a22c2128ff1'
DATASETS = {'cached-monster-spell-facts.jsonl.gz': 'fact', 'cached-monster-profile-coverage.jsonl.gz': 'profile',
            'cached-monster-slot-comparisons.jsonl.gz': 'slot', 'missing-monster-spell-reference-targets.jsonl.gz': 'missing_target'}


def canonical(value): return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()
def slot_key(slot): return '/'.join([slot['source'], slot['monster_source']['path'], slot['group'], str(slot['source_slot_index'])])


def validate_rows(groups, profiles, slots, facts, comparisons):
    def index(rows, key):
        value = {key(row): row for row in rows}; require(len(value) == len(rows), 'CACHED_REFERENCE_DUPLICATE_IDENTITY'); return value
    source_profiles = index(profiles, lambda r: r['candidate_id']); source_slots = index(slots, slot_key)
    cached_facts = index(facts, lambda r: r['monster_name']); cached_comparisons = index(comparisons, lambda r: r['slot_key'])
    fact_rows = index(groups['fact'], lambda r: r['fact']['monster_name'])
    require(set(fact_rows) == set(cached_facts) == {r['name'] for r in profiles} and all(fact_rows[key]['fact'] == fact for key, fact in cached_facts.items()), 'CACHED_REFERENCE_FACT_COHORT_MISMATCH')
    profile_rows = index(groups['profile'], lambda r: r['candidate_id']); slot_rows = index(groups['slot'], lambda r: tuple(r['slot_identity'].values()))
    require(set(profile_rows) == set(source_profiles) and len(slot_rows) == len(source_slots) == len(cached_comparisons), 'CACHED_REFERENCE_SOURCE_POPULATION_MISMATCH')
    slots_by_profile = defaultdict(list); deficits = defaultdict(set)
    for slot in slots:
        key = slot_key(slot); identity = (slot['candidate_id'], slot['group'], slot['source_slot_index']); row = slot_rows.get(identity)
        comparison = cached_comparisons.get(key); require(row is not None and comparison is not None, 'CACHED_REFERENCE_SLOT_IDENTITY_MISMATCH')
        profile = source_profiles[slot['candidate_id']]; fact = cached_facts[profile['name']]
        expected_identity = {k: slot['monster_source'][k] for k in ('revision', 'path', 'sha256', 'git_blob')}
        require(row['source_identity'] == expected_identity and row['source'] == slot['source'] and row['monster_name'] == slot['monster']
                and row['source_slot_sha256'] == digest(canonical(slot)) and row['source_conversion_status'] == slot['conversion_status']
                and row['cached_comparison'] == comparison and row['all_fields_verified'] is False, 'CACHED_REFERENCE_SOURCE_SLOT_CHANGED')
        require(comparison['source_sha256'] == expected_identity['sha256'] and comparison['file'] == expected_identity['path']
                and comparison['source'] == slot['source'] and comparison['block'] == slot['group']
                and canonical({k: v['source_value'] for k, v in comparison['field_comparisons'].items()}) == canonical(slot['source_parameters'])
                and comparison['external_url'] == fact['url'] and comparison['external_page_sha256'] == fact.get('page_sha256')
                and comparison['external_status'] == fact['status'] and comparison['all_fields_verified'] is False, 'CACHED_REFERENCE_COMPARISON_PROVENANCE_MISMATCH')
        unknown = sorted(k for k, v in comparison['field_comparisons'].items() if v['status'] in {'unknown_external_not_published', 'no_unambiguous_external_match'})
        uncertain = sorted(k for k, v in comparison['field_comparisons'].items() if v['status'] != 'equal')
        require(row['unknown_fields'] == unknown and row['source_uncertainty_fields'] == uncertain, 'CACHED_REFERENCE_UNCERTAINTY_CHANGED')
        slots_by_profile[slot['candidate_id']].append(key); deficits[profile['name']].update(uncertain)
    for profile in profiles:
        row = profile_rows[profile['candidate_id']]; fact = cached_facts[profile['name']]
        require(row['source'] == profile['source'] and row['monster_name'] == profile['name']
                and row['source_identity'] == {k: profile['provenance'][k] for k in ('revision', 'path', 'sha256', 'git_blob')}
                and row['cached_page_url'] == fact['url'] and row['cached_page_sha256'] == fact.get('page_sha256') and row['cached_page_status'] == fact['status']
                and row['slot_keys'] == sorted(slots_by_profile[profile['candidate_id']]), 'CACHED_REFERENCE_PROFILE_CHANGED')
    missing = index(groups['missing_target'], lambda r: r['monster_name']); require(set(missing) == set(deficits), 'CACHED_REFERENCE_MISSING_TARGET_COHORT_MISMATCH')
    for name, row in missing.items():
        fact = cached_facts[name]
        require(row['candidate_ids'] == sorted(p['candidate_id'] for p in profiles if p['name'] == name)
                and row['source_uncertainty_fields'] == sorted(deficits[name]) and row['captured_url'] == fact['url'] and row['page_status'] == fact['status']
                and row['reason'] == ('cached_page_unavailable' if fact['status'] != 'attack_facts_read' else 'engine_parameters_and_unique_slot_identity_not_verified'), 'CACHED_REFERENCE_MISSING_TARGET_CHANGED')
    return {'fact_records': len(facts), 'profile_records': len(profiles), 'slot_records': len(slots), 'missing_target_records': len(missing),
            'cached_page_statuses': dict(Counter(r['status'] for r in facts)), 'profiles_without_attack_defense_slots': sum(not r['slot_keys'] for r in groups['profile']),
            'normalized_attack_observations': sum(len(r['attacks']) for r in facts),
            'field_statuses': dict(Counter(v['status'] for c in comparisons for v in c['field_comparisons'].values())), 'all_field_verified_slots': 0, 'full_spell_candidates': 0}


def prepare_import(root, source=None):
    source = source or root / 'docs/reference/spells/r41-source-closure'
    base, base_body, baseline, schemas, _ = read_base(root, BASE_SHA)
    proof_name = 'cached-monster-spell-reference-proof.json'; proof_body = bundle_member_path(source, proof_name).read_bytes(); proof = json.loads(proof_body)
    require(digest(proof_body) == PROOF_SHA, 'CACHED_REFERENCE_QUALIFIED_PROOF_MISMATCH')
    require(proof['runtime_activation'] is False and proof['source_override'] is False and proof['new_network_requests'] == 0, 'CACHED_REFERENCE_SCOPE_MISMATCH')
    pins = {r['source']: r['revision'] for r in baseline['source_pins']['monster_donors']}; require(proof['source_revisions'] == pins, 'CACHED_REFERENCE_SOURCE_PIN_MISMATCH')
    schema_path = 'tools/content-schema/monster-authoring/cached-monster-spell-references.schema.json'; schema_body = (root / schema_path).read_bytes(); schema = json.loads(schema_body)
    require(digest(schema_body) == proof['schema_sha256'], 'CACHED_REFERENCE_SCHEMA_PIN_MISMATCH'); validator = Draft202012Validator(schema)
    cache = root / 'docs/reference/spells/r22-audit/monster-references-current'; cache_manifest_body = (cache / 'artifact-manifest.json').read_bytes()
    require(digest(cache_manifest_body) == proof['cache_manifest_sha256'], 'CACHED_REFERENCE_CACHE_MANIFEST_MISMATCH'); cache_manifest = json.loads(cache_manifest_body)
    cached = {}
    require({Path(r['path']).name for r in proof['cache_inputs']} == {'tibiopedia-monster-facts.jsonl', 'monster-slot-field-comparisons.json.gz', 'summary.json'}, 'CACHED_REFERENCE_CACHE_INPUT_COHORT_MISMATCH')
    for ref in proof['cache_inputs']:
        name = Path(ref['path']).name; require(ref['path'] == (cache / name).relative_to(root).as_posix(), 'CACHED_REFERENCE_CACHE_INPUT_PATH_MISMATCH')
        body = bundle_member_path(cache, name).read_bytes(); payload = gzip.decompress(body) if name.endswith('.gz') else body
        require(digest(body) == ref['sha256'] == cache_manifest[name]['sha256'] and len(body) == cache_manifest[name]['bytes'] and digest(payload) == ref['payload_sha256'], 'CACHED_REFERENCE_CACHE_INPUT_HASH_MISMATCH'); cached[name] = payload
    archive_receipt = (base / 'monster-source-package-manifest.json').read_bytes(); archive = base / 'monster-source-package.tar.gz'
    require(digest(archive_receipt) == proof['source_receipt_sha256'] and digest(archive.read_bytes()) == proof['source_archive_sha256'], 'CACHED_REFERENCE_SOURCE_ARCHIVE_PIN_MISMATCH')
    profiles = []; slots = []
    with tarfile.open(archive) as tar:
        manifest_body = tar.extractfile('monster-package-manifest.json').read(); require(digest(manifest_body) == proof['internal_manifest_sha256'], 'CACHED_REFERENCE_INTERNAL_MANIFEST_MISMATCH')
        entries = {r['path']: r for r in json.loads(manifest_body)['entries']}
        require({r['member'] for r in proof['source_members']} == {s + '-' + k + '.json' for s in ('canary', 'crystal') for k in ('monster-profiles', 'monster-spell-slots')}, 'CACHED_REFERENCE_SOURCE_MEMBER_COHORT_MISMATCH')
        for ref in proof['source_members']:
            body = tar.extractfile(ref['member']).read(); entry = entries[ref['member']]
            require(digest(body) == ref['sha256'] == entry['sha256'] and len(body) == ref['bytes'] == entry['bytes'], 'CACHED_REFERENCE_SOURCE_MEMBER_HASH_MISMATCH')
            records = json.loads(body); require(len(records) == ref['records'], 'CACHED_REFERENCE_SOURCE_MEMBER_COUNT_MISMATCH')
            (profiles if ref['member'].endswith('monster-profiles.json') else slots).extend(records)
    packet = {proof_name: proof_body}; groups = {}
    require({Path(ref['path']).name for ref in proof['datasets']} == set(DATASETS), 'CACHED_REFERENCE_DATASET_COHORT_MISMATCH')
    for ref in proof['datasets']:
        name = Path(ref['path']).name; require(ref['path'] == (source / name).relative_to(root).as_posix(), 'CACHED_REFERENCE_DATASET_PATH_MISMATCH')
        body = bundle_member_path(source, name).read_bytes(); payload = gzip.decompress(body)
        require(digest(body) == ref['sha256'] and digest(payload) == ref['payload_sha256'], 'CACHED_REFERENCE_DATASET_HASH_MISMATCH')
        records = [json.loads(line) for line in payload.splitlines()]; require(len(records) == ref['records'], 'CACHED_REFERENCE_DATASET_COUNT_MISMATCH')
        for row in records:
            validator.validate(row)
            require(row['record_kind'] == DATASETS[name] and row['runtime_activation'] is False and row.get('source_override', False) is False, 'CACHED_REFERENCE_DATASET_SCOPE_MISMATCH')
        groups[DATASETS[name]] = records; packet[name] = body
    facts = [json.loads(line) for line in cached['tibiopedia-monster-facts.jsonl'].splitlines()]; comparisons = json.loads(cached['monster-slot-field-comparisons.json.gz'])
    require(json.loads(cached['summary.json'])['sources'] == pins, 'CACHED_REFERENCE_CACHE_SOURCE_REVISION_MISMATCH')
    counts = validate_rows(groups, profiles, slots, facts, comparisons)
    for key, proof_key in [('fact_records', 'species_count'), ('profile_records', 'profile_count'), ('slot_records', 'slot_count')]: require(counts[key] == proof[proof_key], 'CACHED_REFERENCE_PROOF_POPULATION_MISMATCH')
    for key in ('cached_page_statuses', 'profiles_without_attack_defense_slots', 'normalized_attack_observations', 'field_statuses', 'all_field_verified_slots'): require(counts[key] == proof[key], 'CACHED_REFERENCE_PROOF_COUNT_MISMATCH')
    local_schema = 'schemas/cached-monster-spell-references.schema.json'; schemas[local_schema] = schema_body; source_path = source.relative_to(root).as_posix()
    data = {**schemas, 'base-r28/import-manifest.json': base_body, **{'evidence/' + n: b for n, b in packet.items()},
            'provenance/cache-artifact-manifest.json': cache_manifest_body, 'provenance/monster-source-package-manifest.json': archive_receipt}
    def origin(path):
        if path.startswith('evidence/'): return source_path + '/' + path.split('/', 1)[1]
        if path == local_schema: return schema_path
        if path == 'provenance/cache-artifact-manifest.json': return cache.relative_to(root).as_posix() + '/artifact-manifest.json'
        if path.startswith('provenance/'): return 'imports/spells/r28/monster-source-package-manifest.json'
        return 'imports/spells/r28/import-manifest.json' if path.startswith('base-r28/') else 'imports/spells/r28/' + path
    manifest = {'schema': 'OTERYN_CACHED_MONSTER_REFERENCE_IMPORT/v1', 'revision': 41, 'admission_status': 'source_only_not_active',
                'runtime_activation': False, 'source_override': False, 'native_identity_allocation': False, 'canonical_selection_changed': False, 'native_execution_qualified': False,
                'input_provider_equivalence': False, 'execution_qualified': False, 'native_admission': False, 'full_spell_candidates': 0,
                'base': {'path': 'imports/spells/r28/import-manifest.json', 'sha256': BASE_SHA, 'snapshot': 'base-r28/import-manifest.json'},
                'source_pins': baseline['source_pins'], 'reference_inputs': proof['cache_inputs'], 'source_archive_sha256': proof['source_archive_sha256'], 'counts': counts,
                'source_metadata_path': 'evidence/' + proof_name, 'schemaRefs': baseline['schemaRefs'] + [{'uri': schema['$id'], 'path': local_schema, 'sha256': digest(schema_body)}],
                'artifacts': [{'path': p, 'sourcePath': origin(p), 'sha256': digest(b), 'bytes': len(b), 'role': 'supplemental_reference_evidence' if p.startswith('evidence/') else 'reference_schema' if p.startswith('schemas/') else 'reference_provenance',
                               'schemaRefs': [schema['$id']] if p.startswith('evidence/') and p.endswith('.jsonl.gz') else []} for p, b in sorted(data.items())],
                'limits': proof['limits'] + ['Only cached parsed observations are imported; no new capture, upstream override or candidate inflation is claimed.']}
    return data, manifest


def write_import(root, destination, data, manifest):
    require(manifest['revision'] == 41 and manifest['source_override'] is False and manifest['full_spell_candidates'] == 0, 'CACHED_REFERENCE_OVERRIDE_CLAIM_REFUSED')
    return write_source_only_set(root, destination, 'imports/spells/r41', data, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__); parser.add_argument('--source', type=Path); args = parser.parse_args()
    data, manifest = prepare_import(ROOT, args.source); print(json.dumps(write_import(ROOT, ROOT / 'imports/spells/r41', data, manifest), indent=2))


if __name__ == '__main__': main()
