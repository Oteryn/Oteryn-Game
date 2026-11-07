#!/usr/bin/env python3
"""Author the five source feather step tiles and their atomic final story advance."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-feather/'
BASE='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/'
COUNTER='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/feathers_count'
MESSAGES=["You found some more feathers on the grass near the wheat. Now you should have enough for an entire cloak.","You found some beautiful swan feathers in the dustbin.","You found some beautiful swan feathers entangled in the cactus stings.","You found some beautiful swan feathers beneath the dragon skull.","You found some more feaathers under the dead tree. Now you should have enough for an entire cloak."]
def rows():
    result=[]
    for i,message in enumerate(MESSAGES,1):
        result.append(dict(key=PREFIX+str(i),quest='oteryn:quest.threatened_dreams_quest',trigger='step_in',
            selector=dict(position=None,unique=None,action=25023+i,appearance=None),lever=None,reversible=False,passages=[],teleport=None,offerings=[],
            storage=[dict(track=BASE+'troubled_animals',minimum=13,maximum=13,set=14,preserve=False,set_when_at_least=dict(other_effect=1,minimum=4)),dict(track=COUNTER,minimum=0,maximum=4,set=0,preserve=False,increment=1),dict(track=BASE+f'feathers{i}',minimum=-1,maximum=0,set=1,preserve=False)],
            effects=dict(item_reward=dict(item='oteryn:item.tibia.i25244',count=1)),
            return_on_failure=False,failure_message=None,success_message=message,
            sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/threatened_dreams_quest/movement_swan_feathers.lua canonical quest-state declaration',
                     'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                     'data-global/scripts/quests/threatened_dreams_quest/movement_swan_feathers.lua:1-51 git-blob=4688b9d0a5fb77b7c1633b598622563f3326b208 sha256=87e97ad7e527b47bc70a312ec5ea7d74121ddffbbf9fd55c71359dd8a35cd8be']))
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    path=ROOT/'content/quests/world-callbacks.json';d=json.loads(path.read_text());d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows();d['callbacks'].sort(key=lambda r:r['key']);text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
    if args.check:
        if path.read_text()!=text:raise SystemExit('Threatened feather authoring drift')
    else:path.write_text(text)
if __name__=='__main__':main()
