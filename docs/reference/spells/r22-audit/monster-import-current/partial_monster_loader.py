"""Extract declarative source monster assignments after a top-level load dependency fails.
Never executes skipped encounter/world setup; records every omission as a blocking row.
"""
import re
import hashlib
import subprocess
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
# Resolve only integer leaves, never execute quest libraries or world setup.
REVISIONS = {
 'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
 'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f',
}
TOKENS = re.compile(r"--\[\[.*?\]\]|--[^\n]*|\"(?:\\.|[^\"\\])*\"|'(?:\\.|[^'\\])*'|[A-Za-z_]\w*|[{}=]|[^\s]", re.S)

def integer_leaf(text, root_name, fields):
 """Walk immediate table members; reject expressions and ambiguous leaves."""
 matches = list(re.finditer(r'^' + re.escape(root_name) + r'\s*=\s*', text, re.M))
 if len(matches) != 1:
  raise ValueError('literal table root not unambiguous: ' + root_name)
 table = expression(text, matches[0].end())
 for field in fields:
  tokens = list(TOKENS.finditer(table)); depth = 0; found = []
  for i, token in enumerate(tokens):
   value = token.group()
   if value == '{': depth += 1
   elif value == '}': depth -= 1
   elif depth == 1 and value == field and i + 1 < len(tokens) and tokens[i + 1].group() == '=':
    found.append(tokens[i + 1].end())
  if len(found) != 1:
   raise ValueError('literal table member not unambiguous: ' + field)
  table = expression(table, found[0])
 scalar = re.sub(r'--[^\n]*', '', table).strip().rstrip(',;').strip()
 if not re.fullmatch(r'[0-9]+', scalar):
  raise ValueError('dependency is not an integer literal')
 return int(scalar)

def dependency_text(root, relative):
 path = root / relative
 # Anchor every literal to the existing pinned Git object, including staged copies.
 repo = Path('/workspace/spell-sources') / root.name
 data = subprocess.check_output(['git', '-C', str(repo), 'show', REVISIONS[root.name] + ':' + relative])
 if path.exists() and path.read_bytes() != data:
  raise ValueError('staged literal dependency differs from pinned source: ' + relative)
 return data.decode(), str(path) if path.exists() else relative

def recover_dependencies(lua, root, text, inputs):
 references = sorted(set(re.findall(r'\b(?:CakeQuest\.Items\.[A-Za-z_]\w*|Storage\.Quest\.U12_00\.TheDreamCourts\.DreamScar\.[A-Za-z_]\w*|SoulWarQuest\.goshnarsCrueltyWaveInterval)', text)))
 for reference in references:
  if reference.startswith('CakeQuest.'):
   relative = 'data-global/scripts/lib/a_piece_of_cake_config.lua'
   root_name = 'CakeQuest.Items'; fields = reference.split('.')[2:]
  elif reference.startswith('SoulWarQuest.'):
   directory = 'data-otservbr-global' if root.name == 'canary' else 'data-global'
   relative = directory + '/lib/quests/soul_war.lua'
   root_name = 'SoulWarQuest'; fields = reference.split('.')[1:]
  else:
   relative = 'data-global/lib/core/storages.lua'
   root_name = 'Storage'; fields = reference.split('.')[1:]
  source, path = dependency_text(root, relative)
  value = integer_leaf(source, root_name, fields)
  parts = reference.split('.')
  lua.execute(parts[0] + ' = type(' + parts[0] + ') == "table" and ' + parts[0] + ' or {}')
  for i in range(2, len(parts)):
   key = '.'.join(parts[:i]); lua.execute(key + ' = ' + key + ' or {}')
  lua.execute(reference + ' = ' + str(value))
  inputs.append({'path': path, 'field': reference, 'value': value,
                 'sha256': hashlib.sha256(source.encode()).hexdigest(), 'revision': REVISIONS[root.name]})

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
  recover_dependencies(lua, root, text, inputs)
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
