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
  the quest its script directory explicitly names before inferring ownership
  from cross-quest progress tracks. Explicit directory and track owners are
  retained together for a script that touches both quests.

`data_gaps` counts what the transcription could not express yet, so a quest can
be ready on the engine side and still wait for data. An unresolved line that names
a missing runtime owner (key-value writes, conditions, boss cooldowns, creature
removal, delayed callbacks) counts as a needed feature, not as a data gap.
"""
import collections
import functools
import json
import pathlib
import re

from ots_chests import SOURCES

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
    'outfit': 'Outfit grant owner',
    'mount': 'Mount grant owner',
    'experience': 'Character XP award from an interaction',
    'kv_state': 'owner for key-value writes (none yet)',
    'condition': 'owner for player conditions (none yet)',
    'boss_cooldown': 'boss cooldown owner (none yet)',
    'creature_removal': 'owner for creature removal (none yet)',
    'scheduler': 'owner for delayed callbacks (none yet)',
}
# Unresolved reasons that name a missing runtime owner rather than a transcription gap.
REASON_FEATURE = {
    'kv write without an accepted owner': 'kv_state',
    'condition without an accepted owner': 'condition',
    'boss cooldown without an accepted owner': 'boss_cooldown',
    'creature removal without an accepted owner': 'creature_removal',
    'delayed callback (addEvent) without a scheduler owner': 'scheduler',
}
OWNER_FEATURE = {'Movement': 'teleport', 'WorldObject': 'world_object', 'Ability': 'summon',
                 'Achievement': 'achievement', 'Outfit': 'outfit', 'Mount': 'mount', 'Experience': 'experience'}
# Script directories whose name differs from their catalogue quest key beyond punctuation.
DIRECTORY_QUESTS = {
    'the_order_of_lion': 'canary:quest/the_order_of_the_lion_quest',
    'alawars_vault': 'canary:quest/alawar_s_vault_quest',
    'fathers_burden': 'canary:quest/a_father_s_burden_quest',
    'mintwallin_quest': 'canary:quest/mintwallin_cyclops_quest',
    'thieves_guild': 'canary:quest/the_thieves_guild_quest',
}
EDGE_FEATURE = {'USE': 'trigger_use', 'ON_ENTER': 'trigger_step', 'ON_LEAVE': 'trigger_step',
                'ON_CONTACT': 'trigger_step', 'ON_DEATH': 'trigger_kill', 'ON_KILL': 'trigger_kill'}
# A feature that cannot work without another one: a chest is used, and progress
# gates and NPC transitions read or write the quest-state store.
IMPLIES = {'reward_claim': {'trigger_use'}, 'door_progress': {'quest_state'},
           'npc_dialogue': {'quest_state'}}
DOOR_FEATURE = {'door_key': 'door_key', 'min_level': 'door_level', 'quest_progress': 'door_progress'}


def normalized(name):
    """A quest key or script directory without punctuation and a trailing 'quest(s)'."""
    return re.sub(r'_?quests?$', '', re.sub(r'[^a-z0-9]+', '_', name)).strip('_')


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


def uncertain_progress_keys(progress):
    """A declared source node may still have unresolved semantics; aliases retain it."""
    keys = {p['key'] for p in progress}
    uncertain = {p['key'] for p in progress if p.get('source_checks')}
    while True:
        added = {p['key'] for p in progress if p.get('alias_of')
                 and (p['alias_of'] not in keys or p['alias_of'] in uncertain)} - uncertain
        if not added:
            return uncertain
        uncertain.update(added)


def interaction_facts(inter, declared_tracks=None, uncertain_tracks=None):
    uncertain_tracks = uncertain_tracks or set()
    tracks, features, unresolved_conditions = set(), set(), 0
    blocked_children = 0
    features.add(EDGE_FEATURE[inter['source']['edge']])
    for kind, x in walk_rules(inter['rules']):
        if kind == 'cond':
            if 'quest_stage' in x:
                tracks.add(x['quest_stage']['progress'])
                if declared_tracks is not None and (x['quest_stage']['progress'] not in declared_tracks or x['quest_stage']['progress'] in uncertain_tracks):
                    unresolved_conditions += 1
            if 'unresolved' in x:
                unresolved_conditions += 1
            continue
        blocked_children += x.get('status') == 'blocked'
        owner = x['owner']
        if owner == 'Quest' and 'progress' in x:
            tracks.add(x['progress'])
            if declared_tracks is not None and (x['progress'] not in declared_tracks or x['progress'] in uncertain_tracks):
                blocked_children += 1
        if owner == 'Quest':
            features.add('quest_state')
        elif owner == 'Item':
            features.add('item_hand_out' if x['request'] == 'hand_out' else 'item_consume')
        elif owner in OWNER_FEATURE:
            features.add(OWNER_FEATURE[owner])
    gaps = unresolved_conditions + blocked_children
    for item in inter['unresolved']:
        if item['reason'] in REASON_FEATURE:
            features.add(REASON_FEATURE[item['reason']])
        else:
            gaps += 1
    return tracks, features, gaps


@functools.lru_cache(maxsize=1)
def curated_link_inputs():
    """Committed finite callbacks, not a directory-wide ownership alias."""
    return (json.loads((HERE / 'script_quests.json').read_text())['quests'],
            json.loads((SAMPLES / 'interactions/manifest.json').read_text())['entries'])


def curated_interaction_owners(quests, interactions, entries=None, manifest_entries=None, strict=False):
    if entries is None or manifest_entries is None:
        default_entries, default_manifest = curated_link_inputs()
        entries = default_entries if entries is None else entries
        manifest_entries = default_manifest if manifest_entries is None else manifest_entries
    by_title = collections.defaultdict(list)
    for quest in quests:
        if quest.get('display_name'):
            by_title[quest['display_name']].append(quest['identity']['key'])
    known = {i['identity']['key'] for i in interactions}
    manifest = {row['destination']: row for row in manifest_entries}
    owners, seen = collections.defaultdict(set), set()
    for entry in entries:
        keys = entry.get('interaction_keys')
        if keys is None:
            continue
        if (not isinstance(keys, list) or not keys or any(not isinstance(k, str) for k in keys)
                or len(keys) != len(set(keys)) or entry['title'] in seen
                or not entry.get('coverage_gap') or not entry.get('coverage_scope', '').startswith('partial_')):
            raise ValueError('invalid finite interaction curation: ' + entry['title'])
        seen.add(entry['title'])
        targets = by_title[entry['title']]
        if len(targets) != 1:
            if strict or targets:
                raise ValueError('missing or ambiguous curated quest: ' + entry['title'])
            continue  # A projected conflict alternative may contain only one quest.
        for key in keys:
            if key not in known:
                if strict:
                    raise ValueError('stale curated interaction: ' + key)
                continue  # Conflict/readiness consumers can project one Interaction.
            row = manifest.get(key)
            witnesses = [e for e in entry.get('source_evidence', []) if e.get('interaction_key') == key]
            if not row or not witnesses:
                raise ValueError('missing curated callback provenance: ' + key)
            for witness in witnesses:
                matching = [source for source in row['sources']
                            if source['source'] == witness.get('source')
                            and source['path'] == witness.get('path')
                            and source['callback_line'] == witness.get('line')
                            and source['blob_sha1'] == witness.get('blob_sha1')]
                pin = SOURCES.get(witness.get('source'), {})
                if (not matching or witness.get('repository') != pin.get('repository')
                        or witness.get('revision') != pin.get('revision')
                        or witness.get('role') != 'executable'
                        or witness.get('classification') != 'OTS_HYPOTHESIS_ONLY'
                        or not re.fullmatch(r'[0-9a-f]{64}', witness.get('blob_sha256', ''))):
                    raise ValueError('stale curated callback provenance: ' + key)
            owners[key].add(targets[0])
    return owners


def join_interactions(quests, progress, interactions, curated_entries=None, manifest_entries=None):
    """Explicit directory ownership precedes inferred cross-track ownership.

    Keep direct track owners as well: a boss script may grant another quest's
    outfit while remaining part of its named quest. Ambiguous inference stays
    unlinked; it never selects an arbitrary owner.
    """
    curated_callbacks = curated_interaction_owners(quests, interactions, curated_entries, manifest_entries)
    quest_keys = {q['identity']['key'] for q in quests}
    track_quests, by_name = collections.defaultdict(set), collections.defaultdict(set)
    for q in quests:
        key = q['identity']['key']
        by_name[normalized(key.split('/', 1)[1])].add(key)
        if q.get('start'):
            track_quests[q['start']['progress']].add(key)
        for mission in q.get('missions') or []:
            track_quests[mission['progress']].add(key)
    for track in progress:
        for key in (track.get('auxiliary_of') or []) + track['start_of']:
            track_quests[track['key']].add(key)
    direct, by_directory = {}, collections.defaultdict(set)
    for inter in interactions:
        key = inter['identity']['key']
        tracks, _, _ = interaction_facts(inter)
        direct[key] = set().union(*(track_quests[t] for t in tracks)) & quest_keys
        by_directory[key.split('/')[1]].update(direct[key])
    joined = {}
    for key, owners in sorted(direct.items()):
        directory = key.split('/')[1]
        curated = {DIRECTORY_QUESTS[directory]} & quest_keys if directory in DIRECTORY_QUESTS else set()
        named = curated or by_name[normalized(directory)]
        explicit = named if len(named) == 1 else set()
        candidates = by_directory[directory]
        inferred = candidates if len(candidates) == 1 and not owners and not explicit else set()
        joined[key] = owners | explicit | inferred | curated_callbacks[key]
    return joined


def quest_coverage_holds(manifest, quest_keys):
    """NPC source presence does not imply its non-storage effects were converted."""
    holds, seen = collections.Counter(), set()
    for check in manifest.get('source_checks', {}).get('npc_only_quests', []):
        identity = (check['quest'], check['npc_source'])
        if check['quest'] not in quest_keys or identity in seen or not check.get('coverage_gap'):
            raise ValueError('stale, duplicate or unexplained NPC quest coverage hold')
        seen.add(identity)
        holds[check['quest']] += 1
    for check in manifest.get('source_checks', {}).get('script_coverage_holds', []):
        paths = check.get('source_paths')
        if not isinstance(paths, list) or not paths or any(not isinstance(p, str) or not p for p in paths):
            raise ValueError('script quest coverage hold needs exact source paths')
        identity = (check['quest'], tuple(paths))
        if check['quest'] not in quest_keys or identity in seen or not check.get('coverage_gap'):
            raise ValueError('stale, duplicate or unexplained script quest coverage hold')
        seen.add(identity)
        holds[check['quest']] += 1
    return holds


def interaction_conflict_holds(manifest, joined, quests=None, progress=None):
    holds, seen = collections.Counter(), set()
    for entry in manifest['entries']:
        if entry['status'] != 'conflict':
            continue
        key = entry['destination']
        if key not in joined or key in seen:
            raise ValueError('stale or duplicate interaction source conflict')
        seen.add(key)
        owners = set(joined[key])
        if quests is not None and progress is not None:
            for alternative in entry.get('conflict_alternatives', []):
                graph = alternative['interaction']
                # Source-only alternatives add coverage holds, not accepted effects.
                # Canonicalize only the join key; keep the original graph intact.
                projected = dict(graph, identity=dict(graph['identity'], key=key))
                owners.update(join_interactions(quests, progress, [projected])[key])
        holds.update(owners)
    return holds


def main():
    quests = load('questlog/quests.json', 'quests')
    progress = load('questlog/progress.json', 'progress')
    claims = load('chests/claims.json', 'claims')
    gates = load('doors/gates.json', 'gates')
    interactions = load('interactions/interactions.json', 'interactions')

    info = {q['identity']['key']: {'kind': q['kind'], 'features': set(), 'interactions': [],
                                   'unresolved': 0, 'interactions_with_gaps': 0}
            for q in quests}
    manifest = json.loads((SAMPLES / 'questlog/manifest.json').read_text())
    for key, count in quest_coverage_holds(manifest, set(info)).items():
        info[key]['unresolved'] += count

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

    curated_interaction_owners(quests, interactions, strict=True)
    joined = join_interactions(quests, progress, interactions)
    for key, count in interaction_conflict_holds(
            json.loads((SAMPLES / 'interactions/manifest.json').read_text()), joined, quests, progress).items():
        info[key]['unresolved'] += count
    declared_tracks = {track['key'] for track in progress}
    uncertain_tracks = uncertain_progress_keys(progress)
    unlinked = []
    for inter in interactions:
        key = inter['identity']['key']
        _, features, gaps = interaction_facts(inter, declared_tracks, uncertain_tracks)
        if not joined[key]:
            unlinked.append(key)
            continue
        for quest in joined[key]:
            entry = info[quest]
            entry['features'] |= features
            entry['interactions'].append(key)
            entry['unresolved'] += gaps
            entry['interactions_with_gaps'] += bool(gaps)
    # A declared gate still needs transcription when its predicate track is absent.
    for gate in gates:
        if (gate.get('quest') and gate['quest']['key'] in info
                and gate['condition']['kind'] == 'quest_progress'
                and (gate['condition']['progress'] not in declared_tracks
                     or gate['condition']['progress'] in uncertain_tracks)):
            info[gate['quest']['key']]['unresolved'] += 1

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
            'interactions_joined': len(joined) - len(unlinked),
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
