"""Extract declarative source monster assignments after a top-level load dependency fails.
Never executes skipped encounter/world setup; records every omission as a blocking row.
"""
import re
from pathlib import Path
from lupa.luajit21 import LuaRuntime
import canary_batch as cb
ORIGINAL=cb.load_monster
RECOVERIES={}
def expression(text,start):
 p=start
 while p<len(text) and text[p].isspace() and text[p]!='\n':p+=1
 if p>=len(text):return ''
 if text[p]!='{':return text[p:text.find('\n',p) if '\n' in text[p:] else len(text)].strip()
 depth=0;quote=None;i=p
 while i<len(text):
  c=text[i]
  if quote:
   if c=='\\':i+=2;continue
   if c==quote:quote=None
  elif c in ('"',"'"):quote=c
  elif text[i:i+2]=='--':
   if text[i+2:i+4]=='[[':
    end=text.find(']]',i+4)
    if end<0:raise ValueError('unterminated long comment')
    i=end+2;continue
   end=text.find('\n',i)
   i=len(text) if end<0 else end;continue
  elif c=='{':depth+=1
  elif c=='}':
   depth-=1
   if depth==0:return text[p:i+1]
  i+=1
 raise ValueError('unbalanced source table')
def load(path,errors=None):
 try:return ORIGINAL(path,errors)
 except Exception as exc:
  text=path.read_text();match=re.search(r'^local mType = Game\.createMonsterType\("([^"\n]+)"\)',text,re.M)
  if not match or not re.search(r'^mType:register\(monster\)',text,re.M):raise
  lua=LuaRuntime(unpack_returned_tuples=True);registered,callbacks=lua.execute(cb.LUA_PRELUDE)
  lua.execute(match.group(0).replace('local mType','mType',1)+'\nmonster = {}')
  # A literal declared by the exact pinned SoulWar library; does not evaluate quest logic.
  inputs=[]
  root=next(p for p in path.parents if p.name in ('canary','crystal'))
  libs=[root/'data-otservbr-global/lib/quests/soul_war.lua',root/'data-global/lib/quests/soul_war.lua']
  for lib in libs:
   if lib.exists() and 'SoulWarQuest.goshnarsCrueltyWaveInterval' in text:
    values=re.findall(r'^\s*goshnarsCrueltyWaveInterval\s*=\s*(\d+)\s*[,;]',lib.read_text(),re.M)
    if len(values)!=1:raise ValueError('SoulWar interval literal not unambiguous')
    lua.execute('SoulWarQuest = {goshnarsCrueltyWaveInterval = '+values[0]+'}')
    inputs.append({'path':str(lib),'field':'goshnarsCrueltyWaveInterval','value':int(values[0])})
  omitted=[]
  for assignment in re.finditer(r'^monster\.(\w+)\s*=\s*',text,re.M):
   field=assignment.group(1)
   try:rhs=expression(text,assignment.end());lua.execute('monster.'+field+' = '+rhs)
   except Exception as field_exc:omitted.append({'field':field,'error':str(field_exc).splitlines()[0]})
  monster_value=cb.lua_value(lua.globals().monster)
  bad_elements=[e for e in monster_value.get('elements',[]) if isinstance(e.get('type'),str) and not e['type'].startswith('@COMBAT_')]
  if bad_elements:
   omitted.append({'field':'elements','error':'Raw source resistance types are string literals, not engine enum values; conversion of this field remains unresolved','raw_source_value':monster_value['elements']})
   lua.execute('monster.elements = nil')
  lua.execute('mType:register(monster)')
  RECOVERIES[str(path)]={'original_load_error':str(exc).splitlines()[0],'omitted_fields':omitted,'source_literal_dependencies':inputs,'mode':'declarative_assignments_only_encounter_setup_not_executed'}
  if errors is not None:errors.append('Authoring partial extraction; encounter setup not executed: '+str(exc).splitlines()[0])
  callback_names=set(re.findall(r'^mType\.(\w+)\s*=\s*function',text,re.M))
  return registered['name'],cb.lua_value(registered['monster']),{k:'function' for k in callback_names}
