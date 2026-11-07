#!/usr/bin/env python3
"""Author three animal and stone NPC exchanges without replacing foreign programs."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PREFIX = 'oteryn:quest-reward/questpc3/threatened-animals/'
BASE = 'oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/'
ANIMALS = 'oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/troubled_animals'
STONE = BASE + 'grumpy_stone'
GREETINGS = {
    'oteryn:npc.ikassis': "Nature's blessing, traveler!",
    'oteryn:npc.a_swan': 'I salute you, mortal being.',
    'oteryn:npc.grumpy_stone': "Please don't bother me, human being. I have my own {problems}.",
}
SOURCE = {'ikassis': ('b088ddfd72fa4608ecc1de64e4d9bd042ab8ae42', 'be3c921fb5987d8d1612b6162abd1a957965f4097558e5fd9a620a49e3f50592'), 'a_swan': ('c2af76c8ffbb1e593b025cb421f8c32e625058a1', '4e087a3c28fabd6f339d64003db609921b7c99eb69a63f669b5a9642e53abc54'), 'grumpy_stone': ('3ae5213cd071089acc9b1b5546a6db666ff4bcdd', 'de0740cf81fed12bbeccabd6ee38c1d649d5c4f0e79c7c4658fbc91826db7b95')}

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
                 give=dict(item=f'oteryn:item.tibia.i{give[0]}',count=give[1]) if give else None,outfit=None,
                 achievement=achievement,reply=replies['oteryn:npc.'+npc,reply],
                 sources=['tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                          f'data-global/npc/{npc}.lua:{line} git-blob={blob} sha256={digest}'])
        if minimum:row['minimum_level']=minimum
        rows.append(row)
    add('ikassis-talisman','ikassis',['yes'],1,[guard(ANIMALS,4,4,5)],[],(25295,1),79,'77-85')
    add('swan-feathers','a_swan',['mission'],0,[guard(ANIMALS,14,14,15)],[(25244,5)],None,69,'65-75')
    add('stone-map','grumpy_stone',['mission'],0,[guard(STONE,1,1,2)]+[guard(BASE+f'stone{i}',1,1,1,True) for i in range(1,6)],[],(24945,1),83,'76-88')
    path='content/quests/shared-reward-callbacks.json';d=json.loads((ROOT/path).read_text())
    d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows
    d['callbacks'].sort(key=lambda r:r['key']);result={path:d}
    path='content/dialogues/quests/shared-reward-greetings.json';d=json.loads((ROOT/path).read_text())
    names={r['npc'] for r in rows};d['npcs']=[r for r in d['npcs'] if r['npc'] not in names]
    d['npcs'] += [dict(npc=n,greet=GREETINGS[n],farewell="Nature's blessing!" if n.endswith('.grumpy_stone') else 'Good bye, |PLAYERNAME|!',walkaway='How rude!') for n in sorted(names)]
    d['npcs'].sort(key=lambda r:r['npc']);result[path]=d
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();drift=[]
    for relative,d in documents().items():
        path=ROOT/relative;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
        if path.read_text()!=text:
            if args.check:drift.append(relative)
            else:path.write_text(text)
    if drift:raise SystemExit('Threatened animal authoring drift: '+', '.join(drift))


if __name__=='__main__':main()
