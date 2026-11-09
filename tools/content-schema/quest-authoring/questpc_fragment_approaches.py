"""Chosen clear approaches to explicitly selected quest reward placements."""
import json
from pathlib import Path

SOURCE = 'tools/content-schema/quest-authoring/samples/chests/claims.json'
PREFIX = 'canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_amulet_part'
APPROACHES = {
    'canary:reward-claim/quest/u8_0/shardsof_ancient_winters/inukaya': (32484, 31109, -8, 6684),
    'canary:reward-claim/quest/u7_3/the_medusa_quest': (33050, 32398, -10, 410),
    'canary:reward-claim/quest/key/id3800': (32605, 31907, -3, 436),
    'canary:reward-claim/quest/u8_0/fishing_box': (32016, 31409, -7, 6587),
    'canary:reward-claim/quest/u6_4/berserker_treasure/rewards/white_pearls': (33198, 31923, -11, 353),
    'canary:reward-claim/kv/minohell1': (32131, 32066, -12, 351),
    'canary:reward-claim/quest/key/id4603': (32177, 32132, -9, 355),
    'canary:reward-claim/kv/goblintemple': (31973, 32210, -12, 502),
    'canary:reward-claim/kv/ironhelmet': (32770, 32225, -7, 4516),
    'canary:reward-claim/quest/u8_0/shardsof_ancient_winters/okolnir': (32146, 31431, -8, 4407),
    PREFIX + '1': (33195, 32458, -7, 408),
    PREFIX + '2': (33304, 32277, -10, 410),
    PREFIX + '3': (33211, 32593, -7, 231),
    PREFIX + '4': (33054, 32468, -11, 409),
}

DISTANCES = {
    'canary:reward-claim/quest/u8_0/shardsof_ancient_winters/inukaya': 2,
    'canary:reward-claim/quest/u7_3/the_medusa_quest': 2,
}


def resolve(root, quest, reference, *, definition=None):
    key = reference.removeprefix('claim:')
    records = json.loads((Path(root) / SOURCE).read_text())['claims']
    matches = [(index, claim) for index, claim in enumerate(records)
               if claim['identity']['key'] == key]
    if len(matches) != 1 or key not in APPROACHES:
        raise ValueError('Fragment approach needs one explicitly selected claim: ' + key)
    index, claim = matches[0]
    definitions = ([definition] if definition is not None else [row['definition']
                   for path in (Path(root) / 'content/quests/definitions').glob('quests-*.json')
                   for row in json.loads(path.read_text())['records']
                   if row['definition']['identity']['key'] == quest])
    refs = (definitions[0].get('source_refs', {})
            if len(definitions) == 1 and definitions[0]['identity']['key'] == quest else {})
    if (not isinstance(refs, dict) or claim['quest'] != refs.get('quest')
            or not any(ref['key'] == key for ref in refs.get('claims', []))):
        raise ValueError('Fragment claim must belong to the same canonical quest')
    if len(claim['placements']) != 1:
        raise ValueError('Fragment approach needs one source placement')
    origin = claim['placements'][0]['position']
    x, y, floor, appearance = APPROACHES[key]
    distance = DISTANCES.get(key, 1)
    if not 1 <= distance <= 2 or floor != -origin['z'] or abs(x - origin['x']) + abs(y - origin['y']) != distance:
        raise ValueError('Fragment approach must match its declared distance on the source floor')
    return (dict(x=x, y=y, floor=floor), appearance,
            [SOURCE + '#/claims/' + str(index),
             'https://github.com/opentibiabr/canary/tree/04b83b51',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
