"""Transcribe the quest logs of Canary and CrystalServer into storyline quests with staged missions (D34).

Usage: python ots_questlog.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
                              [--chests samples/chests] [--doors samples/doors]
                              [--coverage samples/quest-coverage-2026-09-27.json] [--out samples/questlog]

Canary keeps one catalog file per quest (`lib/core/quests/catalog/*.lua`), CrystalServer one `Quests` table
(`lib/core/quests.lua`); both describe a quest as missions, each shown while one storage lies between a start and
an end value, with a journal text per value. That is option A of CONTENT-QUEST-01: a mission is a progress
track with named stages. Quests are joined by name, missions by name. The output is the whole quest catalogue:
storyline quests, the reward-only quests of the chest slice that no storyline quest absorbs, and the
script_only quests of `script_quests.json` (a quest the servers implement in scripts with no quest-log
entry, curated from the wiki coverage sample). A writer index lists, per progress track, where each
server's Lua sources set it; the transitions themselves are later work.
"""
import argparse
import glob
import hashlib
import json
import source_reference_tracks
from quest_requirement_interpretations import interpret_requirements
import re
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables
import lua_writers
from ots_chests import CONFLICT_DECISIONS, REVISION, ROOT, SOURCES, check_checkout, decided, git_blob, quest_key, ref, requirements_counts, requirements_of, slug, text_ref, unused_decisions, wiki_matcher



ARENA_FALLBACK_SHA256 = "ec82c49d3c3d76e2156076a399ca1f5c357277db2a2a873f49d66d868796efb0"

# read-only: the NPC authoring census lists every converted NPC bundle key of both servers
NPC_KEYS = {row['key'] for path in sorted((ROOT.parent / 'npc-authoring' / 'samples').glob('census-*.json'))
            for row in json.loads(path.read_text())['rows']}
# the NPC authoring format keys bundles `<source>:npc/<file stem>` (its decision D1)
NPC_NAMESPACE = {'canary': 'canary', 'crystalserver': 'crystal'}


def norm(text):
    return re.sub(r'[^a-z0-9]', '', text.lower())


def track_of(expr):
    """The progress-track path of a storage expression, as the chest and door slices name markers."""
    if isinstance(expr, int):
        return f'storage/{expr}'
    return '/'.join(slug(part) for part in expr.split('.')[1:])


def read_questlogs(name, repo):
    base = Path(repo)
    quests = []
    if name == 'canary':
        pack = SOURCES[name]['datapack']
        for path in sorted(glob.glob(str(base / pack / 'lib/core/quests/catalog/[0-9]*.lua'))):
            table = lua_tables.assignments(Path(path).read_text(), {'quest'})['quest']
            quests.append((str(Path(path).relative_to(base)), table))
    else:
        for pack in ('data-global', 'data-crystal'):
            rel = f'{pack}/lib/core/quests.lua'
            for field in lua_tables.assignments((base / rel).read_text(), {'Quests'})['Quests']['fields']:
                quests.append((rel, field['value']))
    out = []
    for path, table in quests:
        value = lua_tables.as_python(table)
        missions = value.get('missions') or []
        missions = missions if isinstance(missions, list) else [missions[k] for k in sorted(missions)]
        out.append({'server': name, 'path': path, 'line': table['line'], 'name': value['name'],
                    'start': value.get('startStorageId'), 'start_value': value.get('startStorageValue'),
                    'missions': missions})
    return select_questlog_primary(name, base, out)


def select_questlog_primary(name, base, entries):
    """Configured primary wins only over the exact pinned conditional initializer.

    An unexpected duplicate or missing primary is UNKNOWN, never first/last wins.
    The fallback is preserved as Source provenance, including its unexecuted loop.
    """
    primary_pack = SOURCES[name]["datapack"]
    groups = defaultdict(list)
    for entry in entries:
        groups[norm(entry["name"])].append(entry)
    selected = []
    for group in groups.values():
        primary = [q for q in group if q["path"].split("/", 1)[0] == primary_pack]
        alternatives = [q for q in group if q not in primary]
        if len(primary) != 1:
            raise ValueError("UNKNOWN: quest-log duplicate or configured primary missing: " + group[0]["name"])
        choice = dict(primary[0])
        if alternatives:
            allowed_path = "data-crystal/lib/core/quests.lua"
            expected_sha = ARENA_FALLBACK_SHA256
            if (name != "crystalserver" or primary_pack != "data-global" or len(alternatives) != 1
                    or choice["name"] != "The Ultimate Challenges"
                    or alternatives[0]["path"] != allowed_path
                    or SOURCES[name]["revision"] != "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
                    or hashlib.sha256((base / allowed_path).read_bytes()).hexdigest() != expected_sha):
                raise ValueError("UNKNOWN: unreviewed duplicate/conditional quest-log initializer: " + choice["name"])
            expected_entry = {"server": name, "path": allowed_path, "line": 3, "name": choice["name"],
                "start": {"expr": "Storage.Quest.U8_0.BarbarianArena.QuestLogGreenhorn"},
                "start_value": 1, "missions": []}
            if alternatives[0] != expected_entry:
                raise ValueError("UNKNOWN: conditional initializer parsed shape changed")
            modes = lua_tables.as_python(lua_tables.assignments(
                (base / allowed_path).read_text(), {"modes"})["modes"])
            choice["conditional_fallbacks"] = [{**alternatives[0],
                "classification": "OTS_HYPOTHESIS_ONLY", "source_sha256": expected_sha,
                "condition": {"expression": "not Quests", "line_start": 1, "line_end": 30},
                "dynamic_missions": {"declared_modes": modes, "mode_table_lines": [11, 15], "loop_line": 17,
                    "assignment_line": 18, "loop_end_line": 29,
                    "execution_semantics": "UNKNOWN_SOURCE_ONLY_NOT_EVALUATED"},
                "selection_basis": "configured primary data-global; exact pinned conditional fallback retained"}]
        selected.append(choice)
    return selected


