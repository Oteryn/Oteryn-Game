"""Chosen approaches before source key doors, without opening them or granting keys."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/doors/gates.json'
SITES = {
    'oteryn:quest.bear_room_quest': ('4601',(32145,32100,11),(32145,32102,-11,355)),
    'oteryn:quest.ghoul_room_quest': ('3600',(32506,32175,14),(32507,32175,-14,409)),
}


def resolve(root, quest, reference, *, definition):
    site = SITES.get(quest)
    if site is None or reference != 'door-approach:canary:door-gate/key/' + site[0]:
        raise ValueError('Door approach is not declared for this exact quest')
    key, (ox,oy,oz), (x,y,floor,appearance) = site
    rows = json.loads((Path(root)/SOURCE).read_text())['gates']
    matches = [(i,r) for i,r in enumerate(rows)
               if r['identity']['key'] == reference.removeprefix('door-approach:')]
    if len(matches) != 1:
        raise ValueError('Door approach needs one retained gate')
    index, gate = matches[0]
    condition = gate['condition']
    claim = 'canary:reward-claim/quest/key/id' + key
    if (definition['identity']['key'] != quest
            or claim not in [r['key'] for r in definition['source_refs']['claims']]
            or condition['kind'] != 'door_key'
            or condition['key_binding'] != 'canary:door-key/' + key
            or claim not in [r['key'] for r in condition['key_from_claims']]):
        raise ValueError('Door approach key must belong to the canonical quest')
    if gate['placements'] != [{'position':{'x':ox,'y':oy,'z':oz},'appearance':None}]:
        raise ValueError('Pinned source gate placement changed')
    if floor != -oz or not 1 <= abs(x-ox)+abs(y-oy) <= 2:
        raise ValueError('Door approach must remain close on the source floor')
    return (dict(x=x,y=y,floor=floor),appearance,
            [SOURCE+'#/gates/'+str(index),
             'https://github.com/opentibiabr/canary/tree/04b83b51',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
