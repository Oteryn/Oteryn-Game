"""Only two exact Kilmaresh association snapshots; never ignore arbitrary graphs."""
import copy
from source_fix_guard import digest

GRAVE = 'oteryn:quest.grave_danger_quest'
KILMARESH = 'oteryn:quest.kilmaresh_quest'
CORES = {GRAVE, KILMARESH}
TRACKS = {
    'canary:quest-progress/quest/u12_20/kilmaresh_quest/sixth/favor': (
        '8bb9069117f83ea9b7d39c09b6764958747890c9a7732b6d67d30531bd0d6f55',
        '08b52b14e971f210226324b829816da3ba50650863293048989b8ff4f5dca73e'),
    'canary:quest-progress/quest/u12_20/kilmaresh_quest/sixth/four_masks': (
        'e76532827fe149c24d4d14c8f3842761e5eb04169cf93e469c335b7375424fb1',
        '17b32b6c48705fae4de1b400b3e104e90e20b9a867b470b043d6e9e440f5ed29'),
}
GRAPH = 'canary:interaction/kilmaresh_quest/1_fafnars_wrath/7_four_masks'
OLD_GRAPH = '33edcb07c0ac59be9cc5aaa384386c04548967e55c254d24c474f048b68ba483'
PARENT_GRAPH = '62375c1d029ee5d3349931e2bf7a18a330c820807629bac8d980fae5049f9a42'


def reviewed_pair(old, new):
    return _pair(old, new, OLD_GRAPH)


def reviewed_parent_pair(old, new):
    """Fresh R13 delta only; cumulative acceptance still requires original R9."""
    return _pair(old, new, PARENT_GRAPH)


def _pair(old, new, graph_digest):
    key = old['identity']['key']
    if key not in CORES or new['identity']['key'] != key:
        raise ValueError('Kilmaresh correction belongs to another owner')
    result = []
    for side, core in enumerate((old, new)):
        core = copy.deepcopy(core)
        source = core['source_data']
        selected = [p for p in source['progress'] if p['key'] in TRACKS]
        expected = [(k, pair[side]) for k, pair in TRACKS.items()] if (
            (key == GRAVE and side == 0) or (key == KILMARESH and side == 1)) else []
        if [(p['key'], digest(p)) for p in selected] != expected:
            raise ValueError('Kilmaresh track membership/effects/pins differ from reviewed pair')
        source['progress'] = [p for p in source['progress'] if p['key'] not in TRACKS]
        if key == GRAVE:
            graphs = [g for g in source['interactions'] if g['identity']['key'] == GRAPH]
            if (side == 0 and (len(graphs) != 1 or digest(graphs[0]) != graph_digest)) or (side == 1 and graphs):
                raise ValueError('only the exact original Grave FourMasks association can be removed')
            source['interactions'] = [g for g in source['interactions'] if g['identity']['key'] != GRAPH]
        result.append(core)
    # Kilmaresh graphs and all other core fields remain visible to standard guards.
    return tuple(result)
