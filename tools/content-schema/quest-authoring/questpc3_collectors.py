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



def build_stampede(root=ROOT):
    quest = 'oteryn:quest.stampede_mini_world_change'
    path, recipe = chosen(root, quest, ('s1', 'explore', 1, ['Tiquanda stampede grounds']), premium=True)
    if [(s['kind'], s['count'], s['targets']) for s in recipe['stages']] != [
            ('explore', 1, ['Tiquanda stampede grounds']),
            ('kill', 5, ['Terrified Elephant']), ('complete', 1, ['Tiquanda stampede grounds'])]:
        raise ValueError('M01C_STAMPEDE_ROUTE_DRIFT')
    if [(r['kind'], r['count'], r.get('name')) for r in recipe['reward_intents']] != [('achievement', 1, 'Trail of the Ape God')]:
        raise ValueError('M01C_STAMPEDE_REWARD_DRIFT')
    raids = json.loads((root / 'content/encounters/raids/catalogue.json').read_bytes())['records']
    raid = next(r for r in raids if r['key'] == 'raid:porthope.stampede')
    entry = raid['variants'][0]['waves'][0]['entries'][0]
    area = entry['area']
    if (entry['creature'], area['from'], area['to']) != (
            'oteryn:creature.terrified_elephant', {'x':32825,'y':32667,'floor':7}, {'x':32888,'y':32675,'floor':7}):
        raise ValueError('M01C_STAMPEDE_SOURCE_AREA_DRIFT')
    region = area['regions'][0]
    if hashlib.sha256((root / region['path']).read_bytes()).hexdigest() != region['sha256']:
        raise ValueError('M01C_STAMPEDE_SOURCE_REGION_DRIFT')
    achievement = 'oteryn:achievement/trail_of_the_ape_god'
    achievement_path = 'content/achievements/achievements-00500-00570.json'
    matches = [r for r in json.loads((root / achievement_path).read_bytes())['records'] if r['identity']['key'] == achievement]
    if len(matches) != 1 or matches[0]['name'] != 'Trail of the Ape God':
        raise ValueError('M01C_STAMPEDE_ACHIEVEMENT_DRIFT')
    npc = 'oteryn:npc.guide_tiko'
    evidence = npc_sources(root, [npc])
    sources = [path, 'content/encounters/raids/catalogue.json:raid:porthope.stampede',
               region['path'] + ':sha256:' + region['sha256'], evidence[npc], achievement_path + ':' + achievement]
    route = 'oteryn:quest-progress/m01c-stampede-route'
    kills = 'oteryn:quest-progress/m01c-stampede-kills'
    position = lambda x, y: {'x':x, 'y':y, 'floor':-7}
    storage = lambda before, after: [{'track':route,'minimum':before,'maximum':before,'set':after}]
    # Oteryn's chosen personal trial uses retained ground-only cells within the source area.
    # It neither changes nor waits for the daily raid's schedule/world-change state.
    monsters = [position(x,y) for x,y in [(32850,32673),(32851,32672),(32852,32673),(32853,32672),(32854,32673)]]
    callback = {'key':'oteryn:quest-world/m01c-stampede-entry', 'quest':quest,
                'trigger':'step_in', 'selector':{'position':position(32850,32671),'unique':None,'action':None,'appearance':None},
                'lever':None,'reversible':False,'requires_world_admission':True,'passages':[], 'teleport':None,
                'effects':{'encounter':{'minimum_level':1,'players':[],
                    'monsters':[{'position':p,'creature':entry['creature']} for p in monsters],
                    'room_min':position(32825,32667),'room_max':position(32888,32675),
                    'timing':{'spawn_after_ms':1000,'reset_after_ms':900000}}},
                'storage':[{'track':route,'minimum':0,'maximum':1,'set':1,'at_least':True}],'offerings':[],'return_on_failure':False,
                'failure_message':None,'sources':sources[:3] + ['crystal:' + entry['source']['revision'] + ':' + entry['source']['path'] + ':' + str(entry['source']['line'])]}
    finish = {'key':'oteryn:quest-reward/m01c-stampede-collector','quest':quest,'npc':npc,
              'keywords':['stampede'],'topic':0,'next_topic':0,'completes':True,'requires_premium':True,
              'storage':storage(2,3) + [{'track':kills,'minimum':5,'maximum':5,'set':5,'preserve':True}],
              'take':[],'give':None,'outfit':None,'achievement':achievement,
              'reply':['Your victory over five terrified elephants has been recorded.'], 'sources':sources}
    raw = json.dumps(recipe, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    return {'schema':1,'quests':[{'quest':quest,'recipe_sha256':hashlib.sha256(raw).hexdigest(),
            'admission':{'minimum_level':0,'premium':True},
            'tracks':[{'key':route,'initial':0,'minimum':0,'maximum':3}, {'key':kills,'initial':0,'minimum':0,'maximum':5}],
            'stages':[{'key':'s1','completes':False,'trigger':{'kind':'step_in','callback':callback}},
                      {'key':'s2','completes':False,'trigger':{'kind':'kill','counter':{
                          'key':'oteryn:quest-transition/native-kills/m01c-stampede-elephant',
                          'creature':entry['creature'],'track':kills,'gate':route,'gate_value':1,
                          'minimum':0,'maximum':5,'advance_gate_to':2}}},
                      {'key':'s3','completes':True,'trigger':{'kind':'npc_dialogue','callback':finish}}],
            'sources':sources}]}

def content(check=False):
    for output, expected in [(OUTPUT, build_pilgrimage()),
                             (Path('content/quests/authored-bindings/qmech-m01c-stampede.json'), build_stampede())]:
        path = ROOT / output
        if check:
            if not path.is_file() or json.loads(path.read_bytes()) != expected:
                raise ValueError('M01C_COLLECTOR_BINDING_DRIFT:' + str(output))
        else:
            path.write_text(json.dumps(expected, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n')
    print('PASS M01c collectors: Pilgrimage ordered visits and final platinum; Stampede personal trial and achievement')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['content'])
    parser.add_argument('--check', action='store_true')
    content(parser.parse_args().check)
