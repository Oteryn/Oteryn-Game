"""Source-bounded Rookie Guard multi-output grant; no journey completion claim.

Vascalir's mission 11 acceptance, Crystal 9f5a72c, grants both Items after the
second yes. The separate map mark remains outside this item participant.
"""
import json
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TRACK = 'oteryn:quest-progress/quest/u9_1/the_rookie_guard/'
SOURCE = ('https://raw.githubusercontent.com/zimbadev/crystalserver/'
          '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/vascalir.lua')


def outputs():
    storage = [
        dict(track=TRACK+'mission10', minimum=3, maximum=3, set=3, preserve=True),
        dict(track=TRACK+'mission11', minimum=-1, maximum=-1, set=1),
    ]
    callback = dict(
        key='oteryn:quest-reward/m09-rookie-mission11',
        quest='oteryn:quest.the_rookie_guard_quest', npc='oteryn:npc.vascalir',
        topic=91, next_topic=0, keywords=['yes'], storage=storage, take=[],
        give=dict(item='oteryn:item.tibia.i3054', count=1, contents=[],
                  additional=[dict(item='oteryn:item.tibia.i12785', count=1)]),
        outfit=None, achievement=None,
        reply=['Alright. Here is the empty flask to use on a dead wasp. '
               'Be careful and wear your silver amulet for poison protection!'],
        sources=[SOURCE, 'docs/reference/content-prep/qmech-m09/reward-sources.md'])
    offer = dict(npc='oteryn:npc.vascalir', keywords=['yes'], topic=0,
                 next_topic=91, storage=[dict(s, preserve=True, set=0) for s in storage],
                 reply=['I can give you a silver amulet and a flask to collect wasp '
                        'poison. Are you prepared for that mission?'])
    chests = []
    for label, x, y, item, count in [
            ('telescope', 32144, 32968, 9616, 1),
            ('ingots', 32139, 32978, 9058, 3)]:
        # Both cells and their appearance are measured from the admitted map.
        # The item-to-cell allocation is explicitly an Oteryn choice; see notes.
        chests.append(dict(
            key='oteryn:quest-callback/m09-treasure-'+label,
            quest='oteryn:quest.treasure_hunt_quest', trigger='use',
            selector=dict(position=dict(x=x, y=y, floor=-13),
                          unique=None, action=None, appearance=2478),
            lever=None, reversible=False, passages=[], teleport=None,
            storage=[dict(track='oteryn:quest-progress/m09/treasure/'+label,
                          minimum=-1, maximum=0, set=1)],
            offerings=[], item_on_target=None, return_on_failure=False,
            failure_message=None, success_message='You found the hidden treasure.',
            sources=['https://www.tibiawiki.com.br/index.php?title=Treasure_Hunt_Quest&oldid=431833#Recompensa',
                     'docs/reference/content-prep/qmech-m09/reward-sources.md'],
            effects=dict(item_reward=dict(item=f'oteryn:item.tibia.i{item}',
                                          count=count, contents=[]))))
    return {
        'content/quests/npc-callbacks/m09-rookie.json':
            dict(schema=1, callbacks=[callback]),
        'content/quests/npc-offers/m09-rookie.json': dict(schema=1, offers=[offer]),
        'content/quests/world-callbacks/m09-treasure.json':
            dict(schema=1, callbacks=chests),
        'content/dialogues/quests/m09-rookie-greetings.json': dict(schema=1, npcs=[dict(
            npc='oteryn:npc.vascalir', greet='Greetings, |PLAYERNAME|.',
            farewell='Good bye, |PLAYERNAME|.', walkaway='Good bye.',
            sources=[f'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:'
                     f'data/npclib/npc_system/npc_handler.lua:{line}:sha256:'
                     '8479e2aa7904915ef23bcbc50eef3ac9c9a8b2588cc16dc8713f33fa0cfec35a'
                     for line in (81, 82, 83)])]),
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for path, value in outputs().items():
        expected = json.dumps(value, indent=2)+'\n'
        if args.check:
            if (ROOT/path).read_text() != expected:
                raise SystemExit('M09 generated reward differs: '+path)
        else:
            (ROOT/path).write_text(expected)
