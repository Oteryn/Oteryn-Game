"""Explicit Oteryn adjacent-cell choices; retained donor coordinates remain evidence."""
import hashlib
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'world-authoring'))
import world_region_codec as codec


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
    if before != actual_source or before[2] != after[2] or sum(abs(a-b) for a,b in zip(before[:2], after[:2])) != 1:
        raise ValueError('chosen placement must adjoin its exact retained source cell')
    if after in occupied or not isinstance(choice['reason'], str) or not 1 <= len(choice['reason']) <= 512:
        raise ValueError('chosen placement target occupied or decision missing')
    x, y, z = after
    path = f'content/world/placements/region-z{z:02}-x{x//256:03}-y{y//256:03}.b3'
    raw = (root / path).read_bytes()
    matches = [t for _, sector in codec.decode_region(raw)[3] for t in sector if t[:2] == (x,y)]
    if len(matches) != 1 or matches[0][2] or matches[0][3] or len(matches[0][5]) != 1:
        raise ValueError('chosen placement target is not clear native ground')
    return after, {'origin':'oteryn', 'basis':'CHOSEN_OTERYN_APPROXIMATION',
                   'source_cell':{'x':before[0],'y':before[1],'floor':before[2]},
                   'cell':{'x':x,'y':y,'floor':z}, 'decision':choice['reason'],
                   'map':{'path':path,'sha256':hashlib.sha256(raw).hexdigest()}}
