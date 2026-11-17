"""First six selected Feaster milestones; bounded source-native collection and purification."""
import hashlib
import json
QUEST='oteryn:quest.feaster_of_souls_quest'
TRACK='oteryn:quest-progress/questpc7/feaster/programme'
PHASE_ROUTE_KEYS = frozenset({
    'oteryn:quest-callback/questpc7-feaster-entry',
    'oteryn:quest-callback/questpc7-feaster-purification-entry',
})
# The complete definition at 2caf6670 is unchanged; the moving shard also holds
# unrelated quests. Only these two closed routes retain that historical witness.
PHASE_DEFINITION_SHA256 = 'e2b60e02a8e07240b63cf05d88567ff18f0b3b7cb595228d3747f48bf9cd9bbe'
PHASE_SOURCE = 'canonical:content/quests/definitions/quests-00100-00199.json:1#/records/41/definition:sha256:57af470e521006a832fbc046639ee4a5a5a455b2c6552335afff0a0131783f2c'
def track(name):return 'oteryn:quest-progress/questpc7/feaster/'+name
def pos(p):return dict(x=p[0],y=p[1],floor=p[2])
def state(key,lo,hi,value,preserve=False,add=None):
    r=dict(track=key,minimum=lo,maximum=hi,set=value)
    if preserve:r['preserve']=True
    if add is not None:r['add']=add
    return r
