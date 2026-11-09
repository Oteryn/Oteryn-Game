"""Chosen approaches before source key doors, without opening them or granting keys."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/doors/gates.json'


def resolve(root, quest, reference, *, definition):
    if (quest,reference) != ('oteryn:quest.bear_room_quest',
                              'door-approach:canary:door-gate/key/4601'):
        raise ValueError('Door approach is not declared for this exact quest')
    rows = json.loads((Path(root)/SOURCE).read_text())['gates']
    matches = [(i,r) for i,r in enumerate(rows)
               if r['identity']['key'] == reference.removeprefix('door-approach:')]
    if len(matches) != 1:
        raise ValueError('Door approach needs one retained gate')
    index, gate = matches[0]
    condition = gate['condition']
    claim = 'canary:reward-claim/quest/key/id4601'
    if (definition['identity']['key'] != quest
            or claim not in [r['key'] for r in definition['source_refs']['claims']]
            or condition['kind'] != 'door_key'
            or condition['key_binding'] != 'canary:door-key/4601'
            or claim not in [r['key'] for r in condition['key_from_claims']]):
        raise ValueError('Door approach key must belong to the canonical quest')
    if gate['placements'] != [{'position':{'x':32145,'y':32100,'z':11},'appearance':None}]:
        raise ValueError('Pinned Bear Room gate placement changed')
    # Retained stone1791 occupies32145,32101; observe from the clear outer hall.
    return (dict(x=32145,y=32102,floor=-11),355,
            [SOURCE+'#/gates/'+str(index),
             'https://github.com/opentibiabr/canary/tree/04b83b51',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
