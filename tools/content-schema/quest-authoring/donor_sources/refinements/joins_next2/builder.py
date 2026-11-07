"""Nonexclusive Source references through explicit registrations; no runtime/Quest promotion."""
import argparse
import base64
from collections import Counter, defaultdict
import gzip
import importlib.util
import json
from pathlib import Path
import re
import sys


def prior(root):
    path = Path(root) / 'tools/content-schema/quest-authoring/donor_sources/refinements/joins_next/builder.py'
    spec = importlib.util.spec_from_file_location('quest_prior_reference_traversal', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def controller_flow(controller, text, mask):
    candidates = [field['value'] for table in controller['all_tables'] for field in table.get('fields', []) if field['key'].get('name') == '__call' and field['value'].get('kind') == 'AnonymousFunction']
    methods = [statement for statement in controller['definition']['ordered_statements'] if statement.get('kind') == 'Method' and statement['raw'].startswith('function BossLever:onUse(')]
    if len(candidates) != 1 or len(methods) != 1:
        return None
    ctor, method = candidates[0], methods[0]
    for node, raw_key in [(ctor, 'raw_expression'), (method, 'raw')]:
        span = node['span']
        if text[span['start_char']:span['end_char_exclusive']] != node[raw_key]:
            return None
    constructor_code, method_code = mask(ctor['raw_expression']), mask(method['raw'])
    ctor_patterns = [r'function\s*\(self,\s*config\)', r'local\s+boss\s*=\s*config\.boss', r'name\s*=\s*boss\.name:lower\(\)']
    matches = [re.search(pattern, constructor_code) for pattern in ctor_patterns]
    spawn = re.search(r'Game\.createMonster\(self\.name,\s*self\.bossPosition', method_code)
    if not all(matches) or not spawn:
        return None
    if len(re.findall(r'(?<![\w.:])boss\s*=', constructor_code)) != 1 or re.search(r'\bboss\.name\s*=', constructor_code) or re.search(r'\bself\.name\s*=', method_code):
        return None
    # A boss descriptor alias passed to a helper could mutate name before the returned field.
    if re.search(r'\(\s*boss\s*[,)]|=\s*boss\s*[,;\n]', constructor_code):
        return None
    return [(ctor['span']['start_char'] + m.start(), ctor['span']['start_char'] + m.end()) for m in matches] + [(method['span']['start_char'] + spawn.start(), method['span']['start_char'] + spawn.end())]


def named_spells(text, mask, next_builder):
    code, skeleton = mask(text, literals=False), mask(text)
    out = []
    for declaration in re.finditer(r'\blocal\s+([A-Za-z_]\w*)\s*=\s*Spell\s*\(\s*(["\'])([A-Za-z0-9_ -]+)\2\s*\)', code):
        obj = declaration.group(1)
        if not next_builder.single_local_binding(skeleton, obj, r'\s*Spell\s*\('):
            continue
        names = list(re.finditer(r'\b' + re.escape(obj) + r'\s*:\s*name\s*\(\s*(["\'])([A-Za-z0-9_ .:\'-]+)\1\s*\)', code))
        registered = list(re.finditer(r'\b' + re.escape(obj) + r'\s*:\s*register\s*\(\s*\)', skeleton))
        all_names = list(re.finditer(r'\b' + re.escape(obj) + r'\s*:\s*name\s*\(', skeleton))
        if len(names) != 1 or len(all_names) != 1 or len(registered) != 1 or not declaration.end() < names[0].start() < registered[0].start():
            continue
        if not next_builder.top_level_at(skeleton, names[0].start()) or not next_builder.top_level_at(skeleton, registered[0].start()):
            continue
        prefix = skeleton[:registered[0].start()]
        bare = re.sub(r'\blocal\s+' + re.escape(obj) + r'\s*=|\b' + re.escape(obj) + r'\s*[.:]', ' ', prefix)
        if re.search(r'\b' + re.escape(obj) + r'\b', bare) or re.search(r'\b' + re.escape(obj) + r'\s*\.\s*(?:name|register)\s*=', prefix):
            continue
        out.append((names[0].group(2), declaration.span(), names[0].span(), registered[0].span()))
    return out


def attack_names(text, registration_span, mask, next_builder, lua_tables):
    code, skeleton = mask(text, literals=False), mask(text)
    registration = skeleton[registration_span[0]:registration_span[1]]
    table = re.search(r':\s*register\s*\(\s*(\w+)\s*\)', registration).group(1)
    uses = list(re.finditer(r'\b' + re.escape(table) + r'\s*\.\s*attacks\b', skeleton))
    if len(uses) != 1 or uses[0].start() >= registration_span[0] or not next_builder.top_level_at(skeleton, uses[0].start()):
        return []
    assignment = re.match(r'\s*=\s*\{', skeleton[uses[0].end():])
    if not assignment:
        return []
    start = uses[0].end() + assignment.end() - 1
    depth, end = 0, None
    for index in range(start, len(skeleton)):
        depth += (skeleton[index] == '{') - (skeleton[index] == '}')
        if depth == 0:
            end = index + 1
            break
    if end is None or end >= registration_span[0]:
        return []
    try:
        parsed = lua_tables.Parser(code[start:end]).table()
    except lua_tables.LuaError:
        return []
    valid = set()
    for field in parsed.get('fields', []):
        value = field['value']
        if not isinstance(value, dict) or 'fields' not in value:
            continue
        names = [entry['value'] for entry in value['fields'] if entry['key'] == 'name']
        if len(names) == 1 and isinstance(names[0], str):
            valid.add(names[0])
    result = []
    for match in re.finditer(r'\bname\s*=\s*(["\'])([A-Za-z0-9_ .:\'-]+)\1', code[start:end]):
        if match.group(2) in valid and not skeleton[start + match.start()].isspace():
            result.append((match.group(2), (start + match.start(), start + match.end())))
    return result


def boss_descriptor(record, text, next_builder, mask):
    definition = record['definition']
    constructors = definition['boss_constructors']
    if len(constructors) != 1 or len(constructors[0].get('arguments', [])) != 1:
        return None
    constructor = constructors[0]
    argument = constructor['arguments'][0]
    if argument.get('kind') != 'Name':
        return None
    name = argument['name']
    skeleton = mask(text)
    if not next_builder.single_local_binding(skeleton, name, r'\s*\{'):
        return None
    # Exactly one declaration and one factory argument; aliases/mutations/other uses are held.
    if len(re.findall(r'\b' + re.escape(name) + r'\b', skeleton)) != 2:
        return None
    tables = [t['definition'] for t in definition['config_tables'] if t['target'].get('name') == name]
    if len(tables) != 1:
        return None
    boss = [f['value'] for f in tables[0]['fields'] if f['key'].get('name') == 'boss']
    if len(boss) != 1 or boss[0].get('kind') != 'Table':
        return None
    names = [f['value'] for f in boss[0]['fields'] if f['key'].get('name') == 'name']
    if len(names) != 1 or names[0].get('kind') != 'String':
        return None
    boss_name = base64.b64decode(names[0]['value_bytes_base64']).decode('utf-8')
    raw_span = names[0].get('span')
    raw_value = text[raw_span['start_char']:raw_span['end_char_exclusive']] if raw_span else ''
    if raw_value != names[0].get('raw_expression') or len(raw_value) < 2 or raw_value[0] not in {'\"', "'"} or raw_value[-1] != raw_value[0] or raw_value[1:-1] != boss_name:
        return None  # escaped strings/computed Source or inconsistent AST bytes need another bounded decoder
    registration = [r for r in definition['registrations'] if r['callee'].get('name') == 'register']
    if len(registration) != 1:
        return None
    spans = [names[0]['span'], constructor['span'], registration[0]['span']]
    for node in [constructor, registration[0]]:
        span = node.get('span')
        if span is None or text[span['start_char']:span['end_char_exclusive']] != node.get('raw_expression'):
            return None
    if any(span is None for span in spans) or spans[0]['end_char_exclusive'] >= spans[1]['start_char'] or spans[1]['end_char_exclusive'] >= spans[2]['start_char']:
        return None
    return boss_name, spans


def build(repo_root, assignment, corpus_manifest):
    root = Path(repo_root)
    next_builder = prior(root)
    packet = next_builder.build(root, assignment, corpus_manifest)
    next_builder.MONSTER_TYPE = re.compile(r'''\blocal\s+([A-Za-z_]\w*)\s*=\s*Game\s*\.\s*createMonsterType\s*\(\s*(["'])([A-Za-z0-9_ .:'-]+)\2\s*\)''')
    base = next_builder.module(root)
    corpus = base.Corpus(corpus_manifest)
    tool = root / 'tools/content-schema/quest-authoring'
    sys.path.insert(0, str(tool))
    import lua_writers
    import lua_tables
    initial = {row['source_component_id']: set(row['quest_keys']) for row in packet['records']}
    rows_by_identity = {(r['provenance']['source'], r['provenance']['revision'], r['provenance']['path']): r for r in packet['records']}
    events, spell_names, monsters = defaultdict(list), defaultdict(list), defaultdict(list)
    event_counts, monster_counts, spell_counts = Counter(), Counter(), Counter()
    boss_path = tool / 'samples/donor-source/components248/boss.json.gz'
    boss_packet = json.loads(gzip.decompress(boss_path.read_bytes()))
    controller_packets = {(c['provenance']['source'], c['provenance']['revision']): c for c in boss_packet['controllers']}
    controllers = {}
    for file in corpus.files.values():
        source, revision, path = file['source'], file['revision'], file['path']
        if source not in {'canary', 'crystalserver'} or not path.endswith('.lua'):
            continue
        if path == 'data/libs/functions/boss_lever.lua':
            text = corpus.text(file)
            record = controller_packets.get((source, revision))
            found = controller_flow(record, text, lua_writers.mask_code) if record and base.provenance(file) == record['provenance'] else None
            if found:
                controllers[(source, revision)] = (file, found)
        if '/scripts/' not in path and '/monster/' not in path:
            continue
        text = corpus.text(file)
        code, skeleton = lua_writers.mask_code(text, literals=False), lua_writers.mask_code(text)
        prefix = (source, revision, path.split('/')[0])
        if '/scripts/' in path:
            for call in re.finditer(r'\bCreatureEvent\s*\(\s*(["\'])([A-Za-z0-9_ .:-]+)\1\s*\)', code):
                if skeleton[call.start():call.start() + 13] == 'CreatureEvent':
                    event_counts[prefix + (call.group(2),)] += 1
            for name, constructor, registration in base.event_registrations(text, lua_writers.mask_code):
                events[prefix + (name,)].append((file, constructor, registration))
            for name, declaration, naming, registration in named_spells(text, lua_writers.mask_code, next_builder):
                spell_names[prefix + (name,)].append((file, declaration, naming, registration))
            for naming in re.finditer(r':\s*name\s*\(\s*(["\'])([A-Za-z0-9_ .:\'-]+)\1\s*\)', code):
                if skeleton[naming.start()] == ':':
                    spell_counts[prefix + (naming.group(2),)] += 1
        if '/monster/' in path:
            for call in re.finditer(r'\bGame\s*\.\s*createMonsterType\s*\(\s*(["\'])([A-Za-z0-9_ .:\'-]+)\1\s*\)', code):
                if skeleton[call.start():call.start() + 4] == 'Game':
                    monster_counts[prefix + (call.group(2).lower(),)] += 1
            for name, constructor, registration, fields, registered_events in next_builder.monster_registration(text, lua_writers.mask_code):
                if registered_events:
                    monsters[prefix + (name.lower(),)].append((file, constructor, registration, fields, registered_events))
    # Canonical Source graph callbacks define roots; Source folder/title never participates.
    canonical = {}
    for shard in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        for record in base.read(shard)['records']:
            q = record['definition']
            canonical[q['identity']['key']] = {g['identity']['key'] for g in q.get('source_data', {}).get('interactions', [])}
    owned_event_references = defaultdict(list)
    for q in base.read(tool / 'samples/binding_packets/source/crosswalk.json')['quests']:
        for binding in q['trigger_bindings']:
            if binding['source_graph'] not in canonical[q['quest_key']]:
                continue
            for descriptor in binding['sources']:
                files = [f for f in corpus.files.values() if f['source'] == descriptor['source'] and f['path'] == descriptor['path'] and f['git_blob_sha1'] == descriptor['blob_sha1']]
                if len(files) != 1:
                    raise ValueError('Source graph does not have exact unique bytes')
                file = files[0]
                prefix = (file['source'], file['revision'], file['path'].split('/')[0])
                text = corpus.text(file)
                for name, constructor, registration in base.event_registrations(text, lua_writers.mask_code):
                    matches = events[prefix + (name,)]
                    if len(matches) == event_counts[prefix + (name,)] == 1:
                        owned_event_references[prefix + (name,)].append((q['quest_key'], binding['source_graph'], [corpus.witness(file, *constructor), corpus.witness(file, *registration)]))
    monster_refs = defaultdict(list)
    for registry_key, matches in monsters.items():
        if len(matches) != 1 or monster_counts[registry_key] != 1:
            continue
        file, constructor, registration, fields, attached = matches[0]
        prefix = registry_key[:3]
        for event in attached:
            for quest, reference, root_witnesses in owned_event_references[prefix + (event,)]:
                route = root_witnesses + [corpus.witness(file, *constructor), corpus.witness(file, *registration), corpus.witness(file, *fields)]
                monster_refs[registry_key].append((quest, reference, event, route))
    def proof(basis, quest, reference, literal, witnesses, shared=()):
        return {'basis': basis, 'quest_key': quest, 'source_reference': reference, 'binding_literal': literal, 'source_witnesses': witnesses, 'normalization_source_witnesses': [], 'dispatch_binding': 'NOT_PROVEN', 'storage_domain_equivalence': 'NOT_ASSESSED', 'reference_chain_depth': 1, 'association_direction': 'NONEXCLUSIVE_REGISTERED_REFERENCE', 'shared_dependency_witnesses': list(shared)}
    for registry_key, associations in monster_refs.items():
        file, constructor, registration, fields, attached = monsters[registry_key][0]
        prefix = registry_key[:3]
        for event in attached:
            registrar = events[prefix + (event,)]
            if len(registrar) != event_counts[prefix + (event,)] or len(registrar) != 1:
                continue
            target, event_constructor, event_registration = registrar[0]
            identity = (target['source'], target['revision'], target['path'])
            if identity not in rows_by_identity:
                continue
            row = rows_by_identity[identity]
            for quest, reference, root_event, witnesses in associations:
                row['proofs'].append(proof('REGISTERED_MONSTER_COREFERENCES_CANONICAL_QUEST_EVENT_AND_COMPONENT_EVENT', quest, reference, root_event + ':' + event, witnesses + [corpus.witness(target, *event_constructor), corpus.witness(target, *event_registration)]))
    # Named spell attacks are registered reference consumers of the same MonsterType.
    for registry_key, associations in monster_refs.items():
        monster, constructor, registration, fields, attached = monsters[registry_key][0]
        prefix = registry_key[:3]
        for name, attack_span in attack_names(corpus.text(monster), registration, lua_writers.mask_code, next_builder, lua_tables):
            matches = spell_names.get(prefix + (name,), [])
            if len(matches) != 1 or spell_counts[prefix + (name,)] != 1:
                continue
            target, declaration, naming, registered = matches[0]
            identity = (target['source'], target['revision'], target['path'])
            if identity not in rows_by_identity:
                continue
            row = rows_by_identity[identity]
            for quest, reference, event, route in associations:
                row['proofs'].append(proof('REGISTERED_MONSTER_QUEST_EVENT_TO_NAMED_REGISTERED_SPELL_REFERENCE', quest, reference, event + ':' + name, route + [corpus.witness(monster, *attack_span), corpus.witness(target, *declaration), corpus.witness(target, *naming), corpus.witness(target, *registered)]))
    # AST-backed shared factory descriptor follows controller property flow, then the registered Quest event.
    for record in boss_packet['records']:
        identity = (record['provenance']['source'], record['provenance']['revision'], record['provenance']['path'])
        file = corpus.files[identity]
        if base.provenance(file) != record['provenance']:
            raise ValueError('AST boss Source does not match pinned bytes')
        row = rows_by_identity[identity]
        descriptor = boss_descriptor(record, corpus.text(file), next_builder, lua_writers.mask_code)
        controller = controllers.get(identity[:2])
        row['shared_dependencies'] = []
        if not descriptor or not controller:
            continue
        name, spans = descriptor
        controller_file, controller_spans = controller
        shared = [corpus.witness(controller_file, *span) for span in controller_spans]
        local = [corpus.witness(file, span['start_char'], span['end_char_exclusive']) for span in spans]
        row['shared_dependencies'].append({'kind': 'BOSS_FACTORY_NAMED_REFERENCE', 'status': 'DISPATCH_AND_ACTIVATION_NOT_PROVEN', 'binding_literal': name, 'local_witnesses': local, 'provider_witnesses': shared})
        key = identity[:2] + (identity[2].split('/')[0], name.lower())
        for quest, reference, event, route in monster_refs.get(key, []):
            row['proofs'].append(proof('AST_BOSS_FACTORY_DESCRIPTOR_THROUGH_CONTROLLER_TO_REGISTERED_QUEST_EVENT_REFERENCE', quest, reference, name + ':' + event, local + route, shared))
    for row in packet['records']:
        row.setdefault('shared_dependencies', [])
        for item in row['proofs']:
            item.setdefault('association_direction', 'NONEXCLUSIVE_SOURCE_REFERENCE')
            item.setdefault('shared_dependency_witnesses', [])
        row['proofs'] = list({base.canonical_hash(item): item for item in row['proofs']}.values())
        row['quest_keys'] = sorted({item['quest_key'] for item in row['proofs']})
        row['closed_mapping_gaps'] = ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'] if row['quest_keys'] else []
        if row['quest_keys']:
            row['remaining_holds'] = [h for h in row['remaining_holds'] if h != 'EXACT_QUEST_ASSOCIATION_NOT_ESTABLISHED']
            row['association_scope'] = 'MULTIPLE_CANONICAL_QUEST_SOURCE_REFERENCES' if len(row['quest_keys']) > 1 else 'QUEST_ASSOCIATED_SOURCE_NONEXCLUSIVE'
    packet['schema'] = 'OTERYN_EXACT_SOURCE_COMPONENT_QUEST_JOINS/v3'
    packet['input_sha256']['boss_descriptor_packet'] = base.sha(boss_path.read_bytes())
    packet['quests'] = [{'quest_key': key, 'component_ids': sorted(r['source_component_id'] for r in packet['records'] if key in r['quest_keys']), 'closed_mapping_gaps': ['MISSING_EXACT_SOURCE_COMPONENT_QUEST_ASSOCIATION'], 'quest_completeness': 'NOT_ASSESSED'} for key in sorted({key for row in packet['records'] for key in row['quest_keys']})]
    for quest in packet['quests']:
        quest['source_traversal_holes'] = [{'source_component_id': row['source_component_id'], 'code': 'SOURCE_CALLBACK_EFFECTS_AND_REACHABILITY_NOT_CERTIFIED', 'source_path': row['provenance']['path'], 'native_admission': False} for row in packet['records'] if quest['quest_key'] in row['quest_keys']]
    packet['summary'] = {'components': len(packet['records']), 'joined_components_before': sum(bool(v) for v in initial.values()), 'joined_components': sum(bool(r['quest_keys']) for r in packet['records']), 'joined_quests': len(packet['quests']), 'new_joined_components': sum(bool(r['quest_keys']) and not initial[r['source_component_id']] for r in packet['records']), 'proofs_by_basis': dict(sorted(Counter(p['basis'] for row in packet['records'] for p in row['proofs']).items())), 'association_scopes': dict(sorted(Counter(r['association_scope'] for r in packet['records']).items())), 'shared_dependency_records': sum(bool(r['shared_dependencies']) for r in packet['records']), 'missing_source_bytes': 0}
    return packet


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ('repo-root', 'assignment', 'corpus-manifest', 'out'):
        parser.add_argument('--' + arg, required=True)
    args = parser.parse_args()
    result = build(args.repo_root, args.assignment, args.corpus_manifest)
    Path(args.out).write_text(json.dumps(result, sort_keys=True, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(result['summary'], sort_keys=True))
