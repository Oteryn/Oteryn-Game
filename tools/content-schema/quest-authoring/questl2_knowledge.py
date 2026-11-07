"""Source-bound Regalia exchange and knowledge mission compositions."""
import hashlib


def append(add, callbacks, offers, declarations, inputs, pin):
    path='data-global/npc/yonan.lua'
    add('l2-regalia-combine','kilmaresh_quest','yonan',['yes'],[],
        take=[(31572,1),(31573,1),(31574,1),(31575,1)],give=(31576,1),topic=5)
    callbacks[-1]['sources']=[f'crystal:{pin}:{path}:198-209:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']
    callbacks[-1]['reply']=['Well then, let me have a look.']
    offers.append({'npc':'oteryn:npc.yonan','keywords':['regalia of suon'],'topic':0,'next_topic':5,
                   'storage':[],'reply':['Do you have the four parts of the Regalia of Suon?']})


    prefix='oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/'
    frontier='oteryn:quest-progress/quest/u8_54/the_new_frontier/'
    def state(track,before,after,maximum=None,preserve=False):
        return {'track':track,'minimum':before,'maximum':before if maximum is None else maximum,
                'set':after,**({'preserve':True} if preserve else {})}
    def witness(path,lines):
        return f'crystal:{pin}:{path}:{lines}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'
    curos='data-global/npc/curos.lua'
    add('l2-alliance-start','an_uneasy_alliance_quest','curos',['mission'],
        [state(prefix+'questline',0,1),state(prefix+'quest_door',0,0,1)],
        take=[(2873,1),(10219,1),(2948,1)],give=(2873,1),
        requires=[{'track':frontier+'mission06','minimum':5,'maximum':5}])
    callbacks[-1]['take'][0]['fluid_subtype']=6
    callbacks[-1]['sources']=[witness(curos,'70-82'),
        'repo:tools/content-schema/quest-authoring/samples/wiki-source-all373/source-specs-373.json:An Uneasy Alliance Quest:spoiler:7-9:sha256:2788b3c0b943e0b71687494a5e7bc5aa81542e0b1407793c165cd64e00fc2dfd',
        'owner:restore-requested-provisions-at-mission-issuance:return-empty-bucket']
    callbacks[-1]['reply']=['Find the rebel leader and kill him. Take these empty provisions back with you.']
    callbacks[-1]['failure_reply']=['Bring one bucket of milk, one crocodile steak and one wooden flute.']
    for name,before,after,extra,lines,reply in [
        ('rebel-report',1,2,[state(prefix+'quest_door',1,1,preserve=True)],'83-85','With the death of their leader, the rebels return to our camps one by one, deeply humiliated.'),
        ('spy-mission',2,3,[],'86-94','Infiltrate this tower and destroy their scrying device.'),
        ('finish',4,5,[],'95-101',"Here is a reward. It's a strange tome that we've found in the lizard ruins. Maybe it is of some value for you or your allies.")]:
        add('l2-alliance-'+name,'an_uneasy_alliance_quest','curos',['mission'],
            [state(prefix+'questline',before,after)]+extra,give=(10217,1) if name=='finish' else None)
        callbacks[-1]['sources']=[witness(curos,lines)]
        callbacks[-1]['reply']=[reply]

    # Cael owns New Frontier's existing counter, rather than a parallel quest flag.
    cael='data-global/npc/cael.lua'
    counter=frontier+'tomeof_knowledge'
    for number in range(1,14):
        before=min(number-1,12)
        states=[state(counter,before,min(number,12),preserve=number==13)]
        unlocks={7:('snake_head_teleport',-1),8:('corruption_hole',-1),10:('zao_palace_doors',0)}
        if number in unlocks:
            name,initial=unlocks[number]
            states.append(state(frontier+name,initial,1,0 if initial==-1 else None))
        requires=[{'track':frontier+'mission08','minimum':2,'maximum':2}]
        if 7<=number<=12:
            requires.append({'track':frontier+'mission10_1','minimum':2,'maximum':2})
        add('l2-cael-tome-'+str(number),'the_new_frontier_quest','cael',['yes'],states,
            take=[(10217,1)],topic=number,next_topic=number+20,requires=requires)
        callbacks[-1]['sources']=[witness(cael,'84-128')]
        callbacks[-1]['reply']=['Thank you! I look forward to reading this interesting discovery of yours and learn a few things about Zao.']
        if number==13:
            callbacks[-1].update(repeatable=True,experience=5000)
        offers.append({'npc':'oteryn:npc.cael','keywords':['tome','knowledge'],
            'topic':0,'any_topic':True,'next_topic':number,
            'storage':[state(counter,before,before,preserve=True)],
            'requires':requires,'reply':['Have you found a Tome of Knowledge for me to read?']})
    declarations.append({'writable':True,'key':frontier+'corruption_hole','quest':'oteryn:quest.the_new_frontier_quest',
        'initial':-1,'min':-1,'max':1,'sources':[witness(cael,'118-122')]})
    child='oteryn:quest-progress/quest/u8_54/children_of_the_revolution/'
    add('l2-zalamon-tome','children_of_the_revolution_quest','zalamon',['yes'],
        [state(child+'questline',17,18),state(child+'mission04',5,6)],give=(10217,1),topic=12)
    callbacks[-1]['sources']=[witness('data-global/npc/zalamon.lua','325-335')]
    offers.append({'npc':'oteryn:npc.zalamon','keywords':['mission'],'topic':0,'any_topic':True,
        'next_topic':12,'storage':[state(child+'questline',17,17,preserve=True)],
        'reply':['Have you found the way which will lead us to the great gate?']})


def kill_bindings(inputs, pin):
    path='data-global/scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua'
    return {'schema':1,'callbacks':[{
        'creature':'oteryn:creature.renegade_orc',
        'transition':'oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/creature_event_1',
        'sources':[f'crystal:{pin}:{path}:1-15:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']}]}


def shops(inputs,pin):
    path='data-global/npc/cael.lua'
    track='oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge'
    return {'schema':1,'shops':[{'npc':'oteryn:npc.cael','service':'oteryn:service.quest.cael.tomes',
        'sources':[f'crystal:{pin}:{path}:30-35:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'],
        'offers':[{'item':{'family':'Item','key':f'oteryn:item.tibia.i{item}','revision':'definition-r1'},
            'direction':'SellToPlayer','unit_price':price,
            'requirements':[{'quest':'oteryn:quest.the_new_frontier_quest','track':track,
                'initial':0,'minimum':count,'maximum':12}]}
            for item,price,count in [(2965,5000,6),(2966,1000,6),(10216,2500,11),(10288,5000,12)]]}]}
