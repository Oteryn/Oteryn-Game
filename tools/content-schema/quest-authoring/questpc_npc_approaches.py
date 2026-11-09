"""Chosen room approaches from retained NPC position evidence, without promoting NPC placement."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'


def resolve(root, quest, reference):
    if (quest, reference) != ('oteryn:quest.mad_mage_room_quest',
                              'npc-approach:oteryn:npc.a_prisoner'):
        raise ValueError('NPC approach is not declared for this exact quest')
    rows = json.loads((Path(root) / SOURCE).read_text())['candidates']
    matches = [(i,r) for i,r in enumerate(rows)
               if r['identity']['key'] == 'oteryn:npc.a_prisoner']
    if len(matches) != 1 or len(matches[0][1]['placements']) != 1:
        raise ValueError('NPC approach needs one retained position record')
    index, record = matches[0]
    at = record['placements'][0]['position']
    if at != {'x':32394,'y':32138,'z':13}:
        raise ValueError('Pinned prison position evidence changed')
    wiki = record['wiki']
    if wiki != {'pageid':5361,'revid':1114111}:
        raise ValueError('Pinned prisoner wiki reference changed')
    # The retained2184 cell door is at32395,32138,z13. Remain outside it.
    return (dict(x=32396,y=32138,floor=-13),408,
            [SOURCE + '#/candidates/' + str(index),
             'https://tibia.fandom.com/index.php?curid=5361&oldid=1114111',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
