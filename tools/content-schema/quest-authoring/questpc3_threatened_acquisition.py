#!/usr/bin/env python3
"""Retained Threatened Dreams notes source registration; chest links use reward claim tooling."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
KEY='oteryn:quest-transition/questpc3/threatened-acquisition/poacher-notes'
ANIMALS='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/troubled_animals'
NOTES='oteryn:quest-progress/quest/u11_40/threatened_dreams/mission01/poacher_notes'
PATH='content/quests/world-callbacks.json'

def document():
    d=json.loads((ROOT/PATH).read_text())
    row=dict(key=KEY,quest='oteryn:quest.threatened_dreams_quest',trigger='use',
        selector=dict(position=None,unique=None,action=20002,appearance=None),
        lever=None,reversible=False,passages=[],teleport=None,
        storage=[dict(track=ANIMALS,minimum=7,maximum=7,set=7,preserve=True),dict(track=NOTES,minimum=-1,maximum=0,set=1)],
        offerings=[],item_on_target=None,return_on_failure=False,failure_message=None,
        success_message='You have found notes of a poacher.',
        effects=dict(item_reward=dict(item='oteryn:item.tibia.i25242',count=1,contents=[])),
        sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:content/quests/threatened-dreams-npc.json canonical quest-state declaration',
        'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
        'data-global/scripts/quests/threatened_dreams_quest/action_poacher_notes.lua:1-14 git-blob=2814b532928717adb6630c84d384923f3d199328 sha256=9e8b533913ce6e8df365c1ccf9ea9b280f28a2189408c95b849ffaf703736626'])
    d['callbacks']=[r for r in d['callbacks'] if r['key']!=KEY]+[row]
    d['callbacks'].sort(key=lambda r:r['key'])
    return d

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    text=json.dumps(document(),indent=2,ensure_ascii=False)+'\n';path=ROOT/PATH
    if args.check:
        if path.read_text()!=text:raise SystemExit('Threatened acquisition authoring drift')
    else:path.write_text(text)
if __name__=='__main__':main()
