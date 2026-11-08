"""Selected fixed Otherworld expedition and native learning reward."""
import hashlib
import json
QUEST='oteryn:quest.heart_of_destruction_quest'
TRACK='oteryn:quest-progress/questpc7/heart/programme'
PASSAGES='oteryn:quest-progress/questpc7/heart/passages'
DATA={'bosses': [{'name': 'anomaly', 'center': [32271, 31249, -14], 'arrival': [32271, 31252, -14], 'exit': [32270, 31252, -14], 'exit_appearance': 22893}, {'name': 'rupture', 'center': [32332, 31250, -14], 'arrival': [32332, 31253, -14], 'exit': [32331, 31253, -14], 'exit_appearance': 22893}, {'name': 'realityquake', 'center': [32208, 31248, -14], 'arrival': [32208, 31251, -14], 'exit': [32207, 31251, -14], 'exit_appearance': 22893}, {'name': 'eradicator', 'center': [32309, 31283, -14], 'arrival': [32309, 31286, -14], 'exit': [32308, 31286, -14], 'exit_appearance': 22893}, {'name': 'outburst', 'center': [32234, 31284, -14], 'arrival': [32234, 31287, -14], 'exit': [32234, 31286, -14], 'exit_appearance': 22893}, {'name': 'world_devourer', 'center': [32271, 31347, -14], 'arrival': [32271, 31350, -14], 'exit': [32270, 31350, -14], 'exit_appearance': 23098}], 'entry': {'position': [33026, 31367, -8], 'appearance': 10228}, 'yana_approach': {'position': [32201, 32304, -6], 'appearance': 8309}, 'destabilizer': {'position': [32245, 31244, -14], 'appearance': 8911, 'arrival': [32244, 31244, -14], 'source': 'content/world/placements/region-z14-x125-y122.b3', 'sha256': '7428566700a7797e6b7192751f44a0964e148c53844be8480f32ab8242314367'}}

def pos(p):return {'x':p[0],'y':p[1],'floor':p[2]}
def counter(i):return 'oteryn:quest-progress/questpc7/heart/boss_'+str(i)
def state(key,lo,hi,value,preserve=False):
    row={'track':key,'minimum':lo,'maximum':hi,'set':value}
    if preserve:row['preserve']=True
    return row

def build(root,inputs,pin):
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('use',1)]+[('kill',1)]*6+[('talk',1),('complete',1)]:raise ValueError('QUESTPC7_HEART_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    def sources(p):
        region=f'content/world/placements/region-z{-p[2]:02}-x{p[0]//256}-y{p[1]//256}.b3'
        return [canonical,region+':1:sha256:'+hashlib.sha256((root/region).read_bytes()).hexdigest(),
            f'crystal:{pin}:data-global/scripts/quests/heart_of_destruction/actions_anomaly_lever.lua:1-39:sha256:b552a05e52be2c70c107ea0a1034710c081f3ac03ebfff98a8f957478abbc831']
    def cb(name,trigger,p,appearance,storage,message,target):
        return {'key':'oteryn:quest-callback/questpc7-heart-'+name,'quest':QUEST,'trigger':trigger,
            'selector':{'position':pos(p),'appearance':appearance,'unique':None,'action':None},
            'lever':None,'reversible':False,'passages':[],'teleport':pos(target),'storage':storage,
            'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
            'item_on_target':None,'return_on_failure':False,'failure_message':None,
            'requirements':{'min_level':150,'premium':True,'completed_quests':[]},
            'success_message':message,'sources':sources(p)}
    entry=DATA['entry'];device=DATA['destabilizer'];bosses=DATA['bosses']
    worlds=[cb('entry','step_in',entry['position'],entry['appearance'],[state(TRACK,-1,-1,0)],'You enter the Otherworld expedition through the Zao breach.',device['arrival'])]
    row=cb('destabilizer','use',device['position'],device['appearance'],[state(TRACK,0,0,1),state(PASSAGES,-1,-1,0),state(counter(0),-1,-1,0)],'The Destruction Destabilizer secures the expedition route.',bosses[0]['arrival'])
    row['lever']=[8911,8912];worlds.append(row)
    for i,b in enumerate(bosses):
        storage=[state(TRACK,1,1,2 if i==5 else 1,i!=5),state(PASSAGES,i,i,i+1),state(counter(i),1,1,1,True)]
        if i<5:storage.append(state(counter(i+1),-1,-1,0))
        worlds.append(cb('passage-'+str(i),'step_in',b['exit'],b['exit_appearance'],storage,'The breach boss is defeated. The expedition continues.',DATA['yana_approach']['position'] if i==5 else bosses[i+1]['arrival']))
    objectives=[]
    for i,b in enumerate(bosses):
        center=b['center'];objectives.append({'key':'oteryn:quest-death/questpc7-heart-'+b['name'],'quest':QUEST,'creature':'oteryn:creature.'+b['name'],'stage':PASSAGES,'stage_value':i,'counter':counter(i),'required':1,'minimum':pos((center[0]-8,center[1]-8,center[2])),'maximum':pos((center[0]+8,center[1]+8,center[2])),'guardian':{'position':pos(center),'respawn_ms':60000},'sources':sources(center)})
    source='data-global/npc/yana.lua'
    reward={'key':'oteryn:quest-reward/questpc7-heart-yana','quest':QUEST,'npc':'oteryn:npc.yana','keywords':['worth'],'topic':0,'next_topic':0,'storage':[state(TRACK,2,2,3),state(PASSAGES,6,6,6,True)],'take':[],'give':{'item':'oteryn:item.tibia.i22721','count':5,'contents':[]},'outfit':None,'achievement':'oteryn:achievement/ender_of_the_end','completes':True,'reply':['You disrupted the Heart of Destruction and defeated the World Devourer. You have proven your worth. You receive five Gold Tokens and learn Powerful Strike, Void and Vampirism.'],'requirements':{'min_level':150,'premium':True,'completed_quests':[]},'sources':[canonical,f'crystal:{pin}:{source}:185-187:sha256:'+hashlib.sha256(inputs[source]).hexdigest()]}
    return worlds,objectives,[reward]
