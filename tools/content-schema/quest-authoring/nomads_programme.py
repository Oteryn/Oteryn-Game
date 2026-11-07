"""Selected personal three-cache Nomads route; no global camp scheduling."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.authored.nomads_mini_world_change'
TRACK = 'oteryn:quest-progress/nomads_mini/stage'
CAMPS = [('ham', 33220, 32580), ('fish', 33264, 32818), ('fur-bag', 33207, 32610)]


def documents():
    from qengine_access import callback
    path = 'content/quests/definitions/quests-00300-00351.json'
    raw = (ROOT / path).read_bytes()
    definition, = [r['definition'] for r in json.loads(raw)['records'] if r['definition']['identity']['key'] == QUEST]
    recipe = definition['recipe']
    if len(recipe['stages']) != 6 or not recipe['requirements']['premium'] or recipe['repeat']['kind'] != 'once':
        raise ValueError('NOMADS_SELECTED_RECIPE_DRIFT')
    if [(r['name'], r['count']) for r in recipe['reward_intents']] != [('Ham', 10), ('Fish', 50), ('Fur Bag', 1), ('Small Emerald', 3), ('Black Pearl', 1), ('Small Topaz', 1), ('Silver Amulet', 1), ('Chest Robber', 1)]:
        raise ValueError('NOMADS_SELECTED_REWARDS_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/21/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    rows, objects = [], []
    for i, (name, x, y) in enumerate(CAMPS):
        region = f'content/world/placements/region-z07-x{x//256}-y{y//256}.b3'
        region_digest = hashlib.sha256((ROOT / region).read_bytes()).hexdigest()
        objects.append(dict(quest=QUEST, position=[x, y, 7], item=2472, ground=231, description="A fixed nomad supply cache.",
            recipe_path=path, recipe_sha256=hashlib.sha256(raw).hexdigest(),
            source_world=region, source_world_sha256=region_digest))
        for use in [False, True]:
            key = 'authored-nomads-' + name + ('-cache' if use else '-explore')
            before, after = 2*i-1+int(use), 2*i+int(use)
            row = callback(key, 'authored.nomads_mini_world_change', 'use' if use else 'step_in',
                'selected-personal-three-cache-route:' + name, source_pin='oteryn-authored:',
                at=dict(x=x, y=y if use else y-1, floor=-7), appearance=2472 if use else 231)
            row['storage'] = [dict(track=TRACK, minimum=before, maximum=before, set=after)]
            row['premium_required'] = True
            row['sources'] += [source, f'oteryn:source-map:{region}:sha256:{region_digest}']
            if use:
                if i == 0:
                    reward = dict(item='oteryn:item.tibia.i3582', count=10, contents=[])
                elif i == 1:
                    reward = dict(item='oteryn:item.tibia.i3578', count=50, contents=[])
                else:
                    reward = dict(item='oteryn:item.tibia.i7343', count=1, contents=[], backpack_outputs=[
                        dict(item=f'oteryn:item.tibia.i{item}', count=count) for item, count in [(3032, 3), (3027, 1), (9057, 1), (3054, 1)]])
                    row['effects']['achievement_reward'] = 'oteryn:achievement/chest_robber'
                    row['completes'] = True
                row['effects']['item_reward'] = reward
            rows.append(row)
    return rows, dict(schema=1, basis='CHOSEN_OTERYN_APPROXIMATION', objects=objects)
