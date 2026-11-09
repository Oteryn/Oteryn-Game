"""Chosen quest collector delivery through the existing fenced NPC reward owner."""
import argparse
import hashlib
import json
from pathlib import Path
from questpc3_native_entries import chosen

ROOT = Path(__file__).resolve().parents[3]
PILGRIMAGE = 'oteryn:quest.pilgrimage_of_ashes'
OUTPUT = Path('content/quests/authored-bindings/qmech-m01c-pilgrimage.json')


def npc_sources(root, keys):
    found = {}
    for path in sorted((root / 'content/npcs/definitions').glob('npcs-*.json')):
        for record in json.loads(path.read_bytes()).get('records', []):
            key = record.get('declaration', {}).get('identity', {}).get('key')
            if key in keys:
                if key in found:
                    raise ValueError('M01C_COLLECTOR_DUPLICATE_NPC')
                found[key] = path.relative_to(root).as_posix() + ':' + key
    if set(found) != set(keys):
        raise ValueError('M01C_COLLECTOR_NATIVE_NPC_MISSING')
    return found


def build_pilgrimage(root=ROOT):
    path, recipe = chosen(root, PILGRIMAGE, ('s1', 'talk', 1, ['Guide NPC']), premium=True)
    expected = [('talk', ['Guide NPC']), ('talk', ['Norf']), ('talk', ['Humphrey']),
                ('talk', ['Edala']), ('talk', ['Kawill']), ('talk', ['Pydar']),
                ('talk', ['Eremo']), ('complete', ['Guide NPC'])]
    if [(s['kind'], s['targets']) for s in recipe['stages']] != expected or any(s['count'] != 1 for s in recipe['stages']):
        raise ValueError('M01C_PILGRIMAGE_ROUTE_DRIFT')
    rewards = recipe['reward_intents']
    if len(rewards) != 1 or (rewards[0]['kind'], rewards[0]['count'], rewards[0]['item']['key']) != ('item', 20, 'oteryn:item.tibia.i3035'):
        raise ValueError('M01C_PILGRIMAGE_REWARD_DRIFT')
    npcs = ['guide_jonathan', 'norf', 'humphrey', 'edala', 'kawill', 'pydar', 'eremo', 'guide_jonathan']
    npcs = ['oteryn:npc.' + name for name in npcs]
    evidence = npc_sources(root, npcs)
    track = 'oteryn:quest-progress/m01c-pilgrimage-route'
    stages = []
    for index, npc in enumerate(npcs):
        final = index == 7
        stage_key = recipe['stages'][index]['key']
        sources = [path + ':' + stage_key, evidence[npc]]
        callback = {'key': 'oteryn:quest-reward/m01c-pilgrimage-' + stage_key,
                    'quest': PILGRIMAGE, 'npc': npc, 'keywords': ['pilgrimage'],
                    'topic': 0, 'next_topic': 0, 'completes': final, 'requires_premium': True,
                    'storage': [{'track': track, 'minimum': index, 'maximum': index, 'set': index + 1}],
                    'take': [], 'give': {'item': 'oteryn:item.tibia.i3035', 'count': 20, 'contents': [], 'premium': True} if final else None,
                    'outfit': None, 'achievement': None,
                    'reply': ['Your pilgrimage is complete. Here are twenty platinum coins.' if final else 'Your visit has been recorded. Continue your pilgrimage.'],
                    'sources': sources}
        stages.append({'key': stage_key, 'completes': final,
                       'trigger': {'kind': 'npc_dialogue', 'callback': callback}})
    raw = json.dumps(recipe, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    return {'schema': 1, 'quests': [{'quest': PILGRIMAGE,
            'recipe_sha256': hashlib.sha256(raw).hexdigest(),
            'admission': {'minimum_level': 0, 'premium': True},
            'tracks': [{'key': track, 'initial': 0, 'minimum': 0, 'maximum': 8}],
            'stages': stages, 'sources': [path, *sorted(evidence.values())]}]}


def content(check=False):
    expected = build_pilgrimage()
    path = ROOT / OUTPUT
    if check:
        if not path.is_file() or json.loads(path.read_bytes()) != expected:
            raise ValueError('M01C_COLLECTOR_BINDING_DRIFT')
    else:
        path.write_text(json.dumps(expected, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n')
    print('PASS M01c Pilgrimage collector: ordered visits, one final 20-platinum grant')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['content'])
    parser.add_argument('--check', action='store_true')
    content(parser.parse_args().check)
