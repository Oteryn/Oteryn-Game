#!/usr/bin/env python3
"""Source hand-ins, carried evidence and final bag through the shared quest owner."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-reward/questpc3/threatened-final/'
COUPLE='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1'
JOURNAL='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple'
BASE='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/'
NIGHTMARE=BASE+'nightmare_intruders'
CHARGES=[BASE+n for n in ['charged_moon_mirror','charged_starlight_vial','charged_sun_catcher']]
SOURCES={'taegen':('bf435e06539adfba12820ba042310d76da4c630e','cd1bd27585798ae5a732a30c5a6f96f4ec6e8400ad4e2854d1dda945b040e74f'),
         'maelyrra':('9a164f275775f456b81f9736a8153f896c7df08b','16acd67b566785af18a6a5cfce6d03a7a41a9f66717596774a3c309eca0eea88')}


def storage(track,low,high,value,preserve=False):
    return dict(track=track,minimum=low,maximum=high,set=value,preserve=preserve)


def documents():
    dialogue=json.loads((ROOT/'content/dialogues/quests-2/threatened-dreams.json').read_text())
    replies={(n['npc'],b['line']):b['reply'] for n in dialogue['npcs'] for b in n['branches']}
    rows=[]
    def add(name,npc,words,topic,following,tracks,take,present,give,reply,lines):
        blob,digest=SOURCES[npc]
        row=dict(key=PREFIX+name,quest='oteryn:quest.threatened_dreams_quest',npc='oteryn:npc.'+npc,
                 keywords=words,topic=topic,next_topic=following,storage=tracks,
                 take=[dict(item=f'oteryn:item.tibia.i{i}',count=n) for i,n in take],
                 give=None if give is None else dict(item=f'oteryn:item.tibia.i{give}',count=1),
                 outfit=None,achievement=None,reply=replies['oteryn:npc.'+npc,reply],
                 sources=['tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                          f'data-global/npc/{npc}.lua:{lines} git-blob={blob} sha256={digest}'])
        if present:row['present']=[dict(item=f'oteryn:item.tibia.i{i}',count=1) for i in present]
        rows.append(row)
    add('taegen-flower','taegen',['mission'],0,0,[storage(COUPLE,2,2,3),storage(JOURNAL,-1,4,4)],[(25782,1)],[],None,122,'121-128')
    add('taegen-herb','taegen',['yes'],2,3,[storage(COUPLE,3,3,4)],[(5953,1)],[],None,160,'158-169')
    add('maelyrra-held-lights','maelyrra',['yes'],7,0,[storage(NIGHTMARE,6,6,7)],[],[25734,25732,25730],None,146,'143-156')
    add('maelyrra-blossom-bag','maelyrra',['yes'],8,0,[storage(NIGHTMARE,7,7,8)]+[storage(t,0,0,0,True) for t in CHARGES],[],[],25780,159,'157-166')
    path='content/quests/shared-reward-callbacks.json';d=json.loads((ROOT/path).read_text())
    d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows
    d['callbacks'].sort(key=lambda r:r['key']);result={path:d}
    path='content/dialogues/quests/shared-reward-greetings.json';d=json.loads((ROOT/path).read_text());names={r['npc'] for r in d['npcs']}
    for npc in sorted({r['npc'] for r in rows}-names):
        d['npcs'].append(dict(npc=npc,greet='Welcome, |PLAYERNAME|. How can I help you?',farewell='Safe travels, |PLAYERNAME|.',walkaway='Safe travels.'))
    d['npcs'].sort(key=lambda r:r['npc']);result[path]=d
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();drift=[]
    for relative,d in documents().items():
        p=ROOT/relative;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
        if p.read_text()!=text:
            if args.check:drift.append(relative)
            else:p.write_text(text)
    if drift:raise SystemExit('Threatened final source drift: '+', '.join(drift))


if __name__=='__main__':main()
