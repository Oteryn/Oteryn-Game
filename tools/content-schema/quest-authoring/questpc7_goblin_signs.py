"""Three source-backed, personal Goblin Merchant sign placements."""
import hashlib
import json
import sys

QUEST = 'oteryn:quest.goblin_merchant_quest'
TRACK = 'oteryn:quest-progress/questpc7/goblin/'
SITES = [(33055, 32091, -7), (33026, 32091, -7), (33032, 32083, -7)]
REGION = 'content/world/placements/region-z07-x129-y125.b3'
REGION_SHA = 'd1d87901c566bf7fa28034bc3364f123da93f51241b2a08e63c960ca7529597f'
WIKI = 'https://www.tibiawiki.com.br/index.php?title=Task:Rafzan&oldid=401277'
WIKI_SHA = 'f3e823bef7d74e3ee33fa1fd0e40886cabc776b570a90e91770da3c0c494166a'


def build(root):
    path = 'content/quests/definitions/quests-00100-00199.json'
    raw = (root / path).read_bytes()
    index, definition = next((i, r['definition']) for i, r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key'] == QUEST)
    stage = definition['oteryn_recipe']['payload']['recipe']['stages'][1]
    if (stage['kind'], stage['count']) != ('use', 3):
        raise ValueError('QUESTPC7_GOBLIN_SIGN_RECIPE_DRIFT')
    native = (root / REGION).read_bytes()
    if hashlib.sha256(native).hexdigest() != REGION_SHA:
        raise ValueError('QUESTPC7_GOBLIN_SIGN_NATIVE_REGION_DRIFT')
    sys.path.insert(0, str(root / 'tools/content-schema/world-authoring'))
    import world_region_codec as codec
    palette_path = 'content/world/placements/index.json'
    palette_raw = (root / palette_path).read_bytes()
    palette = json.loads(palette_raw)['palette']
    cells = {(x, y): (flags, house, [palette[i]['source_item_id'] for i, _, _ in items]) for _, tiles in codec.decode_region(native)[3] for x, y, flags, house, _, items in tiles}
    witnesses = [[16310, 15625, 17679], [4691, 15627, 17679], [15043, 16293, 17679]]
    if any(cells[(x, y)] != (0, 0, items) for (x, y, _), items in zip(SITES, witnesses)):
        raise ValueError('QUESTPC7_GOBLIN_SIGN_NATIVE_CELL_DRIFT')
    sources = [f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}', f'{REGION}:1:sha256:{REGION_SHA}', f'{palette_path}:1:sha256:{hashlib.sha256(palette_raw).hexdigest()}', f'{WIKI}:sha256:{WIKI_SHA}']
    callbacks = []
    for i, (x, y, floor) in enumerate(SITES, 1):
        storage = [dict(track=TRACK+'programme', minimum=0, maximum=6, set=0, preserve=True), dict(track=TRACK+'signs', minimum=0, maximum=2, set=3, add=1), dict(track=TRACK+f'sign-site-{i}', minimum=-1, maximum=-1, set=1)]
        callbacks.append(dict(key=f'oteryn:quest-callback/questpc7-goblin-sign-{i}', quest=QUEST, trigger='use', selector=dict(position=dict(x=x, y=y, floor=floor), appearance=17679, unique=None, action=None), lever=None, reversible=False, passages=[], teleport=None, storage=storage, activation=dict(track=TRACK+'signs', minimum=0, maximum=2), minimum_level=None, experience=None, monk_reward=None, offerings=[], item_on_target=dict(item='oteryn:item.tibia.i17668', count=1, replacement=None, target_replacement=None), return_on_failure=False, failure_message=None, requirements=dict(min_level=0, premium=False, completed_quests=[]), success_message='You put up one advertisement sign for Rafzan at this marked site.', sources=sources))
    return callbacks