def journal_of(mission):
    """Per-stage texts from `states`, one text from `description`, or a template when the source computes it.

    Journal lines are narrative text (LICENSE-ASSETS.md): only text references are kept."""
    def entry(text):
        if isinstance(text, dict) and text.get('function'):
            return {'template': {'parts': [text_ref(s) for s in text['strings'] if s],
                                 'reads': sorted({track_of(n) for n in text['names']})}}
        return {'text_ref': text_ref(text)}
    states = mission.get('states')
    if isinstance(states, list):
        states = {i + 1: text for i, text in enumerate(states)}
    if isinstance(states, dict) and states:
        return {'kind': 'per_stage', 'stages': [{'value': int(v), **entry(t)} for v, t in sorted(states.items())]}
    description = mission.get('description')
    if description is None:
        return {'kind': 'none'}
    one = entry(description)
    return {'kind': 'template', **one['template']} if 'template' in one else {'kind': 'fixed', 'text_ref': one['text_ref']}


def comparable_mission(mission):
    def strip(value):
        if isinstance(value, dict):
            if value.get('function'):
                return {'function': value['strings']}
            if 'expr' in value:
                return norm(value['expr'])
            return {k: strip(v) for k, v in value.items()}
        if isinstance(value, list):
            return [strip(v) for v in value]
        return value
    return {k: strip(mission.get(k)) for k in ('storageId', 'startValue', 'endValue', 'states', 'description')}


def script_of(path):
    """A script path without its datapack, so the same script in both servers compares equal."""
    return re.sub(r'^(data-otservbr-global|data-global|data-crystal|data)/', '', path)


def effect_of(write):
    return {k: write[k] for k in ('to', 'increment', 'computed') if k in write}


def transition_index(repos):
    """Per progress track: write counts per server and the union of both servers' candidate transitions (D35)."""
    tracks = defaultdict(lambda: {'count': Counter(), 'transitions': {}, 'paths': set()})
    for name, repo in repos.items():
        packs = [SOURCES[name]['datapack'], 'data'] + (['data-crystal'] if name == 'crystalserver' else [])
        for pack in packs:
            for path in sorted(glob.glob(str(Path(repo) / pack / '**/*.lua'), recursive=True)):
                rel = str(Path(path).relative_to(repo))
                raw = Path(path).read_bytes()
                source_text = raw.decode('utf-8', errors='replace')
                source_lines = raw.splitlines()
                blob_sha256 = hashlib.sha256(raw).hexdigest()
                for write in lua_writers.scan(source_text, rel):
                    target = int(write['target']) if write['target'].isdigit() else write['target']
                    track = tracks[norm(track_of(target))]
                    track['count'][name] += 1
                    track['paths'].add((name, track_of(target)))
                    ident = json.dumps([script_of(rel), write['owner'], write['callback'], effect_of(write), write['from']],
                                       sort_keys=True)
                    entry = track['transitions'].setdefault(ident, {
                        'owner': write['owner'], 'callback': write['callback'], 'from': write['from'], **effect_of(write),
                        'script': script_of(rel), 'sources': {}, 'source_occurrences': []})
                    source = {'path': rel, 'line': write['line'],
                              'registrations': write['registrations'],
                              **({'dialogue': write['dialogue']} if 'dialogue' in write else {})}
                    entry['sources'].setdefault(name, source)
                    entry['source_occurrences'].append({
                        'source': name, 'occurrence': track['count'][name],
                        'repository': SOURCES[name]['repository'], 'revision': SOURCES[name]['revision'],
                        'target': write['target'], 'blob_sha256': blob_sha256,
                        'line_sha256': hashlib.sha256(source_lines[write['line'] - 1]).hexdigest(), **source})
    return tracks


def requested_by(entry):
    """The NPC dialogue that requests an NPC-owned transition (D35): the NPC bundle key of the NPC authoring
    format and the player keywords and topics of the if-blocks around the write, from Canary when it has it."""
    server = 'canary' if 'canary' in entry['sources'] else 'crystalserver'
    return requested_by_source(server, entry['sources'][server])


def requested_by_source(server, source):
    """The requester belongs to this exact source occurrence, including its dialogue context."""
    # a keyword of up to two words is a player command; a longer phrase is reserved text (LICENSE-ASSETS.md)
    keywords = [k if len(k.split()) <= 2 and len(k) <= 20 else text_ref(k) for k in source['dialogue']['keywords']]
    return {'npc': f'{NPC_NAMESPACE[server]}:npc/{Path(source["path"]).stem}', 'keywords': keywords,
            'topics': source['dialogue']['topics']}


