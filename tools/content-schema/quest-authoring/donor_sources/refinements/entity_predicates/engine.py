"""Operational donor entity predicate specifications, never proven runtime dispatch."""
import hashlib,json,re
from functools import lru_cache
from pathlib import Path
METHODS={'isPlayer','isMonster','isItem','isTeleport','getPlayer','getMonster','getMaster','isInGhostMode'}
CLASSES={'Creature':'src/lua/functions/creatures/creature_functions.cpp','Player':'src/lua/functions/creatures/player/player_functions.cpp','Monster':'src/lua/functions/creatures/monster/monster_functions.cpp','Npc':'src/lua/functions/creatures/npc/npc_functions.cpp','Item':'src/lua/functions/items/item_functions.cpp','Container':'src/lua/functions/items/container_functions.cpp','Teleport':'src/lua/functions/map/teleport_functions.cpp'}

def sha(raw):return hashlib.sha256(raw).hexdigest()
def simple_name(node):return node.get('fields',{}).get('id')if isinstance(node,dict)and node.get('node_type')=='Name'else None

def cpp_mask(text):
 pattern=r'//[^\n]*|/\*[\s\S]*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\''
 return re.sub(pattern,lambda m:''.join('\n'if c=='\n'else ' 'for c in m.group()),text)
def brace_region(text,start):
 visible=cpp_mask(text);opening=visible.find('{',start)
 if opening<0:raise ValueError('Missing C++ definition body')
 depth=0
 for i in range(opening,len(visible)):
  if visible[i]=='{':depth+=1
  elif visible[i]=='}':
   depth-=1
   if depth==0:return start,i+1
 raise ValueError('Unclosed C++ function')

class Corpus:
 def __init__(self,path):
  self.path=Path(path);self.rows={(r['source'],r['revision'],r['path']):r for r in json.loads(self.path.read_text())['files']};self.cache={}
 def read(self,source,path):
  key=(source['source'],source['revision'],path)
  if key not in self.cache:
   row=self.rows[key];p=Path(row['cache_path']);p=p if p.is_absolute()else self.path.parent/p;raw=p.read_bytes()
   if sha(raw)!=row['sha256']or len(raw)!=row['byte_count']or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=row['git_blob_sha1']:raise ValueError('Pinned API Source bytes mismatch')
   self.cache[key]=(row,raw,raw.decode('utf-8'))
  return self.cache[key]
 def witness(self,source,path,start,end):
  row,raw,text=self.read(source,path);bs,be=len(text[:start].encode()),len(text[:end].encode());piece=raw[bs:be]
  return {'source_component_id':':'.join([row['source'],row['revision'],row['path']]),'provenance':{k:row[k]for k in ['source','repository','revision','path','git_blob_sha1','sha256','byte_count']},'byte_start':bs,'byte_end_exclusive':be,'line_start':text[:start].count('\n')+1,'line_end':text[:end].count('\n')+1,'slice_sha256':sha(piece),'source_text':piece.decode('utf-8')}
 def cpp_function(self,source,path,name):
  _,_,text=self.read(source,path);regex=re.compile(r'^\s*(?:static\s+)?[^;()\n]*\b'+re.escape(name)+r'\([^;()\n]*\)\s*(?:const\s*)?(?:override\s*)?\{',re.M)
  matches=list(regex.finditer(text))
  if len(matches)!=1:raise ValueError('C++ function definition not uniquely qualified: '+path+' '+name)
  start,end=brace_region(text,matches[0].start());return self.witness(source,path,start,end)
 def cpp_registration(self,source,owner,method):
  path=CLASSES[owner];_,_,text=self.read(source,path);m=re.search(r'Lua::registerMethod\(L,\s*"'+owner+r'",\s*"'+method+r'",\s*([\w:]+)\);',text)
  if not m:raise ValueError('Missing exact C++ registry '+owner+'.'+method)
  return m.group(1),self.witness(source,path,m.start(),m.end())
 def lua_function(self,source,owner,method):
  path='data/libs/functions/'+owner.lower()+'.lua';_,_,text=self.read(source,path)
  # These bounded predicates have no nested blocks. Exact body shape is checked
  # against the expected return expression, not accepted as arbitrary Lua.
  m=re.search(r'^function '+owner+r'\.'+method+r'\(self\)\s*\n([\s\S]*?)^end\s*$',text,re.M)
  if not m:raise ValueError('Missing exact Lua API definition '+owner+'.'+method)
  body=m.group(1).strip();return body,self.witness(source,path,m.start(),m.end())
 def lineage(self,source):
  out=[]
  for owner,path in CLASSES.items():
   _,_,text=self.read(source,path);m=re.search(r'Lua::registerSharedClass\(L,\s*"'+owner+r'",\s*"([\w]*)"',text)
   if not m:raise ValueError('Missing pinned class inheritance '+owner)
   end=text.find(';',m.start())+1;out.append({'class':owner,'base':m.group(1),'source_ref':self.witness(source,path,m.start(),end)})
  return out

