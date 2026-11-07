"""Source-bound Regalia exchange and knowledge mission compositions."""
import hashlib


def append(add, callbacks, offers, inputs, pin):
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


def kill_bindings(inputs, pin):
    path='data-global/scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua'
    return {'schema':1,'callbacks':[{
        'creature':'oteryn:creature.renegade_orc',
        'transition':'oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/creature_event_1',
        'sources':[f'crystal:{pin}:{path}:1-15:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']}]}


def shops(inputs,pin):
    return {'schema':1,'shops':[]}
