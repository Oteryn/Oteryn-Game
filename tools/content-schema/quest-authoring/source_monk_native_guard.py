"""Exact owner-approved Monk annotation pair; all SOURCE fields remain visible."""
import copy
from source_fix_guard import DIRECTORY, read_json

CORE = 'oteryn:quest.the_way_of_the_monk_quest'
EVIDENCE = DIRECTORY + 'source-fix-evidence/monk-native-lowering.json'


def reviewed_pair(root, old, new):
    if root is None:
        raise ValueError('Monk annotation proof requires repository root')
    packet = read_json(root / EVIDENCE)
    if packet['key'] != CORE or old['identity']['key'] != CORE or new['identity']['key'] != CORE:
        raise ValueError('Monk annotation belongs to another owner')
    for side, core in (('old', old), ('new', new)):
        if core['native_lowering'] != packet[side + '_native_lowering']:
            raise ValueError('Monk annotation differs from reviewed snapshot')
    result = copy.deepcopy(new)
    result['native_lowering'] = copy.deepcopy(old['native_lowering'])
    return copy.deepcopy(old), result
