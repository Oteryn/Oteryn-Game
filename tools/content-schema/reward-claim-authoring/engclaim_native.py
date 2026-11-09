"""Selected authored M34 allocations. Native placements carry no donor UID."""
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
SHARD = 'content/interactions/reward_claims/engclaim-native.json'
CALLBACKS = 'content/quests/native-reward-callbacks/engclaim.json'
SOURCE = 'docs/reference/content-prep/engclaim-m34/sources.md'

def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()

def track(quest, stage):
    return f'oteryn:quest-progress/authored/{quest}_quest/s{stage}'

# quest, suffix, position, ordinal, appearance, independent outputs,
# reward stage, required preceding stage/value, completion stage, taken inputs.
CASES = [
 ('kingdom_of_kormarak','corpse',(32552,32200,11),1,4240,[(3359,1),(3354,1),(3276,1),(3287,13)],4,[(3,1)],5,[]),
 ('magic_sword','north',(32411,32247,15),1,2472,[(3073,1),(3388,1),(3389,1)],5,[(4,4)],None,[]),
 ('magic_sword','south',(32412,32247,15),1,2472,[(3288,1),(3388,1),(3389,1)],6,[(5,1)],7,[]),
 ('mastermind_potion','wall',(32216,31168,11),1,2473,[(7440,1),(7159,1),(7158,2)],2,[(1,1)],None,[]),
 ('mastermind_potion','fire',(32084,31062,10),2,2472,[(7440,1),(3031,100),(3031,100),(3031,1)],4,[(2,1),(3,1)],5,[]),
 ('mystery_of_the_valley','fragment1',(32463,31092,5),1,10225,[(50336,1)],2,[(1,1)],None,[]),
 ('mystery_of_the_valley','fragment2',(33187,31191,6),1,10225,[(50337,1)],3,[(2,1)],None,[]),
 ('mystery_of_the_valley','fragment3',(33836,31677,4),1,10225,[(50338,1)],4,[(3,1)],None,[]),
 ('mystery_of_the_valley','guardian',(33082,32670,3),1,10225,[(50335,1)],6,[(4,1),(5,1)],6,[(50336,1),(50337,1),(50338,1)]),
]

def records():
    claims, callbacks, tracks = [], [], {}
    for quest,suffix,pos,ordinal,appearance,items,stage,requires,complete,take in CASES:
        quest_key = f'oteryn:quest.authored.{quest}_quest'
        claim_key = f'oteryn:reward-claim.authored.{quest}.{suffix}'
        callback_key = f'oteryn:quest-reward/engclaim-{quest}-{suffix}'
        placement = {'appearance_tibia_id':appearance, 'source_binding':{
            'project_position':dict(zip('xyz',pos)), 'native_entry_ordinal':ordinal},
            'reward_callback':callback_key,'reward':{'items':[{'item':{'family':'Item','key':f'oteryn:item.tibia.i{i}','revision':'definition-r1'},'count':n} for i,n in items]}}
        definition = {'identity':{'key':claim_key,'revision':'engclaim-r1'},
            'definition_profile':'native_authored_v1','readiness':'ready',
            'claim':{'per':'character','repeat':{'kind':'once'}}, 'placements':[placement]}
        digest = hashlib.sha256(canonical({k:definition[k] for k in ('identity','claim','placements')}).rstrip(b'\n')).hexdigest()
        definition['provenance'] = {'pilot_key':claim_key, 'pilot_revision':'engclaim-r1', 'path':SOURCE, 'sha256':hashlib.sha256((ROOT/SOURCE).read_bytes()).hexdigest()}
        claims.append({'definition':definition})
        writes = sorted({stage, *([complete] if complete else [])})
        callback = {'key':callback_key,'quest':quest_key,'npc':'','keywords':[], 'topic':0,'next_topic':0,
            'world_claim':{'key':claim_key,'revision':'engclaim-r1','authority_sha256':digest},
            'storage':[{'track':track(quest,s),'minimum':0,'maximum':0,'set':1} for s in writes],
            'requires':[{'track':track(quest,s),'minimum':n,'maximum':n} for s,n in requires],
            'take':[{'item':f'oteryn:item.tibia.i{i}','count':n} for i,n in take], 'present':[],
            'give':{'item':f'oteryn:item.tibia.i{items[0][0]}','count':items[0][1],'contents':[],
                'additional':[{'item':f'oteryn:item.tibia.i{i}','count':n} for i,n in items[1:]]},
            'outfit':None,'achievement':None,'reply':['You have found your reward.'],'sources':[SOURCE]}
        if complete: callback['completes'] = True
        if quest == 'magic_sword': callback['minimum_level'] = 100
        if quest in ('mastermind_potion','mystery_of_the_valley'): callback['requires_premium'] = True
        for s,n in requires:
            tracks.setdefault(track(quest,s), {'key':track(quest,s),'quest':quest_key,'initial':0,'min':0,'max':max(n,1),'sources':[SOURCE]})
        for s in writes:
            tracks[track(quest,s)] = {'key':track(quest,s),'quest':quest_key,'initial':0,'min':0,'max':1,'writable':True,'sources':[SOURCE]}
        if quest == 'mystery_of_the_valley':
            for foreign,maximum in [('oteryn:quest-progress/quest/u8_0/the_ice_islands/mission06',8),('oteryn:quest-progress/quest/u8_54/the_new_frontier/mission08',2)]:
                callback['requires'].append({'track':foreign,'minimum':maximum,'maximum':maximum})
        callbacks.append(callback)
    return claims, {'schema':1,'tracks':list(tracks.values()),'callbacks':callbacks}

def extend_outputs(outputs, compact):
    claims,callbacks=records()
    outputs[SHARD] = compact({'family':'RewardClaim','schema':'OTERYN_REWARD_CLAIM_SHARD/v1','shard':{'count':len(claims),'start':0,'end':len(claims)-1,'index':5},'records':claims})
    outputs[CALLBACKS] = canonical(callbacks).decode()
    index_path='content/interactions/reward_claims/index.json'
    index=json.loads(outputs[index_path]);index['shards'].append(SHARD)
    index['native_authored_records']=len(claims)
    index['record_count']+=len(claims);index['readiness']['ready']+=len(claims)
    outputs[index_path]=compact(index)
    return claims
