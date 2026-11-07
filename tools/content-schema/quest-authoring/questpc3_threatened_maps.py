#!/usr/bin/env python3
"""Author four atomic source map exchanges without replacing foreign programs."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-reward/questpc3/threatened-maps/'
BASE = 'oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/'
FAIRY, COLLECTION, TREE = (BASE + name for name in ('tooth_fairy','teeth_collection','tired_tree'))
SOURCE = {'tooth_fairy': ('787a128d13ad4f8122cff5bfb0ee105a15c5af35','b34d60fc82f372b04549482a8fde40c14f8f20a067309e0e17ab152a2633e250'),
          'tired_tree': ('183eb1d049ef0f1cb0382909b6dcd1161b89bf92','a0162ecba3b89a4eb215087774bd1929fcf351e8ad62d49881f094d5431794ea')}


def guard(track, low, high, value, preserve=False):
    return dict(track=track,minimum=low,maximum=high,set=value,preserve=preserve)


def documents():
    dialogue=json.loads((ROOT/'content/dialogues/quests-2/threatened-dreams.json').read_text())
    replies={(row['npc'],b['line']):b['reply'] for row in dialogue['npcs'] for b in row['branches']}
    rows=[]
    def add(name,npc,keywords,topic,storage,take,give,reply,line,minimum=None,achievement=None):
        blob,digest=SOURCE[npc]
        row=dict(key=PREFIX+name,quest='oteryn:quest.threatened_dreams_quest',npc='oteryn:npc.'+npc,
                 keywords=keywords,topic=topic,next_topic=0,storage=storage,
                 take=[dict(item=f'oteryn:item.tibia.i{i}',count=n) for i,n in take],
                 give=dict(item=f'oteryn:item.tibia.i{give[0]}',count=give[1]),outfit=None,
                 achievement=achievement,reply=replies['oteryn:npc.'+npc,reply],
                 sources=['tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                          f'data-global/npc/{npc}.lua:{line} git-blob={blob} sha256={digest}'])
        if minimum:row['minimum_level']=minimum
        rows.append(row)
    add('tooth-gifts','tooth_fairy',['yes'],1,[guard(FAIRY,-1,0,1)],[],(25302,3),98,'69-103',30)
    add('tooth-map','tooth_fairy',['mission'],0,[guard(FAIRY,1,1,2)],[(25303,3)],(24943,1),87,'69-103',30)
    add('tooth-collection','tooth_fairy',['yes'],2,[guard(COLLECTION,1,1,2),guard(FAIRY,2,2,2,True)],
        [(i,1) for i in [10196,22649,9685,5893,10275,22052]],(3026,6),121,'119-130',
        achievement='oteryn:achievement/toothfairy_assistant')
    add('tree-map','tired_tree',['seeds of life'],3,[guard(TREE,1,1,2),guard(FAIRY,2,2,2,True)],[],(24944,1),81,'64-87')
    path='content/quests/shared-reward-callbacks.json';d=json.loads((ROOT/path).read_text())
    d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows
    d['callbacks'].sort(key=lambda r:r['key']);result={path:d}
    path='content/quests/shared-reward-offers.json';d=json.loads((ROOT/path).read_text())
    npc='oteryn:npc.tooth_fairy'
    d['offers']=[r for r in d['offers'] if r['npc']!=npc]+[dict(npc=npc,keywords=['mission'],topic=0,next_topic=1,
        minimum_level=30,storage=[guard(FAIRY,-1,0,0,True)],reply=replies[npc,75])]
    result[path]=d
    path='content/dialogues/quests/shared-reward-greetings.json';d=json.loads((ROOT/path).read_text())
    names={r['npc'] for r in rows};d['npcs']=[r for r in d['npcs'] if r['npc'] not in names]
    d['npcs'] += [dict(npc=n,greet='Welcome, |PLAYERNAME|. How can I help you?',farewell='Safe travels, |PLAYERNAME|.',walkaway='Safe travels.') for n in sorted(names)]
    d['npcs'].sort(key=lambda r:r['npc']);result[path]=d
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();drift=[]
    for relative,d in documents().items():
        path=ROOT/relative;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
        if path.read_text()!=text:
            if args.check:drift.append(relative)
            else:path.write_text(text)
    if drift:raise SystemExit('Threatened map authoring drift: '+', '.join(drift))


if __name__=='__main__':main()
