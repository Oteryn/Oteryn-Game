"""Minimal source Brotherhood admission and document USE proving the reusable owner."""
import hashlib

def append(add, storage, callbacks, offers, declarations, inputs, pin):
    root='oteryn:quest-progress/quest/u7_9/'
    rank=root+'brotherhood_outfits/outfits'
    other=root+'nightmare_outfits/outfits'
    door=root+'nightmare_outfits/knightwatch_tower_door'
    claim='oteryn:quest-progress/quest-document/brotherhood/follower'
    npcpath='data-global/npc/the_bone_master.lua'
    docpath='data-global/scripts/quests/dreamers_challenge_quest/actions_documents.lua'
    npcsource=f'crystal:{pin}:{npcpath}:61-99:sha256:{hashlib.sha256(inputs[npcpath]).hexdigest()}'
    docsource=f'crystal:{pin}:{docpath}:27-65:sha256:{hashlib.sha256(inputs[docpath]).hexdigest()}'
    declarations.extend([
        {'key':other,'quest':'oteryn:quest.nightmare_outfits_quest','initial':0,'min':0,'max':4,'sources':[npcsource]},
        {'key':door,'quest':'oteryn:quest.nightmare_outfits_quest','initial':0,'min':0,'max':1,'writable':True,'sources':[npcsource]},
        {'key':claim,'quest':'oteryn:quest.brotherhood_outfits_quest','initial':0,'min':0,'max':1,'writable':True,'sources':[docsource]}])
    requirements=[{'track':other,'minimum':0,'maximum':0}]
    states=[storage(rank,0,1)]
    add('l2-carried-join','brotherhood_outfits_quest','the_bone_master',['yes'],states,give=(768,1),topic=2,requires=requirements,achievement='oteryn:achievement/bone_brother')
    callbacks[-1]['sources']=[npcsource]
    callbacks[-1]['participants']=[{'quest':'oteryn:quest.nightmare_outfits_quest','storage':[storage(door,0,1)]}]
    for words,before,after in [(['join'],0,1),(['yes'],1,2)]:
        offers.append({'npc':'oteryn:npc.the_bone_master','keywords':words,'topic':before,'next_topic':after,'storage':states,'reply':['Do you still want to join the Brotherhood?']})
    add('l2-carried-use','brotherhood_outfits_quest','the_bone_master',['document follower'],[storage(rank,1,1,4,preserve=True),storage(claim,0,1)],take=[(768,1)],outfit=('brotherhood',0),requires=requirements)
    callbacks[-1].update({'carried_item':'oteryn:item.tibia.i768','forbids_outfits':['oteryn:outfit.brotherhood'],'sources':[docsource]})
