"""Reviewed first-survey approach to an independently admitted native Fury layer.

This does not admit Furyosa, cross the portal, or promote held catalogue content.
"""
import json
from pathlib import Path

QUEST = 'oteryn:quest.fury_gates_mini_world_change'
SOURCE = 'content/encounters/world-changes/layers.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'


def resolve(root, quest, reference, *, layers=None):
    if quest != QUEST or reference != 'fury-approach:thais':
        raise ValueError('Fury approach requires its exact reviewed quest and city')
    data = layers if layers is not None else json.loads((Path(root) / SOURCE).read_text())
    if data.get('source_revision') != REVISION:
        raise ValueError('Fury native layer revision changed')
    matches = [layer for layer in data['layers'] if layer['key'] == 'fury_gates/thais']
    if len(matches) != 1 or matches[0]['blob'] != 'a131dd8d0962a448e4e6a357d8eeb0d5fc951f78':
        raise ValueError('Fury native layer source owner changed')
    layer = matches[0]
    tiles = {tuple(tile['position']): tile for tile in layer['tiles']}
    if (tiles.get((32264, 32164, 7), {}).get('entries') !=
            [[7066, None, None], [6117, 9710, None], [777, None, None], [5064, None, None]]
            or tiles.get((32264, 32165, 7), {}).get('entries') != [[7066, None, None]]):
        raise ValueError('Fury portal or adjacent retained approach changed')
    return (dict(x=32264, y=32165, floor=-7), 7066,
            ['https://github.com/zimbadev/crystalserver/blob/' + REVISION + '/' + layer['source'],
             'OTERYN_CHOSEN_EXPLORE_ENTRY:native-layer-approach;first-cycle-only'],
            'fury_gates/thais')
