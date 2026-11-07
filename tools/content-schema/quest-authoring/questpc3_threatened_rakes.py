#!/usr/bin/env python3
"""Author five source rake interactions through the fenced carried-target owner."""
import argparse
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-rake/'
BASE='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/'
TARGETS=[(32617,31863),(32616,31864),(32615,31866),(32620,31864),(32619,31865)]


def rows():
    result=[]
    for number,(x,y) in enumerate(TARGETS,1):
        result.append(dict(key=PREFIX+f'stone{number}',quest='oteryn:quest.threatened_dreams_quest',
            trigger='use_carried_on_target',selector=dict(position=dict(x=x,y=y,floor=-7),unique=None,action=None,appearance=None),
            lever=None,reversible=False,passages=[],teleport=None,offerings=[],
            storage=[dict(track=BASE+'grumpy_stone',minimum=1,maximum=1,set=1,preserve=True),
                     dict(track=BASE+f'stone{number}',minimum=-1,maximum=0,set=1,preserve=False)],
            item_on_target=dict(item='oteryn:item.tibia.i3452',count=1,replacement='oteryn:item.tibia.i3452',target_replacement=None),
            return_on_failure=False,failure_message=None,success_message='*scratch* *scratch*',
            carried_use=dict(actor_region=dict(minimum=dict(x=x-1,y=y-1,floor=-7),maximum=dict(x=x+1,y=y+1,floor=-7)),
                             time_window=dict(from_minute=0,to_minute=1440)),
            sources=['tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                     'data-global/scripts/actions/tools/rake.lua:17-44 git-blob=fccb45af39905199322583cadeefbaf36282374d sha256=c3a636bdb1ed7a1ed0670692eab4d6c3e74f5239581c5cd2411f15a17f37fd61',
                     'crystal:201189f871d23597b82caf88fd611454a2ddefb8:npc/grumpy_stone.lua canonical quest-state declaration']))
    return result


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    path=ROOT/'content/quests/world-callbacks.json';d=json.loads(path.read_text())
    d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows();d['callbacks'].sort(key=lambda r:r['key'])
    text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
    if args.check:
        if path.read_text()!=text:raise SystemExit('Threatened rake authoring drift')
    else:path.write_text(text)


if __name__=='__main__':main()
