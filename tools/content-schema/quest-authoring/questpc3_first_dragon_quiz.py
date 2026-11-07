#!/usr/bin/env python3
"""Author Gelidrazah's five-step quiz using existing fenced dialogue owners."""
import argparse,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
NPC='oteryn:npc.gelidrazah_s_thirst';QUEST='oteryn:quest.the_first_dragon_quest'
TRACK='oteryn:quest-progress/quest/u11_02/the_first_dragon/gelidrazah_access'
KEY='oteryn:quest-reward/questpc3/first-dragon-quiz/access'
INTRO=["There are three questions. First: What is the name of the princess who fell in love with a Thaian nobleman during the regency of pharaoh Uthemath? Second: Who is the author of the book ,The Language of the Wolves'? ...","Third: Which ancient Tibian race reportedly travelled the sky in cloud ships? Can you answer these questions?"]
STEPS=[('yes',0,1,INTRO),('yes',1,2,["So I ask you: What is the name of the princess who fell in love with a Thaian nobleman during the regency of pharaoh Uthemath?"]),('tahmehe',2,3,["That's right. Listen to the second question: Who is the author of the book ,The Language of the Wolves'?"]),('ishara',3,4,["That's right. Listen to the third question: Which ancient Tibian race reportedly travelled the sky in cloud ships?"])]
FINAL="That is correct. You satisfactorily answered all questions. You may pass and enter Gelidrazah's lair."
def gate(low,high,value=0,preserve=True):return dict(track=TRACK,minimum=low,maximum=high,set=value,preserve=preserve)
def documents():
 p='content/quests/shared-reward-callbacks.json';d=json.loads((ROOT/p).read_text());d['callbacks']=[r for r in d['callbacks'] if r['key']!=KEY]+[dict(key=KEY,quest=QUEST,npc=NPC,keywords=['svir'],topic=4,next_topic=0,storage=[gate(0,0,1,False)],take=[],give=None,outfit=None,achievement=None,reply=[FINAL],sources=['data-global/npc/gelidrazahs_thirst.lua:49-85 git-blob=beff8d7c6b9eb11bda431127647792996635f052 sha256=e376c7ae56eaef9dcc7ffb63904dea469ae39fbfe2e821f739cdfb4edbfc5de8'])];out={p:d}
 p='content/quests/shared-reward-offers.json';d=json.loads((ROOT/p).read_text());rows=[]
 def offer(word,topic,next_topic,reply,storage=None):return dict(npc=NPC,keywords=[word],topic=topic,next_topic=next_topic,storage=storage or [gate(0,1)],reply=reply)
 rows += [offer(*s) for s in STEPS];rows.append(offer('svir',4,0,[FINAL],[gate(1,1)]))
 valid={(word,topic) for word,topic,_,_ in STEPS}|{('svir',4)}
 for topic in range(5):
  for word in ('yes','tahmehe','ishara','svir','no','wrong'):
   if (word,topic) not in valid:rows.append(offer(word,topic,topic,["I don't know what you are talking about."]))
 d['offers']=[r for r in d['offers'] if r['npc']!=NPC]+rows;out[p]=d
 p='tools/content-schema/npc-authoring/npc_placement_scope.json';d=json.loads((ROOT/p).read_text());d['npcs']=d['npcs']+([NPC] if NPC not in d['npcs'] else []);out[p]=d
 return out
def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--check',action='store_true');args=p.parse_args()
 for relative,d in documents().items():
  path=ROOT/relative;text=json.dumps(d,indent=2,ensure_ascii=False)+'\n'
  if args.check:
   if path.read_text()!=text:raise SystemExit('First Dragon quiz drift: '+relative)
  else:path.write_text(text)
if __name__=='__main__':main()
