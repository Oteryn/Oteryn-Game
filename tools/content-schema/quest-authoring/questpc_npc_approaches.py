"""Chosen room approaches from retained NPC position evidence, without promoting NPC placement."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
SITES = {
    'oteryn:quest.mad_mage_room_quest': ('oteryn:npc.a_prisoner',
        (32394,32138,13),(5361,1114111),(32396,32138,-13,408)),
    'oteryn:quest.hero_of_rathleton_quest': ('oteryn:npc.strange_pipe',
        (33533,31950,14),(69693,1116848),(33533,31951,-14,20545)),
}


def resolve(root, quest, reference):
    site = SITES.get(quest)
    if site is None or reference != 'npc-approach:' + site[0]:
        raise ValueError('NPC approach is not declared for this exact quest')
    npc, (ox,oy,oz), (page,revision), (x,y,floor,appearance) = site
    rows = json.loads((Path(root) / SOURCE).read_text())['candidates']
    matches = [(i,r) for i,r in enumerate(rows) if r['identity']['key'] == npc]
    if len(matches) != 1 or len(matches[0][1]['placements']) != 1:
        raise ValueError('NPC approach needs one retained position record')
    index, record = matches[0]
    if record['placements'][0]['position'] != {'x':ox,'y':oy,'z':oz}:
        raise ValueError('Pinned NPC position evidence changed')
    if record['wiki'] != {'pageid':page,'revid':revision}:
        raise ValueError('Pinned NPC wiki reference changed')
    if floor != -oz or not 1 <= abs(x-ox)+abs(y-oy) <= 2:
        raise ValueError('NPC approach must remain close on its source floor')
    return (dict(x=x,y=y,floor=floor),appearance,
            [SOURCE + '#/candidates/' + str(index),
             f'https://tibia.fandom.com/index.php?curid={page}&oldid={revision}',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
