"""Chosen Rathleton investigation uses retained physical dungeon placements."""
import hashlib
import json

QUEST='oteryn:quest.hero_of_rathleton_quest'
TRACK='oteryn:quest-progress/questpc7/hero/programme'
KILLS='oteryn:quest-progress/questpc7/hero/constructs'
BOSS='oteryn:quest-progress/questpc7/hero/maxxen'

def build(root):
    path='content/quests/definitions/quests-00100-00199.json'
    raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('use',1),('explore',1),('kill',10),('kill',1),('use',1),('complete',1)]:
        raise ValueError('QUESTPC7_HERO_RECIPE_DRIFT')
    region='content/world/placements/region-z14-x131-y124.b3'
    sources=[f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
             f'{region}:1#selected-physical-dungeon:sha256:{hashlib.sha256((root/region).read_bytes()).hexdigest()}']
    rows=[]
    for name,trigger,x,y,appearance,before,after,message in [
        ('entry','step_in',33564,31951,20536,-1,1,'A voice beneath Rathleton leads you toward the workshop. Examine the machinery for evidence.'),
        ('evidence','use',33567,31950,20476,1,2,'The workshop evidence exposes the plot. Follow the passage east into the dungeon.'),
        ('dungeon','step_in',33568,31951,20545,2,3,'You have earned access to this investigation route. Defeat ten hostile Glooth Golems, then Professor Maxxen.'),
        ('reward','use',33571,31952,1855,3,4,'The conspiracy is ended. You recover the blueprint, four Glooth Bags and five platinum coins.'),
    ]:
        storage=[{'track':TRACK,'minimum':before,'maximum':before,'set':after}]
        if name=='entry':storage += [{'track':t,'minimum':-1,'maximum':-1,'set':0} for t in [KILLS,BOSS]]
        if name=='reward':storage += [{'track':t,'minimum':n,'maximum':n,'set':n,'preserve':True} for t,n in [(KILLS,10),(BOSS,1)]]
        row={'key':'oteryn:quest-callback/questpc7-hero-'+name,'quest':QUEST,'trigger':trigger,
             'selector':{'position':{'x':x,'y':y,'floor':-14},'appearance':appearance,'unique':None,'action':None},
             'lever':None,'reversible':False,'passages':[],'teleport':None,'storage':storage,
             'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
             'requirements':{'min_level':0,'premium':True,'completed_quests':[]},
             'item_on_target':None,'return_on_failure':False,'failure_message':None,'success_message':message,'sources':sources}
        if name=='reward':
            row['storage'].append({'track':'oteryn:quest-progress/hero_rathleton/access_door','minimum':-1,'maximum':1,'set':1})
            row['completes']=True
            row['item_reward']={'item':'oteryn:item.tibia.i21897','count':1,'contents':[],
                'backpack_outputs':[{'item':'oteryn:item.tibia.i21203','count':1} for _ in range(4)]+[{'item':'oteryn:item.tibia.i3035','count':5}]}
        rows.append(row)
    objectives=[]
    for name,creature,counter,required,x,y in [('construct','glooth_golem',KILLS,10,33566,31950),('maxxen','professor_maxxen',BOSS,1,33570,31951)]:
        objectives.append({'key':'oteryn:quest-death/questpc7-hero-'+name,'quest':QUEST,'creature':'oteryn:creature.'+creature,
            'stage':TRACK if name=='construct' else KILLS,'stage_value':3 if name=='construct' else 10,'counter':counter,'required':required,
            'minimum':{'x':33564,'y':31949,'floor':-14},'maximum':{'x':33571,'y':31953,'floor':-14},
            'guardian':{'position':{'x':x,'y':y,'floor':-14},'respawn_ms':60000},'sources':sources})
    return rows,objectives
