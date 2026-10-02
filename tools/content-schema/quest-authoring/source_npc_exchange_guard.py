"""Only the independently reviewed Pemaret track pair; no generic ignore rule."""
import copy
from source_fix_guard import digest

CORE = 'oteryn:quest.marlin_trophy_quest'
TRACK = 'canary:quest-progress/quest/u7_8/marlin_trophy'
OLD = '06ddee6964e811081de37689f4ff96432dc5b013f4c45f8d93614628db580517'
NEW = '24c2b1e4cd811a632189cd766b16cb8f4e356c9da2d7eac48dc3b7d7346ab352'


def reviewed_pair(old, new):
    if old['identity']['key'] != CORE or new['identity']['key'] != CORE:
        raise ValueError('NPC exchange belongs to another canonical owner')
    result = []
    snapshots = []
    for core, expected in ((old, OLD), (new, NEW)):
        tracks = [p for p in core['source_data']['progress'] if p['key'] == TRACK]
        if len(tracks) != 1 or digest(tracks[0]) != expected:
            raise ValueError('NPC exchange track/pins/effects/holds differ from reviewed snapshot')
        snapshots.append(tracks[0])
        result.append(copy.deepcopy(core))
    # Exact new->old substitution makes all other fields visible to the guard.
    result[1]['source_data']['progress'] = [copy.deepcopy(snapshots[0]) if p['key'] == TRACK else p
        for p in result[1]['source_data']['progress']]
    return tuple(result)
