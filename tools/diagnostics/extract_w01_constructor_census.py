#!/usr/bin/env python3
"""Runner-only pinned source namespace census. Emits derived facts, never raw assets.
Static registrations are candidates, not proof that arbitrary Lua executes successfully.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tarfile
import urllib.request

PIN = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
TREE = 'ca1636f57645e072e28e6492f19108e4ae17b86b'
TOKENIZER_SHA = '904c34e4a59c934d6151465299e7fe11e2ce5190bb7ce776e68235b05499dbd8'
MAX_FILE = 16 * 1024 * 1024
MAX_SELECTED_BYTES = 256 * 1024 * 1024
MAX_OUTPUT = 8 * 1024 * 1024
# Required consumer literals/config-resolved names; these select queries only,
# and never supply a missing source constructor or profile.
REQUIRED_NAMES = {
    'Outburst', 'Anomaly', 'bone capsule', 'disgusting ooze',
    'the shielded thorn knight', 'thorn steed', 'the enraged thorn knight',
    'unbound blightwalker', 'unbound demon', 'unbound demon outcast',
    'unbound defiler', 'possessed tree', 'Spark of Destruction',
    'Charged Anomaly', 'Damage Resonance', 'The Hunger', 'The Destruction',
    'The Rage', 'World Devourer', 'Spark of Destruction2', 'Eradicator2',
}
EXTRA_BIRTH_SOURCES = {
    'data-otservbr-global/scripts/quests/heart_of_destruction/actions_anomaly.lua',
    'data-otservbr-global/scripts/quests/heart_of_destruction/actions_rupture.lua',
    'data-otservbr-global/scripts/quests/heart_of_destruction/actions_final_lever.lua',
}
ENGINE = {
    'src/creatures/monsters/monsters.cpp',
    'src/lua/scripts/scripts.cpp',
    'src/lua/functions/creatures/monster/monster_type_functions.cpp',
    'src/lua/functions/core/game/game_functions.cpp',
    'src/lua/functions/lua_functions_loader.cpp',
    'src/game/game.cpp',
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def download(url, bound):
    request = urllib.request.Request(url, headers={'User-Agent': 'Oteryn-pinned-source-census'})
    with urllib.request.urlopen(request, timeout=90) as response:
        raw = response.read(bound + 1)
    if len(raw) > bound:
        raise ValueError('bounded source transport exceeded')
    return raw


def factory_arguments(tokens, opening):
    """Return complete top-level argument expressions; never drop variant/alias."""
    arguments = []
    current = []
    depth = 0
    for token in tokens[opening+1:]:
        if token.kind != 'string' and token.value == ')' and depth == 0:
            if current:
                arguments.append(current)
            elif arguments:
                arguments.append([])
            return [{'literal': expression[0].value} if len(expression) == 1 and expression[0].kind == 'string'
                    else {'dynamic_tokens': [t.value for t in expression]}
                    for expression in arguments]
        if token.kind != 'string' and token.value == ',' and depth == 0:
            arguments.append(current)
            current = []
            continue
        if token.value in {'(', '{', '['} and token.kind != 'string':
            depth += 1
        elif token.value in {')', '}', ']'} and token.kind != 'string':
            depth -= 1
        current.append(token)
        if len(current) > 128 or len(arguments) > 8:
            raise ValueError('factory expression exceeds bound')
    raise ValueError('unclosed factory argument list')


def registry_keys(arguments):
    # Exact pinned luaGameCreateMonsterType semantics. Dynamic names/variants/
    # alternates stay unresolved; first-argument presence alone is insufficient.
    if not arguments or any('literal' not in a for a in arguments[:3]):
        return None
    name = arguments[0]['literal']
    variant = arguments[1]['literal'] if len(arguments) > 1 else ''
    alternate = arguments[2]['literal'] if len(arguments) > 2 else ''
    keys = set()
    if variant.startswith('!'):
        keys.add(name)
        variant = variant[1:]
    keys.add(variant + '|' + name if variant else name)
    if alternate:
        keys.add(alternate)
    return sorted(key.lower() for key in keys)


def scan_lua(text, tokenize):
    tokens = tokenize(text)
    rows = {'constructors': [], 'factory_other_references': [], 'births': [],
            'dynamic_births': [], 'dynamic_execution': []}
    for i, token in enumerate(tokens):
        if token.value in {'load', 'loadstring', 'dofile', 'loadfile', 'require'}:
            rows['dynamic_execution'].append({'line': token.line, 'operation': token.value})
        if token.value not in {'createMonsterType', 'createMonster'}:
            continue
        direct = i >= 2 and [t.value for t in tokens[i-2:i]] == ['Game', '.']
        call = i+1 < len(tokens) and tokens[i+1].value == '('
        literal = (call and i+3 < len(tokens) and tokens[i+2].kind == 'string'
                   and tokens[i+3].value in {',', ')'})
        row = {'line': token.line, 'direct_Game_call': direct}
        if literal:
            row['name'] = tokens[i+2].value
        if token.value == 'createMonsterType' and direct and call:
            row['arguments'] = factory_arguments(tokens, i+1)
            row['derived_registry_keys'] = registry_keys(row['arguments'])
        if token.value == 'createMonsterType':
            rows['constructors' if direct and literal else 'factory_other_references'].append(row)
        elif direct and literal:
            rows['births'].append(row)
        else:
            rows['dynamic_births'].append(row)
    return rows


def extract(repository, output):
    tokenizer = repository / 'tools/content-schema/native-gameplay/death_event_source.py'
    if sha(tokenizer.read_bytes()) != TOKENIZER_SHA:
        raise ValueError('reviewed lexical tokenizer changed')
    spec = importlib.util.spec_from_file_location('w01_lua_tokens', tokenizer)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    audit = json.loads((repository / 'docs/agents/evidence/srx3-creature-event-audit.json').read_bytes())
    donor = next(d for d in audit['donors'] if d['repository'] == 'opentibiabr/canary')
    if donor['revision'] != PIN:
        raise ValueError('W01 source audit revision mismatch')
    birth_sources = set(EXTRA_BIRTH_SOURCES)
    source_pins = {}
    for event in donor['events']:
        for source in event['sources']:
            birth_sources.add(source['path'])
            prior = source_pins.setdefault(source['path'], (source['git_blob'], source['sha256']))
            if prior != (source['git_blob'], source['sha256']):
                raise ValueError('conflicting audit source pin')
    tree_raw = download('https://api.github.com/repos/opentibiabr/canary/git/trees/' + TREE + '?recursive=1', 16*1024*1024)
    tree = json.loads(tree_raw)
    if tree.get('truncated') is not False or tree.get('sha') != TREE:
        raise ValueError('pinned recursive tree is incomplete or mismatched')
    entries = {row['path']: row for row in tree['tree'] if row['type'] == 'blob'}
    if len(entries) != sum(row['type'] == 'blob' for row in tree['tree']):
        raise ValueError('duplicate tree paths')
    selected = {path: row for path, row in entries.items()
                if path.endswith(('.lua', '.cpp', '.hpp', '.h', '.xml')) or path == 'config.lua.dist'}
    expected = set(selected)
    seen = set()
    total = 0
    facts = []
    registrations = {}
    all_factories = []
    references = []
    dynamic = []
    literals = []
    dynamic_births = []
    requested = set(REQUIRED_NAMES)
    engine_facts = []
    # Streaming tar: selected source bytes exist only transiently in memory.
    url = 'https://codeload.github.com/opentibiabr/canary/tar.gz/' + PIN
    with urllib.request.urlopen(url, timeout=90) as response:
        with tarfile.open(fileobj=response, mode='r|gz') as archive:
            for member in archive:
                parts = member.name.split('/', 1)
                if len(parts) != 2:
                    continue
                path = parts[1]
                if path not in selected:
                    continue
                if not member.isfile() or member.size > MAX_FILE or path in seen:
                    raise ValueError('unsafe/duplicate/oversized selected archive entry')
                raw = archive.extractfile(member).read(MAX_FILE+1)
                total += len(raw)
                if total > MAX_SELECTED_BYTES:
                    raise ValueError('selected source corpus exceeds bound')
                blob = hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
                if blob != selected[path]['sha']:
                    raise ValueError('archive/tree Git blob mismatch: '+path)
                seen.add(path)
                record = {'path': path, 'git_blob': blob, 'sha256': sha(raw), 'bytes': len(raw)}
                facts.append(record)
                if path in source_pins and (blob, sha(raw)) != source_pins[path]:
                    raise ValueError('actual W01 source differs from accepted audit: '+path)
                if path.endswith('.lua'):
                    parsed = scan_lua(raw.decode('utf-8-sig'), module.tokens)
                    for row in parsed['constructors']:
                        all_factories.append({**record, **row})
                        registrations.setdefault(row['name'].lower(), []).append({**record, **row,
                            'disabled_filename': Path(path).name.startswith('#')})
                    for kind, destination in [('factory_other_references', references),
                                               ('dynamic_execution', dynamic)]:
                        destination.extend({'path': path, **row} for row in parsed[kind])
                    if path in birth_sources:
                        for row in parsed['births']:
                            requested.add(row['name'])
                            literals.append({**record, **row})
                        dynamic_births.extend({'path': path, **row} for row in parsed['dynamic_births'])
                if path in ENGINE or (path.startswith('src/') and any(word in raw for word in (b'tryAddMonsterType(', b'loadScripts('))):
                    text = raw.decode('utf-8-sig')
                    needles = ('tryAddMonsterType', 'getMonsterType(', 'loadScripts(', 'createMonsterType',
                               'luaCreateMonsterType', 'asLowerCaseString', 'loadFile(', 'registerMethod')
                    engine_facts.append({**record, 'operation_lines': {
                        word: [i for i, line in enumerate(text.splitlines(), 1) if word in line]
                        for word in needles if word in text}})
    if seen != expected or not birth_sources.issubset(seen) or not ENGINE.issubset(seen):
        raise ValueError('archive/source namespace coverage incomplete')
    names = [{'name': name, 'casefold_lookup': name.lower(),
              'literal_constructor_candidates': registrations.get(name.lower(), []),
              'static_candidate_present': bool(registrations.get(name.lower())),
              'runtime_constructor_absence_qualified': False}
             for name in sorted(requested, key=str.lower)]
    result = {'schema': 'OTERYN_W01_PINNED_CONSTRUCTOR_NAMESPACE/v1',
        'source': {'repository': 'opentibiabr/canary', 'revision': PIN, 'tree': TREE,
                   'recursive_tree_sha256': sha(tree_raw), 'truncated': False},
        'coverage': {'selected_files': len(seen), 'selected_bytes': total,
                     'lua_files': sum(p.endswith('.lua') for p in seen),
                     'all_selected_git_blobs_verified': True},
        'names': names, 'explicit_required_consumer_names': sorted(REQUIRED_NAMES),
        'reachability_note': 'Additional literal names discovered in audit scripts may belong to parent branches outside the admitted W01 cohort; namespace presence is not callback admission.',
        'literal_birth_calls': literals, 'dynamic_birth_calls': dynamic_births,
        'all_literal_constructor_names': sorted(registrations),
        'all_literal_factory_argument_facts': all_factories,
        'all_derived_registry_keys': sorted({key for row in all_factories
                                            for key in (row.get('derived_registry_keys') or [])}),
        'unresolved_factory_argument_facts': [row for row in all_factories
                                               if row.get('derived_registry_keys') is None],
        'factory_alias_or_dynamic_references': references, 'dynamic_execution_sites': dynamic,
        'engine_loader_and_registry_facts': engine_facts,
        'source_files': sorted(facts, key=lambda x:x['path']),
        'limitations': ['Static candidates do not prove successful engine execution or configured load roots.',
            'Dynamic factory references/execution must be reviewed before any runtime absence claim.',
            'No missing constructor is mapped to known nil by this extractor.',
            'Lua lexical string escapes beyond quotes/backslashes require manual review.']}
    raw = (json.dumps(result, indent=2, sort_keys=True)+'\n').encode()
    if len(raw) > MAX_OUTPUT:
        raise ValueError('derived output exceeds bound')
    output.write_bytes(raw)
    print('source corpus verified; derived facts only; names='+str(len(names)))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repository', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    extract(args.repository, args.output)
