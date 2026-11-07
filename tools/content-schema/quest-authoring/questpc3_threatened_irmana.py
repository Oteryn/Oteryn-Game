#!/usr/bin/env python3
"""Irmana's source-qualified fur offer and exact held-money payment."""
import argparse
import json
from pathlib import Path
from shared_reward_offers import replace_npc_offers
ROOT=Path(__file__).resolve().parents[3]
ANIMALS='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/troubled_animals'
NOTES='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/poacher_notes'
NPC='oteryn:npc.irmana'
KEY='oteryn:quest-reward/questpc3/threatened-irmana/fur'
SOURCE='data-global/npc/irmana.lua:208-218,247-253 git-blob=76f53aa5ae2e04daabf1a8cca74b63f3ad13e751 sha256=08e9d63f771a43f2fa147e58f7fb474deef6b541a3a749258ec9a040c8993c03'

def guard(track,value,after=None):return dict(track=track,minimum=value,maximum=value,set=value if after is None else after,preserve=after is None)

def documents():
    path='content/quests/shared-reward-callbacks.json';d=json.loads((ROOT/path).read_text())
    d['callbacks']=[r for r in d['callbacks'] if r['key']!=KEY]+[dict(key=KEY,quest='oteryn:quest.threatened_dreams_quest',npc=NPC,keywords=['yes'],topic=8,next_topic=0,
        storage=[guard(ANIMALS,7,8),guard(NOTES,1)],take=[],fee=dict(gold=1000,include_bank=False),give=dict(item='oteryn:item.tibia.i25238',count=1),outfit=None,achievement=None,
        reply=['Alright. Here is the fur.'],sources=['tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',SOURCE])]
    d['callbacks'].sort(key=lambda r:r['key']);result={path:d}
    path='content/quests/shared-reward-offers.json';d=json.loads((ROOT/path).read_text());owned_offers=[dict(npc=NPC,keywords=['fur'],topic=0,next_topic=8,storage=[guard(ANIMALS,7),guard(NOTES,1)],reply=[
        'A wolf whelp fur? Well, some months ago a hunter came here - a rather scruffy, smelly guy. I would have thrown him out instantly, but he had to offer some fine pelts. One of them was the fur of a very young wolf. ...',
        'I was not delighted that he obviously killed such a young animal. When I confronted him, he said he wanted to raise it as a companion but it unfortunately died. A sad story. In the end, I bought some of his pelts, among them the whelp fur. ...',
        'You can have it if this is important for you. I would sell it for 1000 gold. Are you interested?'])];d['offers']=replace_npc_offers(d['offers'],NPC,owned_offers);result[path]=d
    path='content/dialogues/quests/shared-reward-greetings.json';d=json.loads((ROOT/path).read_text());d['npcs']=[r for r in d['npcs'] if r['npc']!=NPC]+[dict(npc=NPC,greet='Welcome to the house of fashion, |PLAYERNAME|!',farewell='Good bye, |PLAYERNAME|!',walkaway='How rude!')];d['npcs'].sort(key=lambda r:r['npc']);result[path]=d
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();drift=[]
    for name,d in documents().items():
        path=ROOT/name;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
        if path.read_text()!=text:
            if args.check:drift.append(name)
            else:path.write_text(text)
    if drift:raise SystemExit('Irmana authoring drift: '+', '.join(drift))
if __name__=='__main__':main()
