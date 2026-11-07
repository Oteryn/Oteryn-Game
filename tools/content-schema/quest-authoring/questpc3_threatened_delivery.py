#!/usr/bin/env python3
"""Author selected-source book and whelp-fur burns without global target mutation."""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/threatened-burn/'
ANIMALS='oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission01/troubled_animals'
CASES=[('book-pallet12648',25235,12648,2,3,'action_poacher_book','5c4c7528ef06f2bc5606a6522e89853fcc84c753','60af5675128e222eeac4b1195e82207a66f59d362819651fb10790e76410eb7b',"You are placing the book on the table, hopefully the poachers will discover it soon."),
       ('book-pallet12649',25235,12649,2,3,'action_poacher_book','5c4c7528ef06f2bc5606a6522e89853fcc84c753','60af5675128e222eeac4b1195e82207a66f59d362819651fb10790e76410eb7b',"You are placing the book on the table, hopefully the poachers will discover it soon."),
       ('whelp-fur',25238,25237,8,9,'action_whelp_fur','bf1b254088733b31ebfffc8fc0957aa9d6f75f30','1b95adb2f1dcb2c722a850a3aff4c889806896b40201abe112ffbda61b7365ad',"You place the whelp fur in the stone's mouth and thus lay it to rest.")]

def rows():
    result=[]
    for name,item,target,before,after,script,blob,digest,message in CASES:
        result.append(dict(key=PREFIX+name,quest='oteryn:quest.threatened_dreams_quest',trigger='use_carried_on_target',
            selector=dict(position=None,unique=None,action=None,appearance=target),lever=None,reversible=False,passages=[],teleport=None,offerings=[],
            storage=[dict(track=ANIMALS,minimum=before,maximum=before,set=after,preserve=False)],
            item_on_target=dict(item=f'oteryn:item.tibia.i{item}',count=1,replacement=None,target_replacement=None),
            return_on_failure=False,failure_message=None,success_message=message,
            carried_use=dict(actor_region=dict(minimum=dict(x=0,y=0,floor=-15),maximum=dict(x=65535,y=65535,floor=0)),time_window=dict(from_minute=0,to_minute=1440)),
            sources=[f'crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/threatened_dreams_quest/{script}.lua canonical quest-state declaration',
                     'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz#trees/crystalserver-9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d.json',
                     f'data-global/scripts/quests/threatened_dreams_quest/{script}.lua:3-19 git-blob={blob} sha256={digest}']))
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    path=ROOT/'content/quests/world-callbacks.json';d=json.loads(path.read_text());d['callbacks']=[r for r in d['callbacks'] if not r['key'].startswith(PREFIX)]+rows();d['callbacks'].sort(key=lambda r:r['key']);text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
    if args.check:
        if path.read_text()!=text:raise SystemExit('Threatened delivery authoring drift')
    else:path.write_text(text)
if __name__=='__main__':main()
