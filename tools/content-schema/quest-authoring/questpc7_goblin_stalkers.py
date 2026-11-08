"""A source-bounded ordinary Marsh Stalker death objective, without a guardian."""
import hashlib
import json


def build(root):
    path='content/world/spawns/spawns-10000-11999.json';raw=(root/path).read_bytes()
    digest='a74b7da322bba5ebea20c772dae368d1337ec4caf9cd146f89b1bb9b8419b051'
    if hashlib.sha256(raw).hexdigest()!=digest:raise ValueError('QUESTPC7_STALKER_SPAWN_DRIFT')
    sites={(p['cell']['x'],p['cell']['y'],p['cell']['floor']) for row in json.loads(raw)['records']
           for p in row['declaration']['points'] if p['creature']=='oteryn:creature.marsh_stalker' and p['respawn_ms']==90000}
    if not {(33057,32109,7),(33052,32119,7),(33043,32136,7),(33054,32149,7),(33026,32155,7)} <= sites:
        raise ValueError('QUESTPC7_STALKER_NATIVE_SITES_DRIFT')
    canonical='content/quests/definitions/quests-00100-00199.json';quest_raw=(root/canonical).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(quest_raw)['records'])
                          if r['definition']['identity']['key']=='oteryn:quest.goblin_merchant_quest')
    stage=definition['oteryn_recipe']['payload']['recipe']['stages'][6]
    if (stage['kind'],stage['count'])!=('kill',5):raise ValueError('QUESTPC7_STALKER_RECIPE_DRIFT')
    track='oteryn:quest-progress/questpc7/goblin/'
    return [dict(key='oteryn:quest-death/questpc7-goblin-stalkers',quest=definition['identity']['key'],
        creature='oteryn:creature.marsh_stalker',stage=track+'programme',stage_value=0,counter=track+'stalkers',required=5,
        minimum=dict(x=33010,y=32100,floor=-7),maximum=dict(x=33074,y=32164,floor=-7),
        sources=[f'{path}:1:sha256:{digest}',f'canonical:{canonical}:1#/records/{index}/definition:sha256:{hashlib.sha256(quest_raw).hexdigest()}',
                 'https://www.tibiawiki.com.br/index.php?title=Task:Rafzan&oldid=401277'])]
