"""Bind M24's selected kill steps to native deaths without changing their chosen counts."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import quest_state_lowering as lowering

ROOT = Path(__file__).resolve().parents[3]
INPUT = 'content/quests/missions/quest-state-completion-candidate.json'
OUTPUT = 'content/quests/native-death-bindings/'
HERALD_INPUT = 'content/quests/definitions/quests-00200-00299.json'
ALLIANCE_OUTPUT = 'qmech-m24-an_uneasy_alliance.json'
HERALD_OUTPUT = 'qmech-m24-targuna-herald.json'
HERALD_QUEST = 'oteryn:quest.targuna_quest'
HERALD_PREFIX = 'oteryn:quest-transition/crystalserver/targuna/herald-death/'
TARGETS = {
    'oteryn:quest.a_piece_of_cake': ('qmech-m24-a_piece_of_cake.json', {'s2': 'cake_golem'}),
    'oteryn:quest.the_ancient_tombs_quest': ('qmech-m24-ancient_tombs.json', {
        's2': 'omruc', 's3': 'thalas', 's5': 'dipthrah', 's6': 'mahrdis',
        's8': 'vashresamun', 's10': 'morguthis', 's12': 'rahemos'}),
}

def generate(root: Path):
    raw = (root / INPUT).read_bytes()
    quests = {q['quest']: q for q in json.loads(raw)['quests']}
    outputs = {}
    for key, (filename, stages) in TARGETS.items():
        quest = quests[key]
        tracks = {t['key']: t for t in quest['tracks']}
        used = set()
        rules = []
        for transition in quest['transitions']:
            stage = transition.get('source', {}).get('chosen_stage', {})
            if stage.get('key') not in stages:
                continue
            if stage['kind'] != 'kill' or stage['count'] != (20 if stages[stage['key']] == 'cake_golem' else 1):
                raise ValueError('M24 must preserve the selected kill stage/count: ' + key)
            effects = []
            guards = []
            for effect in transition['effects']:
                track = effect['track']
                used.add(track)
                op = effect['from']['op'].lower()
                if op not in ('eq', 'lt', 'any') or effect['effect']['kind'] not in ('SET', 'ADD'):
                    raise ValueError('Unqualified M24 stage effect')
                comparison = {'kind': op}
                if op != 'any':
                    comparison['value'] = effect['from']['value']
                operation = effect['effect']['kind'].lower()
                effects.append({'track': track, 'default': tracks[track]['initial'],
                                'from': comparison, operation: effect['effect']['value']})
                if operation == 'set':
                    guards.append(track)
            rules.append({'creature': 'oteryn:creature.' + stages[stage['key']],
                          'transition': transition['key'], 'define_transition': True,
                          'room': None, 'effects': effects, 'epoch_tracks': guards,
                          'sources': quest['completion']['recipe_metadata']['source_refs'],
                          'chosen_stage': stage})
        if len(rules) != len(stages):
            raise ValueError('Missing selected M24 kill stages: ' + key)
        outputs[filename] = {'schema': 1, 'runtime_enabled': True, 'quest': key,
            'scope': {'basis': 'CHOSEN_OTERYN_APPROXIMATION',
                      'native_consumer': 'quest_batch3_c::capture -> quest_kills::settle',
                      'note': 'Only selected kill stages. Does not admit other stages, rewards, or the owner-unavailable Cake bridge.'},
            'inputs': [{'path': INPUT, 'sha256': hashlib.sha256(raw).hexdigest()}],
            'tracks': [{field: tracks[name][field] for field in ('key', 'initial', 'min', 'max')} for name in sorted(used)],
            'rules': rules}
    outputs[ALLIANCE_OUTPUT] = renegade_orc_binding(root)
    outputs[HERALD_OUTPUT] = herald_binding(root)
    return outputs


def renegade_orc_binding(root: Path):
    quest_key = 'oteryn:quest.an_uneasy_alliance_quest'
    prefix = 'oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/'
    script = 'scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua'
    quest = next(q for q in lowering.read(root, lowering.OUTPUT)['quests'] if q['quest'] == quest_key)
    source = next(t for t in quest['transitions'] if t['source']['script'] == script)
    door = {'track': prefix + 'quest_door', 'from': {'op': 'EQ', 'value': 0},
            'from_exact': True, 'effect': {'kind': 'SET', 'value': 1}}
    lowering.require(source['effects'] == [door], 'Renegade Orc source death transition changed')
    defaults = {track['key']: track['initial'] for track in quest['tracks']}
    # An unassigned Crystal character has unset QuestDoor. Oteryn lowers its
    # default to zero, so Curos's Questline assignment disambiguates that zero.
    stage = prefix + 'questline'
    effects = [
        {'track': stage, 'default': defaults[stage], 'from': {'kind': 'eq', 'value': 1}, 'set': 1},
        {'track': door['track'], 'default': defaults[door['track']],
         'from': {'kind': 'eq', 'value': door['from']['value']}, 'set': door['effect']['value']},
    ]
    crystal = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/'
    return {'schema': 1, 'runtime_enabled': True, 'quest': quest_key, 'tracks': [],
            'rules': [{'creature': 'oteryn:creature.renegade_orc',
                       'transition': 'oteryn:quest-transition/an-uneasy-alliance/renegade-orc-death',
                       'define_transition': True, 'room': None, 'effects': effects,
                       'sources': [crystal + script + ':3-8', crystal + 'npc/curos.lua:81-85']}]}


def herald_binding(root: Path):
    # Reuse the closed source witness qualification; do not duplicate or relax it.
    records = [row['definition'] for row in lowering.read(root, HERALD_INPUT)['records']
               if row['definition']['identity']['key'] == HERALD_QUEST]
    lowering.require(len(records) == 1, 'Herald source definition missing or duplicated')
    quests = [lowering.lower_quest(records[0], lowering.owners(records),
                                  lowering.mission_values(records))]
    lowering.append_source_herald_grouped_transitions(quests, records)
    lowering.validate(quests)
    quest = quests[0]
    transitions = [t for t in quest['transitions'] if t['key'].startswith(HERALD_PREFIX)]
    committed = [t for q in lowering.read(root, lowering.OUTPUT)['quests']
                 if q['quest'] == HERALD_QUEST for t in q['transitions']
                 if t['key'].startswith(HERALD_PREFIX)]
    lowering.require(transitions == committed, 'Herald grouped runtime payload drift')
    tracks = {track['key']: track for track in quest['tracks']}
    sources = [witness for progress in records[0]['source_data']['progress']
               for transition in progress['transitions']
               if transition['script'] == transitions[0]['source']['script']
               for witness in transition['source_occurrences']]
    used = set()
    rules = []
    for transition in transitions:
        effects = []
        for effect in transition['effects']:
            track = effect['track']
            used.add(track)
            effects.append({'track': track, 'default': tracks[track]['initial'],
                            'from': {'kind': effect['from']['op'].lower(),
                                     'value': effect['from']['value']},
                            effect['effect']['kind'].lower(): effect['effect']['value']})
        rules.append({'creature': 'oteryn:creature.herald_of_fire',
                      'transition': transition['key'], 'define_transition': False,
                      'room': None, 'top_damage_only': True, 'effects': effects,
                      'sources': sources})
    return {'schema': 1, 'runtime_enabled': True, 'quest': HERALD_QUEST,
            'scope': {'basis': 'QUALIFIED_SOURCE_ONDEATH',
                      'native_consumer': 'quest_batch3_c::capture -> quest_kills::settle',
                      'note': 'Project-qualified present top-damage character only; not donor party credit. Exact grouped writes and source witnesses come from quest_state_lowering.'},
            'inputs': [{'path': path, 'sha256': lowering.digest(root, path)}
                       for path in (HERALD_INPUT, lowering.TOOL, lowering.OUTPUT)],
            'tracks': [{field: tracks[name][field] for field in ('key', 'initial', 'min', 'max')}
                       for name in sorted(used)],
            'rules': rules}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, value in generate(ROOT).items():
        target = ROOT / OUTPUT / name
        expected = (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()
        if args.check:
            if not target.is_file() or target.read_bytes() != expected:
                raise ValueError('Stale M24 death bindings: ' + name)
        else:
            target.write_bytes(expected)
    print('PASS M24: exact 20-Cake-Golem stage, seven pharaoh kill stages, qualified Herald death, and Renegade Orc damage credit')

if __name__ == '__main__':
    main()
