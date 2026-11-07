#!/usr/bin/env python3
"""Bind First Dragon sacrifices and lava travel using existing quest owners."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
PREFIX='oteryn:quest-transition/questpc3/first-dragon-lava/'
BASE='oteryn:quest-progress/quest/u11_02/the_first_dragon/'
QUEST='oteryn:quest.the_first_dragon_quest'
OUTER=(33047,32712,-3)
INNER=(31994,32390,-9)
def pos(p):return dict(zip(('x','y','floor'),p))
def storage(name,low,high,value,**extra):return dict(track=BASE+name,minimum=low,maximum=high,set=value,preserve=False,**extra)
def row(name,at,source):
 return dict(key=PREFIX+name,quest=QUEST,trigger='use',selector=dict(position=pos(at),unique=None,action=None,appearance=25160),lever=None,reversible=False,passages=[],teleport=None,offerings=[],storage=[],item_on_target=None,return_on_failure=False,failure_message=None,success_message=None,effects={},sources=['crystal:201189f871d23597b82caf88fd611454a2ddefb8:scripts/quests/the_first_dragon/'+source+' canonical quest-state declaration'])
def rows():
 result=[]
 for number,name in enumerate(('scale','tooth','horn','bones'),24939):
  r=row('sacrifice-'+name,OUTER,'actions_sacrifice_items.lua');r['trigger']='use_carried_on_target'
  r['storage']=[storage(name,-1,0,1),storage('access_cave',0,3,0,increment=1)]
  r['item_on_target']=dict(item=f'oteryn:item.tibia.i{number}',count=1,replacement=None,target_replacement=None)
  r['carried_use']=dict(actor_region=dict(minimum=pos((33046,32711,-3)),maximum=pos((33048,32713,-3))),time_window=dict(from_minute=0,to_minute=1440))
  r['success_message']="You're plunging the trophy into the lava."
  r['sources'].append('data-global/scripts/quests/the_first_dragon/actions_sacrifice_items.lua:1-47 git-blob=0a9cd03ccaf771390c2c43b5470333229d403fd7 sha256=cc37c76856a23734d61c7d98b4c7f757378f7a99c2c0f1b8d9662f20c7b0b61e');result.append(r)
 for name,at,destination in [('enter',OUTER,(31994,32391,-9)),('return',INNER,(33047,32713,-3))]:
  r=row(name,at,'actions_lair_entrance.lua');gate=storage('access_cave',4,2147483647,0);gate['preserve']=True;r['storage']=[gate];r['teleport']=pos(destination)
  r['sources'].append('data-global/scripts/quests/the_first_dragon/actions_lair_entrance.lua:3-21 git-blob=c26663826161ee397f0290fafe62a65306c4b85a sha256=a423081416594a06acd1fe2c72f85a5dd962041c8ea5cc51667247907721b80e');result.append(r)
 return result
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
 target=ROOT/'content/quests/world-callbacks.json';document=json.loads(target.read_text());document['callbacks']=[r for r in document['callbacks'] if not r['key'].startswith(PREFIX)]+rows();document['callbacks'].sort(key=lambda r:r['key']);text=json.dumps(document,indent=2,ensure_ascii=False)+'\n'
 if args.check:
  if target.read_text()!=text:raise SystemExit('First Dragon lava authoring drift')
 else:target.write_text(text)
if __name__=='__main__':main()
