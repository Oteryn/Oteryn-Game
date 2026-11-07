"""Six source Storkus tiers; explicit rank/access bounds and repository Razing reward."""
import hashlib

def append(callbacks, offers, declarations, inputs, pin):
    path='data-global/npc/storkus.lua'
    raw=inputs[path]
    source=f'crystal:{pin}:{path}:99-199:sha256:{hashlib.sha256(raw).hexdigest()}'
    quest='oteryn:quest.vampire_hunter_quest'
    root='oteryn:quest-progress/quest/u8_2/vampire_hunter_quest/'
    for name,maximum in [('rank',6),('door',1)]:
        declarations.append({'key':root+name,'quest':quest,'initial':0,'min':0,'max':maximum,'writable':True,'sources':[source]})
    for index,(count,xp) in enumerate(zip([1,4,5,10,30,50],[1000,5000,10000,20000,50000,100000])):
        state=[{'track':root+'rank','minimum':index,'maximum':index,'set':index+1}]
        if index==5:state.append({'track':root+'door','minimum':0,'maximum':0,'set':1})
        reply=next(line.split('npcHandler:say("',1)[1].split('", npc, creature)',1)[0] for line in raw.decode().splitlines() if 'tokens needed to advance' in line and ({0:'first',1:'second',2:'third',3:'fourth',4:'fifth',5:'last'}[index]+' vampire hunter rank') in line) if index else "Ye' brought the token needed to advance to the first vampire hunter rank. I consider that a fluke, but still, congrats! Let me share some of my experience with ye'."
        callback={'key':f'oteryn:quest-reward/l2-vampire-tier-{index+1}','quest':quest,'npc':'oteryn:npc.storkus','keywords':['yes'],'topic':index+3,'next_topic':0,'storage':state,'requires':[{'track':'oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline','minimum':11,'maximum':25}],'take':[{'item':'oteryn:item.tibia.i8192','count':count}],'give':{'item':'oteryn:item.tibia.i8191','count':1,'contents':[]} if index==5 else None,'outfit':None,'achievement':'oteryn:achievement/razing' if index==5 else None,'experience':xp,'reply':[reply],'failure_reply':["Ye' don't have enought tokens."],'sources':[source]}
        callbacks.append(callback)
        offers.append({'npc':callback['npc'],'keywords':['vampire lord token'],'topic':0,'next_topic':index+3,'storage':state,'reply':["Would ye' like to give me vampire tokens?"]})
