"""Chosen Goblin Merchant introduction and exact source-priced tool bundle."""
import hashlib
import json
import re

QUEST='oteryn:quest.goblin_merchant_quest'
TRACK='oteryn:quest-progress/questpc7/goblin/'
NPC='oteryn:npc.rafzan'
TOOLS=[(17668,3,75),(17669,1,200),(17670,1,400),(17671,1,150),(17676,1,150)]

def append(root,callbacks,inputs,pin):
    path='content/quests/definitions/quests-00100-00199.json'
    raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('talk',1),('use',3),('use',4),('use',3),('use',1),('use',3),('kill',5),('talk',1),('complete',1)]:
        raise ValueError('GOBLIN_SELECTED_RECIPE_DRIFT')
    npc_path='data-global/npc/rafzan.lua';npc=inputs[npc_path]
    prices={int(m[0]):int(m[1]) for m in re.findall(r'clientId\s*=\s*(\d+),\s*buy\s*=\s*(\d+)',npc.decode())}
    if any(prices.get(item)!=price for item,_,price in TOOLS):raise ValueError('GOBLIN_SOURCE_TOOL_PRICE_DRIFT')
    sources=[f'canonical:{path}:1#/records/{index}/definition:sha256:'+hashlib.sha256(raw).hexdigest(),f'crystal:{pin}:{npc_path}:73-90:sha256:'+hashlib.sha256(npc).hexdigest()]
    def state(name,lo,hi,value,preserve=False):
        row=dict(track=TRACK+name,minimum=lo,maximum=hi,set=value)
        if preserve:row['preserve']=True
        return row
    def row(name,words,storage,reply):
        return dict(key='oteryn:quest-reward/questpc7-goblin-'+name,quest=QUEST,npc=NPC,keywords=words,topic=0,next_topic=0,storage=storage,take=[],give=None,outfit=None,achievement=None,reply=[reply],sources=sources)
    callbacks.append(row('intro',['swamp jobs'],[state(name,-1,-1,0) for name in ['programme','signs','odours','captures','urine','trunks','stalkers']],
        'All six swamp jobs are accepted: place three signs, gather four odours, catch three adventurers, apply rat urine, smash three trunks and hunt five marsh stalkers. Ask to buy swamp kit for the source-priced tools.'))
    kit=row('kit',['buy swamp kit'],[state('programme',0,6,0,True),state('kit_bought',-1,-1,1)],'Here are three advertisement signs, a guardcatcher, a perfume gatherer, rat urine and a trunkhammer. The total is 1125 gold.')
    # The pinned client class makes the three signs one legal stack.
    outputs=[{'item':'oteryn:item.tibia.i'+str(item),'count':count} for item,count,_ in TOOLS]
    kit['give']=dict(outputs[0],contents=[],backpack_outputs=outputs[1:])
    kit['fee']=dict(gold=sum(count*price for _,count,price in TOOLS),include_bank=True)
    callbacks.append(kit)
