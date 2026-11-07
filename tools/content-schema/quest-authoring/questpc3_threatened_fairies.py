#!/usr/bin/env python3
"""Five retained FairyRelease AIDs on the ordinary quest and Channel decay owners."""
import argparse
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[3]
PATH = 'content/quests/world-callbacks.json'
PREFIX = 'oteryn:quest-transition/questpc3/threatened-fairy/'
BASE = 'oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/'
SOURCE = 'data-global/scripts/quests/threatened_dreams_quest/action_fairies_release.lua:1-67 git-blob=96e10ff0166fdc0d6c17fd584721173b973ae560 sha256=73a538f9be2e1afcd0d31632ab68689191229ba4049dc4562caa757587869243'

def rows():
    result = []
    for i in range(5):
        result.append(dict(key=PREFIX+f'{i+1:02}', quest='oteryn:quest.threatened_dreams_quest', trigger='use',
            selector=dict(position=None,unique=None,action=40016+i,appearance=None),
            lever=None,reversible=False,passages=[],teleport=None,
            storage=[dict(track=BASE+'nightmare_intruders',minimum=3,maximum=3,set=3,preserve=True),
                     dict(track=BASE+f'fairy{i+1:02}',minimum=-1,maximum=0,set=1),
                     dict(track=BASE+'fairies_counter',minimum=0,maximum=4,set=0,increment=1)],
            offerings=[],item_on_target=None,return_on_failure=False,failure_message=None,
            success_message="My tainted siblings locked me up in the dark far too long. Now I'm finally free! Thank you, mortal being!",
            effects=dict(temporary_transform=dict(appearance=25797,restore=25796,lifetime_us=30_000_000)),
            sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:content/quests/threatened-dreams-npc.json canonical quest-state declaration',
                     'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',SOURCE]))
    return result

def document():
    d=json.loads((ROOT/PATH).read_text())
    d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows()
    d['callbacks'].sort(key=lambda r:r['key'])
    return d

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    text=json.dumps(document(),indent=2,ensure_ascii=False)+'\n';p=ROOT/PATH
    if args.check:
        if p.read_text()!=text:raise SystemExit('FairyRelease authoring drift')
    else:p.write_text(text)
if __name__=='__main__':main()
