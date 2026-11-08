"""Chosen approaches to the four pinned Koshei fragment placements."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/chests/claims.json'
PREFIX = 'canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_amulet_part'
APPROACHES = {
    PREFIX + '1': (33195, 32458, -7, 408),
    PREFIX + '2': (33304, 32277, -10, 410),
    PREFIX + '3': (33211, 32593, -7, 231),
    PREFIX + '4': (33054, 32468, -11, 409),
}


def resolve(root, quest, reference):
    key = reference.removeprefix('claim:')
    records = json.loads((Path(root) / SOURCE).read_text())['claims']
    matches = [(index, claim) for index, claim in enumerate(records)
               if claim['identity']['key'] == key]
    if len(matches) != 1 or key not in APPROACHES:
        raise ValueError('Fragment approach needs one explicitly selected claim: ' + key)
    index, claim = matches[0]
    if claim['quest']['key'] != 'canary:quest/' + quest.removeprefix('oteryn:quest.'):
        raise ValueError('Fragment claim must belong to the same canonical quest')
    if len(claim['placements']) != 1:
        raise ValueError('Fragment approach needs one source placement')
    origin = claim['placements'][0]['position']
    x, y, floor, appearance = APPROACHES[key]
    if floor != -origin['z'] or abs(x - origin['x']) + abs(y - origin['y']) != 1:
        raise ValueError('Fragment approach must be adjacent to its source placement')
    return (dict(x=x, y=y, floor=floor), appearance,
            [SOURCE + '#/claims/' + str(index),
             'https://github.com/opentibiabr/canary/tree/04b83b51',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
