"""Preserve pinned declarative monster facts independently of typed mapper coverage.
No source Lua text is redistributed. Source-side facts do not activate gameplay.
"""
import argparse
from collections import Counter
import gzip
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[2]
LOADER = REPOSITORY / 'docs/reference/spells/r22-audit/monster-import-current/partial_monster_loader.py'
sys.path.insert(0, str(HERE))
spec = importlib.util.spec_from_file_location('source_profile_partial', LOADER)
partial = importlib.util.module_from_spec(spec)
spec.loader.exec_module(partial)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def fact(value):
    """Tagged values distinguish literal strings from engine symbols without coercion."""
    if value is None:
        return {'kind': 'nil'}
    if isinstance(value, bool):
        return {'kind': 'boolean', 'value': value}
    if isinstance(value, (int, float)):
        if not math.isfinite(value):
            raise ValueError('non-finite source number')
        return {'kind': 'number', 'value': value}
    if isinstance(value, str):
        return {'kind': 'symbol' if value.startswith('@') else 'string', 'value': value[1:] if value.startswith('@') else value}
    if isinstance(value, list):
        return {'kind': 'array', 'items': [fact(v) for v in value]}
    if isinstance(value, dict):
        return {'kind': 'object', 'fields': {str(k): fact(v) for k, v in sorted(value.items(), key=lambda kv: str(kv[0]))}}
    if callable(value):
        return {'kind': 'function_reference', 'value': 'source_callback_not_executed'}
    raise ValueError('unsupported source value type: ' + type(value).__name__)


def lookup(root, pointer):
    if not pointer.startswith('/'):
        raise ValueError('destination must be an absolute JSON pointer')
    for token in pointer[1:].split('/'):
        token = token.replace('~1', '/').replace('~0', '~')
        root = root[int(token)] if isinstance(root, list) else root[token]
    return root


def field_coverage(field, entries, bundle):
    rows = [r for r in entries if r.get('source_index') == 0 and
            (r.get('source_field') == field or r.get('source_field', '').startswith((field + '.', field + '[')))]
    destinations = []
    for row in rows:
        pointer = row.get('destination')
        if pointer:
            try:
                lookup(bundle, pointer)
                present = True
            except (KeyError, IndexError, TypeError, ValueError):
                present = False
            destinations.append({'pointer': pointer, 'present': present})
    return {'field': field, 'source_fact_preserved': True,
            'mapper_statuses': sorted({r['status'] for r in rows}),
            'destinations': sorted(destinations, key=lambda d: (d['pointer'], d['present'])),
            'typed_mapper_claim_present': any(r.get('status') == 'mapped' for r in rows)}


def canonical(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=True).encode() + b'\n'


