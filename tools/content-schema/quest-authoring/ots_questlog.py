"""Transcribe the quest logs of Canary and CrystalServer into storyline quests with staged missions (D34).

Usage: python ots_questlog.py --canary <opentibiabr/canary at 47dfd51f> --crystal <zimbadev/crystalserver at ff7ede59>
                              [--chests samples/chests] [--doors samples/doors]
                              [--coverage samples/quest-coverage-2026-09-27.json] [--out samples/questlog]

Canary keeps one catalog file per quest (`lib/core/quests/catalog/*.lua`), CrystalServer one `Quests` table
(`lib/core/quests.lua`); both describe a quest as missions, each shown while one storage lies between a start and
an end value, with a journal text per value. That is option A of CONTENT-QUEST-01: a mission is a progress
track with named stages. Quests are joined by name, missions by name. The output is the whole quest catalogue:
storyline quests plus the reward-only quests of the chest slice that no storyline quest absorbs. A writer index
lists, per progress track, where each server's Lua sources set it; the transitions themselves are later work.
"""
import argparse
import glob
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import lua_tables
import lua_writers
from ots_chests import CONFLICT_DECISIONS, REVISION, ROOT, SOURCES, check_checkout, decided, git_blob, quest_key, ref, slug, text_ref, unused_decisions, wiki_matcher



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
    return out


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
    tracks = defaultdict(lambda: {'count': Counter(), 'transitions': {}})
    for name, repo in repos.items():
        packs = [SOURCES[name]['datapack'], 'data'] + (['data-crystal'] if name == 'crystalserver' else [])
        for pack in packs:
            for path in sorted(glob.glob(str(Path(repo) / pack / '**/*.lua'), recursive=True)):
                rel = str(Path(path).relative_to(repo))
                for write in lua_writers.scan(Path(path).read_text(errors='replace'), rel):
                    target = int(write['target']) if write['target'].isdigit() else write['target']
                    track = tracks[norm(track_of(target))]
                    track['count'][name] += 1
                    ident = json.dumps([script_of(rel), write['owner'], write['callback'], effect_of(write), write['from']],
                                       sort_keys=True)
                    entry = track['transitions'].setdefault(ident, {
                        'owner': write['owner'], 'callback': write['callback'], 'from': write['from'], **effect_of(write),
                        'script': script_of(rel), 'sources': {}})
                    entry['sources'].setdefault(name, {'path': rel, 'line': write['line'],
                                                       'registrations': write['registrations']})
    return tracks


def mission_transitions(found):
    """Stable transition keys per track: owner and ordinal over the sorted script/line list."""
    out, seen = [], Counter()
    for entry in sorted(found['transitions'].values(),
                        key=lambda e: (e['owner'], e['script'], min(s['line'] for s in e['sources'].values()))):
        seen[entry['owner']] += 1
        effect = {k: entry[k] for k in ('to', 'increment', 'computed') if k in entry}
        out.append({'key': f'{entry["owner"]}_{seen[entry["owner"]]}', 'owner': entry['owner'], 'callback': entry['callback'],
                    'from': entry['from'], **effect, 'servers': sorted(entry['sources']), '_entry': entry})
    return out


def build(repos, chests_dir, doors_dir, coverage):
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
            requirements = {f: wiki[f] for f in ('premium', 'lvl') if wiki.get(f)}
            if requirements:
                quest['requirements_from_wiki'] = requirements
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
    absorbed = []
    for key, quest in sorted(reward_only.items()):
        if key in storyline:
            absorbed.append(key)
        else:
            catalogue.append(quest)
    catalogue.sort(key=lambda q: q['identity']['key'])

    progress = []
    for track, info in sorted(tracks.items()):
        found = index.get(norm(track.split('/', 1)[1]), {'count': Counter(), 'transitions': {}})
        progress.append({'key': track, 'missions': info['missions'], 'start_of': info['start_of'],
                         'read_by_gates': sorted(g['identity']['key'] for g in gates if g['condition'].get('progress')
                                                 and norm(g['condition']['progress'].split('/', 1)[1]) == norm(track.split('/', 1)[1])),
                         'writes': {n: found['count'][n] for n in repos},
                         'transitions': [{'key': t['key'], 'script': t['_entry']['script'], 'sources': t['_entry']['sources']}
                                         for t in mission_transitions(found)]})
    all_transitions = [t for q in storyline.values() for m in q['missions'] for t in m['transitions']]
    counts = Counter(e['status'] for e in manifest_entries)
    unused_decisions('missions', used_decisions)
    manifest = {
        'classification': 'OTS_HYPOTHESIS_ONLY',
        'join': 'quest and mission names (normalised)',
        'sources': [{'kind': 'git', 'repository': SOURCES[n]['repository'], 'revision': SOURCES[n]['revision'], 'path': p,
                     'blob_sha1': git_blob(repos[n], p)} for n in repos for p in sorted({q['path'] for q in logs[n]})],
        'counts': {
            'quest_log_entries': {n: len(q) for n, q in logs.items()},
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
            'transitions_in_both_servers': sum(1 for t in all_transitions if len(t['servers']) == 2),
            'storyline_quests_linked_to_wiki': sum(1 for q in storyline.values() if 'wiki' in q),
            'reward_only_quests_absorbed': len(absorbed),
            'catalogue_quests': len(catalogue),
            'by_status': dict(sorted(counts.items())),
        },
        'reward_only_absorbed': absorbed,
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
