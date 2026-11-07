"""Pinned QENGINE2 shared counter, timer and foreign participant reference."""

def append(add, storage, callbacks, offers, declarations, inputs, PIN, sha):
    citizen_rook='oteryn:quest-progress/quest/'+'u7_8/citizen_outfits_rook/'
    rook_ref='oteryn:quest-progress/outfit_quest/ref'
    ref_source=f'crystal:{PIN}:data-global/npc/amber.lua:103-106:sha256:{sha(inputs["data-global/npc/amber.lua"])}'
    declarations.append({'key':rook_ref,'quest':'oteryn:quest.outfit_and_addon_quests','initial':0,
        'min':0,'max':2147483647,'writable':True,'sources':[ref_source]})
    backpack=citizen_rook+'addon_backpack_rook'
    mission=citizen_rook+'mission_backpack_rook'
    timer=citizen_rook+'addon_backpack_rook_timer'
    states=[storage(backpack,-1,1,0),storage(mission,0,1,4)]
    add('l6-citizen-rook-backpack-start','citizen_outfits_quest_on_rook','amber',['yes'],states,topic=2)
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests','storage':[],
        'state_ops':[{'track':rook_ref,'from':{'kind':'between','minimum':0,'maximum':2147483646},
            'effect':{'kind':'add','value':1}}]}]
    for words,before,after in [(['addon','outfit'],0,1),(['backpack','minotaur','leather'],1,2)]:
        offers.append({'npc':'oteryn:npc.amber','keywords':words,'topic':before,'next_topic':after,
            'storage':states,'reply':['Would you bring 100 minotaur leathers for a handmade backpack?']})
    states=[storage(backpack,1,2),storage(mission,1,2)]
    add('l6-citizen-rook-backpack-materials','citizen_outfits_quest_on_rook','amber',['yes'],states,take=[(5878,100)],topic=3)
    callbacks[-1]['state_ops']=[{'track':timer,'from':{'kind':'any'},'effect':{'kind':'set_now'}}]
    offers.append({'npc':'oteryn:npc.amber','keywords':['addon','outfit','backpack','leather'],'topic':0,'next_topic':3,
        'storage':states,'reply':['Did you bring 100 minotaur leathers? Your backpack takes two hours to make.']})
    add('l6-citizen-rook-backpack','citizen_outfits_quest_on_rook','amber',['addon','outfit','backpack'],
        [storage(backpack,2,3),storage(mission,2,4)],outfit=('citizen',1))
    callbacks[-1]['state_ops']=[{'track':timer,'from':{'kind':'elapsed_at_least','seconds':7201},'effect':{'kind':'add','value':0}}]
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.outfit_and_addon_quests','storage':[storage(rook_ref,0,0,2147483647)]}]
    hat=citizen_rook+'addon_hat_rook'
    hat_mission=citizen_rook+'mission_hat_rook'
    states=[storage(hat,-1,1,0),storage(hat_mission,0,1)]
    add('l6-citizen-rook-hat-start','citizen_outfits_quest_on_rook','norma',['yes'],states,topic=1)
    offers.append({'npc':'oteryn:npc.norma','keywords':['addon','outfit','hat'],'topic':0,'next_topic':1,
        'storage':states,'reply':['Shall I teach you how to make a feather hat?']})
    states=[storage(hat,1,2),storage(hat_mission,1,0)]
    add('l6-citizen-rook-hat','citizen_outfits_quest_on_rook','norma',['yes'],states,
        take=[(3374,1),(5902,50),(5890,100)],outfit=('citizen',2),topic=2)
    offers.append({'npc':'oteryn:npc.norma','keywords':['addon','outfit','hat'],'topic':0,'next_topic':2,
        'storage':states,'reply':['Have you brought a legion helmet, 50 honeycombs and 100 chicken feathers?']})
