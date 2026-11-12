"""Whole selected twelve-grave journey; native actors and retained physical sites."""
import hashlib
import json

QUEST='oteryn:quest.grave_danger_quest'
TRACK='oteryn:quest-progress/questpc7/grave/programme'
VISITS='oteryn:quest-progress/questpc7/grave/visits'
PASSAGES='oteryn:quest-progress/questpc7/grave/passages'
DOOR='oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/king_zelos_door'
OUTFIT='oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/inquisition_outfit_received'
COBRA='oteryn:quest-progress/quest/u12_20/grave_danger/cobra_bastion/access'
GRAVES=[('Edron north', (33222, 31670, -7), 1980, (33221, 31670, -7), 103), ('Dark Cathedral', (32644, 32394, -8), 31112, (32643, 32394, -8), 410), ('Ghostlands', (32190, 31816, -8), 31116, (32189, 31816, -8), 410), ('Cormaya', (33262, 31983, -8), 31109, (33262, 31984, -8), 8230), ('Femor Hills', (32542, 31846, -6), 31104, (32541, 31846, -6), 11422), ('isle northeast of Ankrahmun', (33376, 32806, -8), 31112, (33375, 32806, -8), 410), ('Kilmaresh', (33813, 31624, -9), 31104, (33812, 31624, -9), 410), ('Vengoth', (32959, 31534, -7), 31100, (32958, 31534, -7), 7066), ('Darashia', (33244, 32471, -11), 1982, (33244, 32472, -11), 410), ('old Thais temple', (32347, 32169, -12), 31109, (32347, 32170, -12), 429), ('Orclands entrance', (32776, 31817, -8), 31116, (32775, 31817, -8), 4396), ('southern Ice Islands', (32012, 31558, -7), 31116, (32011, 31558, -7), 436)]
BOSSES=[('count_vlarkorth', (33456, 31434, -13), (33456, 31437, -13), (33455, 31437, -13), 11424), ('duke_krule', (33456, 31473, -13), (33456, 31476, -13), (33455, 31476, -13), 416), ('earl_osam', (33488, 31441, -13), (33488, 31444, -13), (33487, 31444, -13), 6593), ('lord_azaram', (33424, 31473, -13), (33424, 31476, -13), (33423, 31476, -13), 500), ('sir_baeloc', (33422, 31431, -13), (33422, 31434, -13), (33421, 31434, -13), 4394), ('sir_nictros', (33427, 31431, -13), (33427, 31434, -13), (33426, 31434, -13), 4404), ('king_zelos', (33443, 31545, -13), (33443, 31548, -13), (33442, 31548, -13), 410), ('scarlett_etzel', (33396, 32643, -6), (33396, 32646, -6), (33395, 32646, -6), 23897)]
ENTRY=(33221, 31670, -7)
ENTRY_APPEARANCE=103
COBRA_POSITION=(33384, 32627, -7)
COBRA_APPEARANCE=16489
COBRA_APPROACH=(33385, 32627, -7)
REWARD_POSITION=(33398, 32640, -6)
REWARD_APPROACH=(33398, 32639, -6)
REFERENCE={'url': 'https://raw.githubusercontent.com/opentibiabr/canary/666a60f6fef803b043cbaf1c7b5fab219da9bfa9/data-otservbr-global/scripts/quests/grave_danger_quest/actions_grave_sanctify.lua', 'sha256': '5d9d35aaabacfdd4242860446852e76474b4281194afb2352a119f57f3e024a5', 'lines': '1-38', 'rule': 'Seven literal graves; remaining five selected native regional graves, documented conservative bindings.'}

def pos(p):return {'x':p[0],'y':p[1],'floor':p[2]}
def counter(i):return 'oteryn:quest-progress/questpc7/grave/boss_'+str(i)
def state(key,lo,hi,value,preserve=False):
    r={'track':key,'minimum':lo,'maximum':hi,'set':value}
    if preserve:r['preserve']=True
    return r

