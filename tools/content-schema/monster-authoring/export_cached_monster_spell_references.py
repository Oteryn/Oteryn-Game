"""Bind cached public observations to immutable source variants; never qualify runtime."""
import argparse
import gzip
import hashlib
import json
import tarfile
from collections import Counter, defaultdict
from pathlib import Path

SCHEMA = 'OTERYN_CACHED_MONSTER_SPELL_REFERENCE/v1'
UNKNOWN = {'unknown_external_not_published', 'no_unambiguous_external_match'}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':')).encode()


def unique(records, key):
    out = {}
    for row in records:
        value = key(row)
        if value in out:
            raise ValueError('duplicate source/reference identity: ' + str(value))
        out[value] = row
    return out


def slot_key(slot):
    return '/'.join([slot['source'], slot['monster_source']['path'], slot['group'], str(slot['source_slot_index'])])


def bind(profiles, slots, facts, comparisons):
    profile_index = unique(profiles, lambda p: p['candidate_id'])
    fact_index = unique(facts, lambda f: f['monster_name'])
    comparison_index = unique(comparisons, lambda c: c['slot_key'])
    unique(slots, slot_key)
    if set(comparison_index) != {slot_key(s) for s in slots}:
        raise ValueError('cached comparison population differs from source slots')
    if set(fact_index) != {p['name'] for p in profiles}:
        raise ValueError('cached species population differs from source profiles')
    slot_rows = []; slots_by_profile = defaultdict(list); deficits = defaultdict(set)
    for slot in slots:
        key = slot_key(slot); comparison = comparison_index[key]
        profile = profile_index[slot['candidate_id']]; fact = fact_index[profile['name']]
        if slot['monster'] != profile['name'] or slot['source'] != profile['source'] or any(slot['monster_source'][k] != profile['provenance'][k] for k in ['revision', 'path', 'sha256', 'git_blob']):
            raise ValueError('slot/profile source identity mismatch')
        if comparison['source_sha256'] != slot['monster_source']['sha256'] or comparison['monster_name'] != slot['monster'] or comparison['source'] != slot['source'] or comparison['file'] != slot['monster_source']['path'] or comparison['block'] != slot['group']:
            raise ValueError('cached comparison source identity mismatch')
        if canonical({k: v['source_value'] for k, v in comparison['field_comparisons'].items()}) != canonical(slot['source_parameters']):
            raise ValueError('cached comparison source parameters mismatch')
        if comparison['external_url'] != fact['url'] or comparison['external_page_sha256'] != fact.get('page_sha256') or comparison['external_status'] != fact['status']:
            raise ValueError('cached comparison/page identity mismatch')
        if comparison['all_fields_verified']:
            raise ValueError('cached comparison claims unqualified full verification')
        unknown = sorted(k for k, v in comparison['field_comparisons'].items() if v['status'] in UNKNOWN)
        uncertain = sorted(k for k, v in comparison['field_comparisons'].items() if v['status'] not in {'equal'})
        deficits[profile['name']].update(uncertain)
        row = {'schema': SCHEMA, 'record_kind': 'slot', 'slot_identity': {'candidate_id': slot['candidate_id'], 'group': slot['group'], 'source_slot_index': slot['source_slot_index']},
               'source_identity': {k: slot['monster_source'][k] for k in ['revision', 'path', 'sha256', 'git_blob']},
               'source': slot['source'], 'monster_name': slot['monster'], 'source_slot_sha256': sha(canonical(slot)),
               'source_conversion_status': slot['conversion_status'], 'cached_comparison': comparison,
               'unknown_fields': unknown, 'source_uncertainty_fields': uncertain,
               'comparison_scope': 'cached_observation_not_unique_spell_identity_or_engine_constant',
               'source_override': False, 'runtime_activation': False, 'all_fields_verified': False}
        slot_rows.append(row); slots_by_profile[slot['candidate_id']].append(key)
    profile_rows = []
    for profile in profiles:
        fact = fact_index[profile['name']]
        profile_rows.append({'schema': SCHEMA, 'record_kind': 'profile', 'candidate_id': profile['candidate_id'], 'source': profile['source'], 'monster_name': profile['name'],
                             'source_identity': {k: profile['provenance'][k] for k in ['revision', 'path', 'sha256', 'git_blob']},
                             'cached_page_url': fact['url'], 'cached_page_sha256': fact.get('page_sha256'), 'cached_page_status': fact['status'],
                             'slot_keys': sorted(slots_by_profile[profile['candidate_id']]), 'source_override': False, 'runtime_activation': False})
    fact_rows = [{'schema': SCHEMA, 'record_kind': 'fact', 'fact': f,
                  'fact_scope': 'monster_page_attack_observations_not_unique_source_slot', 'source_override': False, 'runtime_activation': False} for f in facts]
    targets = []
    for name, fact in sorted(fact_index.items()):
        if name not in deficits:
            continue  # No source attack/defense slots exist for this species.
        targets.append({'monster_name': name, 'captured_url': fact['url'], 'page_status': fact['status'],
                        'candidate_ids': sorted(p['candidate_id'] for p in profiles if p['name'] == name),
                        'source_uncertainty_fields': sorted(deficits[name]),
                        'required_uncaptured_sources': ['tibiawiki.com.br', 'tibia.fandom.com'],
                        'reason': 'cached_page_unavailable' if fact['status'] != 'attack_facts_read' else 'engine_parameters_and_unique_slot_identity_not_verified',
                        'runtime_activation': False})
    return fact_rows, profile_rows, slot_rows, targets


