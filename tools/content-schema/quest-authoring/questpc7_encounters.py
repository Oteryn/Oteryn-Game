"""Owned selected encounter entry bindings; the native death owner grants kill credit."""
import hashlib
import json


def worlds(root):
    path = 'content/quests/definitions/quests-00200-00299.json'
    # Keep the canonical identity and source shard authoritative.
    for shard in sorted((root / 'content/quests/definitions').glob('quests-*.json')):
        raw = shard.read_bytes()
        for index, row in enumerate(json.loads(raw)['records']):
            definition = row['definition']
            if definition['identity']['key'] != 'oteryn:quest.hidden_threats_quest':
                continue
            recipe = definition.get('recipe') or definition['oteryn_recipe']['payload']['recipe']
            if [(s['kind'], s['count']) for s in recipe['stages']] != [('explore',1),('kill',5),('use',1),('complete',1)]:
                raise ValueError('QUESTPC7_HIDDEN_SELECTED_RECIPE_DRIFT')
            path = shard.relative_to(root).as_posix()
            entry = {'key':'oteryn:quest-callback/questpc7-hidden-entry',
                'quest':definition['identity']['key'], 'trigger':'step_in',
                'selector':{'position':{'x':33079,'y':32015,'floor':-13},'appearance':10113,'unique':None,'action':None},
                'lever':None,'reversible':False,'passages':[],'teleport':None,
                'storage':[{'track':'oteryn:quest-progress/questpc7/hidden/programme','minimum':-1,'maximum':-1,'set':1},
                           {'track':'oteryn:quest-progress/questpc7/hidden/kills','minimum':-1,'maximum':-1,'set':0}],
                'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
                'item_on_target':None,'return_on_failure':False,'failure_message':None,
                'success_message':'You have surveyed the concealed Corym chamber. Defeat five Coryms before searching its valuables.',
                'sources':[f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}']}
            chest = dict(entry, key='oteryn:quest-callback/questpc7-hidden-valuables', trigger='use',
                selector={'position':{'x':33080,'y':32014,'floor':-13},'appearance':2469,'unique':None,'action':None},
                storage=[{'track':'oteryn:quest-progress/questpc7/hidden/programme','minimum':1,'maximum':1,'set':2},
                         {'track':'oteryn:quest-progress/questpc7/hidden/kills','minimum':5,'maximum':5,'set':5,'preserve':True}],
                activation={'track':'oteryn:quest-progress/questpc7/hidden/programme','minimum':1,'maximum':1},
                completes=True, item_reward={'item':'oteryn:item.tibia.i3040','count':2,'contents':[],
                    'backpack_outputs':[{'item':f'oteryn:item.tibia.i{i}','count':1} for i in (3030,3032,3033)]},
                success_message='You recover the valuables hidden in the Corym chamber.')
            return [entry, chest]
    raise ValueError('QUESTPC7_HIDDEN_DEFINITION_MISSING')


def objectives(root):
    entry = worlds(root)[0]
    source_map = 'content/world/placements/region-z13-x129-y125.b3'
    sources = entry['sources'] + [
        'canary:04b83b512:data-otservbr-global/scripts/quests/hidden_threats/movement-chests_check.lua:18-22:sha256:0e00845e4ce1d3cf40dba3139210f86bc96dcb0f8f3242345a4a01aa3badc9b4',
        source_map + ':sha256:' + hashlib.sha256((root / source_map).read_bytes()).hexdigest(),
    ]
    rows=[]
    for species in ['charlatan','skirmisher','vanguard']:
        rows.append({'key':'oteryn:quest-death/questpc7-hidden-'+species,
            'quest':entry['quest'],'creature':'oteryn:creature.corym_'+species,
            'stage':'oteryn:quest-progress/questpc7/hidden/programme','stage_value':1,
            'counter':'oteryn:quest-progress/questpc7/hidden/kills','required':5,
            'minimum':{'x':33077,'y':32014,'floor':-13},
            'maximum':{'x':33081,'y':32017,'floor':-13},
            'guardian':{'position':{'x':33080,'y':32015,'floor':-13},'respawn_ms':60000} if species=='charlatan' else None,
            'sources':sources})
    return rows
