"""Chosen adjacent approach points, anchored to an existing same-quest source entrance."""
import json
from pathlib import Path

SOURCE = 'content/quests/world-callbacks.json'
MARKER = 'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'
APPROACHES = {
    'oteryn:quest-callback/alawars-vault-entrance': (32187, 31623, -8, 409),
}


def resolve(root, quest, reference):
    key = reference.removeprefix('callback:')
    records = json.loads((Path(root) / SOURCE).read_text())['callbacks']
    matches = [(index, callback) for index, callback in enumerate(records)
               if callback['key'] == key]
    if len(matches) != 1:
        raise ValueError('Approach needs one pinned source callback: ' + key)
    index, source = matches[0]
    if source['quest'] != quest or source['trigger'] != 'step_in':
        raise ValueError('Approach source must be the same quest entry: ' + key)
    origin = source['selector']['position']
    if origin is None or key not in APPROACHES:
        raise ValueError('Approach needs explicit source and selected coordinates: ' + key)
    x, y, floor, appearance = APPROACHES[key]
    if (floor != origin['floor'] or abs(x - origin['x']) + abs(y - origin['y']) != 1):
        raise ValueError('Chosen approach must be immediately adjacent to its entry')
    return (dict(x=x, y=y, floor=floor), appearance,
            [SOURCE + '#/callbacks/' + str(index), *source['sources'], MARKER])