def export(r28, cache, out):
    import jsonschema
    if out.exists():
        raise ValueError('new output required; frozen artifacts cannot be overwritten')
    receipt_path = r28 / 'monster-source-package-manifest.json'; receipt = json.loads(receipt_path.read_text())
    archive = r28 / receipt['archive']['path']
    if sha(archive.read_bytes()) != receipt['archive']['sha256']:
        raise ValueError('immutable r28 archive SHA mismatch')
    profiles = []; slots = []; members = []
    with tarfile.open(archive) as tar:
        manifest_bytes = tar.extractfile('monster-package-manifest.json').read(); manifest = json.loads(manifest_bytes)
        entries = {e['path']: e for e in manifest['entries']}
        for source in ['canary', 'crystal']:
            for suffix, dest in [('monster-profiles.json', profiles), ('monster-spell-slots.json', slots)]:
                name = source + '-' + suffix; data = tar.extractfile(name).read()
                if sha(data) != entries[name]['sha256'] or len(data) != entries[name]['bytes']:
                    raise ValueError('archive member SHA/bytes mismatch')
                rows = json.loads(data); dest.extend(rows)
                members.append({'member': name, 'sha256': sha(data), 'bytes': len(data), 'records': len(rows)})
    artifact_manifest = json.loads((cache / 'artifact-manifest.json').read_text()); cache_inputs = []
    def cached(name, compressed=False):
        data = (cache / name).read_bytes()
        if sha(data) != artifact_manifest[name]['sha256'] or len(data) != artifact_manifest[name]['bytes']:
            raise ValueError('cached artifact receipt mismatch: ' + name)
        payload = gzip.decompress(data) if compressed else data
        cache_inputs.append({'path': (cache / name).as_posix(), 'sha256': sha(data), 'payload_sha256': sha(payload)})
        return payload
    facts = [json.loads(line) for line in cached('tibiopedia-monster-facts.jsonl').splitlines()]
    comparisons = json.loads(cached('monster-slot-field-comparisons.json.gz', True)); summary = json.loads(cached('summary.json'))
    pins = {s['source']: s['revision'] for s in manifest['sources']}
    if summary['sources'] != pins or len(profiles) != 5237 or len(slots) != 20742:
        raise ValueError('pinned source population/revisions mismatch')
    groups = bind(profiles, slots, facts, comparisons)
    schema_path = Path(__file__).with_name('cached-monster-spell-references.schema.json')
    validator = jsonschema.Draft202012Validator(json.loads(schema_path.read_text()))
    datasets = []
    for name, rows in zip(['cached-monster-spell-facts', 'cached-monster-profile-coverage', 'cached-monster-slot-comparisons', 'missing-monster-spell-reference-targets'], groups):
        if name.startswith('missing'):
            rows = [{'schema': SCHEMA, 'record_kind': 'missing_target', **row} for row in rows]
        for row in rows:
            validator.validate(row)
        payload = b''.join(canonical(row) + b'\n' for row in rows)
        out.mkdir(parents=True, exist_ok=True); path = out / (name + '.jsonl.gz'); path.write_bytes(gzip.compress(payload, mtime=0))
        datasets.append({'path': path.as_posix(), 'sha256': sha(path.read_bytes()), 'payload_sha256': sha(payload), 'records': len(rows)})
    proof = {'schema': SCHEMA + '/proof', 'source_archive_sha256': sha(archive.read_bytes()), 'source_receipt_sha256': sha(receipt_path.read_bytes()),
             'internal_manifest_sha256': sha(manifest_bytes), 'source_members': members, 'source_revisions': pins, 'cache_inputs': cache_inputs,
             'cache_manifest_sha256': sha((cache / 'artifact-manifest.json').read_bytes()), 'datasets': datasets,
             'schema_path': schema_path.relative_to(Path(__file__).resolve().parents[3]).as_posix(), 'schema_sha256': sha(schema_path.read_bytes()), 'producer_sha256': sha(Path(__file__).read_bytes()),
             'profile_count': len(profiles), 'slot_count': len(slots), 'species_count': len(facts),
             'cached_page_statuses': dict(Counter(f['status'] for f in facts)),
             'profiles_without_attack_defense_slots': sum(not r['slot_keys'] for r in groups[1]),
             'normalized_attack_observations': sum(len(f['attacks']) for f in facts),
             'field_statuses': dict(Counter(v['status'] for c in comparisons for v in c['field_comparisons'].values())),
             'other_source_capture_limitations': summary['blocked_other_sources'],
             'cached_read_method': 'normal_http', 'this_export_read_method': 'local_verified_cached_facts_only',
             'source_override': False, 'runtime_activation': False, 'new_network_requests': 0, 'all_field_verified_slots': 0,
             'limits': summary['limits']}
    (out / 'cached-monster-spell-reference-proof.json').write_text(json.dumps(proof, indent=2) + '\n')
    return proof


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--r28', type=Path, required=True); parser.add_argument('--cache', type=Path, required=True); parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args(); print(json.dumps(export(args.r28, args.cache, args.out)))
