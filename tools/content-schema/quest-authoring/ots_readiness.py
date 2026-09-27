#!/usr/bin/env python3
"""Quest readiness map: which engine features each transcribed quest needs.

Reads the committed samples (quest log, chests, doors, interactions) and writes
`samples/readiness/readiness.json`. Deterministic; no source access, no network.

A quest needs a feature when any record joined to it uses that feature:
- quest log: missions and journal (quest-state store), NPC transitions;
- chests: reward claims;
- doors: gates by kind;
- interactions: trigger edges and child owners. An interaction joins the quests
  whose progress tracks it reads or writes; an interaction touching no track joins
  the one quest its script directory already joins, if exactly one.

`data_gaps` counts what the transcription could not express yet, so a quest can
be ready on the engine side and still wait for data.
"""
import collections
import json
import pathlib

HERE = pathlib.Path(__file__).resolve().parent
SAMPLES = HERE / 'samples'
OUT = SAMPLES / 'readiness' / 'readiness.json'

# Engine feature -> the runtime owner it waits on (quest format §8 readiness table).
FEATURES = {
    'quest_state': 'quest-state store (D35)',
    'npc_dialogue': 'NPC dialogue runtime',
    'reward_claim': 'DUR-03 inventory mint and RewardClaim (D39-D42)',
    'door_key': 'door gate: key item',
    'door_level': 'door gate: level',
    'door_progress': 'door gate: quest progress',
    'trigger_use': 'GAME-INTERACTION-01 USE edge',
    'trigger_step': 'GAME-INTERACTION-01 step edges',
    'trigger_kill': 'GAME-INTERACTION-01 death/kill edges',
    'teleport': 'relocation owner (D37)',
    'world_object': 'world-object overlay (D38)',
    'summon': 'GAME-AI-01 spawn',
    'item_hand_out': 'DUR-03 inventory mint',
    'item_consume': 'DUR-03 consumption',
    'achievement': 'Achievement owner',
}
EDGE_FEATURE = {'USE': 'trigger_use', 'ON_ENTER': 'trigger_step', 'ON_LEAVE': 'trigger_step',
                'ON_CONTACT': 'trigger_step', 'ON_DEATH': 'trigger_kill', 'ON_KILL': 'trigger_kill'}
# A feature that cannot work without another one: a chest is used, and progress
# gates and NPC transitions read or write the quest-state store.
IMPLIES = {'reward_claim': {'trigger_use'}, 'door_progress': {'quest_state'},
           'npc_dialogue': {'quest_state'}}
DOOR_FEATURE = {'door_key': 'door_key', 'min_level': 'door_level', 'quest_progress': 'door_progress'}


def load(rel, key):
    return json.loads((SAMPLES / rel).read_text())[key]


def walk_rules(rules):
    """Yield ('child', child) and ('cond', condition) for every rule element."""
    for rule in rules:
        if 'branch' in rule:
            for arm in rule['branch']:
                yield from walk_condition(arm['when'])
                yield from walk_rules(arm['then'])
            yield from walk_rules(rule.get('otherwise', []))
        else:
            yield 'child', rule


def walk_condition(cond):
    if 'all' in cond or 'any' in cond:
        for sub in cond.get('all', cond.get('any')):
            yield from walk_condition(sub)
    else:
        yield 'cond', cond


def interaction_facts(inter):
    tracks, features, unresolved_conditions = set(), set(), 0
    features.add(EDGE_FEATURE[inter['source']['edge']])
    for kind, x in walk_rules(inter['rules']):
        if kind == 'cond':
            if 'quest_stage' in x:
                tracks.add(x['quest_stage']['progress'])
            if 'unresolved' in x:
                unresolved_conditions += 1
            continue
        owner = x['owner']
        if owner == 'Quest' and 'progress' in x:
            tracks.add(x['progress'])
        if owner == 'Quest':
            features.add('quest_state')
        elif owner == 'Movement':
            features.add('teleport')
        elif owner == 'WorldObject':
            features.add('world_object')
        elif owner == 'Ability':
            features.add('summon')
        elif owner == 'Item':
            features.add('item_hand_out' if x['request'] == 'hand_out' else 'item_consume')
        elif owner == 'Achievement':
            features.add('achievement')
    gaps = len(inter['unresolved']) + unresolved_conditions
    return tracks, features, gaps


