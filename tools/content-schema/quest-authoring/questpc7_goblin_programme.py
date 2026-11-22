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

    wiki = 'https://www.tibiawiki.com.br/index.php?title=Task:Rafzan&oldid=401277'
    clocks = []
    for keyword, progress, target, pay, xp, tool in [
        ('advertisement', 'signs', 3, 1, 600, (17668, 3)),
        ('perfume', 'odours', 4, 2, 1000, (17670, 1)),
        ('guards', 'captures', 3, 1, 600, (17669, 1)),
        ('busy', 'urine', 1, 3, 1200, (17671, 1)),
        ('destroy', 'trunks', 3, 3, 1200, (17676, 1)),
        ('kill', 'stalkers', 5, 1, 600, None),
    ]:
        clock = TRACK + progress + '/last-report'
        clocks.append(dict(key=clock, quest=QUEST, initial=0, min=0,
                           max=9223372036854775807, writable=True, sources=[wiki]))
        start = row('accept-' + keyword, ['get ' + keyword],
                    [state('programme', -1, 0, 0), state(progress, -1, -1, 0)],
                    'Your ' + keyword + ' task is ready.')
        start['sources'] = sources + [wiki]
        start['state_ops'] = [dict(track=clock, **{
            'from': {'kind':'elapsed_at_least','seconds':72000},
            'effect': {'kind':'add','value':0},
        })]
        if progress in ('signs', 'trunks'):
            prefix = 'sign' if progress == 'signs' else 'trunk'
            start['storage'].extend(state(f'{prefix}-site-{i}', -1, 1, -1) for i in range(1, 4))
        if progress == 'odours':
            start['storage'].extend(
                dict(track='oteryn:quest-progress/goblin_merchant/' + sample,
                     minimum=0,maximum=1,set=0)
                for sample in ('slug_sample', 'rotworm_sample')
            )
        if tool:
            start['give'] = dict(item=f'oteryn:item.tibia.i{tool[0]}', count=tool[1], contents=[])
            start['per_occurrence_mint'] = True
        callbacks.append(start)
        report = row('report-' + keyword, ['report ' + keyword, keyword], [state(progress, target, target, -1)],
                     'Rafzan pays you for the completed ' + keyword + ' task.')
        if progress == 'signs':
            report['keywords'].append('advertising')
        report['sources'] = sources + [wiki]
        report['state_ops'] = [dict(track=clock, **{
            'from': {'kind':'any'},
            'effect': {'kind':'set_now'},
        })]
        if progress == 'odours':
            report['take'] = [dict(item='oteryn:item.tibia.i17675', count=1)]
        report.update(give=dict(item='oteryn:item.tibia.i3035', count=pay, contents=[]),
                      experience=xp,
                      counter=dict(track=TRACK+'reports', maximum=1000000000000),
                      achievement_progress=[dict(key='oteryn:achievement/task_manager', target=500)])
        callbacks.append(report)
    return clocks
