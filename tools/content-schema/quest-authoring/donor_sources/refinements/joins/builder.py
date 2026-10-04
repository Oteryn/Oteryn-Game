"""Bounded, offline Source joins; never certifies executable dispatch or Native readiness."""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import sys


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical_hash(value):
    return sha(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode())


def read(path):
    return json.loads(Path(path).read_text())


def provenance(file):
    return {key: file[key] for key in ('source', 'repository', 'revision', 'path', 'git_blob_sha1', 'sha256', 'byte_count')}


class Corpus:
    def __init__(self, manifest):
        self.manifest = Path(manifest)
        self.files = {}
        self.texts = {}
        for file in read(manifest)['files']:
            key = (file['source'], file['revision'], file['path'])
            if key in self.files:
                raise ValueError('duplicate source identity')
            self.files[key] = file

    def text(self, file):
        key = (file['source'], file['revision'], file['path'])
        if key not in self.texts:
            path = Path(file['cache_path'])
            if not path.is_absolute():
                path = self.manifest.parent / path
            raw = path.read_bytes()
            if len(raw) != file['byte_count'] or sha(raw) != file['sha256'] or hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != file['git_blob_sha1']:
                raise ValueError('source byte/Git digest mismatch')
            self.texts[key] = raw.decode('utf-8')
        return self.texts[key]

    def witness(self, file, start, end):
        text = self.text(file)
        line = text.count('\n', 0, start) + 1
        return dict(provenance(file), line=line, line_sha256=sha(text.splitlines()[line - 1].encode()), char_start=start, char_end=end, token_sha256=sha(text[start:end].encode()))


EVENT = re.compile(r'\blocal\s+([A-Za-z_]\w*)\s*=\s*CreatureEvent\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\2\s*\)')
CALLER = re.compile(r':\s*registerEvent\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*\)')
STORAGE = re.compile(r'\b(?:getStorageValue|setStorageValue)\s*\(\s*(Storage\.[A-Za-z0-9_.]+(?:\[\d+\])?|\d+)(?=\s*[,\)])')


def event_registrations(text, mask):
    code = mask(text, literals=False)
    skeleton = mask(text)
    out = []
    for match in EVENT.finditer(code):
        if skeleton[match.start():match.start() + 5] != 'local':
            continue
        receiver, name = match.group(1), match.group(3)
        registrations = list(re.finditer(r'\b' + re.escape(receiver) + r'\s*:\s*register\s*\(\s*\)', skeleton))
        if len(registrations) == 1:
            out.append((name, match.span(), registrations[0].span()))
    return out


def literal_event_calls(text, mask):
    code = mask(text, literals=False)
    skeleton = mask(text)
    return [match for match in CALLER.finditer(code) if not skeleton[match.start()].isspace()]


