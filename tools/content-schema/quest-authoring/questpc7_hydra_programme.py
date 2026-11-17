"""Selected Hydra route keeps ordinary native nest pickup and map-reset ownership."""
import hashlib
import json

QUEST = 'oteryn:quest.hydra_egg_quest'
TRACK = 'oteryn:quest-progress/questpc7/hydra/programme'
KILLS = 'oteryn:quest-progress/questpc7/hydra/kills'
EGG = (33004, 32664, -3, 2)
MAP_SOURCE = (EGG[0] << 32) | (EGG[1] << 16) | (-EGG[2] << 8) | EGG[3]


def build(root):
    for path in sorted((root/'content/quests/definitions').glob('quests-*.json')):
        raw = path.read_bytes()
        for index, row in enumerate(json.loads(raw)['records']):
            definition = row['definition']
            if definition['identity']['key'] != QUEST:
                continue
            recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
            if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('kill',1),('use',1),('complete',1)]:
                raise ValueError('QUESTPC7_HYDRA_RECIPE_DRIFT')
            region='content/world/placements/region-z03-x128-y127.b3'
            sources=[f'canonical:{path.relative_to(root).as_posix()}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
                f'{region}:1#(33004,32664,-3)/2:sha256:{hashlib.sha256((root/region).read_bytes()).hexdigest()}']
            requirements={'min_level':0,'premium':True,'completed_quests':[]}
            entry={'key':'oteryn:quest-callback/questpc7-hydra-entry','quest':QUEST,'trigger':'step_in',
                'selector':{'position':{'x':33003,'y':32665,'floor':-3},'appearance':1019,'unique':None,'action':None},
                'lever':None,'reversible':False,'passages':[],'teleport':None,
                'storage':[{'track':TRACK,'minimum':-1,'maximum':-1,'set':1},
                           {'track':KILLS,'minimum':-1,'maximum':-1,'set':0}],
                'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
                'item_on_target':None,'return_on_failure':False,'failure_message':None,
                'requirements':requirements,'success_message':'You have reached the Hydra nest. Defeat its guardian and recover an egg.',
                'sources':sources}
            nest=dict(entry,key='oteryn:quest-callback/questpc7-hydra-nest',trigger='use',
                selector={'position':{'x':33004,'y':32664,'floor':-3},'appearance':1063,'unique':None,'action':None},
                storage=[{'track':TRACK,'minimum':1,'maximum':1,'set':2},
                         {'track':KILLS,'minimum':1,'maximum':1,'set':1,'preserve':True}],
                requirements=dict(requirements,owned_items=[{'item':'oteryn:item.tibia.i4839','count':1,'map_source':MAP_SOURCE}]),
                completes=True,success_message='You have recovered an egg from the Hydra nest.')
            objective={'key':'oteryn:quest-death/questpc7-hydra-guardian','quest':QUEST,
                'creature':'oteryn:creature.hydra','stage':TRACK,'stage_value':1,'counter':KILLS,'required':1,
                'minimum':{'x':33002,'y':32662,'floor':-3},'maximum':{'x':33006,'y':32666,'floor':-3},
                'guardian':{'position':{'x':33005,'y':32665,'floor':-3},'respawn_ms':60000},'sources':sources}
            return [entry,nest],[objective]
    raise ValueError('QUESTPC7_HYDRA_DEFINITION_MISSING')
