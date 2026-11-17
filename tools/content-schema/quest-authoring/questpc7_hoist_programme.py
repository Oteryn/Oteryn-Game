"""Selected knowledge milestones on admitted hoist/chest/fitting placements."""
import hashlib
import json

SKELETON='oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_skeleton'
CHEST='oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_chest'
TRACK='oteryn:quest-progress/questpc7/hoist/programme'


def build(root):
    path='content/quests/definitions/quests-00100-00199.json'
    raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']=='oteryn:quest.formogar_mine_hoist')
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('collect',1),('collect',1),('use',1),('complete',1)] or recipe['repeat']['kind']!='once':
        raise ValueError('QUESTPC7_HOIST_SELECTED_RECIPE_DRIFT')
    sources=[f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}']
    requirements={'min_level':0,'premium':True,'completed_quests':['oteryn:quest.the_ice_islands_quest']}
    rows=[]
    for name,trigger,position,appearance,before,after,extra,message,completes in [
        ('entry','step_in',{'x':32154,'y':31118,'floor':-9},7066,-1,0,[],
         'You have found the old surveying route. Search the nearby chest for the hoist plan.',False),
        ('plan','use',{'x':32154,'y':31116,'floor':-9},2472,0,1,
         [{'track':CHEST,'minimum':-1,'maximum':-1,'set':1}],
         'The recovered plan describes the hoist fittings. Examine them beside the platform.',False),
        ('instructions','use',{'x':32156,'y':31125,'floor':-9},5772,1,2,
         [{'track':SKELETON,'minimum':-1,'maximum':-1,'set':1}],
         'The fittings match the plan. You understand the operating instructions. Enter the platform and set the lower lever.',False),
        ('control','use',{'x':32156,'y':31125,'floor':-10},2773,2,3,
         [{'track':CHEST,'minimum':1,'maximum':1,'set':1,'preserve':True},
          {'track':SKELETON,'minimum':1,'maximum':1,'set':1,'preserve':True}],
         'The hoist is ready. You have completed the surveying route.',True),
    ]:
        row={'key':'oteryn:quest-callback/questpc7-hoist-'+name,'quest':definition['identity']['key'],'trigger':trigger,
             'selector':{'position':position,'unique':None,'action':None,'appearance':appearance},
             'lever':[2773,2772] if completes else None,'reversible':False,'passages':[],'teleport':None,
             'storage':[{'track':TRACK,'minimum':before,'maximum':before,'set':after}]+extra,
             'minimum_level':None,'requirements':requirements,'experience':None,'monk_reward':None,
             'offerings':[],'item_on_target':None,'return_on_failure':trigger=='step_in',
             'failure_message':'Complete The Ice Islands Quest and follow the surveying route in order.',
             'success_message':message,'sources':sources}
        if completes:row['completes']=True
        rows.append(row)
    bindings=[]
    for uid,z in [(3059,9),(3060,10),(3061,11)]:
        map_path=f'content/world/placements/region-z{z:02}-x125-y121.b3'
        bindings.append({'callback':f'oteryn:quest-callback/formogar-hoist-{uid}',
            'expected':{'position':None,'unique':uid,'action':None,'appearance':None},
            'selector':{'position':{'x':32157,'y':31125,'floor':-z},'unique':None,'action':None,'appearance':5770},
            'sources':[map_path+':sha256:'+hashlib.sha256((root/map_path).read_bytes()).hexdigest()]})
    return rows,bindings
