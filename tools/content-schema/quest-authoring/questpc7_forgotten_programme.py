"""Finite chosen Halls of Hope route; original source form/party state is separate."""
import hashlib
import json
QUEST='oteryn:quest.forgotten_knowledge_quest'
TRACK='oteryn:quest-progress/questpc7/forgotten/programme'
FATHER='oteryn:quest-progress/questpc7/forgotten/father'
def pos(p):return dict(x=p[0],y=p[1],floor=p[2])
def counter(i):return 'oteryn:quest-progress/questpc7/forgotten/boss_'+str(i)
def state(key,lo,hi,value,preserve=False):
    r=dict(track=key,minimum=lo,maximum=hi,set=value)
    if preserve:r['preserve']=True
    return r
# DATA injected from checked native positions before publication.
DATA={'bosses': [{'name': 'lloyd', 'source_center': [32799, 32827, -14], 'center': [32799, 32828, -14], 'arrival': [32799, 32831, -14], 'exit': [32798, 32831, -14], 'exit_appearance': 15469}, {'name': 'lady_tenebris', 'source_center': [32912, 31599, -14], 'center': [32912, 31599, -14], 'arrival': [32912, 31602, -14], 'exit': [32911, 31602, -14], 'exit_appearance': 23950}, {'name': 'the_enraged_thorn_knight', 'source_center': [32624, 32880, -14], 'center': [32624, 32880, -14], 'arrival': [32624, 32883, -14], 'exit': [32623, 32883, -14], 'exit_appearance': 1019}, {'name': 'soul_of_dragonking_zyrtarch', 'source_center': [33357, 31182, -10], 'center': [33357, 31182, -10], 'arrival': [33357, 31185, -10], 'exit': [33356, 31185, -10], 'exit_appearance': 10998}, {'name': 'the_time_guardian', 'source_center': [32977, 31662, -14], 'center': [32977, 31662, -14], 'arrival': [32977, 31665, -14], 'exit': [32976, 31665, -14], 'exit_appearance': 23908}, {'name': 'the_last_lore_keeper', 'source_center': [31986, 32840, -14], 'center': [31986, 32840, -14], 'arrival': [31986, 32843, -14], 'exit': [31985, 32843, -14], 'exit_appearance': 410}], 'ivalisse_approach': {'position': [32327, 32093, -7], 'appearance': 23898}, 'tome': {'position': [32334, 32090, -7], 'appearance': 23899}, 'egg_approach': {'position': [33004, 32663, -3], 'appearance': 4405}, 'fire_entry': {'position': [32328, 32093, -7], 'appearance': 4525}, 'egg': {'position': [33003, 32662, -3], 'appearance': 4839, 'path': 'content/world/placements/region-z03-x128-y127.b3', 'sha256': '4143a9190fa20ae867b2cc015dcc6e974fdfbd786c6eb91f750cb5d832ba9072'}}
def build(root,inputs,pin):
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,d=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    stages=d['oteryn_recipe']['payload']['recipe']['stages']
    if [(s['kind'],s['count']) for s in stages]!=[(k,1) for k in ['talk','collect','kill','kill','use','kill','talk','kill','kill','kill','complete']]:raise ValueError('QUESTPC7_FORGOTTEN_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    def sources(p):
        region=f'content/world/placements/region-z{-p[2]:02}-x{p[0]//256}-y{p[1]//256}.b3'
        return [canonical,region+':1:sha256:'+hashlib.sha256((root/region).read_bytes()).hexdigest(),f'crystal:{pin}:data-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua:1-27:sha256:a253bd0bd92d9acba483a5a181998f59a09d48aef91c50b20de2128661599907']
    def cb(name,trigger,site,storage,reply,target=None):
        return dict(key='oteryn:quest-callback/questpc7-forgotten-'+name,quest=QUEST,trigger=trigger,selector=dict(position=pos(site['position']),appearance=site['appearance'],unique=None,action=None),lever=None,reversible=False,passages=[],teleport=None if target is None else pos(target),storage=storage,minimum_level=None,experience=None,monk_reward=None,offerings=[],item_on_target=None,return_on_failure=False,failure_message=None,requirements=dict(min_level=0,premium=True,completed_quests=[]),success_message=reply,sources=sources(site['position']))
    bosses=DATA['bosses'];ivalisse=DATA['ivalisse_approach'];egg=DATA['egg']
    tome=cb('tome','use',DATA['tome'],[state(TRACK,0,0,1)],'You recover a Heavy Old Tome for the temple record.',ivalisse['position'])
    tome['item_reward']=dict(item='oteryn:item.tibia.i23986',count=1,contents=[])
    worlds=[tome]
    # The separately gated portal cannot initiate or bypass the tome exchange.
    worlds.append(cb('energy-entry','step_in',ivalisse,[state(TRACK,2,2,2,True),state(counter(0),-1,-1,0)],'The restored record opens the Energy Portal.',bosses[0]['arrival']))
    phases=[2,3,5,7,8,9]
    next_phases=[3,4,6,8,9,10]
    targets=[bosses[1]['arrival'],DATA['egg_approach']['position'],ivalisse['position'],bosses[4]['arrival'],bosses[5]['arrival'],ivalisse['position']]
    for i,b in enumerate(bosses):
        storage=[state(TRACK,phases[i],phases[i],next_phases[i]),state(counter(i),1,1,1,True)]
        if i in [0,3,4]:storage.append(state(counter(i+1),-1,-1,0))
        worlds.append(cb('passage-'+str(i),'step_in',dict(position=b['exit'],appearance=b['exit_appearance']),storage,'The ancient guardian is defeated. Follow the restored path.',targets[i]))
    worlds.append(cb('egg','use',egg,[state(TRACK,4,4,5),state(counter(2),-1,-1,0)],'You secure the Dragon Mother egg and restore the selected Ice Portal route.',bosses[2]['arrival']))
    father={'npc': [33419, 31125, -9], 'approach': [33419, 31124, -9], 'approach_appearance': 11000, 'exit': [33420, 31124, -9], 'exit_appearance': 11002}
    fire=DATA['fire_entry']
    worlds.append(cb('fire-entry','step_in',fire,[state(TRACK,60,60,60,True),state(counter('father_visit'),-1,-1,1)],"Find Ivalisse's father, disguised as a chalice.",father['approach']))
    worlds.append(cb('father-passage','step_in',dict(position=father['exit'],appearance=father['exit_appearance']),[state(TRACK,7,7,7,True),state(FATHER,6,6,6,True),state(counter(3),-1,-1,0)],'The password zzubaran opens the fire route.',bosses[3]['arrival']))
    objectives=[]
    for i,b in enumerate(bosses):
        p=b['center'];objectives.append(dict(key='oteryn:quest-death/questpc7-forgotten-'+b['name'],quest=QUEST,creature='oteryn:creature.'+b['name'],stage=TRACK,stage_value=phases[i],counter=counter(i),required=1,minimum=pos([p[0]-8,p[1]-8,p[2]]),maximum=pos([p[0]+8,p[1]+8,p[2]]),guardian=dict(position=pos(p),respawn_ms=60000),sources=sources(p)))
    rewards=[]
    for name,word,before,after,reply in [('mission','mission',-1,0,'Recover a Heavy Old Tome for the temple record.'),('tome','tome',1,2,'The record is restored. Enter the Energy Portal.'),('father','father',6,60,'My father is Silus. Follow the fire path to find his aid and confront the Dragon King.'),('complete','knowledge',10,11,'You restored the selected six paths and defeated the Last Lore Keeper. You have earned powerful imbuing knowledge.')]:
        r=dict(key='oteryn:quest-reward/questpc7-forgotten-'+name,quest=QUEST,npc='oteryn:npc.ivalisse',keywords=[word],topic=0,next_topic=0,storage=[state(TRACK,before,before,after)],take=[dict(item='oteryn:item.tibia.i23986',count=1)] if name=='tome' else [],give=None,outfit=None,achievement=None,reply=[reply],requirements=dict(min_level=0,premium=True,completed_quests=[]),sources=sources(ivalisse['position']))
        if name=='father':r['storage'].append(state(FATHER,-1,-1,0))
        r['sources'].append(f'crystal:{pin}:data-global/npc/ivalisse.lua:60-114:sha256:455bf9ca148de6aec55a2801f9234bb492b161bac253afd3703cb8c609f11bda')
        if name=='complete':r['completes']=True
        rewards.append(r)
    for i,word in enumerate(['chalice','daughter','story','disguised','thirsty','learned']):
        storage=[state(TRACK,60,60,7 if i==5 else 60,i!=5),state(FATHER,i,i,i+1)]
        rewards.append(dict(key='oteryn:quest-reward/questpc7-forgotten-silus-'+word,quest=QUEST,npc='oteryn:npc.a_strange_chalice',keywords=[word],topic=0,next_topic=0,storage=storage,take=[],give=None,outfit=None,achievement=None,reply=["I am Ivalisse's father. My research taught me the fire-route password: zzubaran." if i==5 else 'You listen to the disguised scholar. Ask about '+['daughter','story','disguised','thirsty','learned','learned'][i]+'.'],requirements=dict(min_level=0,premium=True,completed_quests=[]),sources=sources(father['npc'])+[f'crystal:{pin}:data-global/npc/a_strange_chalice.lua:60-113:sha256:7eb9bf22d46533d4584fc67a1fa60e49cda9395e05f2e1dc4b50c250c18243ea']))
    return worlds,objectives,rewards
