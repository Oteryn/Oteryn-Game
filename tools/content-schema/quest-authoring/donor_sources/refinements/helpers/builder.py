"""Closed donor helper contracts. Describes Source evaluation; never evaluates Lua."""
import argparse,base64,gzip,hashlib,json,re,tarfile
from pathlib import Path

def sha(raw):return hashlib.sha256(raw).hexdigest()
def pointer(value,path):
 for part in path.split('/')[1:]:value=value[int(part)]if isinstance(value,list)else value[part]
 return value

def walk(x,path=''):
 if isinstance(x,dict):
  if 'node_type'in x:yield path,x
  for k,v in x.items():yield from walk(v,path+'/'+k)
 elif isinstance(x,list):
  for i,v in enumerate(x):yield from walk(v,path+'/'+str(i))

def name(n):
 if not isinstance(n,dict):return None
 f=n.get('fields',{});kind=n.get('node_type')
 if kind=='Name':return f['id']
 if kind=='Index':
  a,b=name(f['value']),name(f['idx']);return a+'.'+b if a and b else None
 if kind=='Call':return name(f['func'])
 return None

HELPERS={
 'table.contains':{'paths':['data/global.lua','data/libs/functions/tables.lua'],'signature':'table.contains = function(array, value) for _, targetColumn in pairs(array) do if targetColumn == value then return true end end return false end','contract':{'operation':'SOURCE_PAIRS_EQUALITY_ANY','iteration':'pairs(array)','iteration_order':'SOURCE_PAIRS_ORDER_UNSPECIFIED','equality':'LUA_EQUALITY_WITH_METAMETHOD_AND_ERRORS','match_return':True,'miss_return':False,'short_circuit_first_match':True,'argument_evaluation':'SOURCE_LUA_FUNCTION_ARGUMENT_RULES_NOT_REORDERED','errors_preserved':True}},
 'table.find':{'paths':['data/libs/functions/tables.lua'],'signature':'table.find = function(table, value) for i, v in pairs(table) do if v == value then return i end end return nil end','contract':{'operation':'SOURCE_PAIRS_EQUALITY_FIRST_KEY','iteration':'pairs(table)','iteration_order':'SOURCE_PAIRS_ORDER_UNSPECIFIED','equality':'LUA_EQUALITY_WITH_METAMETHOD_AND_ERRORS','match_return':'MATCHED_ORIGINAL_KEY','miss_return':'NIL','short_circuit_first_match':True,'zero_key_truthy':True,'argument_evaluation':'SOURCE_LUA_FUNCTION_ARGUMENT_RULES_NOT_REORDERED','errors_preserved':True}},
 'CakeQuest.getStage':{'paths':['data-global/scripts/lib/a_piece_of_cake_config.lua'],'signature':'function CakeQuest.getStage() return CakeQuest.get(CakeQuest.Keys.Stage, 0) end','contract':{'operation':'SOURCE_HELPER_STAGE_KV_READ','ordered_lookup':['CakeQuest','get','Keys','Stage','KV','get'],'default_only_on':'NIL','false_is_not_defaulted':True,'default_value':0,'side_effects':'KV_GET_AS_IMPLEMENTED_SOURCE_NOT_ASSUMED_PURE','errors_preserved':True}},
 'CakeQuest.get':{'paths':['data-global/scripts/lib/a_piece_of_cake_config.lua'],'signature':'function CakeQuest.get(key, default) local value = CakeQuest.KV:get(key) if value == nil then return default end return value end','contract':{'operation':'SOURCE_KV_GET_NIL_DEFAULT','default_only_on':'NIL','false_is_not_defaulted':True,'side_effects':'KV_GET_AS_IMPLEMENTED_SOURCE_NOT_ASSUMED_PURE','errors_preserved':True}},
 'SoulWarQuest.ebbAndFlow.isActive':{'paths':['data-otservbr-global/lib/quests/soul_war.lua'],'signature':'function() return SoulWarQuest.ebbAndFlow.kv:get("is-active") end','contract':{'operation':'SOURCE_HELPER_KV_RAW_READ','ordered_lookup':['SoulWarQuest','ebbAndFlow','kv','get'],'key':'is-active','return':'RAW_KV_VALUE_WITH_NIL_FALSE_DISTINCT','side_effects':'KV_GET_AS_IMPLEMENTED_SOURCE_NOT_ASSUMED_PURE','errors_preserved':True}},
}

