"""Offline Source associations; never promotes semantic completion or Native admission."""
import argparse
from collections import defaultdict, Counter
import hashlib
import json
from pathlib import Path
import re
import sys


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def storage_mentions(text, mask):
    code = mask(text)
    pattern = re.compile(r'\b(?:getStorageValue|setStorageValue)\s*\(\s*(Storage\.[A-Za-z0-9_.]+(?:\[\d+\])?|\d+)(?=\s*[,\)])')
    return [(m.group(1), m.start(1), m.end(1)) for m in pattern.finditer(code)]


def build(repo_root, assignment, corpus_manifest):
    root = Path(repo_root)
    tool = root / 'tools/content-schema/quest-authoring'
    sys.path.insert(0, str(tool))
    import lua_writers
    samples = tool / 'samples'
    bundle = read_json(samples / 'source_migration/bundle.json')
    crosswalk = read_json(samples / 'binding_packets/source/crosswalk.json')
    records = read_json(assignment)['records']
    ids = [r['source_component_id'] for r in records]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate component ID')
    defs = {}
    input_digests = {}
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        input_digests[str(path.relative_to(root))] = digest(path.read_bytes())
        for record in read_json(path)['records']:
            d = record['definition']
            key = d['identity']['key']
            if key in defs:
                raise ValueError('duplicate canonical quest')
            defs[key] = d
    donor_to_canonical = {d['source_refs']['quest']['key']: key for key, d in defs.items() if d.get('source_refs', {}).get('quest')}
    progress_owners = defaultdict(set)
    for q in bundle['quests']:
        canonical = donor_to_canonical.get(q['identity']['key'])
        if canonical:
            for track in [(q.get('start') or {}).get('progress')] + [m.get('progress') for m in q.get('missions', [])]:
                if track:
                    progress_owners[track].add(canonical)
    targets = defaultdict(list)
    for track in bundle['progress']:
        owners = progress_owners[track['key']]
        if not owners:
            continue
        for transition in track.get('transitions', []):
            for occurrence in transition.get('source_occurrences', []):
                targets[(occurrence['source'], occurrence['revision'], occurrence['target'])].append((track['key'], sorted(owners), occurrence))
    exact_paths = defaultdict(list)
    for q in crosswalk['quests']:
        for binding in q['trigger_bindings']:
            for source in binding['sources']:
                exact_paths[(source['source'], source['path'], source['blob_sha1'])].append((q['quest_key'], binding['source_graph']))
    manifest = read_json(corpus_manifest)
    files = {(f['source'], f['revision'], f['path']): f for f in manifest['files']}
    output = []
    perquest = {key: {'quest_key': key, 'display_name': d['display_name'], 'definition_readiness': d['readiness'], 'original_missing_data': d.get('missing_data', []), 'accepted_component_ids': [], 'candidate_component_ids': [], 'quest_completeness': 'NOT_ASSESSED', 'native_admission': 'NOT_ASSESSED'} for key, d in sorted(defs.items())}
    for record in records:
        p = record['provenance']
        f = files.get((p['source'], p['revision'], p['path']))
        proof = []
        candidates = [k for k in record.get('same_source_directory_owner_candidates', []) if k in defs]
        holds = []
        if not f:
            holds.append('MISSING_PINNED_SOURCE_BYTES')
            raw = None
        else:
            cache = Path(f['cache_path'])
            if not cache.is_absolute():
                cache = Path(corpus_manifest).parent / cache
            raw = cache.read_bytes()
            if digest(raw) != p['sha256'] or len(raw) != p['byte_count'] or f['git_blob_sha1'] != p['git_blob_sha1']:
                raise ValueError('Source hash mismatch: ' + record['source_component_id'])
            if hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != p['git_blob_sha1']:
                raise ValueError('Source Git hash mismatch')
        if raw is not None:
            text = raw.decode('utf-8')
            storage_pristine = lua_writers.builtin_binding_is_pristine(text.splitlines(), 'Storage')
            for target, start, end in storage_mentions(text, lua_writers.mask_code):
                matches = targets.get((p['source'], p['revision'], target), [])
                if not matches:
                    continue
                line = text.count('\n', 0, start) + 1
                evidence = {'line': line, 'line_sha256': digest(text.splitlines()[line - 1].encode()), 'token': target, 'token_sha256': digest(text[start:end].encode()), 'char_start': start, 'char_end': end}
                for track, owners, occurrence in matches:
                    if target.startswith('Storage.') and not storage_pristine:
                        holds.append('STORAGE_ROOT_BINDING_NOT_PRISTINE')
                        candidates.extend(owners)
                        continue
                    proof.append({'basis': 'LEXICAL_STORAGE_ACCESSOR_TARGET_TO_DECLARED_TRACK', 'quest_keys': owners, 'track_key': track, 'accessor_dispatch_binding': 'NOT_PROVEN', 'component_evidence': evidence, 'existing_occurrence': {k: occurrence[k] for k in ['source', 'revision', 'path', 'line', 'target', 'blob_sha256', 'line_sha256']}, 'limit': 'Track association only; does not prove callback completeness or sole Quest ownership.'})
            for quest, graph in exact_paths.get((p['source'], p['path'], p['git_blob_sha1']), []):
                proof.append({'basis': 'EXACT_PINNED_GRAPH_SOURCE_PATH_AND_GIT_BLOB', 'quest_keys': [quest], 'graph_key': graph, 'limit': 'Existing Source association; not full file semantic coverage.'})
        accepted = sorted({q for item in proof for q in item['quest_keys']})
        candidates = sorted(set(candidates) - set(accepted))
        if not proof:
            holds.append('QUEST_ASSOCIATION_CONVERTER_MISSING')
        holds.append('WHOLE_FILE_SEMANTIC_COVERAGE_NOT_ESTABLISHED')
        verdict = 'ACCEPTED_SOURCE_ASSOCIATION_PARTIAL_SEMANTICS' if accepted else 'DIRECTORY_CANDIDATE_ONLY' if candidates else 'UNLINKED_SOURCE_COMPONENT'
        out = {'source_component_id': record['source_component_id'], 'provenance': p, 'accepted_quest_keys': accepted, 'candidate_quest_keys': candidates, 'proofs': proof, 'verdict': verdict, 'holds': sorted(set(holds)), 'role': 'QUEST_FOLDER_COMPONENT_NOT_INDEPENDENT_QUEST', 'native_admission': 'NOT_ASSESSED'}
        output.append(out)
        for q in accepted:
            perquest[q]['accepted_component_ids'].append(record['source_component_id'])
        for q in candidates:
            perquest[q]['candidate_component_ids'].append(record['source_component_id'])
    for rel in ['tools/content-schema/quest-authoring/samples/source_migration/bundle.json', 'tools/content-schema/quest-authoring/samples/binding_packets/source/crosswalk.json']:
        input_digests[rel] = digest((root / rel).read_bytes())
    return {'schema': 'OTERYN_QUEST_COMPONENT_ASSOCIATIONS/v1', 'assignment_sha256': digest(Path(assignment).read_bytes()), 'corpus_manifest_sha256': digest(Path(corpus_manifest).read_bytes()), 'input_sha256': input_digests, 'summary': {'components': len(output), 'quests': len(perquest), 'verdicts': dict(sorted(Counter(o['verdict'] for o in output).items())), 'quests_with_accepted_components': sum(bool(q['accepted_component_ids']) for q in perquest.values()), 'quests_with_candidates': sum(bool(q['candidate_component_ids']) for q in perquest.values())}, 'components': output, 'quests': list(perquest.values()), 'semantic_quest_completeness': 'NOT_ESTABLISHED', 'external_wiki_read': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repo-root', required=True)
    parser.add_argument('--assignment', required=True)
    parser.add_argument('--corpus-manifest', required=True)
    parser.add_argument('--out', required=True)
    args = parser.parse_args()
    packet = build(args.repo_root, args.assignment, args.corpus_manifest)
    Path(args.out).write_text(json.dumps(packet, indent=2, sort_keys=True) + '\n')
    print(json.dumps(packet['summary'], sort_keys=True))