def mission_transitions(found):
    """Stable transition keys per track: owner and ordinal over the sorted script/line list."""
    out, seen = [], Counter()
    for entry in sorted(found['transitions'].values(),
                        key=lambda e: (e['owner'], e['script'], min(s['line'] for s in e['sources'].values()))):
        seen[entry['owner']] += 1
        effect = {k: entry[k] for k in ('to', 'increment', 'computed') if k in entry}
        out.append({'key': f'{entry["owner"]}_{seen[entry["owner"]]}', 'owner': entry['owner'], 'callback': entry['callback'],
                    'from': entry['from'], **effect, 'servers': sorted(entry['sources']),
                    **({'requested_by': requested_by(entry)} if entry['owner'] == 'npc' else {}), '_entry': entry})
    return out


def progress_transition(transition):
    """Preserve every scanned write and its requester while retaining the primary source view."""
    entry = transition['_entry']
    write = {k: v for k, v in transition.items() if k != '_entry'}
    occurrences = []
    for source in sorted(entry.get('source_occurrences', []),
                         key=lambda source: (source['source'], source['occurrence'])):
        actual_write = {**write, 'servers': [source['source']]}
        if entry['owner'] == 'npc':
            actual_write['requested_by'] = requested_by_source(source['source'], source)
        occurrences.append({**source, 'write': actual_write})
    return {'key': transition['key'], 'script': entry['script'], 'sources': entry['sources'],
            'write': write, 'source_occurrences': occurrences}


TRACK_OWNERS = {norm(track): owner for track, owner in json.loads((ROOT / 'track_owners.json').read_text())['tracks'].items()}
SCRIPT_QUESTS = json.loads((ROOT / 'script_quests.json').read_text())['quests']
DEFERRED_TRACKS = json.loads((ROOT / 'track_owners.json').read_text()).get('deferred_tracks', {})


def validate_source_curations(entries, repos):
    """Exact pinned blobs support new source curations; names alone do not prove execution."""
    for entry in entries:
        if not entry.get('source_paths'):
            continue  # Historical curations retain their independently reviewed basis.
        evidence = entry.get('source_evidence', [])
        canonical = [e for e in evidence if e.get('source') in repos]
        if not canonical or not entry.get('coverage_gap'):
            raise ValueError('missing pinned canonical source or explicit coverage gap: ' + entry['title'])
        if not {script_of(path) for path in entry['source_paths']} <= {script_of(e['path']) for e in canonical}:
            raise ValueError('unproven source path: ' + entry['title'])
        for item in canonical:
            source = item['source']
            expected = SOURCES[source]
            path = Path(item['path'])
            if (path.is_absolute() or '..' in path.parts
                    or item.get('repository') != expected['repository']
                    or item.get('revision') != expected['revision']
                    or item.get('role') != 'executable'
                    or item.get('classification') != 'OTS_HYPOTHESIS_ONLY'):
                raise ValueError('invalid source curation provenance: ' + entry['title'])
            actual = Path(repos[source]) / path
            if not actual.is_file() or hashlib.sha256(actual.read_bytes()).hexdigest() != item.get('blob_sha256'):
                raise ValueError('stale source curation blob: ' + entry['title'])
            if not isinstance(item.get('line'), int) or not 1 <= item['line'] <= len(actual.read_text().splitlines()):
                raise ValueError('invalid source curation line: ' + entry['title'])


def curation_coverage_holds(catalogue, entries):
    """An executable source outside the transcribed slice remains an explicit data gap."""
    by_title = {q['display_name']: q['identity']['key'] for q in catalogue}
    return [{'quest': by_title[e['title']], 'source_paths': e['source_paths'],
             'coverage_gap': e['coverage_gap'], 'classification': 'UNKNOWN'}
            for e in entries if e.get('source_paths') and not e.get('npc_source')
            and e.get('coverage_gap') and e['title'] in by_title]


def script_only_quests(coverage):
    """Quests the servers implement in scripts (chests, bosses, world state) with no quest-log entry (§6.2 gap):
    curated matches of a wiki quest to a script directory and/or the wiki_quest of an auxiliary track
    (`script_quests.json`), each with its own basis. The namespace follows coverage (D33); the identity slug
    is the curated key when the wiki spelling and the script directory diverge, else `slug(title)`."""
    by_title = {row['title']: row for row in coverage['quests']}
    quests, seen_titles = [], set()
    for entry in SCRIPT_QUESTS:
        if entry['title'] in seen_titles:
            raise SystemExit(f'script_quests.json: duplicate title {entry["title"]!r}')
        seen_titles.add(entry['title'])
        wiki = by_title.get(entry['title'])
        if wiki is None:
            raise SystemExit(f'script_quests.json: no coverage entry for {entry["title"]!r}')
        namespace = 'canary' if wiki.get('canary') in ('IMPLEMENTED', 'PARTIAL') else 'crystalserver'
        key = f'{namespace}:quest/{entry.get("key") or slug(entry["title"])}'
        quest = {'identity': {'key': key, 'revision': REVISION}, 'display_name': wiki['title'],
                 'kind': 'script_only', 'shown_in_quest_log': False,
                 'wiki': {k: wiki[k] for k in ('title', 'pageid', 'revid')}, 'claims': []}
        quest.update(requirements_of(wiki))
        quests.append(quest)
    return quests


