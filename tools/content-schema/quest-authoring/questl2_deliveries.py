"""Ahmet/Jerom source hand-ins and the pinned authored three-landmark survey."""
import hashlib
import json
from pathlib import Path


def append(add, storage, callbacks, offers, declarations, inputs, pin):
    steal='oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/steal_from_thieves'
    troll='oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline'
    def witness(npc,lines):
        p=f'data-global/npc/{npc}.lua'
        return f'crystal:{pin}:{p}:{lines}:sha256:{hashlib.sha256(inputs[p]).hexdigest()}'
    def offer(npc,words,topic,next_topic,states,reply):
        offers.append({'npc':'oteryn:npc.'+npc,'keywords':words,'topic':topic,'next_topic':next_topic,'storage':states,'reply':[reply]})
    states=[storage(steal,-1,1,0)]
    add('l2-steal-start','steal_from_thieves_quest','ahmet',['yes'],states,topic=2)
    callbacks[-1]['sources']=[witness('ahmet','118-148')]
    offer('ahmet',['mission','quest'],0,2,states,'Want to earn some quick money?')
    states=[storage(steal,1,3,2)]
    add('l2-steal-return','steal_from_thieves_quest','ahmet',['yes'],states,take=[(235,1)],give=(3031,100),topic=3)
    callbacks[-1]['give']['backpack_outputs']=[{'item':'oteryn:item.tibia.i3725','count':100}]
    callbacks[-1]['sources']=[witness('ahmet','149-159')]
    offer('ahmet',['mission','quest'],0,3,states,'Did you find my stuff?')
    states=[storage(troll,-1,1,0)]
    add('l2-troll-start','troll_sabotage_quest','jerom',['yes'],states,topic=1)
    callbacks[-1]['sources']=[witness('jerom','67-84')]
    offer('jerom',['mission','quest'],0,1,states,'Could you look for this mean beast and find my necklace?')
    states=[storage(troll,1,2)]
    add('l2-troll-necklace','troll_sabotage_quest','chief_grarkharok',['grubokk'],states,give=(7754,1),topic=3)
    callbacks[-1]['sources']=[witness('chief_grarkharok','73-116')]
    for words,before,after,reply in [(['kill','hurt','pain'],0,1,'Me Chief Grarkharok!!'),(['lady','queen','woman','cave','house'],1,2,'You help?? Human know troll lady for Grarkharok??'),(['yes'],2,3,'What name of troll lady??')]:
        offer('chief_grarkharok',words,before,after,states,reply)
    add('l2-troll-return','troll_sabotage_quest','jerom',['mission','quest'],[storage(troll,2,3)],take=[(7754,1)],give=(646,1))
    callbacks[-1]['sources']=[witness('jerom','75-78')]
    # Donor Rotem is a registered NPC shell. Restore the existing authored recipe,
    # with explicit physical map identity and exact repository landmark anchors.
    q='authored.cartography_101_quest';root='oteryn:quest-progress/l2/cartography/'
    here=Path(__file__).resolve().parents[3]
    recipe_path='tools/content-schema/quest-authoring/samples/authored68/recipes.json'
    source=f'repo:{recipe_path}:1:sha256:{hashlib.sha256((here/recipe_path).read_bytes()).hexdigest()}'
    add('l2-cartography-start',q,'rotem_valos',['yes'],[storage(root+'mission',-1,1,0)],give=(22107,1),topic=1)
    callbacks[-1]['sources']=[witness('rotem_valos','1-62'),source]
    offer('rotem_valos',['mission','survey','map'],0,1,[storage(root+'mission',-1,1,0)],'Will you survey the Asura Palace, Dryad Gardens and Edron Vampire Crypt?')
    states=[storage(root+'mission',1,2)]+[storage(root+name,1,1,preserve=True) for name in ['asura','dryad','crypt']]
    add('l2-cartography-return',q,'rotem_valos',['yes'],states,take=[(22107,1)],achievement='oteryn:achievement/cartography_101',topic=2)
    callbacks[-1]['sources']=[witness('rotem_valos','1-62'),source]
    offer('rotem_valos',['mission','survey','map'],0,2,states,'Have you charted all three landmarks?')
