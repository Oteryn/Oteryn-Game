"""Bind the three Smithhammer boxes to the existing personal cache owner."""
import hashlib
import json

QUEST = 'oteryn:quest.giant_smithhammer_quest'
TRACK = 'oteryn:quest-progress/chosen/smith-cache/'
MAP = 'content/world/placements/region-z08-x128-y125.b3'
RETIRED_PROP_PATH = 'content/world/placements/authored-questpc7-smith.json'
# The QMECH overlay changes only (32769, 32026, 8); the complete 31x31
# Smith area and its three source anchors retain their reviewed map semantics.
MAP_SHA = 'c5d2aefe37c52ffa0648e346bbd928e3291cc9541d1fc6d4fdae3583719318ef'
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
    rewards = []
    for name, x, item, count in [
        ('talon', 32774, 3034, 1), ('hammer', 32776, 3208, 1), ('gold', 32778, 3031, 100),
    ]:
        for last in (False, True):
            progress = dict(track=TRACK+'boxes', minimum=2 if last else 0, maximum=2 if last else 1, set=3 if last else 2)
            if not last:
                progress['add'] = 1
            stage = state('stage', 1, 3) if last else {**state('stage', 1, 1), 'preserve':True}
            reward = callback('-reward-'+name+('-last' if last else ''), 'use', 32253,
                              [stage, kill, state('claimed-'+name, -1, 1), progress])
            reward['selector']['position'] = position(x, 32253)
            reward['activation'] = {k: progress[k] for k in ('track','minimum','maximum')}
            reward['track_initials'] = {TRACK+'boxes': 0}
            if last:
                reward['completes'] = True
            reward['effects'] = dict(item_reward=dict(item=f'oteryn:item.tibia.i{item}',count=count,contents=[]))
            reward['sources'] = sources + ['crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/chest.lua:1170-1186']
            rewards.append(reward)
    objective = dict(key='oteryn:quest-death/authored-smith-guardian',quest=QUEST,creature='oteryn:creature.giant_spider',stage=TRACK+'stage',stage_value=1,counter=TRACK+'kills',required=1,
        minimum=position(32760,32240),maximum=position(32790,32270),guardian=dict(position=position(32775,32254),respawn_ms=90000),sources=sources)
    return [entry,*rewards],[objective]


def placements(root, generated):
    index_path = 'content/world/placements/index.json'
    before = generated[index_path]
    index = json.loads(before)
    index['authored_props'] = [r for r in index.get('authored_props', []) if r['path'] != RETIRED_PROP_PATH]
    rendered = json.dumps(index, ensure_ascii=False, indent=2) if b'\n  "' in before else json.dumps(index, ensure_ascii=False, separators=(',', ':'))
    generated[index_path] = (rendered+'\n').encode()