def build(repo_root, assignment, corpus_manifest):
    repo = Path(repo_root)
    tool = repo / 'tools/content-schema/quest-authoring'
    sys.path.insert(0, str(tool))
    import lua_writers
    corpus = Corpus(corpus_manifest)
    assignments = read(assignment)['records']
    ids = [r['source_component_id'] for r in assignments]
    if len(ids) != len(set(ids)):
        raise ValueError('duplicate component')
    source_slices = []
    quests = {}
    track_targets = defaultdict(list)
    path_anchors = defaultdict(list)
    for path in sorted((repo / 'content/quests/definitions').glob('quests-*.json')):
        for record in read(path)['records']:
            quest = record['definition']
            key = quest['identity']['key']
            if key in quests:
                raise ValueError('duplicate Quest')
            quests[key] = quest
            source_slices.append({name: quest[name] for name in ('identity', 'source_refs', 'source_data') if name in quest})
            for track in quest.get('source_data', {}).get('progress', []):
                for transition in track.get('transitions', []):
                    for occurrence in transition.get('source_occurrences', []):
                        identity = (occurrence['source'], occurrence['revision'], occurrence['path'])
                        file = corpus.files.get(identity)
                        if not file or file['sha256'] != occurrence['blob_sha256']:
                            raise ValueError('Quest progress occurrence does not resolve to pinned source')
                        text = corpus.text(file)
                        line = occurrence['line']
                        lines = text.splitlines()
                        if not 1 <= line <= len(lines) or sha(lines[line - 1].encode()) != occurrence['line_sha256']:
                            raise ValueError('Quest progress occurrence line mismatch')
                        target = occurrence['target']
                        start = sum(len(l) + 1 for l in text.split('\n')[:line - 1])
                        offset = text.find(target, start, start + len(text.split('\n')[line - 1]))
                        if offset < 0:
                            continue  # scoped numeric aliases have a separate Source guard
                        evidence = corpus.witness(file, offset, offset + len(target))
                        item = (key, track['key'], evidence)
                        track_targets[(occurrence['source'], occurrence['revision'], occurrence['path'].split('/')[0], target)].append(item)
                        path_anchors[identity].append(item)
    crosswalk = read(tool / 'samples/binding_packets/source/crosswalk.json')
    graph_projection = []
    graph_files = defaultdict(list)
    for quest in crosswalk['quests']:
        if quest['quest_key'] not in quests:
            raise ValueError('unknown canonical Quest anchor')
        allowed_graphs = {g['identity']['key'] for g in quests[quest['quest_key']].get('source_data', {}).get('interactions', [])}
        for binding in quest['trigger_bindings']:
            if binding['source_graph'] not in allowed_graphs:
                continue
            graph_projection.append({'quest_key': quest['quest_key'], 'source_graph': binding['source_graph'], 'sources': binding['sources']})
            for source in binding['sources']:
                matching = [f for f in corpus.files.values() if f['source'] == source['source'] and f['path'] == source['path'] and f['git_blob_sha1'] == source['blob_sha1']]
                if len(matching) != 1:
                    raise ValueError('graph Source identity ambiguous or absent')
                file = matching[0]
                identity = (file['source'], file['revision'], file['path'])
                graph_files[identity].append((quest['quest_key'], binding['source_graph'], file))
    # Registrations are fenced by donor/pin/datapack, with uniqueness checked across the whole captured pack.
    registrars = defaultdict(list)
    for file in corpus.files.values():
        if file['source'] not in {'canary', 'crystalserver'} or not file['path'].endswith('.lua') or '/scripts/' not in file['path']:
            continue
        pack = file['path'].split('/')[0]
        text = corpus.text(file)
        for name, constructor, registration in event_registrations(text, lua_writers.mask_code):
            registrars[(file['source'], file['revision'], pack, name)].append((file, constructor, registration))
    caller_links = defaultdict(list)
    for identity, anchors in graph_files.items():
        file = anchors[0][2]
        text = corpus.text(file)
        for call in literal_event_calls(text, lua_writers.mask_code):
            name = call.group(2)
            matches = registrars.get((file['source'], file['revision'], file['path'].split('/')[0], name), [])
            if len(matches) != 1:
                continue
            target_file, constructor, registration = matches[0]
            target = (target_file['source'], target_file['revision'], target_file['path'])
            for quest, graph, _ in anchors:
                caller_links[target].append({'basis': 'PINNED_QUEST_GRAPH_REGISTER_EVENT_TO_UNIQUE_PACK_REGISTRAR', 'quest_key': quest, 'source_reference': graph, 'binding_literal': name, 'source_witnesses': [corpus.witness(file, *call.span()), corpus.witness(target_file, *constructor), corpus.witness(target_file, *registration)], 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED'})
    output = []
    for record in assignments:
        source = record['provenance']
        identity = (source['source'], source['revision'], source['path'])
        file = corpus.files.get(identity)
        if not file or provenance(file) != source:
            raise ValueError('component identity mismatch or missing pinned bytes')
        text = corpus.text(file)
        code = lua_writers.mask_code(text)
        proofs = list(caller_links.get(identity, []))
        for quest, track, evidence in path_anchors.get(identity, []):
            proofs.append({'basis': 'EXACT_SOURCE_PATH_AND_SHA_TO_CANONICAL_PROGRESS_OCCURRENCE', 'quest_key': quest, 'source_reference': track, 'binding_literal': '', 'source_witnesses': [evidence], 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED'})
        for quest, graph, _ in graph_files.get(identity, []):
            proofs.append({'basis': 'EXACT_SOURCE_PATH_AND_GIT_SHA_TO_CANONICAL_INTERACTION_GRAPH', 'quest_key': quest, 'source_reference': graph, 'binding_literal': '', 'source_witnesses': [corpus.witness(file, 0, min(len(text), len(text.splitlines()[0])))], 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED'})
        for match in STORAGE.finditer(code):
            target = match.group(1)
            if target.startswith('Storage.') and not lua_writers.builtin_binding_is_pristine(text.splitlines(), 'Storage'):
                continue
            anchors = track_targets.get((source['source'], source['revision'], source['path'].split('/')[0], target), [])
            # Retain one deterministic verified anchor per Quest/track instead of redundant NPC writes.
            distinct = {}
            for quest, track, anchor in anchors:
                distinct.setdefault((quest, track), anchor)
            for (quest, track), anchor in distinct.items():
                proofs.append({'basis': 'PINNED_STORAGE_SYMBOL_CORRELATION_TO_CANONICAL_PROGRESS_TRACK', 'quest_key': quest, 'source_reference': track, 'binding_literal': target, 'source_witnesses': [corpus.witness(file, *match.span(1)), anchor], 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED'})
        proofs = list({canonical_hash(proof): proof for proof in proofs}.values())
        keys = sorted({proof['quest_key'] for proof in proofs})
        output.append({'source_component_id': record['source_component_id'], 'provenance': source, 'quest_keys': keys, 'proofs': proofs, 'closed_mapping_gaps': ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'] if keys else [], 'remaining_holds': ['COMPONENT_SEMANTIC_COVERAGE_NOT_ESTABLISHED', 'RUNTIME_ACTIVATION_NOT_ASSESSED'] + ([] if keys else ['EXACT_QUEST_ASSOCIATION_NOT_ESTABLISHED']), 'native_admission': False})
    perquest = [{'quest_key': key, 'component_ids': sorted(row['source_component_id'] for row in output if key in row['quest_keys']), 'closed_mapping_gaps': ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'], 'quest_completeness': 'NOT_ASSESSED'} for key in sorted({q for row in output for q in row['quest_keys']})]
    return {'schema': 'OTERYN_EXACT_SOURCE_COMPONENT_QUEST_JOINS/v1', 'input_sha256': {'assignment': sha(Path(assignment).read_bytes()), 'corpus_manifest': sha(Path(corpus_manifest).read_bytes()), 'canonical_source_semantic_slice': canonical_hash(source_slices), 'graph_source_provenance_slice': canonical_hash(graph_projection)}, 'records': output, 'quests': perquest, 'summary': {'components': len(output), 'joined_components': sum(bool(r['quest_keys']) for r in output), 'joined_quests': len(perquest), 'proofs_by_basis': dict(sorted(Counter(p['basis'] for row in output for p in row['proofs']).items())), 'missing_source_bytes': 0}, 'native_admission': False, 'quest_semantic_completeness': 'NOT_ESTABLISHED', 'external_wiki_read': False}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--repo-root', required=True)
    p.add_argument('--assignment', required=True)
    p.add_argument('--corpus-manifest', required=True)
    p.add_argument('--out', required=True)
    args = p.parse_args()
    packet = build(args.repo_root, args.assignment, args.corpus_manifest)
    Path(args.out).write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True, indent=2) + '\n')
    print(json.dumps(packet['summary'], sort_keys=True))
