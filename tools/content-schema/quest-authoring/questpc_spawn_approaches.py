"""Chosen clear entries beside explicitly retained quest creature spawns."""
import json
from pathlib import Path

SOURCE = 'content/world/spawns/spawns-42000-43999.json'
FRAME = 'global-target-2026-09-27'
ANCHORS = {
    'oteryn:spawn.x33230_y31696_z12#3': (
        SOURCE, 'oteryn:quest.behemoth_quest', 'oteryn:creature.behemoth',
        (33234, 31695, -12), (33235, 31695, -12, 499)),
    'oteryn:spawn.x32590_y31645_z5#0': (
        'content/world/spawns/spawns-02000-03999.json',
        'oteryn:quest.authored.elvenbane_quest', 'oteryn:creature.dragon',
        (32590, 31645, -5), (32591, 31645, -5, 436)),
}


def resolve(root, quest, reference):
    key = reference.removeprefix('spawn-approach:')
    if key not in ANCHORS:
        raise ValueError('Spawn approach needs an explicitly selected source point')
    source, owner, creature, origin, (x, y, floor, appearance) = ANCHORS[key]
    if quest != owner:
        raise ValueError('Spawn approach must belong to the same quest binding')
    identity, ordinal = key.rsplit('#', 1)
    document = json.loads((Path(root) / source).read_text())
    if document.get('coordinate_frame') != FRAME:
        raise ValueError('Pinned spawn coordinate frame changed')
    records = document['records']
    matches = [(index, row['declaration']) for index, row in enumerate(records)
               if row['declaration']['identity']['key'] == identity]
    if len(matches) != 1:
        raise ValueError('Spawn approach needs one retained declaration')
    index, declaration = matches[0]
    point = declaration['points'][int(ordinal)]
    cell = point['cell']
    if point['creature'] != creature or (cell['x'], cell['y'], -cell['floor']) != origin:
        raise ValueError('Pinned source creature or spawn position changed')
    if floor != origin[2] or abs(x - origin[0]) + abs(y - origin[1]) != 1:
        raise ValueError('Chosen spawn approach must be immediately adjacent')
    return (dict(x=x, y=y, floor=floor), appearance,
            [source + '#/records/' + str(index) + '/declaration/points/' + ordinal,
             f'content/world/placements/region-z{-floor:02}-x{x // 256}-y{y // 256}.b3',
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