def storage_declarations(repos):
    """Read exact Storage declarations; a numeric ID is qualified by its repository."""
    declarations = {}
    for server, repo in repos.items():
        packs = [SOURCES[server]['datapack']] + (['data-crystal'] if server == 'crystalserver' else [])
        for pack in packs:
            path = f"{pack}/lib/core/storages.lua"
            table = lua_tables.assignments((Path(repo) / path).read_text(), {'Storage'})['Storage']
            def walk(node, parts):
                for field in node.get('fields', []):
                    names = parts + [str(field['key'])]
                    value = field['value']
                    if isinstance(value, dict) and 'fields' in value:
                        walk(value, names)
                    elif isinstance(value, int):
                        key = f"{server}:quest-progress/" + '/'.join(slug(n) for n in names)
                        declaration = {'repository': SOURCES[server]['repository'],
                                       'revision': SOURCES[server]['revision'], 'path': path,
                                       'line': field['line'], 'storage_id': value,
                                       'expression': 'Storage.' + '.'.join(names)}
                        previous = declarations.get(key)
                        if previous and previous.get('state') == 'CONFLICT':
                            previous['candidates'].append(declaration)
                        elif previous and previous['storage_id'] != value:
                            declarations[key] = {'state': 'CONFLICT', 'candidates': [previous, declaration]}
                        elif previous is None:
                            declarations[key] = declaration
            walk(table, [])
    return declarations


def storage_evidence(declarations, key, required):
    if not required:
        return {}
    if key not in declarations:
        return {'source_checks': {'storage_declaration': f'UNKNOWN: no exact Storage declaration for {key}'}}
    declaration = declarations[key]
    if declaration.get('state') == 'CONFLICT':
        return {'source_checks': {'storage_declaration': f'CONFLICT: incompatible source Storage declarations for {key}'},
                'source_storage_candidates': declaration['candidates']}
    return {'source_storage': declaration}


def merge_source_checks(owner, evidence):
    """Preserve independent ownership and declaration gaps on the same track."""
    merged = {**owner, **evidence}
    checks = {**owner.get('source_checks', {}), **evidence.get('source_checks', {})}
    if checks:
        merged['source_checks'] = checks
    return merged


def exclusive_npc_writers(found, recorded):
    """A curated NPC owner requires every observed writer, with no extra shared writer."""
    expected = set(recorded.get('npc_sources') or ([recorded['npc_source']] if recorded.get('npc_source') else []))
    writes = list(found['transitions'].values())
    return bool(expected and writes and {t['script'] for t in writes} == expected
                and all(t['owner'] == 'npc' for t in writes))


def unique_track_prefix(paths, prefixes):
    """Match whole path segments; similarly named storage fields are separate owners."""
    matches = [prefix for prefix, owners in prefixes.items() if len(owners) == 1
               and any(path == prefix or path.startswith(prefix + '/') for path in paths)]
    if not matches:
        return None
    depth = max(len(prefix.split('/')) for prefix in matches)
    nearest = [prefix for prefix in matches if len(prefix.split('/')) == depth]
    return nearest[0] if len({tuple(sorted(prefixes[p])) for p in nearest}) == 1 else None


def npc_auxiliary_prefix(found, prefix):
    """Retain a single NPC writer family's source progress; broad version roots,
    mixed callers and colliding normalized paths do not prove a quest owner."""
    entries = list(found['transitions'].values())
    paths = {path for _, path in found['paths']}
    if (not prefix or not entries or len(paths) != 1
            or any(entry['owner'] != 'npc' for entry in entries)
            or len({entry['script'] for entry in entries}) != 1):
        return False
    parts = prefix.split('/')
    version_root = len(parts) >= 2 and parts[0] == 'quest' and re.fullmatch(r'u\d+(?:_\d+)*', parts[1])
    return len(parts) >= (3 if version_root else 2)