def build(root):
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',12),('kill',5),('kill',1),('explore',1),('kill',1),('complete',1)]:raise ValueError('QUESTPC7_GRAVE_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    def sources(p):
        path=f'content/world/placements/region-z{-p[2]:02}-x{p[0]//256}-y{p[1]//256}.b3'
        return [canonical,path+':1:sha256:'+hashlib.sha256((root/path).read_bytes()).hexdigest(),REFERENCE['url']+':1-38:sha256:'+REFERENCE['sha256']]
    def callback(name,trigger,p,appearance,storage,message):
        return {'key':'oteryn:quest-callback/questpc7-grave-'+name,'quest':QUEST,'trigger':trigger,
            'selector':{'position':pos(p),'appearance':appearance,'unique':None,'action':None},
            'lever':None,'reversible':False,'passages':[],'teleport':None,'storage':storage,
            'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
            'item_on_target':None,'return_on_failure':False,'failure_message':None,
            'requirements':{'min_level':250,'premium':True,'completed_quests':[]},
            'success_message':message,'sources':sources(p)}
    worlds=[callback('entry','step_in',ENTRY,ENTRY_APPEARANCE,
        [state(TRACK,-1,-1,1),state(VISITS,-1,-1,0)],'You sense twelve haunted graves. Examine this grave to follow the chosen spectral trail.')]
    for i,(name,p,appearance,arrival,_) in enumerate(GRAVES):
        row=callback('survey-'+str(i),'use',p,appearance,[state(TRACK,1,1,1,True),state(VISITS,i,i,i+1)],'You have visited the haunted grave in '+name+'. The spectral trail continues.')
        row['teleport']=pos(GRAVES[i+1][3]) if i<11 else pos(BOSSES[0][2])
        if i==11:row['storage'] += [state(PASSAGES,-1,-1,0),state(counter(0),-1,-1,0)]
        worlds.append(row)
    # Five encounters: the Sirs form one encounter and require both native deaths.
    encounters=[([0],1),([1],2),([2],3),([3],4),([4,5],6)]
    for stage,(members,next_boss) in enumerate(encounters):
        b=BOSSES[members[0]]
        storage=[state(TRACK,1,1,1,True),state(VISITS,12,12,12,True),state(PASSAGES,stage,stage,stage+1)]
        storage += [state(counter(i),1,1,1,True) for i in members]
        storage += [state(counter(i),-1,-1,0) for i in ([4,5] if next_boss==4 else [next_boss])]
        if next_boss==6:storage.append(state(DOOR,-1,1,1))
        row=callback('knight-passage-'+str(stage),'step_in',b[3],b[4],storage,'The Lich Knight encounter is settled. Your expedition may continue.')
        row['teleport']=pos(BOSSES[next_boss][2]);worlds.append(row)
    king=BOSSES[6]
    row=callback('zelos-passage','step_in',king[3],king[4],[state(TRACK,1,1,2),state(PASSAGES,5,5,6),state(counter(6),1,1,1,True)],'King Zelos is defeated. Investigate the Cobra Bastion.')
    row['teleport']=pos(COBRA_APPROACH);worlds.append(row)
    row=callback('cobra','use',COBRA_POSITION,COBRA_APPEARANCE,[state(TRACK,2,2,3),state(COBRA,1,1,1,True),state(counter(7),-1,-1,0)],'You have investigated the Cobra Bastion. Confront Scarlett Etzel.')
    row['activation']={'track':TRACK,'minimum':2,'maximum':2};row['teleport']=pos(BOSSES[7][2]);worlds.append(row)
    scar=BOSSES[7]
    row=callback('scarlett-passage','step_in',scar[3],scar[4],[state(TRACK,3,3,3,True),state(PASSAGES,6,6,7),state(counter(7),1,1,1,True)],'Scarlett is defeated. Examine the armour to close the investigation.')
    row['teleport']=pos(REWARD_APPROACH);worlds.append(row)
    row=callback('reward','use',REWARD_POSITION,31482,[state(TRACK,3,3,4),state(PASSAGES,7,7,7,True),state(counter(7),1,1,1,True),state(OUTFIT,-1,1,1)],'The twelve graves, Lich Knights, Zelos and Cobra threat are resolved. You earn Hand of the Inquisition and Inquisition\'s Hand.')
    row.update(completes=True,outfit_reward={'outfit':'oteryn:outfit.hand_of_the_inquisition','addons':0},achievements=['oteryn:achievement/inquisition_s_hand'],activation={'track':TRACK,'minimum':3,'maximum':3});worlds.append(row)
    objectives=[]
    for i,(name,center,arrival,exit,appearance) in enumerate(BOSSES):
        stage=(0,1,2,3,4,4,5)[i] if i<7 else 3
        objectives.append({'key':'oteryn:quest-death/questpc7-grave-'+name,'quest':QUEST,'creature':'oteryn:creature.'+name,
            'stage':PASSAGES if i<7 else TRACK,'stage_value':stage,'counter':counter(i),'required':1,
            'minimum':pos((center[0]-8,center[1]-8,center[2])),'maximum':pos((center[0]+8,center[1]+8,center[2])),
            'guardian':{'position':pos(center),'respawn_ms':60000},'sources':sources(center)})
    return worlds,objectives
