"""Reuse questpc8's chosen smith cache with the existing main callback/guardian schema."""
import hashlib
import json

QUEST = 'oteryn:quest.giant_smithhammer_quest'
TRACK = 'oteryn:quest-progress/chosen/smith-cache/'
MAP = 'content/world/placements/region-z08-x128-y125.b3'
MAP_SHA = '76c361de85ac9c6306f4e7eb5d52a52117f695a15bed7629b1ed735c789a593f'
PEER = '3b315d18dbb0a3a00149a2a14b9a19fcb42053a1'


def build(root):
    path = 'content/quests/definitions/quests-00100-00199.json'
    raw = (root/path).read_bytes()
    if hashlib.sha256((root/MAP).read_bytes()).hexdigest()!=MAP_SHA: raise ValueError('QUESTPC7_SMITH_NATIVE_ANCHOR_DRIFT')
    index, definition = next((i,r['definition']) for i,r in enumerate(json.loads(raw)['records']) if r['definition']['identity']['key']==QUEST)
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (recipe['requirements']['min_level'],recipe['requirements']['premium'],recipe['requirements']['prerequisites'])!=(0,False,[]): raise ValueError('QUESTPC7_SMITH_REQUIREMENTS_DRIFT')
    if [(s['kind'],s['count']) for s in recipe['stages']] != [('explore',1),('kill',1),('use',1),('complete',1)]:
        raise ValueError('QUESTPC7_REUSED_SMITH_RECIPE_DRIFT')
    sources = [f'canonical:{path}:1#/records/{index}/definition:sha256:{hashlib.sha256(raw).hexdigest()}',
        f'reused:questpc8:{PEER}:content/quests/authored-world-callbacks.json:authored-smith-cache',
        'https://tibia.fandom.com/index.php?curid=4377&oldid=748685',MAP+':sha256:'+MAP_SHA]
    position = lambda x,y: dict(x=x,y=y,floor=-8)
    state = lambda name,before,after: dict(track=TRACK+name,minimum=before,maximum=before,set=after)
    def callback(name,trigger,y,storage):
        return dict(key='oteryn:quest-callback/authored-smith-cache'+name,quest=QUEST,trigger=trigger,
            selector=dict(position=position(32776,y),appearance=2472 if name else None,unique=None,action=None),
            lever=None,reversible=False,passages=[],teleport=None,storage=storage,offerings=[],
            return_on_failure=False,failure_message=None,requirements=dict(min_level=0,premium=False,completed_quests=[]),sources=sources)
    entry = callback('','step_in',32254,[state('stage',-1,1),state('kills',-1,0)])
    kill = state('kills',1,1); kill['preserve'] = True
    reward = callback('-reward','use',32253,[state('stage',1,3),kill])
    reward['selector']['position'] = position(32777,32254)
    reward['completes'] = True
    reward['effects'] = dict(item_reward=dict(item='oteryn:item.tibia.i3208',count=1,contents=[],backpack_outputs=[dict(item='oteryn:item.tibia.i3034',count=1),dict(item='oteryn:item.tibia.i3031',count=100)]))
    objective = dict(key='oteryn:quest-death/authored-smith-guardian',quest=QUEST,creature='oteryn:creature.giant_spider',stage=TRACK+'stage',stage_value=1,counter=TRACK+'kills',required=1,
        minimum=position(32760,32240),maximum=position(32790,32270),guardian=dict(position=position(32775,32254),respawn_ms=90000),sources=sources)
    return [entry,reward],[objective]


def placements(root, generated):
    path='content/world/placements/authored-questpc7-smith.json'; index_path='content/world/placements/index.json'
    worlds,_=build(root); source_path='content/quests/definitions/quests-00100-00199.json'
    packet=dict(schema=1,basis='CHOSEN_OTERYN_APPROXIMATION',quest=dict(key=QUEST,path=source_path,sha256=hashlib.sha256((root/source_path).read_bytes()).hexdigest()),sources=worlds[0]['sources'],props=[dict(key='oteryn:world-prop/questpc7-smith-cache',item='oteryn:item.tibia.i2472',position=dict(x=32777,y=32254,floor=-8),role='Giant Smithhammer cache')])
    raw=(json.dumps(packet,ensure_ascii=False,indent=2)+'\n').encode(); before=generated[index_path]; index=json.loads(before)
    index['authored_props']=sorted([r for r in index.get('authored_props',[]) if r['path']!=path]+[dict(path=path,sha256=hashlib.sha256(raw).hexdigest())],key=lambda r:r['path'])
    rendered=json.dumps(index,ensure_ascii=False,indent=2) if b'\n  "' in before else json.dumps(index,ensure_ascii=False,separators=(',',':'))
    generated[path]=raw; generated[index_path]=(rendered+'\n').encode()