def api_spec(corpus,source,method):
 cases=[];refs=[]
 def lua(owner,expected,operation):
  body,ref=corpus.lua_function(source,owner,method)
  if body!=expected:raise ValueError('Lua API operation mismatch '+owner+'.'+method)
  cases.append({'declaring_class':owner,'implementation_kind':'LUA','operation':operation,'source_ref':ref})
 def cpp(owner,function_suffix,operation,tokens):
  function,registration=corpus.cpp_registration(source,owner,method)
  if function.split('::')[-1]!=function_suffix:raise ValueError('CPP API registry/function identity mismatch')
  body=corpus.cpp_function(source,CLASSES[owner],function)
  if any(token not in body['source_text']for token in tokens):raise ValueError('CPP operational body mismatch '+function)
  operation=dict(operation);operation['native_payload_type_precondition']='CPP_TEMPLATE_PAYLOAD_TYPE_MUST_MATCH_DONOR_USERDATA';operation['type_mismatch_behavior']='UNPROVEN_OUTSIDE_NATIVE_PRECONDITIONS'
  cases.append({'declaring_class':owner,'implementation_kind':'CPP','operation':operation,'registration_ref':registration,'source_ref':body})
 if method in {'isPlayer','isMonster','isItem','isTeleport'}:
  # Actual Lua false defaults are essential: Creature's default isPlayer is not
  # the same implementation as Player's registered C++ isPlayer override.
  if method!='isItem':lua('Creature','return false',{'kind':'RETURN_CONSTANT','value':False})
  else:lua('Creature','return false',{'kind':'RETURN_CONSTANT','value':False})
  if method!='isItem':lua('Item','return false',{'kind':'RETURN_CONSTANT','value':False})
  if method=='isPlayer':cpp('Player','luaPlayerIsPlayer',{'kind':'RETURN_SHARED_USERDATA_VALIDITY','accepted_class':'Player','missing_userdata_result':False},['getUserdataShared<Player>','!= nullptr','Lua::pushBoolean'])
  elif method=='isMonster':cpp('Monster','luaMonsterIsMonster',{'kind':'RETURN_SHARED_USERDATA_VALIDITY','accepted_class':'Monster','missing_userdata_result':False},['getUserdataShared<Monster>','!= nullptr','Lua::pushBoolean'])
  elif method=='isItem':cpp('Item','luaItemIsItem',{'kind':'RETURN_SHARED_USERDATA_VALIDITY','accepted_class':'Item','missing_userdata_result':False},['getUserdataShared<','Item>','!= nullptr','Lua::pushBoolean'])
  else:lua('Teleport','return true',{'kind':'RETURN_CONSTANT','value':True})
 elif method in {'getPlayer','getMonster'}:
  dependency='isPlayer'if method=='getPlayer'else'isMonster'
  lua('Creature','return self:'+dependency+'() and self or nil',{'kind':'RETURN_SAME_RECEIVER_IF_NESTED_PREDICATE_ELSE_NIL','nested_method':dependency,'nested_api_spec_ref':source['source']+':'+source['revision']+':entity-api/'+dependency,'predicate_truthiness':'FALSE_ONLY_FOR_NIL_OR_BOOLEAN_FALSE','true_result':'SAME_RECEIVER_IDENTITY','false_result':'NIL','nested_dispatch_unproven':True})
 elif method=='getMaster':
  cpp('Creature','luaCreatureGetMaster',{'kind':'READ_MASTER_WEAK_REFERENCE_OR_NIL','accepted_class':'Creature','invalid_receiver_result':'NIL','expired_or_absent_master_result':'NIL','valid_master_result':'CREATURE_SHARED_USERDATA_WITH_DYNAMIC_CREATURE_METATABLE'},['getUserdataShared<Creature>','creature->getMaster()','lua_pushnil','setCreatureMetatable'])
  refs.append(corpus.cpp_function(source,'src/creatures/creature.hpp','getMaster'))
  if 'm_master.lock()'not in refs[-1]['source_text']:raise ValueError('Master weak reference implementation mismatch')
 elif method=='isInGhostMode':
  cpp('Creature','luaCreatureIsInGhostMode',{'kind':'READ_VIRTUAL_GHOST_MODE_OR_NIL','accepted_class':'Creature','invalid_receiver_result':'NIL','base_creature_result':False,'player_result':'CURRENT_GHOST_MODE_FLAG'},['getUserdataShared<Creature>','creature->isInGhostMode()','lua_pushnil','pushBoolean'])
  refs.append(corpus.cpp_function(source,'src/creatures/creature.hpp','isInGhostMode'));refs.append(corpus.cpp_function(source,'src/creatures/players/player.hpp','isInGhostMode'))
  if 'return false'not in refs[-2]['source_text']or'return ghostMode'not in refs[-1]['source_text']:raise ValueError('Ghost virtual implementation mismatch')
 else:raise ValueError('Outside entity method scope')
 # Underlying class conversion and dynamic metatable assignment are explicit
 # Source support, not a guess based on a receiver variable name.
 conversion=corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.hpp','getUserdataShared');refs.append(conversion)
 conversion_kind='METATABLE_TEST_AND_INHERITANCE_CHECK'if'luaL_testudata'in conversion['source_text']else'TYPED_SHARED_PAYLOAD_CAST_WITHOUT_METATABLE_CHECK'
 if conversion_kind=='TYPED_SHARED_PAYLOAD_CAST_WITHOUT_METATABLE_CHECK'and'lua_touserdata'not in conversion['source_text']:raise ValueError('Unmodeled userdata conversion helper')
 refs.append(corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.cpp','setCreatureMetatable'))
 refs.append(corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.cpp','setItemMetatable'))
 refs.append(corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.cpp','registerMethod'))
 refs.append(corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.cpp','registerClass'))
 if conversion_kind=='METATABLE_TEST_AND_INHERITANCE_CHECK':refs.append(corpus.cpp_function(source,'src/lua/functions/lua_functions_loader.cpp','checkMetatableInheritance'))
 for owner in ['Creature','Item','Teleport']:
  path='data/libs/functions/load.lua';_,_,load=corpus.read(source,path);target='/libs/functions/'+owner.lower()+'.lua';match=re.search(r'^dofile\(CORE_DIRECTORY \.\. \"'+re.escape(target)+r'\"\)\s*$',load,re.M)
  if not match:raise ValueError('Missing bounded library-load witness '+owner)
  refs.append(corpus.witness(source,path,match.start(),match.end()))
 return {'api_spec_id':source['source']+':'+source['revision']+':entity-api/'+method,'method':method,'explicit_argument_count':0,'userdata_conversion':conversion_kind,'cases':cases,'class_inheritance':corpus.lineage(source),'supporting_source_refs':refs,
  'dispatch_semantics':{'lookup':'LUA_COLON_MEMBER_LOOKUP_USING_CURRENT_RECEIVER_METATABLE_AND_INHERITANCE','candidate_case_selection':'RUNTIME_METHOD_IDENTITY_MUST_MATCH_A_PINNED_CASE','no_matching_case':'CUSTOM_METHOD_OR_MISSING_METHOD_EXECUTION_UNPROVEN','receiver_nil_or_nonindexable':'LUA_MEMBER_LOOKUP_ERROR_BEFORE_CPP_BODY','noncallable_member':'LUA_CALL_ERROR_BEFORE_API_BODY','member_or_class_overrides':'CURRENT_RUNTIME_BINDING_UNPROVEN','argument_order':['READ_RECEIVER_ONCE','LOOKUP_MEMBER','PASS_SAME_RECEIVER_AS_ARGUMENT_1','CALL_NO_EXPLICIT_ARGUMENTS']},
  'status':'SOURCE_SPECIFIED_EXECUTION_UNPROVEN','native_admission':False}

@lru_cache(maxsize=2)
def shared_corpus(path,size,mtime_ns):return Corpus(path)

class Engine:
 def __init__(self,provenance,raw_bytes,ast,corpus_manifest_path,bindings):
  if sha(raw_bytes)!=provenance['sha256']:raise ValueError('Wrong quest Source bytes')
  self.source=provenance;self.raw=raw_bytes;self.text=raw_bytes.decode('utf-8');self.ast=ast;self.bindings=bindings
  if isinstance(corpus_manifest_path,Corpus):self.corpus=corpus_manifest_path
  else:
   manifest_path=Path(corpus_manifest_path).resolve();stat=manifest_path.stat();self.corpus=shared_corpus(str(manifest_path),stat.st_size,stat.st_mtime_ns)
  self.api_specs={}
 def project(self,node,pointer):
  if not isinstance(node,dict)or node.get('node_type')!='Invoke':return None
  actual=self.ast
  try:
   for part in pointer.split('/')[1:]:actual=actual[int(part)]if isinstance(actual,list)else actual[part]
  except (KeyError,IndexError,TypeError,ValueError)as error:raise ValueError('Entity atom pointer outside qualified AST')from error
  if actual!=node:raise ValueError('Entity atom differs from qualified AST node')
  f=node['fields'];method=simple_name(f.get('func'));receiver=simple_name(f.get('source'))
  if method not in METHODS or not receiver or f['args']:return None
  binding=self.bindings.resolve(receiver,pointer+'/fields/source')
  if binding is None:return None
  if method not in self.api_specs:self.api_specs[method]=api_spec(self.corpus,self.source,method)
  if method in {'getPlayer','getMonster'}:
   dependency='isPlayer'if method=='getPlayer'else'isMonster'
   if dependency not in self.api_specs:self.api_specs[dependency]=api_spec(self.corpus,self.source,dependency)
  span=node.get('span')
  if span is None:return None
  bs,be=len(self.text[:span['start_char']].encode()),len(self.text[:span['end_char_exclusive']].encode())
  return {'kind':'source_entity_api_operation','method':method,'receiver':{'symbol':receiver,'lexical_binding':binding,'runtime_class_not_inferred':True},'api_spec_ref':self.api_specs[method]['api_spec_id'],'source_ref':{'ast_pointer':pointer,'byte_start':bs,'byte_end_exclusive':be,'slice_sha256':sha(self.raw[bs:be]),'source_expression':self.raw[bs:be].decode('utf-8')},'call_semantics':{'receiver_evaluated_once':True,'colon_self_first':True,'explicit_arguments':[],'member_lookup_may_error':True,'dispatch_may_be_overridden':True},'status':'SOURCE_SPECIFIED_EXECUTION_UNPROVEN','dispatch_unknown':True,'runtime_activation':False,'native_admission':False}