def auxiliary_tracks(index, progress, catalogue, gates, repos):
    """Declare every track a quest script writes outside the missions (D35): seal doors, counters, cooldowns.

    The owning quest comes from the longest mission-track prefix that names one quest, else from the script
    directory whose scripts write the missions of one quest, else from `track_owners.json`.
    """
    declared = {norm(t['key'].split('/', 1)[1]) for t in progress}
    gate_tracks = defaultdict(list)
    for gate in gates:
        if gate['condition'].get('progress'):
            gate_tracks[norm(gate['condition']['progress'].split('/', 1)[1])].append(gate)
    source_storage = storage_declarations(repos)
    catalogue_keys = {q['identity']['key'] for q in catalogue}
    by_title = {q['display_name']: q['identity']['key'] for q in catalogue}
    prefixes, directories = defaultdict(set), defaultdict(set)
    for quest in catalogue:
        for mission in quest.get('missions', []):
            path = mission['progress'].split('/', 1)[1]
            parts = path.split('/')
            for k in range(2, len(parts)):
                prefixes['/'.join(parts[:k])].add(quest['identity']['key'])
            for t in index.get(norm(path), {'transitions': {}})['transitions'].values():
                if t['script'].startswith('scripts/quests/'):
                    directories[t['script'].split('/')[2]].add(quest['identity']['key'])
    out, used, missing = [], set(), []
    for key_norm in sorted(set(index) | set(gate_tracks)):
        found = index.get(key_norm, {'count': Counter(), 'transitions': {}, 'paths': set()})
        scripts = sorted({t['script'] for t in found['transitions'].values() if t['script'].startswith('scripts/quests/')})
        npc_owned = exclusive_npc_writers(found, TRACK_OWNERS.get(key_norm, {}))
        owner = {}
        source_paths = {path for _, path in found['paths']} | {
            g['condition']['progress'].split('/', 1)[1] for g in gate_tracks.get(key_norm, [])}
        prefix = unique_track_prefix(source_paths, prefixes)
        npc_prefix_owned = npc_auxiliary_prefix(found, prefix)
        if key_norm in declared or (not scripts and key_norm not in gate_tracks
                                   and not npc_owned and not npc_prefix_owned):
            continue
        by_directory = set().union(*(directories.get(s.split('/')[2], set()) for s in scripts))
        gate_owners = {g['quest']['key'] for g in gate_tracks.get(key_norm, [])
                       if g.get('quest') and g['quest']['key'] in catalogue_keys}
        if key_norm in TRACK_OWNERS and key_norm in gate_tracks:
            used.add(key_norm)
        shared_gate = TRACK_OWNERS.get(key_norm, {})
        expected_npcs = set(shared_gate.get('npc_sources') or ([shared_gate['npc_source']] if shared_gate.get('npc_source') else []))
        if expected_npcs and not npc_owned and not shared_gate.get('shared_npc_gate'):
            used.add(key_norm)
            owner = {'auxiliary_of': [], 'owner_basis': 'UNKNOWN',
                     'source_checks': {'owner': 'UNKNOWN: curated exclusive NPC writer set changed; observed '
                                       + ', '.join(sorted({t['script'] for t in found['transitions'].values()}))
                                       + '; expected ' + ', '.join(sorted(expected_npcs))}}
        elif shared_gate.get('shared_npc_gate'):
            actual_npcs = {t['script'] for t in found['transitions'].values() if t['owner'] == 'npc'}
            if actual_npcs != set(shared_gate['npc_sources']):
                raise SystemExit(f'shared NPC gate curation is stale for {key_norm}')
            used.add(key_norm)
            owner = {'auxiliary_of': [], 'owner_basis': 'UNKNOWN',
                     'source_checks': {'owner': 'UNKNOWN: ' + shared_gate['note']}}
        elif npc_owned and key_norm in TRACK_OWNERS:
            used.add(key_norm)
            recorded = TRACK_OWNERS[key_norm]
            quest = recorded.get('quest') or by_title.get(recorded.get('wiki_quest'))
            owner = {'auxiliary_of': [quest] if quest else [],
                     'owner_basis': 'exclusive curated NPC writers',
                     **({'source_checks': {'owner': 'UNKNOWN: curated NPC has no catalogue quest'}} if not quest else {})}
        elif len(gate_owners) == 1:
            owner = {'auxiliary_of': sorted(gate_owners), 'owner_basis': 'gate catalogue link'}
        elif key_norm in TRACK_OWNERS:
            used.add(key_norm)
            recorded = TRACK_OWNERS[key_norm]
            quest = recorded.get('quest') or by_title.get(recorded.get('wiki_quest'))
            owner = {'auxiliary_of': [quest] if quest else [], 'owner_basis': 'track_owners.json',
                     **({'wiki_quest': recorded['wiki_quest']} if not quest and recorded.get('wiki_quest') else {}),
                     **({'note': recorded['note']} if recorded.get('note') else {}),
                     **({'source_checks': {'owner': 'UNKNOWN: ' + recorded['note']}}
                        if recorded.get('note', '').startswith('Unknown quest owner:') else {})}
        elif prefix:
            owner = {'auxiliary_of': sorted(prefixes[prefix]), 'owner_basis': 'mission track prefix'}
        elif len(by_directory) == 1:
            owner = {'auxiliary_of': sorted(by_directory), 'owner_basis': 'script directory'}
        elif key_norm in gate_tracks:
            owner = {'auxiliary_of': [], 'owner_basis': 'UNKNOWN',
                     'source_checks': {'owner': 'UNKNOWN; gate/source declaration does not establish a unique catalogue owner'}}
        else:
            missing.append(key_norm)
            continue
        server = 'canary' if found['count']['canary'] else 'crystalserver'
        paths = sorted(p for n, p in found['paths'] if n == server)
        if not paths:
            server, path = gate_tracks[key_norm][0]['condition']['progress'].split(':quest-progress/')
        else:
            path = paths[0]
        out.append({'key': f'{server}:quest-progress/{path}', 'missions': [], 'start_of': [],
                    'read_by_gates': sorted(g['identity']['key'] for g in gates if g['condition'].get('progress')
                                            and norm(g['condition']['progress'].split('/', 1)[1]) == key_norm),
                    **merge_source_checks(owner, storage_evidence(source_storage, f'{server}:quest-progress/{path}', key_norm in gate_tracks or npc_prefix_owned or TRACK_OWNERS.get(key_norm, {}).get('source_declaration_required', False))), 'writes': {n: found['count'][n] for n in repos},
                    'transitions': [progress_transition(t) for t in mission_transitions(found)]})
    # Gates retain their exact source-qualified track keys. A matching path under
    # another namespace is an explicit alias of the transcription, not numeric-ID equality.
    for group in gate_tracks.values():
        for gate in group:
            key = gate['condition']['progress']
            all_tracks = {t['key']: t for t in progress + out}
            if key in all_tracks:
                continue
            source_path = key.split(':quest-progress/', 1)[1]
            matches = [t for t in all_tracks.values() if t['key'].split(':quest-progress/', 1)[1] == source_path
                       and 'alias_of' not in t]
            if len(matches) != 1:
                raise SystemExit(f'gate progress {key} has no unique declared path')
            original = matches[0]
            alias_owners = sorted(set(original.get('auxiliary_of', []))
                                  | set(original.get('start_of', []))
                                  | {mission.split('#', 1)[0] for mission in original.get('missions', [])})
            alias_owner = {'auxiliary_of': alias_owners, 'owner_basis': 'exact source-path alias'}
            if not alias_owners:
                alias_owner['source_checks'] = {'owner': original.get('source_checks', {}).get(
                    'owner', 'UNKNOWN: aliased track has no explicit mission/start/auxiliary owner')}
            out.append({'key': key, 'missions': [], 'start_of': [],
                        'read_by_gates': sorted(g['identity']['key'] for g in group
                                                if g['condition']['progress'] == key),
                        'alias_of': original['key'],
                        **merge_source_checks(alias_owner, storage_evidence(source_storage, key, True)),
                        'writes': original['writes'], 'transitions': original['transitions']})
    if missing:
        raise SystemExit(f'no owner for the progress tracks {missing}; record them in track_owners.json')
    stale = sorted(set(TRACK_OWNERS) - used)
    if stale:
        raise SystemExit(f'track_owners.json names tracks that need no record: {stale}')
    return out


