"""Lower each Quest's source progress into QuestState tracks and transitions (QUEST-LOWER-1).

Reads the committed Quest definition shards (`content/quests/definitions/`) and writes
`content/quests/missions/quest-state.json`: per quest, its tracks (Oteryn key, owner quest,
initial value, `[min, max]`) and its transitions (closed effect kinds, `completes`,
`requested_by`), the catalogue QUEST-STATE-1's `QuestStateCatalogue` loads
(QUEST-STATE-0 §3, §4 and §13.2). Store keys are Oteryn keys; each source key stays a source
binding beside it and never reaches the store. XP (QUEST-XP-1) and gates or triggers
(QUEST-CONTENT-2) are not lowered here.

Rules, all deterministic over the source data:
- track key: `canary:quest-progress/<p>` -> `oteryn:quest-progress/<p>` (the Canary namespace is
  the default identity, format §5 rule 1); `crystalserver:quest-progress/<p>` ->
  `oteryn:quest-progress/crystalserver/<p>`, since a CrystalServer alias is not a proven native
  equivalent of the Canary track;
- transition key: `oteryn:quest-transition/<p>/<source transition key>`, with `<p>` the track
  key's tail;
- owner: the definition whose source progress lists the track; a track listed by several
  definitions belongs to the quest of its first mission, otherwise the lowering fails;
- initial: -1 where the track's source data uses -1 (a write, a comparison or a mission value),
  else 0 (§3);
- bounds: the least and greatest of the initial value and every written, compared and mission
  value; a track with an `ADD` or computed effect is bounded above by the source storage width
  (2^31 - 1), and a `SET_NOW` track by `i64::MAX`;
- effect: `to` -> `SET`, `increment` -> `ADD`, `computed: timestamp` -> `SET_NOW`,
  `computed: expression` -> `COMPUTED` (refused `NOT_SUPPORTED` until an amendment, §4);
- from: the source comparison (`==`, `~=`, `<`, `<=`, `>`, `>=`), or `ANY`; a comparison the
  source reads as not exact (a compound condition) is kept, failing closed, and marked;
- completes: only where every mission of the quest reads one track, for a `SET` of that track to
  the greatest mission end value; any other quest's completion stays unlowered and says so.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

DEFINITIONS = 'content/quests/definitions/'
OUTPUT = 'content/quests/missions/quest-state.json'
MARKER = 'content/quests/missions/index.json'
TOOL = 'tools/content-schema/quest-authoring/quest_state_lowering.py'
SCHEMA = 'OTERYN_QUEST_STATE_LOWERING/v1'
# QUESTSTATE0-RL-02 and -06, as `apps/game-server/src/quest/mod.rs`.
MAX_EFFECTS = 8
MAX_KEY_BYTES = 128
KEY = re.compile(r'^oteryn:[A-Za-z0-9._:/-]+$')
SOURCE_TRACK = re.compile(r'^(canary|crystalserver):quest-progress/([A-Za-z0-9._:/-]+)$')
STORAGE_WIDTH_MAX = 2**31 - 1
I64_MAX = 2**63 - 1
OPS = {'==': 'EQ', '~=': 'NE', '<': 'LT', '<=': 'LE', '>': 'GT', '>=': 'GE'}


class LoweringError(ValueError):
    pass


def compact(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n'


def read(root, relative):
    return json.loads((root / relative).read_text(encoding='utf-8'))


def digest(root, relative):
    return hashlib.sha256((root / relative).read_bytes()).hexdigest()


def require(ok, message):
    if not ok:
        raise LoweringError(message)


def valid_key(key):
    return len(key.encode('utf-8')) <= MAX_KEY_BYTES and bool(KEY.fullmatch(key))


def track_key(source_key):
    match = SOURCE_TRACK.fullmatch(source_key)
    require(match is not None, f'unsupported source track key: {source_key}')
    namespace, path = match.groups()
    key = 'oteryn:quest-progress/' + ('' if namespace == 'canary' else namespace + '/') + path
    require(valid_key(key), f'track key over the key rule: {key}')
    return key


def transition_key(track, source_key):
    require(re.fullmatch(r'[a-z_]+_[0-9]+', source_key) is not None, f'unsupported source transition key: {source_key}')
    key = 'oteryn:quest-transition/' + track[len('oteryn:quest-progress/'):] + '/' + source_key
    require(valid_key(key), f'transition key over the key rule: {key}')
    return key


def definitions(root):
    index = read(root, DEFINITIONS + 'index.json')
    return index['shards'], [row['definition'] for shard in index['shards'] for row in read(root, shard)['records']]


def owners(records):
    """Source track key -> the one owning quest definition (Oteryn key)."""
    listed = {}
    source_quest = {}
    for definition in records:
        data = definition.get('source_data') or {}
        quest = definition['identity']['key']
        if data.get('progress'):
            source_quest[data['quest']['identity']['key']] = quest
        for progress in data.get('progress', []):
            listed.setdefault(progress['key'], []).append((quest, progress))
    result = {}
    for key, entries in listed.items():
        if len(entries) == 1:
            result[key] = entries[0][0]
            continue
        missions = entries[0][1]['missions']
        require(all(progress == entries[0][1] for _, progress in entries), f'track {key} differs between its quests')
        first = source_quest.get(missions[0].split('#')[0]) if missions else None
        require(first in {quest for quest, _ in entries}, f'track {key} has no single owner')
        result[key] = first
    return result


def lower_from(write):
    source = write.get('from')
    if source is None:
        return {'op': 'ANY'}, True
    require(source['op'] in OPS and type(source['value']) is int, f'unsupported comparison {source}')
    return {'op': OPS[source['op']], 'value': source['value']}, bool(source['exact'])


def lower_effect(write):
    kinds = [name for name in ('to', 'increment', 'computed') if name in write]
    require(len(kinds) == 1, f'write {write["key"]} has no single effect')
    if 'to' in write:
        require(type(write['to']) is int, f'write {write["key"]} has a non-integer value')
        return {'kind': 'SET', 'value': write['to']}
    if 'increment' in write:
        require(type(write['increment']) is int, f'write {write["key"]} has a non-integer step')
        return {'kind': 'ADD', 'value': write['increment']}
    require(write['computed'] in ('timestamp', 'expression'), f'write {write["key"]} has an unknown computed kind')
    return {'kind': 'SET_NOW'} if write['computed'] == 'timestamp' else {'kind': 'COMPUTED'}


def requested_by(write):
    source = write.get('requested_by')
    if source is None:
        return None
    return {'npc': source['npc'], 'keywords': list(source['keywords']), 'topics': list(source['topics'])}


def mission_values(records):
    """Source track key -> every mission start and end value and quest start value on it."""
    values = {}
    for definition in records:
        source = (definition.get('source_data') or {}).get('quest') or {}
        for mission in source.get('missions', []):
            values.setdefault(mission['progress'], []).extend([mission['start_value'], mission['end_value']])
        start = source.get('start')
        if start and start.get('at_least') is not None:
            values.setdefault(start['progress'], []).append(start['at_least'])
    return values


def lower_quest(definition, owned, mission_values):
    data = definition['source_data']
    quest = definition['identity']['key']
    source = data['quest']
    missions = source.get('missions', [])
    mission_tracks = {mission['progress'] for mission in missions}
    completion_track = next(iter(mission_tracks)) if len(mission_tracks) == 1 else None
    completion_value = max(mission['end_value'] for mission in missions) if completion_track else None
    progress = [p for p in data['progress'] if owned[p['key']] == quest]
    require(mission_tracks <= {p['key'] for p in data['progress']}, f'{quest} has a mission track outside its progress')
    tracks, transitions = [], []
    for entry in progress:
        key = track_key(entry['key'])
        values = list(mission_values.get(entry['key'], []))
        kinds = set()
        for transition in entry['transitions']:
            write = transition['write']
            require(write['key'] == transition['key'], f'{entry["key"]} {transition["key"]} names another write')
            comparison, exact = lower_from(write)
            effect = lower_effect(write)
            kinds.add(effect['kind'])
            if 'value' in comparison:
                values.append(comparison['value'])
            if effect['kind'] == 'SET':
                values.append(effect['value'])
            completes = (entry['key'] == completion_track and effect == {'kind': 'SET', 'value': completion_value})
            transitions.append({
                'key': transition_key(key, transition['key']), 'quest': quest, 'completes': completes,
                'effects': [{'track': key, 'from': comparison, 'from_exact': exact, 'effect': effect}],
                'requested_by': requested_by(write),
                'source': {'key': transition['key'], 'owner': write['owner'], 'callback': write['callback'],
                           'script': transition['script'], 'servers': list(write['servers'])},
            })
        initial = -1 if -1 in values else 0
        low = min([initial, *values])
        high = max([initial, *values])
        basis = 'observed_values'
        if 'SET_NOW' in kinds:
            high, basis = I64_MAX, 'set_now_unbounded'
        elif kinds & {'ADD', 'COMPUTED'}:
            high, basis = max(high, STORAGE_WIDTH_MAX), 'source_storage_width'
        tracks.append({'key': key, 'quest': quest, 'initial': initial, 'min': low, 'max': high,
                       'bounds_basis': basis, 'source_key': entry['key']})
    if not missions:
        completion = 'NOT_LOWERED_NO_MISSIONS'
    elif completion_track is None:
        completion = 'NOT_LOWERED_MULTI_TRACK'
    elif completion_track not in {p['key'] for p in progress}:
        completion = 'NOT_LOWERED_FOREIGN_TRACK'
    else:
        completion = 'LOWERED'
    return {'quest': quest, 'source_quest': source['identity']['key'], 'completion': completion,
            'tracks': sorted(tracks, key=lambda t: t['key']),
            'transitions': sorted(transitions, key=lambda t: t['key'])}


def validate(quests):
    """The QuestStateCatalogue::new rules, so the loader never meets a refused catalogue."""
    tracks, transition_keys = {}, set()
    for quest in quests:
        require(valid_key(quest['quest']), f'invalid quest key {quest["quest"]}')
        for track in quest['tracks']:
            require(valid_key(track['key']) and track['quest'] == quest['quest'], f'invalid track {track["key"]}')
            require(track['key'] not in tracks, f'duplicate track {track["key"]}')
            require(track['min'] <= track['initial'] <= track['max'], f'invalid bounds {track["key"]}')
            require(-2**63 <= track['min'] and track['max'] <= I64_MAX, f'bounds over i64 {track["key"]}')
            tracks[track['key']] = track
        for transition in quest['transitions']:
            require(valid_key(transition['key']) and transition['quest'] == quest['quest'], f'invalid transition {transition["key"]}')
            require(transition['key'] not in transition_keys, f'duplicate transition {transition["key"]}')
            transition_keys.add(transition['key'])
            require(transition['effects'] or transition['completes'], f'empty transition {transition["key"]}')
            require(len(transition['effects']) <= MAX_EFFECTS, f'too many effects {transition["key"]}')
            seen = set()
            for effect in transition['effects']:
                track = tracks.get(effect['track'])
                require(track is not None and track['quest'] == quest['quest'], f'foreign track in {transition["key"]}')
                require(effect['track'] not in seen, f'duplicate effect track in {transition["key"]}')
                seen.add(effect['track'])
                if effect['effect']['kind'] == 'SET':
                    require(track['min'] <= effect['effect']['value'] <= track['max'], f'SET out of bounds in {transition["key"]}')


def expected(root):
    shards, records = definitions(root)
    owned = owners(records)
    values = mission_values(records)
    quests = [lower_quest(d, owned, values) for d in records if (d.get('source_data') or {}).get('progress')]
    quests.sort(key=lambda q: q['quest'])
    validate(quests)
    transitions = [t for q in quests for t in q['transitions']]
    effects = [t['effects'][0]['effect']['kind'] for t in transitions]
    payload = {
        'schema': SCHEMA, 'classification': 'OTS_HYPOTHESIS_ONLY', 'family': 'Quest',
        'contract': 'docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md',
        'authoring_sources': [{'path': path, 'sha256': digest(root, path)} for path in (DEFINITIONS + 'index.json', *shards, TOOL)],
        'counts': {
            'quests': len(quests), 'tracks': sum(len(q['tracks']) for q in quests), 'transitions': len(transitions),
            'effects': {kind: effects.count(kind) for kind in sorted(set(effects))},
            'inexact_from': sum(1 for t in transitions if not t['effects'][0]['from_exact']),
            'requested_by': sum(1 for t in transitions if t['requested_by']),
            'completes': sum(1 for t in transitions if t['completes']),
            'completion': {state: sum(1 for q in quests if q['completion'] == state) for state in sorted({q['completion'] for q in quests})},
        },
        'quests': quests,
    }
    marker = read(root, MARKER)
    marker.update(population_state='POPULATED', notes='Mission/node authoring. quest-state.json: QuestState tracks and transitions lowered from the Quest definitions by ' + TOOL + '.')
    return {OUTPUT: compact(payload), MARKER: json.dumps(marker, ensure_ascii=False, indent=2) + '\n'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    files = expected(args.root)
    differences = []
    for relative, text in files.items():
        path = args.root / relative
        if args.check:
            if not path.is_file() or path.read_text(encoding='utf-8') != text:
                differences.append(relative)
        else:
            path.write_text(text, encoding='utf-8')
    if differences:
        parser.exit(1, 'QuestState lowering differences: ' + ', '.join(differences) + '\n')
    counts = json.loads(files[OUTPUT])['counts']
    print(f"QuestState lowering: {counts['quests']} quests, {counts['tracks']} tracks, {counts['transitions']} transitions; check={args.check}")
    return 0


if __name__ == '__main__':
    sys.exit(main())
