"""Personal rat-urine application at the retained Rafzan pond."""
import hashlib
import json
import sys


def build(root):
    region = 'content/world/placements/region-z10-x129-y125.b3'
    raw = (root / region).read_bytes()
    digest = '49f46bde1d2fd6d8e3083ba3aea1e02bd60b2199efe086d3760178f8a7e348a1'
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError('QUESTPC7_GOBLIN_POND_REGION_DRIFT')
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    import world_region_codec as codec
    palette = json.loads((root / 'content/world/placements/index.json').read_bytes())['palette']
    cells = [(flags, house, [palette[i]['source_item_id'] for i, _, _ in items])
             for _, tiles in codec.decode_region(raw)[3] for x, y, flags, house, _, items in tiles
             if (x, y) == (33134, 32093)]
    if cells != [(0, 0, [9527, 16299])]:
        raise ValueError('QUESTPC7_GOBLIN_POND_CELL_DRIFT')
    path = 'content/quests/definitions/quests-00100-00199.json'
    canonical = (root / path).read_bytes()
    index, definition = next((i, r['definition']) for i, r in enumerate(json.loads(canonical)['records'])
                             if r['definition']['identity']['key'] == 'oteryn:quest.goblin_merchant_quest')
    stage = definition['oteryn_recipe']['payload']['recipe']['stages'][4]
    if (stage['kind'], stage['count']) != ('use', 1):
        raise ValueError('QUESTPC7_GOBLIN_POND_RECIPE_DRIFT')
    track = 'oteryn:quest-progress/questpc7/goblin/'
    return [dict(key='oteryn:quest-callback/questpc7-goblin-pond', quest=definition['identity']['key'],
        trigger='use', selector=dict(position=dict(x=33134, y=32093, floor=-10), appearance=9527, unique=None, action=None),
        lever=None, reversible=False, passages=[], teleport=None,
        storage=[dict(track=track+'programme', minimum=0, maximum=6, set=0, preserve=True),
                 dict(track=track+'urine', minimum=0, maximum=0, set=1)],
        minimum_level=None, experience=None, monk_reward=None, offerings=[],
        item_on_target=dict(item='oteryn:item.tibia.i17671', count=1, replacement=None, target_replacement=None),
        return_on_failure=False, failure_message=None,
        success_message='You apply the rat urine to the marked pond for Rafzan.',
        sources=[f'{region}:1:sha256:{digest}',
                 f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(canonical).hexdigest()}',
                 'https://www.tibiawiki.com.br/index.php?title=Task:Rafzan&oldid=401277'])]
