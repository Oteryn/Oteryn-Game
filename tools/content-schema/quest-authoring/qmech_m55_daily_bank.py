"""Generate the chosen personal daily Bank Robbery dialogue/death/custody route.

This binds mission delivery, not a physical world-change activation or an encounter
spawn. Jesse's recovery encounter ingress remains a separate map authoring task.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
INPUT = 'content/quests/definitions/quests-00200-00299.json'
OUTPUT = 'content/quests/authored-bindings/qmech-m55-daily-bank.json'
QUEST = 'oteryn:quest.authored.bank_robbery_mini_world_change'
PHASE = 'oteryn:quest-progress/qm55/bank/phase'
KILLS = 'oteryn:quest-progress/qm55/bank/kills'
SOURCES = [INPUT + '#/records/88/definition/recipe',
           'CHOSEN_OTERYN_APPROXIMATION: Naji is the canonical Thais bank service NPC; personal server-save scope does not disable banks or delete retained bags.']


def storage(track, minimum, maximum, value):
    return dict(track=track, minimum=minimum, maximum=maximum, set=value)


def callback(key, keywords, topic, next_topic, state, reply):
    return dict(key=key, quest=QUEST, npc='oteryn:npc.naji', keywords=keywords,
                topic=topic, next_topic=next_topic, storage=state, take=[], give=None,
                outfit=None, achievement=None, reply=[reply], sources=SOURCES)


def generate(root: Path):
    definition = json.loads((root / INPUT).read_text())['records'][88]['definition']
    if definition['identity']['key'] != QUEST:
        raise ValueError('Bank recipe source moved')
    recipe = definition['recipe']
    if recipe['repeat']['kind'] != 'daily' or [(s['key'], s['kind'], s['count']) for s in recipe['stages']] != [('s1', 'talk', 1), ('s2', 'kill', 1), ('s3', 'collect', 1), ('s4', 'complete', 1)]:
        raise ValueError('Bank recipe changed; requalify explicit native bindings')
    if (recipe['requirements']['min_level'] != 0 or recipe['requirements']['premium']
            or recipe['requirements']['prerequisites']
            or [s['targets'] for s in recipe['stages']] != [['Thais banker'], ['Jesse the Wicked'], ['Bag With Stolen Gold'], ['Thais banker']]
            or recipe['stages'][2]['target_items'][0]['item']['key'] != 'oteryn:item.tibia.i13429'
            or [(r['kind'], r['count'], r['name']) for r in recipe['reward_intents']] != [('item', 10, 'Platinum Coin'), ('achievement', 1, 'Honest Finder')]
            or recipe['reward_intents'][0]['item']['key'] != 'oteryn:item.tibia.i3035'):
        raise ValueError('Bank requirements/targets/rewards changed; requalify native delivery')
    reset = callback('oteryn:quest-transition/save-event-restart/authored.bank_robbery_mini_world_change',
                     ['robbery'], 0, 55055,
                     [storage(PHASE, 0, 4, 0), storage(KILLS, 0, 1, 0)],
                     'Would you help recover the bank\'s stolen gold today?')
    start = callback('oteryn:quest-reward/qm55-bank-start', ['yes'], 55055, 0,
                     [storage(PHASE, 0, 0, 1)],
                     'Find Jesse the Wicked and recover one bag of stolen gold. Bring the stolen gold back to me.')
    finish = callback('oteryn:quest-reward/qm55-bank-return', ['stolen gold'], 0, 0,
                      [storage(PHASE, 3, 3, 4)], 'The stolen gold is back in the bank. Here is your reward.')
    finish.update(take=[dict(item='oteryn:item.tibia.i13429', count=1)],
                  give=dict(item='oteryn:item.tibia.i3035', count=10),
                  achievement='oteryn:achievement/honest_finder')
    return dict(schema=1, quests=[dict(
        quest=QUEST, recipe_sha256=hashlib.sha256(json.dumps(recipe, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest(),
        sources=SOURCES,
        tracks=[dict(key=PHASE, initial=0, minimum=0, maximum=4), dict(key=KILLS, initial=0, minimum=0, maximum=1)],
        occurrence=dict(kind='server_save', world_change='server_save', restart=reset),
        stages=[dict(key='s1', completes=False, trigger=dict(kind='npc_dialogue', callback=start)),
                dict(key='s2', completes=False, trigger=dict(kind='kill', counter=dict(
                    key='oteryn:quest-transition/native-kills/qm55-bank-jesse', creature='oteryn:creature.jesse_the_wicked',
                    track=KILLS, gate=PHASE, gate_value=1, maximum=1, advance_gate_to=2))),
                dict(key='s3', completes=False, trigger=dict(kind='collect', collection=dict(
                    key='oteryn:quest-collection/qm55-bank-stolen-gold',
                    items=[dict(item='oteryn:item.tibia.i13429', count=1)], storage=[storage(PHASE, 2, 2, 3)], sources=SOURCES))),
                dict(key='s4', completes=True, trigger=dict(kind='npc_dialogue', callback=finish))])])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    target = ROOT / OUTPUT
    expected = (json.dumps(generate(ROOT), sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
    if args.check:
        if not target.is_file() or target.read_bytes() != expected:
            raise ValueError('Stale daily Bank Robbery bindings')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(expected)
    print('PASS M55: daily Bank Robbery NPC/death/custody bindings')


if __name__ == '__main__':
    main()
