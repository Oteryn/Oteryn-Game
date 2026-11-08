"""Selected Cook contacts: retained NPC cells and one exact empty spirit container."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.authored.20_years_a_cook_quest'
TRACK = 'oteryn:quest-progress/cook-twenty-years/stage'


def extend_rewards(callbacks, greetings):
    path = 'content/quests/definitions/quests-00200-00299.json'
    raw = (ROOT / path).read_bytes()
    definition = json.loads(raw)['records'][84]['definition']
    recipe = definition['recipe']
    if (definition['identity']['key'] != QUEST
            or not recipe['requirements']['premium']
            or recipe['repeat']['kind'] != 'once'
            or [(s['kind'], s['count']) for s in recipe['stages']] != [
                ('talk', 1), ('talk', 1), ('explore', 1), ('use', 3),
                ('use', 1), ('explore', 1), ('complete', 1)]):
        raise ValueError('COOK_SELECTED_RECIPE_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/84/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    for i, (npc, reply) in enumerate([
            ('oteryn:npc.cheaty', 'The Draccoon needs your help. Meet him at his Venore lair.'),
            ('oteryn:npc.the_draccoon_npc', 'Find the Rest of Ratha in Kazordoon. This empty spirit container is for the three essences of the selected rescue.')]):
        greetings.append(dict(npc=npc, greet='Welcome. Ask about your mission.',
            farewell='Farewell.', walkaway='Return when you are ready.', sources=[source]))
        callbacks.append(dict(key=f'oteryn:quest-reward/cook-contact-{i}',
            quest=QUEST, npc=npc, keywords=['mission'], topic=0, next_topic=0,
            premium_required=True,
            storage=[dict(track=TRACK, minimum=i-1, maximum=i-1, set=i)],
            take=[], give=None if i == 0 else dict(item='oteryn:item.tibia.i44527', count=1, contents=[]),
            outfit=None, achievement=None, reply=[reply], sources=[source]))


def extend_world(rows):
    from qengine_access import callback
    import sys
    sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
    import world_region_codec
    region = 'content/world/placements/region-z07-x128-y125.b3'
    raw = (ROOT / region).read_bytes()
    palette = json.loads((ROOT / 'content/world/placements/index.json').read_bytes())['palette']
    cells = {(x, y): items for _, tiles in world_region_codec.decode_region(raw)[3]
             for x, y, flags, house, zones, items in tiles}
    if [palette[i[0]]['source_item_id'] for i in cells[(32974, 32110)]] != [4515, 44563]:
        raise ValueError('COOK_SOURCE_TRASHCAN_GEOMETRY_DRIFT')
    row = callback('authored-cook-venore-entry', 'authored.20_years_a_cook_quest',
        'use', 'selected-personal-programme:venore-ominous-trashcan',
        source_pin='oteryn-authored:', at=dict(x=32974, y=32110, floor=-7),
        appearance=44563, teleport=dict(x=32973, y=32079, floor=-8))
    row['premium_required'] = True
    row['storage'] = [dict(track=TRACK, minimum=0, maximum=7, set=0, preserve=True)]
    recipe_path = 'content/quests/definitions/quests-00200-00299.json'
    row['sources'] += [
        f'oteryn:selected-recipe:{recipe_path}:/records/84/definition:sha256:{hashlib.sha256((ROOT / recipe_path).read_bytes()).hexdigest()}',
        'https://www.tibiawiki.com.br/index.php?curid=61193&oldid=445567:42:sha256:983ebc56f56394cbacf6e69c4dfc41a0cb2489fdbc1a799bb5ffa09fa18f5f2f',
        f'oteryn:source-map:{region}:sha256:{hashlib.sha256(raw).hexdigest()}']
    rows.append(row)
