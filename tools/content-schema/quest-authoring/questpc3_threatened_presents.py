#!/usr/bin/env python3
"""Author the three exact present deliveries from the pinned donor action."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-present/'
BASE='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission04/'
CASES=[('thais',32391,32220,-6),('venore',33001,32060,-5),('carlin',32328,31794,-6)]
def rows():
    result=[]
    for name,x,y,floor in CASES:
        result.append(dict(key=PREFIX+name,quest='oteryn:quest.threatened_dreams_quest',trigger='use_carried_on_target',
            selector=dict(position=dict(x=x,y=y,floor=floor),unique=None,action=None,appearance=None),lever=None,reversible=False,passages=[],teleport=None,offerings=[],
            storage=[dict(track=BASE+'tooth_fairy',minimum=1,maximum=1,set=1,preserve=True),dict(track=BASE+'bed_'+name,minimum=-1,maximum=0,set=1,preserve=False),dict(track=BASE+'presents_placed',minimum=0,maximum=2,set=0,preserve=False,increment=1)],
            item_on_target=dict(item='oteryn:item.tibia.i25302',count=1,replacement=None,target_replacement=None),
            effects=dict(item_reward=dict(item='oteryn:item.tibia.i25303',count=1)),
            return_on_failure=False,failure_message=None,success_message='You carefully place a present on the bed and grab a milk tooth from under the pillow.',
            carried_use=dict(actor_region=dict(minimum=dict(x=0,y=0,floor=-15),maximum=dict(x=65535,y=65535,floor=0)),time_window=dict(from_minute=0,to_minute=1440)),
            sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/threatened_dreams_quest/action_present.lua canonical quest-state declaration',
                     'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                     'data-global/scripts/quests/threatened_dreams_quest/action_present.lua:3-30 git-blob=c4f45316f050502d1ffe4b8a97b817fdcccf62fd sha256=f2360e2a53c2697cf4b90a83f2d3e5971b456af1e57945335625c71276050186']))
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    path=ROOT/'content/quests/world-callbacks.json';d=json.loads(path.read_text());d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows();d['callbacks'].sort(key=lambda r:r['key']);text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
    if args.check:
        if path.read_text()!=text:raise SystemExit('Threatened present authoring drift')
    else:path.write_text(text)
if __name__=='__main__':main()
