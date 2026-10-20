"""Build a NON-ADMITTED first-cycle input for the existing native QuestState reducer.

This proves independent boss flags and the all-five gate are expressible without a
new Quest engine. The unregistered sample is not source-equivalent gameplay:
identity/credit, NPC eligibility, expiry/reset, cooldowns and rewards stay unbound.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

KEY = 'oteryn:quest.soul_war_quest'
REVISION = 'soul-war-first-cycle-qualification-r1'
PACKET = 'tools/content-schema/quest-authoring/samples/soul-war-reconstruction/reconstruction.json'
OUTPUT = 'tools/content-schema/quest-authoring/samples/soul-war-native-progression/quest-state.json'
BOSSES = ('malice', 'greed', 'spite', 'cruelty', 'hatred')
TRACK_PREFIX = 'oteryn:quest-progress/soul-war-first-cycle/'
TRANSITION_PREFIX = 'oteryn:quest-transition/soul-war-first-cycle/'
HOLDS = (
    'QUALIFICATION_ONLY_NOT_INDEXED_OR_EMBEDDED',
    'NPC_START_ELIGIBILITY_AND_DISPATCH_UNBOUND',
    'CURRENT_ACTOR_AND_DAMAGE_PARTICIPATION_CREDIT_UNBOUND',
    'TAINT_EFFECTS_EXPIRY_RESET_AND_REPEAT_CYCLES_UNBOUND',
    'PER_BOSS_COOLDOWNS_AND_WORLD_ENTRY_UNBOUND',
    'MEGALOMANIA_OUTCOME_AND_ROOM_LIFECYCLE_UNBOUND',
    'REVENANT_OUTFIT_AND_RANDOM_ONE_OF_18_TRANSACTION_UNBOUND',
)


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode('utf-8')


def expected(root: Path):
    raw = (root / PACKET).read_bytes()
    packet = json.loads(raw)
    if packet['quest']['key'] != KEY or packet['runtime_activation'] is not False:
        raise ValueError('Soul War identity/runtime fence differs')
    if packet['source_policy']['primary_revision'] != '00ce02a57ca5a12e48f32a3476e37471167e4c3f':
        raise ValueError('Soul War primary donor differs')
    expected_bosses = {"Goshnar's " + name.title() for name in BOSSES}
    if (len(packet['progression']['mini_bosses']) != 5
            or set(packet['progression']['mini_bosses']) != expected_bosses
            or not packet['progression']['mini_boss_order'].startswith('ANY_ORDER_SOURCE_FLAGS')
            or packet['progression']['final_access'] != 'all five mini-boss completion flags are required'):
        raise ValueError('Soul War independent-boss/final-gate evidence differs')
    tracks = []
    for name, maximum in [('started', 1), *[(b, 1) for b in BOSSES],
                          ('taints', 5), ('first-taint-at', 2**63 - 1), ('final-access', 1), ('victory', 1)]:
        tracks.append({'key': TRACK_PREFIX + name, 'quest': KEY, 'initial': 0,
                       'min': 0, 'max': maximum, 'bounds_basis': 'BOUNDED_QUALIFICATION_PROFILE',
                       'source_key': 'qualification/soul-war-first-cycle/' + name})

    def effect(name, before, op='SET', value=None):
        result = {'track': TRACK_PREFIX + name, 'from': {'op': 'EQ', 'value': before},
                  'from_exact': True, 'effect': {'op': op}}
        if op not in {'SET_NOW', 'COMPUTED'}:
            result['effect']['value'] = before if value is None else value
        return result

    transitions = []

    def transition(name, effects, intent):
        if len(effects) > 8 or len({e['track'] for e in effects}) != len(effects):
            raise ValueError('native effect limit or duplicate track violated')
        transitions.append({'key': TRANSITION_PREFIX + name, 'quest': KEY,
                            'effects': effects, 'completes': False, 'requested_by': None,
                            'source': {'qualification_intent': intent, 'native_dispatch_binding': None}})

    transition('start', [effect('started', 0, value=1)], 'Already-authorized first-cycle start only')
    for boss in BOSSES:
        for rank in range(5):
            effects = [effect('started', 1), effect('final-access', 0),
                       effect(boss, 0, value=1), effect('taints', rank, 'ADD', 1)]
            if rank == 0:
                effects.append(effect('first-taint-at', 0, 'SET_NOW'))
            transition(f'{boss}/taint-{rank + 1}', effects,
                       'Already-qualified distinct boss credit; persistent writer must lock/recheck all comparisons')
    transition('final-access', [effect('started', 1), *[effect(b, 1) for b in BOSSES],
                               effect('taints', 5), effect('final-access', 0, value=1)],
               'All five flags in the same transition; this is not a world teleport')
    transition('victory', [effect('started', 1), effect('final-access', 1), effect('victory', 0, value=1)],
               'Already-qualified Megalomania outcome; no completion or reward granted')
    transition('finish', [effect('victory', 1, 'COMPUTED', 1)],
               'NOT_SUPPORTED until outfit/random-reward transaction and completion policy are bound')
    return {'schema': 'OTERYN_QUEST_STATE_LOWERING/v1',
            'classification': 'QUALIFICATION_ONLY_NOT_RUNTIME_AUTHORITY', 'family': 'Quest',
            'contract': {'existing_native_owner': 'QUEST-STATE-0', 'runtime_enabled': False,
                         'scope': 'FIRST_CYCLE_SCALAR_PROGRESS_ONLY', 'holds': list(HOLDS),
                         'does_not_replace': 'content/quests/missions/quest-state.json'},
            'authoring_sources': [{'path': PACKET, 'sha256': hashlib.sha256(raw).hexdigest()}],
            'counts': {'quests': 1, 'tracks': len(tracks), 'transitions': len(transitions),
                       'native_dispatch_bindings': 0, 'completes': 0},
            'quests': [{'quest': KEY, 'source_quest': packet['quest'],
                        'completion': {'state': 'QUALIFICATION_ONLY', 'runtime_enabled': False},
                        'tracks': tracks, 'transitions': transitions}]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = encoded(expected(args.root))
    target = args.root / OUTPUT
    if args.check:
        if not target.is_file() or target.read_bytes() != raw:
            raise ValueError('Soul War native qualification input is stale')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
    print('Soul War first-cycle qualification: 10 tracks, 29 transitions; native admission=false')


if __name__ == '__main__':
    main()