def reward_script_owner(quest, by_key, by_page):
    """Only exact curator key or a unique wiki-page identity joins the two slices."""
    key = quest['identity']['key']
    if key in by_key:
        return by_key[key]
    matches = by_page.get((quest.get('wiki') or {}).get('pageid'), [])
    return matches[0] if len(matches) == 1 else None


def build(repos, chests_dir, doors_dir, coverage):
    validate_source_curations(SCRIPT_QUESTS, repos)
    logs = {name: read_questlogs(name, repo) for name, repo in repos.items()}
    claims = json.loads((chests_dir / 'claims.json').read_text())['claims']
    reward_only = {q['identity']['key']: q for q in json.loads((chests_dir / 'quests.json').read_text())['quests']}
    gates = json.loads((doors_dir / 'gates.json').read_text())['gates']
    match_quest = wiki_matcher(coverage)
    index = transition_index(repos)

    by_name = defaultdict(dict)
    for name, quests in logs.items():
        for quest in quests:
            by_name[norm(quest['name'])][name] = quest

    storyline, manifest_entries, tracks = {}, [], defaultdict(lambda: {'missions': [], 'start_of': []})
    used_decisions = set()
    for key_name, pair in sorted(by_name.items()):
        primary = pair.get('canary') or pair['crystalserver']
        namespace = 'canary' if 'canary' in pair else 'crystalserver'
        wiki = match_quest(primary['name'])
        key = quest_key(wiki) if wiki else f'{namespace}:quest/{slug(primary["name"])}'
        if key in storyline:
            # two quest-log entries for one wiki quest (CrystalServer's data-crystal The Ultimate Challenges)
            manifest_entries.append({'quest': primary['name'], 'status': 'unresolved_semantics',
                                     'resolution': f'second quest-log entry for {key}'})
            continue
        missions, seen, mission_rows = [], set(), []
        order = [(norm(m['name']), 'canary' if 'canary' in pair else 'crystalserver', m) for m in primary['missions']]
        if 'canary' in pair and 'crystalserver' in pair:
            known = {n for n, _, _ in order}
            order += [(norm(m['name']), 'crystalserver', m) for m in pair['crystalserver']['missions'] if norm(m['name']) not in known]
        other_missions = {norm(m['name']): m for m in pair['crystalserver']['missions']} if 'canary' in pair and 'crystalserver' in pair else {}
        for mission_name, server, mission in order:
            source_mission = mission
            decision = CONFLICT_DECISIONS['missions'].get(f'{key}#{slug(mission["name"]) or "mission"}')
            if decision and server == 'canary' and mission_name in other_missions:
                used_decisions.add(f'{key}#{slug(mission["name"]) or "mission"}')
                if decision['decision'] == 'crystalserver':
                    mission = other_missions[mission_name]
            else:
                decision = None
            storage = mission.get('storageId')
            if not isinstance(storage, (dict, int)):
                manifest_entries.append({'quest': primary['name'], 'mission': mission['name'], 'status': 'unresolved_semantics',
                                         'resolution': 'mission without a storage', 'source': server})
                continue
            track = f'{namespace}:quest-progress/{track_of(storage["expr"] if isinstance(storage, dict) else storage)}'
            mission_key = slug(mission['name']) or 'mission'
            while mission_key in seen:
                mission_key += '_2'
            seen.add(mission_key)
            start, end = mission.get('startValue'), mission.get('endValue')
            transitions = mission_transitions(index.get(norm(track.split('/', 1)[1]), {'transitions': {}}))
            missions.append({'key': mission_key, 'name': mission['name'], 'progress': track,
                             'start_value': start, 'end_value': end, 'journal': journal_of(mission),
                             'transitions': [{k: v for k, v in t.items() if k != '_entry'} for t in transitions]})
            tracks[track]['missions'].append(f'{key}#{mission_key}')
            status, resolution = 'mapped', 'present in both servers and identical'
            if server == 'crystalserver' and 'canary' in pair:
                resolution = 'mission present only in crystalserver'
            elif len(pair) == 1:
                resolution = f'present only in {server}'
            elif mission_name not in other_missions:
                resolution = 'mission present only in canary'
            else:
                a, b = comparable_mission(source_mission), comparable_mission(other_missions[mission_name])
                fields = sorted(k for k in a if a[k] != b[k])
                if fields:
                    status = 'conflict'
                    resolution = 'servers disagree on ' + ', '.join(fields) + '; the Canary value is kept'
                    if decision:
                        status, resolution = 'mapped', 'servers disagree on ' + ', '.join(fields) + '; ' + decided(decision)
            mission_rows.append({'quest': key, 'mission': mission_key, 'status': status, 'resolution': resolution,
                                 'sources': [{'source': n, 'path': q['path'], 'quest_line': q['line']} for n, q in pair.items()]})
        manifest_entries.extend(mission_rows)
        start = None
        if isinstance(primary['start'], dict):
            start_track = f'{namespace}:quest-progress/{track_of(primary["start"]["expr"])}'
            start = {'progress': start_track, 'at_least': primary['start_value'] if isinstance(primary['start_value'], int) else 1}
            tracks[start_track]['start_of'].append(key)
        quest = {'identity': {'key': key, 'revision': REVISION}, 'display_name': wiki['title'] if wiki else primary['name'],
                 'kind': 'storyline', 'shown_in_quest_log': True, 'source_name': primary['name'], 'start': start,
                 'missions': missions, 'claims': [], 'gates': []}
        if wiki:
            quest['wiki'] = {k: wiki[k] for k in ('title', 'pageid', 'revid')}
            quest.update(requirements_of(wiki))
        storyline[key] = quest

    # link claims and gates: through their quest link, or through a track one of the quest's missions uses
    # tracks are compared by path letters: CrystalServer-only quests use its namespace, and the servers
    # spell some storages differently (`GraveDanger.Questline`, `QuestLine`)
    track_owner = {norm(track.split('/', 1)[1]): mission.split('#')[0] for track, t in tracks.items() for mission in t['missions']}
    for claim in claims:
        owner = (claim['quest'] or {}).get('key')
        if owner in storyline:
            storyline[owner]['claims'].append(ref('RewardClaim', claim['identity']['key']))
    for gate in gates:
        progress = gate['condition'].get('progress')
        owner = (track_owner.get(norm(progress.split('/', 1)[1])) if progress else None) or (gate['quest'] or {}).get('key')
        if owner in storyline:
            storyline[owner]['gates'].append(ref('Gate', gate['identity']['key']))

    catalogue = list(storyline.values())
    script_only = script_only_quests(coverage)
    script_only_by_key = {q['identity']['key']: q for q in script_only}
    script_only_by_page = defaultdict(list)
    for q in script_only:
        script_only_by_page[q['wiki']['pageid']].append(q)
    source_aliases = []
    absorbed = []
    for key, quest in sorted(reward_only.items()):
        if key in storyline:
            absorbed.append(key)
        elif key in script_only_by_key:
            # a curated chest link (§6, chest_quest_links.json) named the same quest script_quests.json
            # already curated from its script directory: the script owns the identity, the chest its claims
            absorbed.append(key)
            script_only_by_key[key]['claims'].extend(quest['claims'])
        elif reward_script_owner(quest, script_only_by_key, script_only_by_page):
            # The same immutable wiki page identifies the chest and script slices.
            # Keep the curated script key and every RewardClaim reference; the parent
            # chest converter applies this source alias to its reciprocal owner refs.
            primary = reward_script_owner(quest, script_only_by_key, script_only_by_page)
            absorbed.append(key)
            primary['claims'].extend(quest['claims'])
            source_aliases.append({'source': key, 'target': primary['identity']['key'],
                                   'wiki_pageid': quest['wiki']['pageid'],
                                   'basis': 'same source wiki page; script identity retained'})
        else:
            catalogue.append(quest)
    for quest in script_only:
        key = quest['identity']['key']
        if key in storyline:
            raise SystemExit(f'script_quests.json: {key} collides with an existing quest')
        catalogue.append(quest)
    catalogue.sort(key=lambda q: q['identity']['key'])
    catalogue, requirement_checks = interpret_requirements(
        catalogue, json.loads((ROOT / 'wiki_quest_facts.json').read_text()),
        json.loads((ROOT / 'quest_requirement_curations.json').read_text())['entries'])

    progress = []
    for track, info in sorted(tracks.items()):
        found = index.get(norm(track.split('/', 1)[1]), {'count': Counter(), 'transitions': {}})
        progress.append({'key': track, 'missions': info['missions'], 'start_of': info['start_of'],
                         'read_by_gates': sorted(g['identity']['key'] for g in gates if g['condition'].get('progress')
                                                 and norm(g['condition']['progress'].split('/', 1)[1]) == norm(track.split('/', 1)[1])),
                         'writes': {n: found['count'][n] for n in repos},
                         'transitions': [progress_transition(t) for t in mission_transitions(found)]})
    progress += auxiliary_tracks(index, progress, catalogue, gates, repos)
    requested_reader_keys = set(json.loads((ROOT / 'source_read_track_requests.json').read_text())['keys'])
    reader_declarations = storage_declarations(repos)
    reader_evidence = source_reference_tracks.storage_readers(
        repos, SOURCES, track_of, requested_reader_keys, reader_declarations)
    reader_tracks, reader_checks = source_reference_tracks.retain_reader_tracks(
        progress, reader_evidence, reader_declarations, index, track_of,
        mission_transitions, progress_transition)
    progress += reader_tracks
    all_transitions = [t for q in storyline.values() for m in q['missions'] for t in m['transitions']]
    counts = Counter(e['status'] for e in manifest_entries)
    unused_decisions('missions', used_decisions)
    manifest = {
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'join': 'quest and mission names (normalised)',
        'sources': [{'kind': 'git', 'repository': SOURCES[n]['repository'], 'revision': SOURCES[n]['revision'], 'path': p,
                     'blob_sha1': git_blob(repos[n], p)} for n in repos for p in sorted({q['path'] for q in logs[n]} | {f['path'] for q in logs[n] for f in q.get('conditional_fallbacks', [])})],
        'counts': {
            'quest_log_entries': {n: sum(1 + len(q.get('conditional_fallbacks', [])) for q in rows) for n, rows in logs.items()},
            'quest_log_selected_entries': {n: len(rows) for n, rows in logs.items()},
            'storyline_quests': len(storyline),
            'quest_log_entries_only_crystalserver': sum(1 for pair in by_name.values() if 'canary' not in pair),
            'missions': sum(len(q['missions']) for q in storyline.values()),
            'journals': dict(Counter(m['journal']['kind'] for q in storyline.values() for m in q['missions'])),
            'progress_tracks': len(progress),
            'tracks_written_by_both_servers': sum(1 for p in progress if all(p['writes'][n] for n in repos)),
            'tracks_without_a_writer': sum(1 for p in progress if not any(p['writes'][n] for n in repos)),
            'transitions': len(all_transitions),
            'transitions_by_owner': dict(sorted(Counter(t['owner'] for t in all_transitions).items())),
            'transitions_by_effect': dict(sorted(Counter(next(k for k in ('to', 'increment', 'computed') if k in t)
                                                        for t in all_transitions).items())),
            'transitions_with_known_from': sum(1 for t in all_transitions if t['from']),
            'npc_transitions': {
                'total': sum(1 for t in all_transitions if 'requested_by' in t),
                'with_keywords': sum(1 for t in all_transitions if t.get('requested_by', {}).get('keywords')),
                'with_topics': sum(1 for t in all_transitions if t.get('requested_by', {}).get('topics')),
                'npc_in_npc_census': sum(1 for t in all_transitions if t.get('requested_by', {}).get('npc') in NPC_KEYS),
            },
            'transitions_in_both_servers': sum(1 for t in all_transitions if len(t['servers']) == 2),
            'storyline_quests_linked_to_wiki': sum(1 for q in storyline.values() if 'wiki' in q),
            'reward_only_quests_absorbed': len(absorbed),
            'script_only_quests': len(script_only),
            'catalogue_quests': len(catalogue),
            'by_status': dict(sorted(counts.items())),
            'requirements_parsed': requirements_counts(catalogue),
        },
        'reward_only_absorbed': absorbed,
        'quest_source_aliases': source_aliases,
        'source_checks': {'quest_log_primary_selection': [
            {'quest_name': q['name'], 'source': n, 'primary_path': q['path'], 'primary_line': q['line'],
             'conditional_fallbacks': q['conditional_fallbacks']}
            for n, rows in logs.items() for q in rows if q.get('conditional_fallbacks')],
            'quest_requirement_interpretations': requirement_checks,
                          'source_reference_readers': reader_checks, 'progress_track_unknowns': [
            {'key': p['key'], **p['source_checks']} for p in progress if p.get('source_checks')],
            'deferred_track_owners': DEFERRED_TRACKS,
            'script_coverage_holds': curation_coverage_holds(catalogue, SCRIPT_QUESTS),
            'npc_only_quests': [
                {'quest': q['identity']['key'], 'npc_source': entry['npc_source'],
                 'coverage_gap': 'NPC item/achievement/dialogue side effects have not been transcribed as quest interactions',
                 'classification': 'UNKNOWN'}
                for q in catalogue for entry in SCRIPT_QUESTS
                if entry.get('npc_source') and entry['title'] == q['display_name']]},
        'entries': sorted(manifest_entries, key=lambda e: json.dumps(e, sort_keys=True)),
    }
    return {'quests.json': {'quests': catalogue}, 'progress.json': {'progress': progress}, 'manifest.json': manifest}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--chests', type=Path, default=ROOT / 'samples/chests')
    parser.add_argument('--doors', type=Path, default=ROOT / 'samples/doors')
    parser.add_argument('--coverage', type=Path, default=ROOT / 'samples/quest-coverage-2026-09-27.json')
    parser.add_argument('--out', type=Path, default=ROOT / 'samples/questlog')
    args = parser.parse_args()
    repos = {'canary': args.canary, 'crystalserver': args.crystal}
    for name, repo in repos.items():
        check_checkout(name, repo)
    outputs = build(repos, args.chests, args.doors, json.loads(args.coverage.read_text()))
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in outputs.items():
        (args.out / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps(outputs['manifest.json']['counts'], indent=2))
    print('output sha256 ' + hashlib.sha256(json.dumps(outputs, sort_keys=True).encode()).hexdigest())


if __name__ == '__main__':
    main()