class Cache:
 def __init__(self,manifest,ast_root):
  self.path=manifest;self.manifest=json.loads(manifest.read_text());self.files={(x['source'],x['revision'],x['path']):x for x in self.manifest['files']};self.ast_root=ast_root;self.captures={}
  index=ast_root/'index.json';self.index=json.loads(index.read_bytes() if index.exists() else gzip.decompress((ast_root/'index.json.gz').read_bytes()));self.entries={x['sha256']:x for x in self.index['captures']}
 def raw(self,p):
  x=self.files[(p['source'],p['revision'],p['path'])]
  if any(p[k]!=x[k]for k in ('sha256','git_blob_sha1')if k in p):raise ValueError('Source provenance mismatch')
  path=Path(x['cache_path']);raw=(path if path.is_absolute()else self.path.parent/path).read_bytes()
  if sha(raw)!=x['sha256']or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=x['git_blob_sha1']:raise ValueError('Source hash mismatch')
  return raw
 def ast(self,p):
  ident=p['sha256']
  if ident not in self.captures:
   path=self.ast_root/'captures'/(ident+'.json.gz')
   if path.exists():packed=path.read_bytes()
   else:
    archive_manifest=json.loads((self.ast_root/'manifest.json').read_text())
    archive=next(x for x in archive_manifest['archives']if ident in x['capture_sha256s'])
    container=self.ast_root/archive['path']
    if sha(container.read_bytes())!=archive['sha256']:raise ValueError('AST shard mismatch')
    with tarfile.open(container,'r:gz')as tar:
     member=next(x for x in tar if x.name.endswith('/'+ident+'.json.gz'))
     packed=tar.extractfile(member).read()
   entry=self.entries[ident];decoded=gzip.decompress(packed)
   if sha(packed)!=entry['container_sha256']or sha(decoded)!=entry['capture_sha256']:raise ValueError('AST hash mismatch')
   cap=json.loads(decoded)
   if cap['status']!='PARSED'or base64.b64decode(cap['raw_bytes_base64'])!=self.raw(p):raise ValueError('AST/source mismatch')
   self.captures[ident]=cap['ast']
  return self.captures[ident]
 def proof(self,p,path,n):
  span=n['span'];raw=self.raw(p);text=raw.decode('utf-8');s,e=span['start_char'],span['end_char_exclusive'];start=len(text[:s].encode());end=len(text[:e].encode())
  return {'provenance':{k:p[k]for k in ('source','revision','path','sha256','git_blob_sha1')},'ast_pointer':path,'byte_start':start,'byte_end_exclusive':end,'expression':text[s:e],'slice_sha256':sha(raw[start:end])}
 def builtin_witnesses(self,p):
  path='src/lua/functions/lua_functions_loader.cpp';row=self.files.get((p['source'],p['revision'],path))
  if not row:return [{'kind':'SOURCE_HOST_STANDARD_LIBRARY_DEPENDENCY','body_available':False,'execution_proven':False}]
  raw=self.raw(row);needle=b'luaL_openlibs(L);';start=raw.index(needle)
  return [{'kind':'SOURCE_HOST_STANDARD_LIBRARY_OPEN','provenance':{k:row[k]for k in ('source','revision','path','sha256','git_blob_sha1')},'byte_start':start,'byte_end_exclusive':start+len(needle),'expression':needle.decode(),'slice_sha256':sha(needle),'host_builtin_body_in_corpus':False,'contract_condition':'STANDARD_LUA_BUILTIN_BINDING_IF_UNREPLACED','execution_proven':False}]

 def dependencies(self,p,callee):
  spec=HELPERS[callee];out=[]
  for path in spec['paths']:
   row=self.files.get((p['source'],p['revision'],path))
   if not row:continue
   for ptr,n in walk(self.ast(row)):
    chosen=False
    if n['node_type']=='Function'and name(n['fields'].get('name'))==callee:chosen=True
    if n['node_type']=='Assign'and len(n['fields']['targets'])==1 and name(n['fields']['targets'][0])==callee:chosen=True
    if callee.endswith('.isActive')and n['node_type']=='Field':
     f=n['fields'];key=name(f.get('key'))
     if key=='isActive'and f.get('value',{}).get('node_type')=='AnonymousFunction':n=f['value'];ptr+='/fields/value';chosen=True
    if not chosen:continue
    proof=self.proof(row,ptr,n)
    if re.sub(r'\s+','',proof['expression'])!=re.sub(r'\s+','',spec['signature']):raise ValueError('Helper implementation drift: '+callee)
    binding_refs=[]
    for pattern in ([rb'CakeQuest\.KV = kv:scoped\("a_piece_of_cake"\)',rb'Stage = "stage"']if callee.startswith('CakeQuest.')else [rb'kv = KV\.scoped\("quest"\):scoped\("soul-war"\):scoped\("ebb-and-flow-maps"\)']if callee.startswith('SoulWarQuest.')else []):
     raw=self.raw(row);match=re.search(pattern,raw)
     if not match:raise ValueError('Helper KV binding witness absent')
     binding_refs.append({'provenance':proof['provenance'],'byte_start':match.start(),'byte_end_exclusive':match.end(),'expression':match.group().decode(),'slice_sha256':sha(match.group()),'runtime_value_snapshot':False})
    out.append({'callee':callee,'body_source':proof,'contract':spec['contract'],'source_binding_initializers':binding_refs,'binding_status':'PINNED_BODY_CAPTURED_LOAD_ORDER_AND_GLOBAL_REBINDING_NOT_PROVEN'})
  if not out:raise ValueError('Helper source definition absent: '+callee)
  if callee=='CakeQuest.getStage':out+=self.dependencies(p,'CakeQuest.get')
  return out


