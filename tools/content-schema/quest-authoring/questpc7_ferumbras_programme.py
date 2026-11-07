"""Selected solo seals use actual arenas and the original physical reward chest."""
import hashlib
import json
import tarfile

QUEST='oteryn:quest.ferumbras_ascension_quest'
TRACK='oteryn:quest-progress/questpc7/ferumbras/programme'
PORTALS='oteryn:quest-progress/questpc7/ferumbras/passages'
PIN='9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
# Native-clear cells beside the retained donor arena anchors; no forced solid spawn.
ARENAS=[
 ('plagirath',(33172,31501,13),(33173,31504,13),(33174,31504,10534)),
 ('zamulosh',(33643,32756,11),(33644,32760,11),(33645,32760,20714)),
 ('mazoran',(33584,32688,14),(33584,32693,14),(33583,32693,7063)),
 ('razzagorn',(33422,32465,14),(33419,32467,14),(33420,32467,10548)),
 ('ragiaz',(33481,32336,13),(33479,32339,13),(33479,32340,20375)),
 ('tarbaz',(33459,32844,11),(33459,32848,11),(33460,32848,10549)),
 ('shulgrax',(33485,32786,13),(33485,32791,13),(33486,32791,19273)),
 ('ferumbras_mortal_shell',(33392,31473,14),(33392,31479,14),(33393,31479,9857)),
]

def position(p):return {'x':p[0],'y':p[1],'floor':-p[2]}
def track(i):return 'oteryn:quest-progress/questpc7/ferumbras/seal_'+str(i)
def storage(key,low,high,value,preserve=False):
    row={'track':key,'minimum':low,'maximum':high,'set':value}
    if preserve:row['preserve']=True
    return row

def build(root):
    path='content/quests/definitions/quests-00000-00099.json';raw=(root/path).read_bytes()
    index,definition=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe=definition['oteryn_recipe']['payload']['recipe']
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('collect',10),('use',1)]+[('kill',1)]*8+[('complete',1)]:
        raise ValueError('QUESTPC7_FERUMBRAS_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    donor={}
    with tarfile.open(root/'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest=json.load(corpus.extractfile('corpus-manifest.json'))
        for name in ['actions_reward.lua','movements_entrance.lua']+[('actions_ferumbras_lever.lua' if i==7 else 'actions_'+a[0]+'.lua') for i,a in enumerate(ARENAS)]:
            source='data-global/scripts/quests/ferumbras_ascension/'+name
            row=next(r for r in manifest['files'] if r['source']=='crystalserver' and r['revision']==PIN and r['path']==source)
            body=corpus.extractfile(row.get('blob_path',row.get('cache_path'))).read()
            if hashlib.sha256(body).hexdigest()!=row['sha256']:raise ValueError('QUESTPC7_FERUMBRAS_DONOR_DRIFT')
            donor[name]=f"crystal:{PIN}:{source}:1:sha256:{row['sha256']}"
    def map_source(p):
        path=f'content/world/placements/region-z{-p["floor"]:02}-x{p["x"]//256}-y{p["y"]//256}.b3'
        return path+':1:sha256:'+hashlib.sha256((root/path).read_bytes()).hexdigest()
    def callback(name,trigger,p,appearance,state,message,source):
        return {'key':'oteryn:quest-callback/questpc7-ferumbras-'+name,'quest':QUEST,'trigger':trigger,
            'selector':{'position':p,'appearance':appearance,'unique':None,'action':None},
            'lever':None,'reversible':False,'passages':[],'teleport':None,'storage':state,
            'minimum_level':None,'experience':None,'monk_reward':None,'offerings':[],
            'item_on_target':None,'return_on_failure':False,'failure_message':None,
            'requirements':{'min_level':150,'premium':True,'completed_quests':[]},
            'success_message':message,'sources':[canonical,donor[source],map_source(p)]}
    entry=callback('entry','step_in',position((33275,32390,8)),409,
        [storage(TRACK,-1,-1,1),storage(PORTALS,-1,-1,0),storage(track(0),-1,-1,0)],
        'You have found the infernal entrance. Offer ten Demonic Essences at its ritual flame.', 'movements_entrance.lua')
    ritual=callback('ritual','use',position((33275,32388,8)),1949,[storage(TRACK,1,1,2)],
        'The ten essences empower your chosen god-slaying ritual. The first seal awaits.','movements_entrance.lua')
    ritual['offerings']=[{'position':position((33275,32389,8)),'item':'oteryn:item.tibia.i6499','count':10}]
    ritual['teleport']=position(ARENAS[0][2])
    worlds=[entry,ritual];objectives=[]
    for i,(name,center,arrival,portal) in enumerate(ARENAS):
        source='actions_ferumbras_lever.lua' if i==7 else 'actions_'+name+'.lua'
        p=position((portal[0],portal[1],center[2]))
        transition=callback('passage-'+str(i),'step_in',p,portal[2],
            [storage(TRACK,2,2,2,True),storage(PORTALS,i,i,i+1),storage(track(i),1,1,1,True)],
            'This seal is broken. You may continue the chosen expedition.',source)
        if i<7:transition['storage'].append(storage(track(i+1),-1,-1,0))
        transition['teleport']=position(ARENAS[i+1][2]) if i<7 else position((33388,31410,14))
        worlds.append(transition)
        objectives.append({'key':'oteryn:quest-death/questpc7-ferumbras-'+name,'quest':QUEST,
            'creature':'oteryn:creature.'+name,'stage':TRACK if i==0 else track(i-1),'stage_value':2 if i==0 else 1,
            'counter':track(i),'required':1,'minimum':position((center[0]-8,center[1]-8,center[2])),
            'maximum':position((center[0]+8,center[1]+8,center[2])),
            'guardian':{'position':position(center),'respawn_ms':60000},
            'sources':[canonical,donor[source],map_source(position(center))]})
    reward=callback('reward','use',position((33388,31409,14)),4073,
        [storage(TRACK,2,2,3),storage(PORTALS,8,8,8,True),storage(track(7),1,1,1,True)],
        'Ferumbras is defeated. You earn five Silver Tokens, the complete Rift Warrior outfit and Hat Hunter.','actions_reward.lua')
    reward.update(completes=True,reward_bundle=True,achievements=['oteryn:achievement/hat_hunter'],
        outfit_reward={'outfit':'oteryn:outfit.rift_warrior','addons':3},
        item_reward={'item':'oteryn:item.tibia.i22516','count':5,'contents':[]})
    worlds.append(reward)
    return worlds,objectives
