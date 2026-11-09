"""Reviewed approaches to exact pinned script destinations absent from canonical Areas."""
import json
from pathlib import Path

QUEST = 'oteryn:quest.ferumbras_ascension_quest'
SOURCE_QUEST = 'canary:quest/ferumbras_ascension_quest'
URL = ('https://raw.githubusercontent.com/opentibiabr/canary/04b83b51/'
       'data-otservbr-global/scripts/quests/ferumbras_ascension/movements_entrance.lua')


def resolve(root, quest, reference):
    if (quest, reference) != (QUEST, 'source-approach:ferumbras-entrance'):
        raise ValueError('Source approach is not declared for this exact quest')
    definitions = [r['definition']
                   for p in (Path(root) / 'content/quests/definitions').glob('quests-*.json')
                   for r in json.loads(p.read_text())['records']
                   if r['definition']['identity']['key'] == quest]
    if (len(definitions) != 1
            or definitions[0]['source_refs']['quest']['key'] != SOURCE_QUEST):
        raise ValueError('Pinned source approach quest owner changed')
    # The level-150 portal's uncompleted destination is (33275,32390,8).
    # Its clear eastern neighbour preserves the source portal and native landing.
    return (dict(x=33276, y=32390, floor=-8), 410,
            [URL, 'OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only'])
