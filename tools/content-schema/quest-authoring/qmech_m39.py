"""M39 source NPC offers, carried guards and knowledge hand-ins; reuse L2 be6626b4."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
NPCS = ('cael', 'curos', 'zalamon', 'muriel')
ITEMS = {131, 4852}
SHARD = 'content/quests/npc-callbacks/m39-stories.json'
WORLD_SHARD = 'content/quests/world-callbacks/m39-stories.json'


def append(add, callbacks, offers, declarations, inputs, pin):
    frontier = 'oteryn:quest-progress/quest/u8_54/the_new_frontier/'
    def state(track, before, after, maximum=None, preserve=False):
        return {'track':track,'minimum':before,'maximum':before if maximum is None else maximum,
                'set':after,**({'preserve':True} if preserve else {})}
    def witness(path, lines):
        return f'crystal:{pin}:{path}:{lines}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'
    curos = 'data-global/npc/curos.lua'
    add('l2-alliance-finish', 'an_uneasy_alliance_quest', 'curos', ['mission'],
        [state('oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline',4,5)], give=(10217,1))
    callbacks[-1].update(sources=[witness(curos,'95-101')], reply=["Here is a reward. It's a strange tome that we've found in the lizard ruins. Maybe it is of some value for you or your allies."])
    # Cael owns New Frontier's existing counter, rather than a parallel quest flag.
    cael='data-global/npc/cael.lua'
    counter=frontier+'tomeof_knowledge'
    for number in range(1,13):
        before=min(number-1,12)
        states=[state(counter,-1 if number==1 else before,min(number,12),maximum=0 if number==1 else None)]
        unlocks={7:('snake_head_teleport',-1),8:('corruption_hole',-1),10:('zao_palace_doors',0)}
        if number in unlocks:
            name,initial=unlocks[number]
            states.append(state(frontier+name,-1,1,1))
        requires=[{'track':frontier+'mission08','minimum':2,'maximum':2}]
        if 7<=number<=12:
            requires.append({'track':frontier+'mission10_1','minimum':2,'maximum':2})
        add('l2-cael-tome-'+str(number),'the_new_frontier_quest','cael',['yes'],states,
            take=[(10217,1)],topic=number,next_topic=number+20,requires=requires)
        callbacks[-1]['sources']=[witness(cael,'84-128')]
        callbacks[-1]['reply']=['Thank you! I look forward to reading this interesting discovery of yours and learn a few things about Zao.']
        callbacks[-1].update(inventory=[{'item':'oteryn:item.tibia.i10217','minimum':1,'maximum':4294967295}], failure_reply=['You dont have one!'])
        if number==13:
            callbacks[-1].update(repeatable=True,experience=5000)
        offers.append({'npc':'oteryn:npc.cael','keywords':['tome','knowledge'],
            'topic':0,'any_topic':True,'next_topic':number,
            'storage':[state(counter,-1 if number==1 else before,before,maximum=0 if number==1 else None,preserve=True)],
            'requires':requires,'failure_reply':["I'm sorry I'm busy. Speak with Ongulf to get some missions!"],'reply':['Have you found a Tome of Knowledge for me to read?']})
    declarations.append({'writable':True,'key':frontier+'corruption_hole','quest':'oteryn:quest.the_new_frontier_quest',
        'initial':-1,'min':-1,'max':1,'sources':[witness(cael,'118-122')]})
    child='oteryn:quest-progress/quest/u8_54/children_of_the_revolution/'
    add('l2-zalamon-tome','children_of_the_revolution_quest','zalamon',['yes'],
        [state(child+'questline',17,18),state(child+'mission04',5,6)],give=(10217,1),topic=12)
    callbacks[-1]['sources']=[witness('data-global/npc/zalamon.lua','325-335')]
    offers.append({'npc':'oteryn:npc.zalamon','keywords':['mission'],'topic':0,'any_topic':True,
        'next_topic':12,'storage':[state(child+'questline',17,17,preserve=True)],
        'reply':['Have you found the way which will lead us to the great gate?']})

    bone = 'oteryn:quest-progress/quest/u8_1/tibia_tales/into_the_bone_pit'
    warning = 'Indeed there is something to be done, but I need someone more experienced. Come back later if you want to.'
    muriel = 'data-global/npc/muriel.lua'
    if warning not in inputs[muriel].decode():
        raise ValueError('M39 Muriel source refusal drift')
    add('l2-bone-start', 'tibia_tales', 'muriel', ['yes'], [state(bone,-1,1,0)], give=(4852,1), topic=1)
    callbacks[-1].update(minimum_level=35, failure_reply=[warning], sources=[witness(muriel,'61-96')],
        reply=['Great! Here is the container for the bone. Once, I used it to collect ectoplasma of ghosts, but it will work here as well. ...'])
    offers.append({'npc':'oteryn:npc.muriel','keywords':['mission'],'topic':0,'next_topic':1,
        'storage':[state(bone,-1,-1,0,preserve=True)], 'minimum_level':35,'failure_reply':[warning],
        'reply':['Find that pit, dig for a well-preserved human skeleton and conserve a sample in a special container which you receive from me. Are you going to help me?']})
    for name, minimum, maximum, take, give, experience, reply in [
        ('l2-bone-return',1,4294967295,[(131,1)],(6299,1),1000,
         'Excellent! Now I can try to put my theoretical thoughts into practice and find a cure for the symptoms of undead. Here, take this for your efforts.'),
        ('bone-pit-lost-sample',0,0,[],None,None,
         'I am so glad you are still alive. Benjamin found the container with the bone sample inside. Fortunately, I inscribe everything with my name, so he knew it was mine. ...')]:
        add(name,'tibia_tales','muriel',['mission'],[state(bone,2,3)],take=take,give=give)
        callbacks[-1].update(minimum_level=35, failure_reply=[warning],
            inventory=[{'item':'oteryn:item.tibia.i131','minimum':minimum,'maximum':maximum}],
            sources=[witness(muriel,'61-96')],reply=[reply])
        if experience is not None:
            callbacks[-1]['experience']=experience


def shops(inputs,pin):
    path='data-global/npc/cael.lua'
    track='oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge'
    return {'schema':1,'shops':[{'npc':'oteryn:npc.cael','service':'oteryn:service.quest.cael.tomes',
        'sources':[f'crystal:{pin}:{path}:30-35:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'],
        'offers':[{'item':{'family':'Item','key':f'oteryn:item.tibia.i{item}','revision':'definition-r1'},
            'direction':'SellToPlayer','unit_price':price,
            'requirements':[{'quest':'oteryn:quest.the_new_frontier_quest','track':track,
                'initial':0,'minimum':count,'maximum':12}]}
            for item,price,count in [(2965,5000,6),(2966,1000,6),(10216,2500,11),(10288,5000,12)]]}]}


WORLD_BINDINGS = ROOT / 'tools/content-schema/quest-authoring/samples/qmech-m39/world-bindings.json'

def source_paths():
    callbacks = json.loads(WORLD_BINDINGS.read_bytes())['callbacks']
    return sorted({source.split(':',2)[2].rsplit(':sha256:',1)[0].rsplit(':',1)[0]
                   for callback in callbacks for source in callback['sources']})


def worlds(inputs):
    document = json.loads(WORLD_BINDINGS.read_bytes())
    for callback in document['callbacks']:
        for source in callback['sources']:
            descriptor, digest = source.rsplit(':sha256:',1)
            path = descriptor.split(':',2)[2].rsplit(':',1)[0]
            if hashlib.sha256(inputs[path]).hexdigest() != digest:
                raise ValueError('M39 donor world binding drift: '+path)
    return document
