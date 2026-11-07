"""Reproduce only the pinned seasonal Cake Golem through existing donor converters.

Uses the installed Lua5.4 shared library through stdlib ctypes; installs nothing.
The bounded shim preserves the exact source MonsterSpell undefined-damage default.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parents[2]
def sha(data):return hashlib.sha256(data).hexdigest()
def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--canary',type=Path,required=True);parser.add_argument('--crystal',type=Path,required=True)
 parser.add_argument('--output',type=Path,required=True);args=parser.parse_args()
 sys.path.insert(0,str(ROOT/'tools/content-schema/monster-authoring'))
 sys.path.insert(0,str(ROOT/'tools/content-migration'))
 import canary_batch as converter
 from creature_admission_stage import Stage,Mapper
 spec=importlib.util.spec_from_file_location('cake_source_lua',Path(__file__).with_name('cake_source_lua.py'))
 lua=importlib.util.module_from_spec(spec);spec.loader.exec_module(lua)
 current=json.loads((ROOT/'tools/content-schema/native-gameplay/seasonal-cake-creature.json').read_bytes())
 source=current['source'];monster=args.crystal/source['path'];config=args.crystal/source['config_path']
 for path,expected in [(monster,source['sha256']),(config,source['config_sha256'])]:
  if sha(path.read_bytes())!=expected:raise ValueError('CAKE_SOURCE_DIGEST')
 def load(path,errors=None):
  if path.resolve()!=monster.resolve():raise ValueError('CAKE_CAPTURE_CLOSED_PATH')
  name,body,callbacks=lua.capture(path,converter.LUA_PRELUDE,config.read_text())
  attack=body['attacks'][1]
  if not(attack.get('type') is None and attack['name']=='combat' and attack['minDamage']==0 and attack['maxDamage']==-40):raise ValueError('CAKE_UNDEFINED_SOURCE_SHAPE')
  attack['type']='@COMBAT_UNDEFINEDDAMAGE'
  return name,body,callbacks
 converter.load_monster=load
 converter.DAMAGE['UNDEFINEDDAMAGE']='untyped'
 objects=converter.load_appearance_objects(args.canary/'data/items/appearances.dat');items=converter.load_items_xml(args.canary/'data/items/items.xml');names,index=converter.name_index(objects,items)
 engine=converter.Converter(args.canary,objects,items,names,index)
 engine.monster_root=args.crystal;engine.monster_dir='data-global/monster';engine.source={'repository':source['repository'],'revision':source['revision']}
 slug,monster,dependencies,catalog,manifest,*_=engine.convert('quests/a_piece_of_cake/cake_golem')
 stage=Stage(Mapper({i:f'oteryn:item.tibia.i{i}' for i in [12141,12143,2886]}))
 stage.stage_dependencies(dependencies,slug);stage.stage_monster(monster,'cake_golem')
 result=dict(current);result['manifest']=manifest;result['stage']={'records':list(stage.records.values()),'profiles':list(stage.profiles.values()),'bindings':stage.bindings}
 if result!=current:raise ValueError('CAKE_CAPTURE_EXISTING_SEALED_INPUT_DRIFT')
 args.output.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
 print('SRV7_CAKE_PINNED_SOURCE_CAPTURE_PASS records=14 profiles=13')
if __name__=='__main__':main()
