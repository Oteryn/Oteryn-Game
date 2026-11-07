"""Selected Newhaven recipe on the shared bounded NPC reward owner.

The recipe deliberately selects ten Trolls, Guardian Ring and 1600XP instead
of Crystal's superseded twenty-Muglex/3052 branch. Source strings remain intact.
"""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUEST = 'oteryn:quest.newhaven_quest'
PREFIX = 'oteryn:quest-progress/newhaven/'
NPC = 'oteryn:npc.gustavo_the_guard'


def storage(name, minimum, maximum, value, preserve=False):
    return dict(track=PREFIX+name, minimum=minimum, maximum=maximum,
                set=value, **({'preserve': True} if preserve else {}))


def extend(callbacks, offers, greetings):
    path = 'content/quests/definitions/quests-00200-00299.json'
    raw = (ROOT/path).read_bytes()
    rows = json.loads(raw)['records']
    definition, = [r['definition'] for r in rows if r['definition']['identity']['key'] == QUEST]
    recipe = definition['oteryn_recipe']['payload']['recipe']
    if (recipe['requirements']['min_level'] != 2
            or recipe['stages'][2]['count'] != 10
            or recipe['stages'][2]['targets'] != ['Troll']
            or [(r['kind'], r['count']) for r in recipe['reward_intents']]
               != [('item',1),('item',1),('experience',1600)]
            or [r['item']['key'] for r in recipe['reward_intents'] if r['kind']=='item']
               != ['oteryn:item.tibia.i52732','oteryn:item.tibia.i3572']):
        raise ValueError('NEWHAVEN_SELECTED_RECIPE_DRIFT')
    source = f'oteryn:selected-recipe:{path}:/records/78/definition:sha256:{hashlib.sha256(raw).hexdigest()}'
    def add(name, words, states, reply, give=None, take=(), experience=None, completes=False):
        row = dict(key='oteryn:quest-reward/newhaven-'+name, quest=QUEST,
            npc=NPC, keywords=words, topic=0, next_topic=0, storage=states,
            take=[dict(item=f'oteryn:item.tibia.i{i}', count=n) for i,n in take],
            give=dict(item=f'oteryn:item.tibia.i{give}',count=1) if give else None,
            outfit=None,achievement=None,reply=[reply],sources=[source],minimum_level=2)
        if experience is not None:
            row['experience'] = experience
        if completes:
            row['completes'] = True
        callbacks.append(row)
    add('welcome',['welcome','newhaven','training'],[storage('stage',-1,-1,0)],
        'Welcome to Newhaven. Visit the training school and the village supply point, then return for your defence task.')
    add('training-ready',['mission','training'],[
        storage('stage',0,0,1),storage('school',1,1,1,True),
        storage('supply',1,1,1,True),storage('native_run',-1,-1,0)],
        'Your tour is complete. Defeat ten Trolls on the training route, then report back.')
    add('guardian',['mission','report'],[
        storage('stage',1,1,2),storage('native_run',0,0,1),
        storage('troll_kills',10,10,10,True)],
        'The village is safer. Take this guardian ring. Recover the Book of Wisdom and bring it to me.',
        give=52732,experience=800)
    add('wisdom',['book','wisdom'],[storage('stage',2,2,3)],
        'Thank you for the Book of Wisdom. Take this scarf and the rest of your training experience.',
        give=3572,take=[(51977,1)],experience=800,completes=True)
    source_path = 'tools/content-schema/quest-authoring/batches/quests4/newhaven.json'
    document = json.loads((ROOT/source_path).read_bytes())
    npc, = [n for n in document['npc_source']['entries'] if n['npc_source_name']=='gustavo_the_guard']
    text = npc['full_source_lua']
    # Exact already-committed donor bytes, not a fabricated default greeting.
    if hashlib.sha256(text.encode()).hexdigest() != npc['source_blob_sha256']:
        raise ValueError('NEWHAVEN_GUSTAVO_SOURCE_PIN')
    import re
    messages = {}
    for key, constant in [('greet','GREET'),('farewell','FAREWELL'),('walkaway','WALKAWAY')]:
        matches = re.findall(r'npcHandler:setMessage\(MESSAGE_'+constant+r', "([^"\n]+)"\)',text)
        if len(matches)!=1:
            raise ValueError('NEWHAVEN_GUSTAVO_GREETING')
        messages[key] = matches[0]
    greetings.append(dict(npc=NPC,**messages,sources=[
        'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gustavo_the_guard.lua:130-132:sha256:'+npc['source_blob_sha256']]))
