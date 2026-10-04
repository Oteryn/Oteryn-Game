"""Five sealed comment-only storage writes; preserve every other Source field."""
import copy
from source_fix_guard import digest

TRACKS = {'oteryn:quest.grave_danger_quest': {'old': [('canary:quest-progress/quest/u12_20/kilmaresh_quest/treze/presente', 'a7a6617eb0e83c07915cc68f7c1eee37be28f3543f89e330262044c820159935')], 'new': []}, 'oteryn:quest.kilmaresh_quest': {'old': [('crystalserver:quest-progress/kilmaresh/treze/presente', 'bcd8686f1173f1000dd81400d9c48affcf5886994e72a45141e23fa64157616c')], 'new': []}, 'oteryn:quest.rotten_blood_quest': {'old': [('crystalserver:quest-progress/storage/10126', '3068e348ddb1e3fbe6dc3d53ef8d173bdd20ece9c2b6a3f50e57f920c8c51dac')], 'new': [('crystalserver:quest-progress/storage/10126', '0cf8fcc697c80990839d1d30bb9eef287aba017f5793eeaa1980993068db8db3')]}}
CORES = set(TRACKS)
AFFECTED = ('canary:quest-progress/quest/u12_20/kilmaresh_quest/treze/presente', 'crystalserver:quest-progress/kilmaresh/treze/presente', 'crystalserver:quest-progress/storage/10126')

def reviewed_pair(old, new):
    key = old['identity']['key']
    if key not in CORES or new['identity']['key'] != key:
        raise ValueError('comment-write correction belongs to another owner')
    selected_old = [p for p in old['source_data']['progress'] if p['key'] in AFFECTED]
    selected_new = [p for p in new['source_data']['progress'] if p['key'] in AFFECTED]
    if selected_old == selected_new:
        return copy.deepcopy(old), copy.deepcopy(new)
    result = []
    for side, core in zip(('old', 'new'), (old, new)):
        core = copy.deepcopy(core)
        progress = core['source_data']['progress']
        selected = [(p['key'], digest(p)) for p in progress if p['key'] in AFFECTED]
        if selected != [tuple(x) for x in TRACKS[key][side]]:
            raise ValueError('comment-write track pins/effects/membership differ from sealed capsule')
        core['source_data']['progress'] = [p for p in progress if p['key'] not in AFFECTED]
        result.append(core)
    return tuple(result)
