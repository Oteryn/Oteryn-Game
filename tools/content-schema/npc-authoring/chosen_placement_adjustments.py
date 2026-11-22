"""Explicit Oteryn adjacent-cell choices; retained donor coordinates remain evidence."""
import hashlib
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'world-authoring'))
import world_region_codec as codec

MAX_MISSING_TILE_DISTANCE = 4


def ground_item(root, palette_index):
    """Walkable native ground: a terrain of kind ground that is known walkable."""
    if not hasattr(ground_item, 'cache'):
        palette = json.loads((root / 'content/world/placements/index.json').read_bytes())['palette']
        grounds = set()
        for path in sorted((root / 'content/world/terrain').glob('terrain-*.json')):
            for row in json.loads(path.read_bytes())['records']:
                if (row['kind'].get('value') == 'ground'
                        and row.get('walkable') == {'state': 'KNOWN', 'value': True}):
                    grounds.add(row['identity']['key'].rsplit('.i', 1)[1])
        ground_item.cache = {index for index, entry in enumerate(palette)
                             if str(entry['source_item_id']) in grounds}
    return palette_index in ground_item.cache


def choose_cell(root, actual_source, choice, occupied):
    if set(choice) != {'source', 'target', 'reason'}:
        raise ValueError('chosen placement requires source, target and reason')
    def point(value):
        if set(value) != {'x', 'y', 'z'} or any(type(v) is not int for v in value.values()):
            raise ValueError('invalid chosen placement coordinates')
        x, y, z = value['x'], value['y'], value['z']
        if not (0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= z <= 15):
            raise ValueError('chosen placement outside native frame')
        return x, y, z
    before, after = point(choice['source']), point(choice['target'])
    if before != actual_source or before[2] != after[2] or after in occupied:
        raise ValueError('chosen placement must adjoin its exact retained source cell')
    if not isinstance(choice['reason'], str) or not 1 <= len(choice['reason']) <= 512:
        raise ValueError('chosen placement target occupied or decision missing')
    x, y, z = after
    path = f'content/world/placements/region-z{z:02}-x{x//256:03}-y{y//256:03}.b3'
    raw = (root / path).read_bytes()
    tiles = {t[:2]: t for _, sector in codec.decode_region(raw)[3] for t in sector}
    def clear(cell, walkable=False):
        tile = tiles.get(cell)
        return (tile is not None and not tile[2] and not tile[3] and len(tile[5]) == 1
                and (not walkable or ground_item(root, tile[5][0][0])))
    distance = sum(abs(a-b) for a, b in zip(before[:2], after[:2]))
    if distance != 1:
        # A source cell with no native tile at all may move to the uniquely nearest
        # clear ground of its own region and floor, never past a nearer clear cell.
        if (before[:2] in tiles or (before[0]//256, before[1]//256) != (x//256, y//256)
                or not 1 < distance <= MAX_MISSING_TILE_DISTANCE
                or not clear((x, y), walkable=True)
                or any(clear(cell, walkable=True) for cell in tiles
                       if cell != (x, y)
                       and abs(cell[0]-before[0]) + abs(cell[1]-before[1]) <= distance)):
            raise ValueError('chosen placement must adjoin its exact retained source cell')
    if not clear((x, y)):
        raise ValueError('chosen placement target is not clear native ground')
    return after, {'origin':'oteryn', 'basis':'CHOSEN_OTERYN_APPROXIMATION',
                   'source_cell':{'x':before[0],'y':before[1],'floor':before[2]},
                   'cell':{'x':x,'y':y,'floor':z}, 'decision':choice['reason'],
                   'map':{'path':path,'sha256':hashlib.sha256(raw).hexdigest()}}
