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


def kill_bindings(inputs, pin):
    path='data-global/scripts/quests/an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua'
    return {'schema':1,'callbacks':[{
        'creature':'oteryn:creature.renegade_orc',
        'transition':'oteryn:quest-transition/quest/u8_54/an_uneasy_alliance/quest_door/creature_event_1',
        'sources':[f'crystal:{pin}:{path}:1-15:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']}]}
