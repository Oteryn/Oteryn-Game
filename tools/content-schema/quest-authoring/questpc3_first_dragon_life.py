#!/usr/bin/env python3
"""Bind the three distinct source life-site visits through existing StepIn owners."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/first-dragon-life/'
BASE='oteryn:quest-progress/quest/u11_02/the_first_dragon/'
QUEST='oteryn:quest.the_first_dragon_quest'
SITES=[('desert_tile',"You enter the beautiful oasis. By visiting this sacred site you're infused with the power of water bringing life to the desert."),('stone_sculpture_tile',"You enter the circle of trees and flowers. By visiting this sacred site you're infused with the power of nature and plants."),('suntower_tile',"You entered the suntower of Ab'dendriel. By visiting this sacred site you're infused with the power of the life-giving sun.")]
def rows():
 result=[]
 for action,(flag,message) in enumerate(SITES,25002):
  result.append(dict(key=PREFIX+flag,quest=QUEST,trigger='step_in',selector=dict(position=None,unique=None,action=action,appearance=None),lever=None,reversible=False,passages=[],teleport=None,offerings=[],item_on_target=None,return_on_failure=False,failure_message=None,success_message=message,effects={},storage=[dict(track=BASE+flag,minimum=-1,maximum=0,set=1,preserve=False),dict(track=BASE+'secrets_counter',minimum=0,maximum=2147483646,set=0,preserve=False,increment=1)],sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/the_first_dragon/movements_zorvorax_secrets.lua canonical quest-state declaration','data-global/scripts/quests/the_first_dragon/movements_zorvorax_secrets.lua:1-42 git-blob=36772967418a88dad3de4ca43c7743edff47944b sha256=e7a353f54abc2012643984127a9ce22821f28c21fbdd053a20641eb6d6e68efb']))
 return result
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
 target=ROOT/'content/quests/world-callbacks.json';document=json.loads(target.read_text());document['callbacks']=[r for r in document['callbacks'] if not r['key'].startswith(PREFIX)]+rows();document['callbacks'].sort(key=lambda r:r['key']);text=json.dumps(document,indent=2,ensure_ascii=False)+'\n'
 if args.check:
  if target.read_text()!=text:raise SystemExit('First Dragon life-site authoring drift')
 else:target.write_text(text)
if __name__=='__main__':main()
