"""Explicit clear approaches beside canonical Area markers; source records stay intact."""
import json
from pathlib import Path

SOURCE = 'content/world/areas/hunting-places/hunting-places-00000-00444.json'
APPROACHES = {
    'oteryn:area.hunting_place.rookgaard_bear_cave': ((32147, 32208, -7), (32147, 32207, -7, 4409)),
    'oteryn:area.hunting_place.edron_dragon_lair': ((33098, 31701, -7), (33098, 31703, -7, 4408)),
    'oteryn:area.hunting_place.shadowthorn': ((33075, 32170, -7), (33075, 32169, -7, 408)),
    'oteryn:area.hunting_place.helheim': ((32478, 31179, -7), (32479, 31179, -7, 6593)),
}
DISTANCES = {'oteryn:area.hunting_place.edron_dragon_lair': 2}


def resolve(root, reference):
    key = reference.removeprefix('area-approach:')
    records = json.loads((Path(root) / SOURCE).read_text())['records']
    matches = [(index, record) for index, record in enumerate(records)
               if record['declaration']['identity']['key'] == key]
    if len(matches) != 1 or key not in APPROACHES:
        raise ValueError('Area approach needs one explicitly selected canonical Area: ' + key)
    index, record = matches[0]
    source = record['declaration']['position']
    origin, (x, y, floor, appearance) = APPROACHES[key]
    if source is None or (source['x'], source['y'], -source['floor']) != origin:
        raise ValueError('Pinned Area marker changed: ' + key)
    if floor != origin[2] or abs(x - origin[0]) + abs(y - origin[1]) != DISTANCES.get(key, 1):
        raise ValueError('Chosen Area approach must match its declared distance')
    bindings = [b for b in record['source_bindings']
                if b['identity_namespace'] == 'tibiawiki-fandom/page-id'
                and b['disposition'] == 'EXACT']
    if len(bindings) != 1:
        raise ValueError('Area approach needs one exact pinned source: ' + key)
    binding, = bindings
    url = ('https://tibia.fandom.com/index.php?curid=' + binding['external_id']
           + '&oldid=' + binding['source_revision'])
    return (dict(x=x, y=y, floor=floor), appearance,
            [SOURCE + '#/records/' + str(index), url,
             'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
