"""Generate M01c's source-pinned, noncompleting native entry bindings."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path('content/quests/authored-bindings/qmech-m01c-dead-archer.json')
QUEST = 'oteryn:quest.dead_archer_quest'
TRACK = 'oteryn:quest-progress/m01c-dead-archer-route'
ENTRANCE = (32493, 32259, 7)
WIKI = 'https://www.tibiawiki.com.br/index.php?curid=53187&oldid=433964'
BODY_SHA = 'baa2bccd9eb87e55245e538350a84a44db3c2696bb567ff5117d3d8dd25a5b06'


def chosen(root=ROOT, quest=QUEST, expected=('s1', 'use', 1, ['Shovel', 'Thais troll cave'])):
    matches = []
    for path in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        for record in json.loads(path.read_bytes())['records']:
            definition = record['definition']
            if definition['identity']['key'] == quest:
                matches.append((path.relative_to(root).as_posix(), definition))
    if len(matches) != 1:
        raise ValueError('M01C_DEAD_ARCHER_IDENTITY')
    path, definition = matches[0]
    selected = definition['oteryn_recipe']
    if selected['profile'] != 'chosen_source_completion_v1' or not selected['chosen_data_complete']:
        raise ValueError('M01C_DEAD_ARCHER_CHOSEN_RECIPE')
    recipe = selected['payload']['recipe']
    req = recipe['requirements']
    first = recipe['stages'][0]
    if (req['min_level'], req['premium'], req['prerequisites']) != (0, False, []):
        raise ValueError('M01C_DEAD_ARCHER_ACTOR_ADMISSION')
    if (first['key'], first['kind'], first['count'], first['targets']) != expected:
        raise ValueError('M01C_DEAD_ARCHER_FIRST_STEP')
    return path, recipe


def build(root=ROOT):
    path, recipe = chosen(root)
    raw = json.dumps(recipe, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    # Map format: x<<32 | y<<16 | Tibia depth<<8 | top-level ordinal; ground=0.
    x, y, depth = ENTRANCE
    key = (x << 32) | (y << 16) | (depth << 8)
    openings = {}
    for record in json.loads((root / 'content/craft/recipes.json').read_bytes())['records']:
        conversion = record['recipe']
        if conversion.get('tool') != 'shovel':
            continue
        used = conversion['input_item']
        if used['key'] not in ('oteryn:item.tibia.i3457', 'oteryn:item.tibia.i5710'):
            continue
        result = int(conversion['opened_target']['key'].rsplit('i', 1)[1])
        openings.setdefault(result, []).append({'key':used['key'], 'revision':used['revision']})
    if set(openings) != {594, 607, 609, 868}:
        raise ValueError('M01C_DEAD_ARCHER_PINNED_SHOVEL_RESULTS')
    stages = []
    for result, tools in sorted(openings.items()):
        stages.append({'key':f's1-shovel-{result}', 'completes':False,
            'trigger':{'kind':'terrain_use', 'usage':{
                'key':f'oteryn:quest-terrain-use/m01c-dead-archer-{result}',
                'tools':sorted(tools, key=lambda tool:tool['key']), 'placement_key':key, 'result':result,
                'storage':[{'track':TRACK,'minimum':0,'maximum':0,'set':1,'preserve':False}],
                'sources':[WIKI, f'wikitext-sha256:{BODY_SHA}', 'content/craft/recipes.json:shovel openings']}}})
    return {'schema':1,'quests':[{'quest':QUEST,'recipe_sha256':hashlib.sha256(raw).hexdigest(),
        'tracks':[{'key':TRACK,'initial':0,'minimum':0,'maximum':3}], 'stages':stages,
        'sources':[path+':s1', WIKI, 'docs/reference/content-prep/questpc3-m01c/dead-archer-sources.md']}]}


def build_fanfare(root=ROOT):
    quest = 'oteryn:quest.fanfare_quest'
    path, recipe = chosen(root, quest, ('s1', 'collect', 1, ['Key 3520', 'Carlin boat-side box']))
    source_path = root / 'tools/content-schema/quest-authoring/samples/chests/claims.json'
    claims = json.loads(source_path.read_bytes())
    # The accepted donor claim binds both the physical copper key and its number.
    def rewards(value):
        if isinstance(value, dict):
            if value.get('key_binding') == 'canary:door-key/3520':
                yield value
            for child in value.values():
                yield from rewards(child)
        elif isinstance(value, list):
            for child in value:
                yield from rewards(child)
    matches = list(rewards(claims))
    if len(matches) != 1 or [(r['item']['key'], r['count']) for r in matches[0]['items']] != [('canary:item/2973', 1)]:
        raise ValueError('M01C_FANFARE_KEY_SOURCE')
    raw = json.dumps(recipe, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    track = 'oteryn:quest-progress/m01c-fanfare-route'
    evidence = 'tools/content-schema/quest-authoring/samples/chests/claims.json:key_binding=canary:door-key/3520'
    return {'schema':1, 'quests':[{'quest':quest, 'recipe_sha256':hashlib.sha256(raw).hexdigest(),
        'tracks':[{'key':track,'initial':0,'minimum':0,'maximum':4}],
        'stages':[{'key':'s1-key-3520','completes':False,'trigger':{'kind':'collect','collection':{
            'key':'oteryn:quest-collection/m01c-fanfare-key',
            'items':[{'item':'oteryn:item.tibia.i2973','count':1,'key_number':3520}],
            'storage':[{'track':track,'minimum':0,'maximum':0,'set':1}],
            'sources':[evidence]}}}], 'sources':[path+':s1', evidence]}]}


def content(check=False):
    documents = [(OUTPUT, build()), (Path('content/quests/authored-bindings/qmech-m01c-fanfare.json'), build_fanfare())]
    for output, value in documents:
        path = ROOT / output
        if check:
            if not path.is_file() or json.loads(path.read_bytes()) != value:
                raise ValueError('M01C_NATIVE_ENTRY_BINDING_DRIFT')
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))+'\n')
    print('PASS M01c Dead Archer and Fanfare entries; completion/rewards remain held')



if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['content'])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    content(args.check)
