"""Offline named-reference traversal; Source association never becomes runtime admission."""
import argparse
from collections import defaultdict, deque, Counter
import importlib.util
import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET


CREATE_MONSTER = re.compile(r'\bGame\s*\.\s*createMonster\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*[,)]')
MONSTER_TYPE = re.compile(r'\blocal\s+([A-Za-z_]\w*)\s*=\s*Game\s*\.\s*createMonsterType\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\2\s*\)')
EVENTS = re.compile(r'\b([A-Za-z_]\w*)\s*\.\s*events\s*=\s*\{([^{}]*)\}', re.S)
STRING = re.compile(r'\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*')


def module(root):
    path = Path(root) / 'tools/content-schema/quest-authoring/donor_sources/refinements/joins/builder.py'
    spec = importlib.util.spec_from_file_location('quest_previous_exact_joins', path)
    previous = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(previous)
    return previous


def literal_calls(text, mask):
    code, skeleton = mask(text, literals=False), mask(text)
    return [m for m in CREATE_MONSTER.finditer(code) if skeleton[m.start():m.start() + 4] == 'Game']


def top_level_at(skeleton, offset):
    depth, awaiting_loop_do = 0, 0
    for match in re.finditer(r'\b(function|if|for|while|repeat|do|end|until)\b', skeleton[:offset]):
        token = match.group()
        if token in {'for', 'while'}:
            depth += 1
            awaiting_loop_do += 1
        elif token == 'do':
            if awaiting_loop_do:
                awaiting_loop_do -= 1
            else:
                depth += 1
        elif token in {'function', 'if', 'repeat'}:
            depth += 1
        else:
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


def single_local_binding(skeleton, name, initializer_pattern):
    prefix = r'(?<![\w.:])' + re.escape(name)
    declarations = list(re.finditer(r'\blocal\s+' + re.escape(name) + r'\s*=', skeleton))
    assignments = list(re.finditer(prefix + r'\s*=', skeleton))
    competing = re.search(prefix + r'\s*,|\blocal\s+[^=\n]*,\s*' + re.escape(name) + r'\s*[,=]', skeleton)
    parameters = re.search(r'\bfunction\b[^\n(]*\([^)]*\b' + re.escape(name) + r'\b', skeleton)
    return (len(declarations) == len(assignments) == 1 and top_level_at(skeleton, declarations[0].start()) and not competing and not parameters
            and re.match(initializer_pattern, skeleton[assignments[0].end():]))


def monster_registration(text, mask):
    code, skeleton = mask(text, literals=False), mask(text)
    results = []
    for factory in MONSTER_TYPE.finditer(code):
        if skeleton[factory.start():factory.start() + 5] != 'local':
            continue
        receiver = factory.group(1)
        if not single_local_binding(skeleton, receiver, r'\s*Game\s*\.\s*createMonsterType\s*\('):
            continue
        registered = list(re.finditer(r'\b' + re.escape(receiver) + r'\s*:\s*register\s*\(\s*([A-Za-z_]\w*)\s*\)', skeleton))
        if len(registered) != 1 or registered[0].start() <= factory.end() or not top_level_at(skeleton, registered[0].start()):
            continue
        table_name = registered[0].group(1)
        if not single_local_binding(skeleton, table_name, r'\s*\{\s*\}'):
            continue
        # Bare table/factory uses must be their single declarations and the final register.
        # Unknown function arguments, return values or aliases can mutate either binding.
        prefix = skeleton[:registered[0].start()]
        allowed = re.compile(r'\blocal\s+(?:' + re.escape(table_name) + '|' + re.escape(receiver) + r')\s*=|(?<![\w.:])(?:' + re.escape(table_name) + '|' + re.escape(receiver) + r')\s*\.')
        masked_prefix = list(prefix)
        for match in allowed.finditer(prefix):
            masked_prefix[match.start():match.end()] = ' ' * len(match.group())
        if re.search(r'\b(?:' + re.escape(table_name) + '|' + re.escape(receiver) + r')\b', ''.join(masked_prefix)):
            continue
        fields = [m for m in EVENTS.finditer(code) if m.group(1) == table_name and not skeleton[m.start()].isspace()]
        event_uses = list(re.finditer(r'\b' + re.escape(table_name) + r'\s*\.\s*events\b', skeleton))
        declaration = re.search(r'\blocal\s+' + re.escape(table_name) + r'\s*=', skeleton)
        if len(fields) != 1 or len(event_uses) != 1 or fields[0].start() <= declaration.start() or fields[0].end() > registered[0].start() or not top_level_at(skeleton, fields[0].start()):
            results.append((factory.group(3), factory.span(), registered[0].span(), None, []))
            continue
        values = fields[0].group(2)
        parts = [v.strip() for v in values.split(',') if v.strip()]
        names = [STRING.fullmatch(value) for value in parts]
        if not names or not all(names):
            results.append((factory.group(3), factory.span(), registered[0].span(), None, []))
            continue  # computed table entries or escaped names remain unresolved
        results.append((factory.group(3), factory.span(), registered[0].span(), fields[0].span(), [n.group(2) for n in names]))
    return results


