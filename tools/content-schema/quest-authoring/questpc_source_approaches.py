"""Reviewed approaches to exact pinned script destinations absent from canonical Areas."""
import json
from pathlib import Path
from questpc_grave_approaches import SITES as GRAVE_SITES, DISTANCES

QUEST = 'oteryn:quest.ferumbras_ascension_quest'
BASE = ('https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/'
        'data-otservbr-global/scripts/quests/')
SITES = {
    'hero-into-the-fire': ('oteryn:quest.hero_of_rathleton_quest',
        'canary:quest/hero_of_rathleton_quest', 'hero_of_rathleton/movements_lava.lua',
        (33552, 32049, -15), (33553, 32049, -15, 20537)),
    'deeplings-navigator-hub': ('oteryn:quest.deeplings_worldchange',
        'canary:quest/deeplings_worldchange', 'deeplings_worldchange/actions_golden_anchor.lua',
        (33640, 31379, -9), (33641, 31379, -9, 8287)),
    'ferumbras-entrance': (QUEST, 'canary:quest/ferumbras_ascension_quest',
        'ferumbras_ascension/movements_entrance.lua',
        (33275, 32390, -8), (33276, 32390, -8, 410)),
    'heart-zao-breach': ('oteryn:quest.heart_of_destruction_quest',
        'canary:quest/heart_of_destruction_quest', 'heart_of_destruction/movements_teleport.lua',
        (32117, 31355, -13), (32118, 31355, -13, 23054)),
}

SITES.update(GRAVE_SITES)


def resolve(root, quest, reference, *, definition=None):
    site_key = reference.removeprefix('source-approach:')
    site = (SITES.get(site_key)
            if reference.startswith('source-approach:') else None)
    if site is None or site[0] != quest:
        raise ValueError('Source approach is not declared for this exact quest')
    _, source_quest, script, origin, (x, y, floor, appearance) = site
    definitions = ([definition] if definition is not None else [r['definition']
                   for p in (Path(root) / 'content/quests/definitions').glob('quests-*.json')
                   for r in json.loads(p.read_text())['records']
                   if r['definition']['identity']['key'] == quest])
    if (len(definitions) != 1
            or definitions[0]['identity']['key'] != quest
            or definitions[0]['source_refs']['quest']['key'] != source_quest
            or definitions[0]['source_refs']['quest']['revision']
                != 'canary-04b83b51+crystalserver-9f5a72c6'):
        raise ValueError('Pinned source approach quest owner changed')
    distance = DISTANCES.get(site_key, 1)
    if not 1 <= distance <= 4 or floor != origin[2] or abs(x - origin[0]) + abs(y - origin[1]) != distance:
        raise ValueError('Source approach must match its declared distance and original floor')
    return (dict(x=x, y=y, floor=floor), appearance,
            [BASE + script, 'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