def operand(cache,p,node,path,condition):
 kind=node['node_type'];f=node['fields']
 if kind=='String':return {'operation':'SOURCE_LUA_STRING_LITERAL_TOKEN','source_pointer':path,'decode':'LUA_STRING_ESCAPE_AND_LONG_BRACKET_INITIAL_NEWLINE_RULES','value_not_assumed_from_parser_bytes':True}
 if kind in ('Number','TrueExpr','FalseExpr','Nil'):
  value=f.get('n',f.get('value'))
  if kind=='TrueExpr':value=True
  if kind=='FalseExpr':value=False
  return {'operation':'SOURCE_LITERAL','lua_type':kind,'value':value}
 if kind=='Name':return {'operation':'SOURCE_LEXICAL_VALUE_READ','name':f['id'],'binding':'ENCLOSING_SOURCE_AST_BINDING_REQUIRED','errors_preserved':True}
 if kind=='Index':
  return {'operation':'SOURCE_INDEX_READ','receiver':operand(cache,p,f['value'],path+'/fields/value',condition),'key':{'operation':'SOURCE_LITERAL','lua_type':'String','value':name(f['idx'])} if f.get('notation',{}).get('name')=='DOT'else operand(cache,p,f['idx'],path+'/fields/idx',condition),'lookup':'LUA_TABLE_OR_USERDATA_INDEX_WITH_METAMETHOD_ERRORS','errors_preserved':True}
 if kind=='Table':
  entries=[];implicit_index=0
  for i,field in enumerate(f['fields']):
   ff=field['fields']
   if ff.get('key')is None:implicit_index+=1
   entries.append({'source_ordinal':i,'key':operand(cache,p,ff['key'],path+'/fields/fields/'+str(i)+'/fields/key',condition)if ff.get('key')else {'operation':'SOURCE_IMPLICIT_ARRAY_KEY','ordinal':implicit_index},'value':operand(cache,p,ff['value'],path+'/fields/fields/'+str(i)+'/fields/value',condition)})
  return {'operation':'SOURCE_TABLE_CONSTRUCTOR','entries':entries,'allocation_and_source_evaluation_preserved':True}
 if kind=='SubOp':return {'operation':'SOURCE_LUA_SUBTRACT','left':operand(cache,p,f['left'],path+'/fields/left',condition),'right':operand(cache,p,f['right'],path+'/fields/right',condition),'metamethod_coercion_and_errors_preserved':True}
 if kind=='Call':
  callee=name(node)
  if callee=='os.time'and not f['args']:return {'operation':'SOURCE_LUA_OS_TIME_NOW','callee_lookup':'os.time','wall_clock_not_monotonic':True,'lookup_and_call_errors_preserved':True,'host_builtin_execution_not_proven':True}
 return {'operation':'SOURCE_OPERAND_OUTSIDE_HELPER_VOCABULARY','ast_pointer':path}