def build(repo_root, assignment, corpus_manifest):
    base = module(repo_root)
    packet = base.build(repo_root, assignment, corpus_manifest)
    corpus = base.Corpus(corpus_manifest)
    root = Path(repo_root)
    tool = root / 'tools/content-schema/quest-authoring'
    sys.path.insert(0, str(tool))
    import lua_writers
    initial = {row['source_component_id']: set(row['quest_keys']) for row in packet['records']}
    components = {(row['provenance']['source'], row['provenance']['revision'], row['provenance']['path']): row for row in packet['records']}
    registrars, registrar_counts, monsters, monster_counts, lower_rules = defaultdict(list), Counter(), defaultdict(list), Counter(), {}
    for file in corpus.files.values():
        source, revision, path = file['source'], file['revision'], file['path']
        if source not in {'canary', 'crystalserver'}:
            continue
        if path == 'src/creatures/monsters/monsters.cpp':
            text = corpus.text(file)
            get = re.search(r'Monsters::getMonsterType\(.*?asLowerCaseString\(name\)', text, re.S)
            # Source versions use a separate getOrCreateMonsterType function. Bind the exact lowercase assignment itself.
            assignments = list(re.finditer(r'const std::string lowerName = asLowerCaseString\(name\)', text))
            if get and assignments:
                lower_rules[(source, revision)] = [corpus.witness(file, *get.span()), corpus.witness(file, *assignments[0].span())]
        if '/monster/' in path and path.endswith('.xml'):
            try:
                descriptor = ET.fromstring(corpus.text(file))
            except ET.ParseError:
                descriptor = None
            if descriptor is not None and descriptor.tag == 'monster' and descriptor.get('name'):
                registry_key = (source, revision, path.split('/')[0], descriptor.get('name').lower())
                monster_counts[registry_key] += 1
                monsters[registry_key].append((file, None, None, None, []))
        if not path.endswith('.lua') or ('/scripts/' not in path and '/monster/' not in path):
            continue
        text = corpus.text(file)
        pack = path.split('/')[0]
        if '/scripts/' in path:
            code, skeleton = lua_writers.mask_code(text, literals=False), lua_writers.mask_code(text)
            for call in re.finditer(r'\bCreatureEvent\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*\)', code):
                if skeleton[call.start():call.start() + 13] == 'CreatureEvent':
                    registrar_counts[(source, revision, pack, call.group(2))] += 1
            for name, constructor, registration in base.event_registrations(text, lua_writers.mask_code):
                registrars[(source, revision, pack, name)].append((file, constructor, registration))
        if '/monster/' in path:
            code, skeleton = lua_writers.mask_code(text, literals=False), lua_writers.mask_code(text)
            for call in re.finditer(r'\bGame\s*\.\s*createMonsterType\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*\)', code):
                if skeleton[call.start():call.start() + 4] == 'Game':
                    monster_counts[(source, revision, pack, call.group(2).lower())] += 1
            for name, constructor, registration, fields, events in monster_registration(text, lua_writers.mask_code):
                monsters[(source, revision, pack, name.lower())].append((file, constructor, registration, fields, events))
    anchors = defaultdict(dict)
    # Existing bounded Source proofs seed components. Their dispatch/authority qualifiers remain in every derived chain.
    for identity, row in components.items():
        for proof in row['proofs']:
            anchors[identity].setdefault(proof['quest_key'], (proof['source_reference'], proof['source_witnesses'], 0))
    graph_projection, progress_projection = [], []
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        for record in base.read(path)['records']:
            q = record['definition']
            qkey = q['identity']['key']
            for track in q.get('source_data', {}).get('progress', []):
                for transition in track.get('transitions', []):
                    for occurrence in transition.get('source_occurrences', []):
                        identity = (occurrence['source'], occurrence['revision'], occurrence['path'])
                        file = corpus.files.get(identity)
                        if not file or file['sha256'] != occurrence['blob_sha256']:
                            raise ValueError('canonical occurrence does not resolve')
                        text = corpus.text(file)
                        line = occurrence['line']
                        actual = text.splitlines()[line - 1]
                        if base.sha(actual.encode()) != occurrence['line_sha256']:
                            raise ValueError('canonical progress source line mismatch')
                        start = sum(len(l) + 1 for l in text.split('\n')[:line - 1])
                        witness = corpus.witness(file, start, start + len(actual))
                        anchors[identity].setdefault(qkey, (track['key'], [witness], 0))
                        progress_projection.append((qkey, track['key'], occurrence))
    canonical = {}
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        for record in base.read(path)['records']:
            q = record['definition']
            canonical[q['identity']['key']] = {g['identity']['key'] for g in q.get('source_data', {}).get('interactions', [])}
    for q in base.read(tool / 'samples/binding_packets/source/crosswalk.json')['quests']:
        for binding in q['trigger_bindings']:
            if binding['source_graph'] not in canonical[q['quest_key']]:
                continue
            for descriptor in binding['sources']:
                matches = [f for f in corpus.files.values() if f['source'] == descriptor['source'] and f['path'] == descriptor['path'] and f['git_blob_sha1'] == descriptor['blob_sha1']]
                if len(matches) != 1:
                    raise ValueError('ambiguous graph source')
                f = matches[0]
                text = corpus.text(f)
                line = descriptor['callback_line']
                start = sum(len(l) + 1 for l in text.split('\n')[:line - 1])
                witness = corpus.witness(f, start, start + len(text.split('\n')[line - 1]))
                identity = (f['source'], f['revision'], f['path'])
                anchors[identity].setdefault(q['quest_key'], (binding['source_graph'], [witness], 0))
                graph_projection.append((q['quest_key'], binding['source_graph'], descriptor))
    edges = defaultdict(list)
    # Evaluate the reachable Source file graph, not every possible runtime call.
    pending = deque(anchors)
    scanned = set()
    while pending:
        identity = pending.popleft()
        if identity in scanned:
            continue
        scanned.add(identity)
        file = corpus.files[identity]
        text = corpus.text(file)
        source, revision, path = identity
        pack = path.split('/')[0]
        for call in base.literal_event_calls(text, lua_writers.mask_code):
            matches = registrars.get((source, revision, pack, call.group(2)), [])
            if len(matches) == 1 and registrar_counts[(source, revision, pack, call.group(2))] == 1:
                target, constructor, registration = matches[0]
                target_id = (target['source'], target['revision'], target['path'])
                edges[identity].append((target_id, 'PINNED_TRANSITIVE_REGISTER_EVENT_REFERENCE', call.group(2), [corpus.witness(file, *call.span()), corpus.witness(target, *constructor), corpus.witness(target, *registration)], []))
                pending.append(target_id)
        if (source, revision) not in lower_rules:
            continue
        for call in literal_calls(text, lua_writers.mask_code):
            name = call.group(2)
            matches = monsters.get((source, revision, pack, name.lower()), [])
            if len(matches) != 1 or monster_counts[(source, revision, pack, name.lower())] != 1:
                continue
            monster, constructor, registration, fields, events = matches[0]
            for event in events:
                matches = registrars.get((source, revision, pack, event), [])
                if len(matches) != 1 or registrar_counts[(source, revision, pack, event)] != 1:
                    continue
                target, event_constructor, event_registration = matches[0]
                target_id = (target['source'], target['revision'], target['path'])
                evidence = [corpus.witness(file, *call.span()), corpus.witness(monster, *constructor), corpus.witness(monster, *registration), corpus.witness(monster, *fields), corpus.witness(target, *event_constructor), corpus.witness(target, *event_registration)]
                edges[identity].append((target_id, 'PINNED_LITERAL_MONSTER_CREATION_TO_REGISTERED_EVENT_REFERENCE', name + ':' + event, evidence, lower_rules[(source, revision)]))
                pending.append(target_id)
    # One finite shortest Source reference route per canonical Quest/file; cycles never self-generate ownership.
    queue = deque((identity, qkey, value) for identity, values in anchors.items() for qkey, value in values.items())
    seen = {(identity, qkey) for identity, values in anchors.items() for qkey in values}
    found = defaultdict(list)
    while queue:
        identity, qkey, (reference, route, depth) = queue.popleft()
        for target, basis, literal, witnesses, normalization in edges[identity]:
            if (target, qkey) in seen:
                continue
            seen.add((target, qkey))
            chain = route + witnesses
            found[target].append({'basis': basis, 'quest_key': qkey, 'source_reference': reference, 'binding_literal': literal, 'source_witnesses': chain, 'normalization_source_witnesses': normalization, 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED', 'reference_chain_depth': depth + 1})
            queue.append((target, qkey, (reference, chain, depth + 1)))
    for identity, row in components.items():
        for proof in row['proofs']:
            proof['normalization_source_witnesses'] = []
            proof['reference_chain_depth'] = 0
        row['proofs'].extend(found.get(identity, []))
        row['quest_keys'] = sorted({p['quest_key'] for p in row['proofs']})
        row['closed_mapping_gaps'] = ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'] if row['quest_keys'] else []
        if row['quest_keys']:
            row['remaining_holds'] = [h for h in row['remaining_holds'] if h != 'EXACT_QUEST_ASSOCIATION_NOT_ESTABLISHED']
        text = corpus.text(corpus.files[identity])
        masked = lua_writers.mask_code(text)
        row['association_scope'] = ('MULTIPLE_CANONICAL_QUEST_SOURCE_REFERENCES' if len(row['quest_keys']) > 1 else 'QUEST_ASSOCIATED_SOURCE_NONEXCLUSIVE' if row['quest_keys'] else 'WORLD_EVENT_WITHOUT_PROVEN_QUEST_OWNER' if re.search(r'\bGlobalEvent\s*\(', masked) else 'SHARED_FACTORY_INSTANCE_WITHOUT_PROVEN_QUEST_OWNER' if re.search(r'\bBossLever\s*\(', masked) else 'NO_PROVEN_QUEST_OWNER')
    packet['quests'] = [{'quest_key': key, 'component_ids': sorted(row['source_component_id'] for row in packet['records'] if key in row['quest_keys']), 'closed_mapping_gaps': ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'], 'quest_completeness': 'NOT_ASSESSED'} for key in sorted({q for row in packet['records'] for q in row['quest_keys']})]
    packet['schema'] = 'OTERYN_EXACT_SOURCE_COMPONENT_QUEST_JOINS/v2'
    packet['input_sha256']['reference_roots_semantic_slice'] = base.canonical_hash([progress_projection, graph_projection])
    packet['summary'] = {'components': len(packet['records']), 'joined_components_before': sum(bool(v) for v in initial.values()), 'joined_components': sum(bool(r['quest_keys']) for r in packet['records']), 'joined_quests': len(packet['quests']), 'new_joined_components': sum(bool(r['quest_keys']) and not initial[r['source_component_id']] for r in packet['records']), 'proofs_by_basis': dict(sorted(Counter(p['basis'] for row in packet['records'] for p in row['proofs']).items())), 'association_scopes': dict(sorted(Counter(r['association_scope'] for r in packet['records']).items())), 'missing_source_bytes': 0}
    return packet


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('repo-root', 'assignment', 'corpus-manifest', 'out'):
        parser.add_argument('--' + arg, required=True)
    args = parser.parse_args()
    packet = build(args.repo_root, args.assignment, args.corpus_manifest)
    Path(args.out).write_text(json.dumps(packet, ensure_ascii=False, sort_keys=True, indent=2) + '\n')
    print(json.dumps(packet['summary'], sort_keys=True))