DATA={'bells': [{'name': 'carlin', 'position': [32408, 31788, -7], 'appearance': 436, 'arrival': [32407, 31788, -7], 'path': 'content/world/placements/region-z07-x126-y124.b3', 'sha256': '71d0f001114257d94cc70fc7bf7405d0e7dc5f05266e71eb87cfab37ba0d43f6'}, {'name': 'ghostlands', 'position': [32177, 31836, -8], 'appearance': 3114, 'arrival': [32176, 31836, -8], 'path': 'content/world/placements/region-z08-x125-y124.b3', 'sha256': 'c470089dbd8903d3be04a5c95ce9bcba46df5eda62ea1401452af54789c09828'}, {'name': 'cathedral', 'position': [32629, 32417, -9], 'appearance': 409, 'arrival': [32628, 32417, -9], 'path': 'content/world/placements/region-z09-x127-y126.b3', 'sha256': 'e3c91c61d2da1374ab7e55826f36bb1bdd140171ec1ace1603b1fa8901181671'}], 'entry': {'position': [32627, 32076, -7], 'appearance': 231}, 'potion': {'position': [32625, 32076, -7], 'appearance': 231}, 'samples_return': {'position': [31915, 32354, -8], 'appearance': 1020}, 'souls_return': {'position': [31917, 32354, -8], 'appearance': 1020}, 'norbert': [32628, 32076, -7], 'norbert_approach': [32627, 32076, -7]}
def build(root):
    import questpc7_props
    props={r['name']:r for r in questpc7_props.SITES}
    path='content/quests/definitions/quests-00100-00199.json';raw=(root/path).read_bytes()
    index,d=next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    definition_bytes = json.dumps(d, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    if hashlib.sha256(definition_bytes).hexdigest() != PHASE_DEFINITION_SHA256:
        raise ValueError('QUESTPC7_FEASTER_DEFINITION_DRIFT')
    stages=d['oteryn_recipe']['payload']['recipe']['stages']
    if [(s['kind'],s['count']) for s in stages[:6]]!=[('talk',1),('use',3),('use',1),('use',1),('use',20),('use',20)]:raise ValueError('QUESTPC7_FEASTER_PREFIX_RECIPE_DRIFT')
    canonical=f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    sources=[canonical,'https://www.tibiawiki.com.br/index.php?title=Feaster_of_Souls_Quest&oldid=421868','https://www.tibiawiki.com.br/index.php?title=Para_Norbert&oldid=419674']
    requirements=dict(min_level=250,premium=True,completed_quests=[])
    def cb(name,trigger,site,storage,reply,target=None):
        return dict(key='oteryn:quest-callback/questpc7-feaster-'+name,quest=QUEST,trigger=trigger,selector=dict(position=pos(site['position']),appearance=site['appearance'],unique=None,action=None),lever=None,reversible=False,passages=[],teleport=None if target is None else pos(target),storage=storage,minimum_level=None,experience=None,monk_reward=None,offerings=[],item_on_target=None,return_on_failure=False,failure_message=None,requirements=dict(requirements),success_message=reply,sources=sources)
    def reward(name,item,count,contents=None):return dict(item='oteryn:item.tibia.i'+str(item),count=count,contents=contents or [])
    def carried(row,item,count=1):row['requirements']['owned_items']=[dict(item='oteryn:item.tibia.i'+str(item),count=count)]
    def consume(row,item):row['item_on_target']=dict(item='oteryn:item.tibia.i'+str(item),count=1,replacement=None,target_replacement=None)
    bells=DATA['bells'];worlds=[cb('entry','step_in',DATA['entry'],[state(TRACK,0,0,0,True),state(track('entry'),-1,-1,1)],'Ring the Death Knell at the three marked grave sites.',bells[0]['arrival'])]
    for i,bell in enumerate(bells):
        row=cb('bell-'+str(i),'use',bell,[state(TRACK,0,0,1 if i==2 else 0,i!=2),state(track('bells'),i,i,i+1)],'The silver Death Knell rings over the grave site.',props['grinder']['arrival'] if i==2 else bells[i+1]['arrival']);carried(row,32582);row['activation']=dict(track=track('bells'),minimum=i,maximum=i);worlds.append(row)
    row=cb('grinder','use',props['grinder'],[state(TRACK,1,1,2)],'You grind the skull into bone meal. Place a grave flower beside the apparatus and distil it.');consume(row,3114);row['item_reward']=reward('meal',32602,1);worlds.append(row)
    row=cb('apparatus','use',props['apparatus'],[state(TRACK,2,2,3)],'The bone meal and grave flower become a potion with a skull cork. Ask Norbert for the scalpel.');consume(row,32602);row['offerings']=[dict(position=pos(props['apparatus']['arrival']),item='oteryn:item.tibia.i3661',count=1)];row['item_reward']=reward('potion',32584,1);worlds.append(row)
    row=cb('potion','use',DATA['potion'],[state(TRACK,3,3,4),state(track('scalpel_given'),1,1,1,True),state(track('samples'),-1,-1,0)],'You drink the potion and enter the Brain Grounds. Use the scalpel on the ulcer twenty times.',props['ulcer']['arrival']);consume(row,32584);carried(row,32225);worlds.append(row)
    row=cb('sample','use',props['ulcer'],[state(TRACK,4,4,4,True),state(track('samples'),0,19,20,add=1)],'You take one personal ulcer probe.');carried(row,32225);row['activation']=dict(track=track('samples'),minimum=0,maximum=19);row['item_reward']=reward('sample',32197,1);worlds.append(row)
    worlds.append(cb('samples-return','step_in',DATA['samples_return'],[state(TRACK,4,4,4,True),state(track('samples'),20,20,20,True),state(track('samples_return'),-1,-1,1)],'Return the twenty personal samples to Para Norbert.',DATA['norbert_approach']))
    worlds.append(cb('purification-entry','step_in',DATA['entry'],[state(TRACK,5,5,5,True),state(track('purification_entry'),-1,-1,1)],'Collect souls at the fixed rift and purify them at the nearby apparatus.',props['soul_source']['arrival']))
    for name,counter,item,site in [('gather-soul','raw_souls',32227,'soul_source'),('purify','purified',32228,'purification')]:
        row=cb(name,'use',props[site],[state(TRACK,5,5,5,True),state(track(counter),0,19,20,add=1)],'You collect a personal lost soul.' if name=='gather-soul' else 'You purify one lost soul.');carried(row,32226);row['activation']=dict(track=track(counter),minimum=0,maximum=19);row['item_reward']=reward(name,item,1)
        if name=='purify':consume(row,32227)
        worlds.append(row)
    worlds.append(cb('souls-return','step_in',DATA['souls_return'],[state(TRACK,5,5,5,True),state(track('raw_souls'),20,20,20,True),state(track('purified'),20,20,20,True),state(track('souls_return'),-1,-1,1)],'Return the twenty purified souls to Norbert.',DATA['norbert_approach']))
    callbacks=[]
    for name,word,before,after,give,take,extra,reply in [
        ('mission','mission',-1,0,32582,[],[state(track('bells'),-1,-1,0)],'Ring this Death Knell at Carlin graveyard, Ghostlands and Dark Cathedral; grind one skull and distil it with a grave flower.'),
        ('scalpel','scalpel',3,3,32225,[],[state(track('scalpel_given'),-1,-1,1)],'Use this scalpel on the ulcer after drinking the potion at the Brain Grounds entrance.'),
        ('probes','probes',4,5,32226,[dict(item='oteryn:item.tibia.i32197',count=20)],[state(track('samples'),20,20,20,True),state(track('raw_souls'),-1,-1,0),state(track('purified'),-1,-1,0)],'The twenty personal probes are accepted. Use this purifier for twenty lost souls.'),
        ('souls','souls',5,6,None,[dict(item='oteryn:item.tibia.i32228',count=20)],[state(track('purified'),20,20,20,True)],'Your twenty purified souls are accepted. The Brain Head investigation may continue.'),
    ]:
        callbacks.append(dict(key='oteryn:quest-reward/questpc7-feaster-'+name,quest=QUEST,npc='oteryn:npc.para_norbert',keywords=[word],topic=0,next_topic=0,storage=[state(TRACK,before,before,after,before==after)]+extra,take=take,give=None if give is None else reward(name,give,1),outfit=None,achievement=None,reply=[reply],requirements=dict(requirements),sources=sources))
    for row in worlds:
        if row['key'] in PHASE_ROUTE_KEYS:
            if row['requirements'] != requirements:
                raise ValueError('QUESTPC7_FEASTER_PHASE_REQUIREMENTS_DRIFT')
            del row['requirements']
            row['minimum_level'] = 250
            row['admission'] = {'premium': True}
            row['requires_world_admission'] = True
            row['sources'] = [PHASE_SOURCE, *sources[1:]]
    return worlds,[],callbacks