def main():
    quests = load('questlog/quests.json', 'quests')
    progress = load('questlog/progress.json', 'progress')
    claims = load('chests/claims.json', 'claims')
    gates = load('doors/gates.json', 'gates')
    interactions = load('interactions/interactions.json', 'interactions')

    info = {q['identity']['key']: {'kind': q['kind'], 'features': set(), 'interactions': [],
                                   'unresolved': 0, 'interactions_with_gaps': 0}
            for q in quests}

    track_quests = collections.defaultdict(set)
    for q in quests:
        key = q['identity']['key']
        if q.get('start'):
            track_quests[q['start']['progress']].add(key)
        for mission in q.get('missions') or []:
            track_quests[mission['progress']].add(key)
            info[key]['features'].add('quest_state')
            if any(t['key'].startswith('npc_') for t in mission.get('transitions', [])):
                info[key]['features'].add('npc_dialogue')
        if q.get('claims'):
            info[key]['features'].add('reward_claim')
    for track in progress:
        for key in track.get('auxiliary_of') or []:
            track_quests[track['key']].add(key)
        for key in track['start_of']:
            track_quests[track['key']].add(key)
        if any(t['key'].startswith('npc_') for t in track['transitions']):
            for key in track_quests[track['key']]:
                info[key]['features'].add('npc_dialogue')
    for claim in claims:
        if claim['quest'] and claim['quest']['key'] in info:
            info[claim['quest']['key']]['features'].add('reward_claim')
    for gate in gates:
        if gate['quest'] and gate['quest']['key'] in info:
            info[gate['quest']['key']]['features'].add(DOOR_FEATURE[gate['condition']['kind']])

    facts = {}
    by_directory = collections.defaultdict(set)
    for inter in interactions:
        key = inter['identity']['key']
        tracks, features, gaps = interaction_facts(inter)
        joined = set().union(*(track_quests.get(t, set()) for t in tracks)) & info.keys()
        facts[key] = (joined, features, gaps)
        by_directory[key.split('/')[1]].update(joined)
    unlinked = []
    for key, (joined, features, gaps) in sorted(facts.items()):
        if not joined:
            candidates = by_directory[key.split('/')[1]]
            joined = candidates if len(candidates) == 1 else set()
        if not joined:
            unlinked.append(key)
            continue
        for quest in joined:
            entry = info[quest]
            entry['features'] |= features
            entry['interactions'].append(key)
            entry['unresolved'] += gaps
            entry['interactions_with_gaps'] += bool(gaps)

    rows = []
    for key in sorted(info):
        e = info[key]
        for feature in list(e['features']):
            e['features'] |= IMPLIES.get(feature, set())
        rows.append({'quest': key, 'kind': e['kind'], 'features': sorted(e['features']),
                     'interactions': len(e['interactions']),
                     'data_gaps': {'interactions_with_gaps': e['interactions_with_gaps'],
                                   'unresolved_items': e['unresolved']}})

    # Greedy unlock order: at each step build the feature that completes most quests.
    need = {r['quest']: set(r['features']) for r in rows}
    built, order = set(), []
    while any(need[q] - built for q in need):
        def gain(feature):
            return sum(1 for q in need if need[q] - built and need[q] <= built | {feature})
        remaining = sorted({f for q in need for f in need[q] - built})
        best = max(remaining, key=lambda f: (gain(f), sum(f in need[q] for q in need), f))
        built.add(best)
        playable = [q for q in need if need[q] <= built]
        order.append({'feature': best, 'owner': FEATURES[best], 'quests_complete_on_engine_side': len(playable),
                      'of_which_without_data_gaps': sum(1 for r in rows if r['quest'] in playable
                                                        and not r['data_gaps']['unresolved_items'])})

    usage = collections.Counter(f for r in rows for f in r['features'])
    report = {
        'classification': 'DERIVED from committed samples; no runtime authority',
        'features': FEATURES,
        'counts': {
            'quests': len(rows),
            'by_kind': dict(sorted(collections.Counter(r['kind'] for r in rows).items())),
            'quests_needing_feature': dict(sorted(usage.items())),
            'quests_without_data_gaps': sum(1 for r in rows if not r['data_gaps']['unresolved_items']),
            'interactions_joined': len(facts) - len(unlinked),
            'interactions_unlinked': len(unlinked),
        },
        'unlock_order': order,
        'quests': rows,
        'interactions_unlinked': unlinked,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=1, ensure_ascii=False) + '\n')
    print(json.dumps(report['counts'], indent=1))
    for step in order:
        print(step['feature'], step['quests_complete_on_engine_side'], step['of_which_without_data_gaps'])


if __name__ == '__main__':
    main()