def generate(inputs, archive, out):
    from jsonschema import Draft202012Validator
    schema_path = HERE / 'source-profile-facts.schema.json'
    validator = Draft202012Validator(json.loads(schema_path.read_text()))
    profiles = json.loads((archive / 'monster-profiles.json').read_text())
    out.mkdir(parents=True, exist_ok=True)
    facts_path = out / 'source-profile-facts.jsonl.gz'
    counts = Counter()
    fields = Counter()
    unmapped = Counter()
    callback_counts = Counter()
    summary_profiles = []
    payload_hash = hashlib.sha256()
    seen = set()
    with facts_path.open('wb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0) as compressed:
        for profile in sorted(profiles, key=lambda p: p['candidate_id']):
            key = profile['candidate_id']
            if key in seen:
                raise ValueError('duplicate candidate: ' + key)
            seen.add(key)
            provenance = profile['provenance']
            if provenance['revision'] != partial.REVISIONS[profile['source']]:
                raise ValueError('profile source revision mismatch: ' + key)
            path = inputs / profile['source'] / provenance['path']
            data = path.read_bytes()
            if digest(data) != provenance['sha256']:
                raise ValueError('source hash mismatch: ' + key)
            late = []
            name, monster, callbacks = partial.load(path, late)
            if not isinstance(monster, dict):
                raise ValueError('expected registered monster object: ' + key)
            recovery = partial.RECOVERIES.get(str(path), {})
            declared = sorted(set(re.findall(r'^monster\.(\w+)\s*=', data.decode(), re.M)))
            omitted = recovery.get('omitted_fields', [])
            raw_omissions = {r['field']: r['raw_source_value'] for r in omitted if 'raw_source_value' in r}
            preserved = dict(monster)
            preserved.update(raw_omissions)
            missing = sorted(set(declared) - set(preserved))
            target = archive / profile['bundle_path']
            manifest = json.loads((target / 'manifest.json').read_text())
            bundle = {'monster': json.loads((target / 'monster.json').read_text())}
            entries = manifest['entries']
            coverage = [field_coverage(field, entries, bundle) for field in sorted(preserved)]
            absent_claims = [r['field'] for r in coverage if not r['typed_mapper_claim_present']]
            callback_names = sorted(callbacks)
            dependencies = []
            for dependency in recovery.get('source_literal_dependencies', []):
                item = dict(dependency)
                dependency_path = Path(item['path'])
                if dependency_path.is_absolute():
                    item['path'] = dependency_path.relative_to(inputs / profile['source']).as_posix()
                dependencies.append(item)
            record = {'schema': 'OTERYN_SOURCE_MONSTER_PROFILE_FACTS/v1', 'candidate_id': key,
                      'provenance': provenance, 'runtime_activation': False,
                      'facts': fact(preserved), 'declared_fields': declared,
                      'unrecovered_declared_fields': missing, 'callback_names': callback_names,
                      'callbacks_executed': False,
                      'declarative_load_completed': not bool(recovery or late),
                      'encounter_setup_executed': False,
                      'source_literal_dependencies': dependencies,
                      'omitted_fields': omitted, 'field_coverage': coverage}
            validator.validate(record)
            line = canonical(record)
            payload_hash.update(line)
            compressed.write(line)
            counts['profiles'] += 1
            counts['preserved_top_level_fields'] += len(preserved)
            counts['unrecovered_declared_fields'] += len(missing)
            counts['partial_or_late_load_profiles'] += bool(recovery or late)
            counts['profiles_with_callbacks'] += bool(callback_names)
            fields.update(preserved.keys())
            unmapped.update(absent_claims)
            callback_counts.update(callback_names)
            summary_profiles.append({'candidate_id': key, 'preserved_fields': len(preserved),
                                     'without_typed_mapper_claim': absent_claims,
                                     'unrecovered_declared_fields': missing})
    summary = {'schema': 'OTERYN_MONSTER_PROFILE_FIELD_COVERAGE/v1',
               'scope': 'source-side declarative preservation; mapper claims are not semantic equivalence or runtime qualification',
               'runtime_activation': False, 'counts': dict(counts),
               'declarative_field_data_complete': counts['unrecovered_declared_fields'] == 0,
               'source_profile_inventory_sha256': digest((archive / 'monster-profiles.json').read_bytes()),
               'facts_file': facts_path.name, 'facts_gzip_sha256': digest(facts_path.read_bytes()),
               'facts_payload_sha256': payload_hash.hexdigest(), 'facts_schema': 'tools/content-schema/monster-authoring/source-profile-facts.schema.json',
               'facts_schema_sha256': digest(schema_path.read_bytes()),
               'field_population': dict(sorted(fields.items())),
               'without_typed_mapper_claim_population': dict(sorted(unmapped.items())),
               'callback_name_population': dict(sorted(callback_counts.items())),
               'limitations': ['Callback function bodies and encounter setup are not executed or translated by this data sidecar.',
                              'Tagged source symbols are foreign source facts, not admitted Game runtime identifiers.',
                              'A mapped manifest row does not establish complete nested-field or engine-behavior equivalence.'],
               'profiles': summary_profiles}
    (out / 'monster-profile-field-coverage.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-inputs', type=Path, required=True)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    result = generate(args.source_inputs, args.archive, args.out)
    print(json.dumps(result['counts'], sort_keys=True))