class Engine:
 """Root composition adapter: only exact closed helper/builtin call operands."""
 def __init__(self,cache,provenance):self.cache,self.provenance=cache,provenance
 def project(self,node,path):
  if node.get('node_type')!='Call':return None
  callee=name(node);f=node['fields'];p=self.provenance
  if callee not in HELPERS and callee not in ('type','os.time'):return None
  if callee=='type'and len(f['args'])!=1:return None
  if callee=='os.time'and f['args']:return None
  dependencies=self.cache.dependencies(p,callee)if callee in HELPERS else []
  if callee=='type':op={'operation':'SOURCE_LUA_TYPE_QUERY','returns':'LUA_TYPE_NAME_STRING','arguments':[operand(self.cache,p,x,path+'/fields/args/'+str(i),None)for i,x in enumerate(f['args'])],'missing_argument_error_preserved':True,'nil_userdata_distinction_preserved':True,'builtin_binding':'LOCAL_FILE_AND_GLOBAL_ENVIRONMENT_BINDING_NOT_PROVEN'}
  elif callee=='os.time':op=operand(self.cache,p,node,path,None)
  else:op={'operation':'SOURCE_PINNED_HELPER_CALL','callee':callee,'contract':HELPERS[callee]['contract'],'arguments':[operand(self.cache,p,x,path+'/fields/args/'+str(i),None)for i,x in enumerate(f['args'])],'lookup':'SOURCE_LUA_GLOBAL_TABLE_CHAIN_NO_CACHE_OR_REORDER','callee_binding_execution_proven':False}
  if callee in ('type','os.time'):
   dependencies=self.cache.builtin_witnesses(p)
  return {'lowered':op,'dependencies':dependencies,'binding_witness':{'source_specification':'EXACT_PINNED_CALLSITE_AND_HELPER_BODY_WHERE_AVAILABLE','execution_binding_proven':False}}


def build(rows,cache):
 records=[]
 for row in rows:
  p=row['provenance'];ast=cache.ast(p);rootptr=row['source_ref']['condition_ast_pointer'];condition=pointer(ast,rootptr);engine=Engine(cache,p)
  for rel,n in walk(condition):
   ptr=rootptr+rel;projected=engine.project(n,ptr)
   if projected is None:continue
   proof=cache.proof(p,ptr,n)
   records.append({'source_component_id':row['source_component_id'],'condition_ast_pointer':rootptr,'atom_ast_pointer':ptr,'source_ref':proof,**projected,'status':'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN','prior_partial_status_retained':True,'native_admission':False,'canonical_gap_replaced':False,'runtime_activation':False})
 return {'schema':'OTERYN_CLOSED_SOURCE_HELPER_OPERANDS/v1','records':records,'summary':{'operand_records':len(records),'conditions':len({(r['source_component_id'],r['condition_ast_pointer'])for r in records}),'execution_proven':0},'native_admission':False}


def main():
 a=argparse.ArgumentParser();a.add_argument('--input',type=Path,required=True);a.add_argument('--corpus-manifest',type=Path,required=True);a.add_argument('--ast-root',type=Path,required=True);a.add_argument('--out',type=Path,required=True);args=a.parse_args();packet=build(json.loads(args.input.read_text()),Cache(args.corpus_manifest,args.ast_root));args.out.write_text(json.dumps(packet,indent=2)+'\n');print(packet['summary'])
if __name__=='__main__':main()
