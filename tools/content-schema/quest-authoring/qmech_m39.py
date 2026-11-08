"""M39 source NPC offers, carried guards and knowledge hand-ins; reuse L2 be6626b4."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
NPCS = ('muriel',)
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
