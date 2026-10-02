"""Build the complete wiki-title rollout backlog; never activates runtime content."""
import argparse
import copy
from collections import Counter
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

INPUTS = ('samples/catalogue/catalogue.json', 'samples/source_migration/bundle.json',
          'samples/wiki-source-all373/source-specs-373.json')
OUTPUT = 'samples/rollout/quest-rollout.json'
FLAGS = ('imported', 'partial', 'needs_source', 'needs_runtime')


def read(path):
    return json.loads(path.read_text())


def build(catalogue, bundle, definitions, source_specs):
    """Join by exact source keys, including explicit family bindings only."""
    def indexed(rows, field):
        result = {row[field]: row for row in rows}
        if len(result) != len(rows):
            raise ValueError('duplicate rollout source identity: ' + field)
        return result
    specs = indexed(source_specs['entries'], 'wiki_title')
    coverage = indexed(source_specs['coverage'], 'wiki_title')
    titles = {row['wiki_title'] for row in catalogue['quests']}
    if set(specs) != titles or set(coverage) != titles:
        raise ValueError('SOURCE specifications must cover exact catalogue titles')
    if (source_specs['titles'] != len(titles) or source_specs['no_runtime_promotion'] is not True
            or source_specs['source_definition_complete'] is not False or source_specs['runtime_readiness'] != 'UNKNOWN'):
        raise ValueError('SOURCE specification count or authority mismatch')
    native = {}
    for definition in definitions:
        ref = definition['source_refs']['quest']['key']
        if ref in native:
            raise ValueError('duplicate canonical source binding: ' + ref)
        native[ref] = definition
    gaps = {row['quest']: row['gaps'] for row in bundle['quest_gaps']}
    source_keys = {row['identity']['key'] for row in bundle['quests']}
    records, seen = [], set()
    for row in catalogue['quests']:
        title = row['wiki_title']
        spec, proof = specs[title], coverage[title]
        if (proof['proof_level'] not in ('RECORDED_PRIOR_BODY_FACTS', 'PINNED_STRUCTURED_FIELDS_ONLY')
                or proof['full_walkthrough_complete'] is not False or proof['raw_body_rechecked'] is not False
                or spec['definition_complete'] is not False or spec['runtime_readiness'] != 'UNKNOWN'
                or spec['source_field_listing_complete'] is not False):
            raise ValueError('SOURCE proof cannot claim completeness or runtime: ' + title)
        if title in seen:
            raise ValueError('duplicate wiki title: ' + title)
        seen.add(title)
        keys = sorted({c['identity']['key'] for c in row['authored_candidates']})
        family = row.get('family_representation')
        scope = 'direct' if keys else 'family' if family else 'unbound'
        if family and not keys:
            keys = [family['target_key']]
        if any(key not in source_keys for key in keys):
            raise ValueError('rollout references absent source quest: ' + title)
        canonical = [native[key] for key in keys if key in native]
        missing = []
        if not keys:
            missing.append({'code': 'source_binding_missing'})
        if scope == 'family':
            missing.append({'code': 'partial_mission_family'})
        for key in keys or [None]:
            if key not in native:
                missing.append({'code': 'canonical_definition_missing', 'source_key': key})
        missing.extend({'code': 'source_specification_gap', 'detail': copy.deepcopy(hold)}
                       for hold in spec['unresolved'])
        for key in keys:
            for gap in gaps.get(key, []):
                missing.append({'code': 'source_gap', 'source_key': key,
                                'detail': gap})
        for definition in canonical:
            for issue in definition['missing_data']:
                missing.append({'code': 'definition_gap',
                                'canonical_key': definition['identity']['key'],
                                'detail': issue})
        flags = []
        if keys:
            flags.append('imported')
        if missing:
            flags.append('partial')
        if not keys:
            flags.append('needs_source')
        flags.append('needs_runtime')
        ready = bool(keys) and len(canonical) == len(keys) and all(d['readiness'] == 'definition_ready' for d in canonical)
        records.append({
            'wiki_title': title, 'wiki_source': row['source'],
            'source_specification': {'proof_level': proof['proof_level'],
                'source_refs_count': len(spec['source_refs']), 'source_refs': copy.deepcopy(spec['source_refs']),
                'unresolved': copy.deepcopy(spec['unresolved']),
                'source_field_listing_complete': False, 'full_walkthrough_complete': False,
                'raw_body_rechecked': False, 'runtime_readiness': 'UNKNOWN'},
            'binding_scope': scope, 'source_quest_keys': keys,
            'canonical_quest_refs': [d['identity'] for d in canonical],
            'definition_fields_ready': ready,
            'flags': flags, 'known_gaps': missing,
            'source_page_count': len(row.get('fresh_sources', [])),
            'fidelity_policy': 'pragmatic_oteryn',
            'approximation_applied': False,
            'runtime_enabled': False,
            'smoke_verification': {'start': 'NOT_RUN', 'progress': 'NOT_RUN',
                                   'finish': 'NOT_RUN', 'reward': 'NOT_RUN',
                                   'repeat_denial': 'NOT_RUN', 'persistence': 'NOT_RUN'},
        })
    return {'schema': 'OTERYN_QUEST_ROLLOUT_BACKLOG/v1',
            'scope': 'Authoring and rollout planning metadata; no runtime activation',
            'source_preference': ['canary', 'crystalserver', 'wiki'],
            'approximation_policy': 'Allow explicit Oteryn simplifications; preserve donor facts and record each actual override separately',
            'summary': {'titles': len(records),
                        'bindings': dict(Counter(r['binding_scope'] for r in records)),
                        'flags': dict(Counter(f for r in records for f in r['flags'])),
                        'runtime_enabled': 0, 'smoke_verified': 0},
            'records': records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    root = here.parents[2]
    index_path = root / 'content/quests/definitions/index.json'
    index = read(index_path)
    definitions = [r['definition'] for name in index['shards']
                   for r in read(root / name)['records']]
    payload = build(read(here / INPUTS[0]), read(here / INPUTS[1]), definitions, read(here / INPUTS[2]))
    paths = [here / name for name in INPUTS] + [index_path] + [root / name for name in index['shards']]
    payload['input_provenance'] = [
        {'path': str(path.relative_to(root)), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
        for path in paths]
    Draft202012Validator(read(here / 'quest_rollout.schema.json')).validate(payload)
    content = json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n'
    output = here / OUTPUT
    if args.check:
        if not output.is_file() or output.read_text() != content:
            raise SystemExit('Rollout backlog missing or stale; regenerate in AUTHORING')
    else:
        output.parent.mkdir(exist_ok=True)
        output.write_text(content)
    print(json.dumps(payload['summary'], sort_keys=True))


if __name__ == '__main__':
    main()
