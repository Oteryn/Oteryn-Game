"""Typed pinned getter Source operations; dispatch and live values remain unproven."""
import argparse,base64,gzip,hashlib,json,pathlib,re
from dependency_model import dependency_catalog
METHODS={'getName','getTypeName','getId','getEmptySlots','getFreeCapacity','getOutfit','getVocation','getBaseId','getLevel','getPosition','getItemById','getGoshnarSymbolTormentCounter','lower','getStorageValue'}
PINS={'canary':'04b83b512114bfd888000d6e1433ed8ecaec7c5b','crystalserver':'9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'}
TYPES={'getName':'STRING','getTypeName':'STRING','getId':'NUMBER','getEmptySlots':'NUMBER','getFreeCapacity':'NUMBER','getOutfit':'OUTFIT_TABLE','getVocation':'VOCATION_USERDATA','getBaseId':'NUMBER','getLevel':'NUMBER','getPosition':'POSITION_TABLE','getItemById':'ITEM_USERDATA_OR_NIL','getStorageValue':'NUMBER'}
def stable(v):return json.dumps(v,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
def sha(b):return hashlib.sha256(b).hexdigest()
def walk(v,p=''):
 if isinstance(v,dict):
  if 'node_type'in v:yield p,v
  for k,x in v.items():yield from walk(x,p+'/'+k)
 elif isinstance(v,list):
  for i,x in enumerate(v):yield from walk(x,p+'/'+str(i))
def get(v,p):
 for k in p.strip('/').split('/'):
  if k:v=v[int(k)] if isinstance(v,list) else v[k]
 return v

def masked_cpp(text):
 out=list(text);i=0
 while i<len(text):
  start=i
  if text.startswith('//',i):
   i=text.find('\n',i);i=len(text) if i<0 else i
  elif text.startswith('/*',i):
   end=text.find('*/',i+2);i=len(text) if end<0 else end+2
  elif text.startswith('R\"',i):
   raw_open=re.match(r'R\"([^ ()\\\t\r\n]{0,16})\(',text[i:])
   if not raw_open:i+=1;continue
   close=')'+raw_open.group(1)+'\"';end=text.find(close,i+len(raw_open.group(0)));i=len(text) if end<0 else end+len(close)
  elif text[i] in ('"', "'"):
   quote=text[i];i+=1
   while i<len(text):
    if ord(text[i])==92:i+=2
    elif text[i]==quote:i+=1;break
    else:i+=1
  else:i+=1;continue
  for j in range(start,min(i,len(text))):
   if text[j]!='\n':out[j]=' '
 return ''.join(out)

def cpp_body(text,symbol):
 mask=masked_cpp(text);m=re.search(r'\b(?:int|void)\s+'+re.escape(symbol)+r'\s*\([^)]*\)\s*\{',mask)
 if not m:return None
 brace=mask.index('{',m.start());depth=1;i=brace+1
 while i<len(mask) and depth:
  depth+=(mask[i]=='{')-(mask[i]=='}');i+=1
 if depth:raise ValueError('Unterminated CPP body '+symbol)
 return m.start(),i,text[m.start():i]

def read_source(manifest_path,row):
 path=pathlib.Path(row.get('cache_path',row.get('blob_path','')));path=path if path.is_absolute() else manifest_path.parent/path;raw=path.read_bytes()
 if sha(raw)!=row['sha256'] or len(raw)!=row['byte_count'] or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=row['git_blob_sha1']:raise ValueError('Source corpus witness mismatch')
 return raw

def span_witness(row,text,start,end):
 raw=text[start:end].encode();return {k:row[k] for k in ['source','revision','repository','path','sha256','git_blob_sha1']}|{'start_char':start,'end_char_exclusive':end,'slice_sha256':sha(raw),'line':text.count('\n',0,start)+1,'source_text':text[start:end]}

def cpp_semantics(method,body,cls=None):
 # Actual pushed expressions and conversions are retained, with an operation model
 # independent from the opaque caller AST and without evaluating live C++ state.
 pushes=re.findall(r'(?:lua_pushnumber|Lua::pushString|Lua::pushPosition|Lua::pushOutfit|Lua::pushSharedUserdata<[^>]+>|Lua::pushUserdata<[^>]+>)\(L,\s*(.*?)\);',body,re.S)
 params=re.findall(r'Lua::get(?:Number<([^>]+)>|Boolean|String)\(L,\s*([2-9])(?:,\s*([^)]*))?\)',body)
 model={'operation':'SOURCE_RECEIVER_GETTER','return_value_type':TYPES[method],'nil_is_possible':'lua_pushnil(L)'in body,'live_value':'UNKNOWN','pushed_source_expressions':[x.strip() for x in pushes],'lua_argument_conversions':[{'cpp_numeric_type':n or None,'lua_stack_index':int(i),'source_default_expression':d.strip() or None} for n,i,d in params],'userdata_casts':re.findall(r'Lua::getUserdata(?:Shared)?<([^>]+)>\(L,\s*1',body),'error_behavior':{'invalid_receiver_nil':'lua_pushnil(L)'in body,'source_report_error':'reportError'in body,'live_error':'NOT_EXECUTED'}}
 if method=='getStorageValue':model['algorithm']={'operation':'PLAYER_STORAGE_LOOKUP','key_stack_index':2,'key_conversion':'UINT32','key_domain':'UNKNOWN_DYNAMIC_OR_SOURCE_EXPRESSION','missing_key_value':'SOURCE_ENGINE_GETTER_NOT_EXECUTED'}
 if method=='getEmptySlots':model['algorithm']={'initial':'container.capacity - container.size','recursive_flag_stack_index':2,'default':False,'when_recursive':'ADD_CAPACITY_MINUS_SIZE_FOR_EACH_DESCENDANT_CONTAINER','overflow_behavior':'CPP_UINT32_SOURCE_ARITHMETIC_NOT_EMULATED'}
 if method=='getItemById':model['algorithm']={'operation':'ITEM_ID_OR_NAME_LOOKUP_AND_FIND_ITEM_OF_TYPE','item_id_argument_stack_index':2,'deep_search_argument_stack_index':3 if cls=='Player' else None,'deep_search_constant':False if cls=='Tile' else None,'subtype_argument_stack_index':4 if cls=='Player' else 3,'no_item_result':'NIL','actor_specific_receiver_class_unproven':True}
 if method=='getFreeCapacity':model['algorithm']={'operation':'RETURN_RECEIVER_FREE_CAPACITY','units':'SOURCE_ENGINE_VALUE_NO_CONVERSION_IN_THIS_WRAPPER'}
 if method=='getVocation':model['algorithm']={'operation':'RETURN_VOCATION_USERDATA_FROM_RECEIVER','next_method_dispatch':'UNPROVEN'}
 if method in ('getOutfit','getPosition'):model['algorithm']={'operation':'CPP_GETTER_THEN_LUA_TABLE_MARSHAL','table_read_values':'UNKNOWN'}
 return model

class Catalog:
 def __init__(self,corpus_manifest,ast_root):
  self.path=pathlib.Path(corpus_manifest);self.manifest=json.loads(self.path.read_bytes());self.rows={(r['source'],r['revision'],r['path']):r for r in self.manifest['files']};self.ast_root=pathlib.Path(ast_root);self.apis={};self.bootstrap={};self.lua_methods={};self.dependencies={};self.marshal={};self.constructors={};self.external=dependency_catalog()
  for row in self.manifest['files']:
   if row['source'] not in PINS or row['revision']!=PINS[row['source']] or not row['path'].startswith('src/lua/') or not row['path'].endswith('.cpp'):continue
   text=read_source(self.path,row).decode('utf-8')
   for m in re.finditer(r'Lua::registerMethod\(L,\s*"([^"]+)",\s*"([^"]+)",\s*([\w:]+)\)',text):
    cls,method,symbol=m.groups()
    if method not in TYPES and not (cls,method) in [('Player','kv'),('KV','scoped'),('KV','get')]:continue
    definition=cpp_body(text,symbol)
    if not definition:continue
    start,end,body=definition
    rec={'class':cls,'method':method,'cpp_symbol':symbol,'registration':span_witness(row,text,m.start(),m.end()),'implementation':span_witness(row,text,start,end),'semantics':cpp_semantics(method,body,cls) if method in TYPES else {'operation':'KV_SOURCE_OPERATION','body_source_preserved':True,'value_and_dispatch':'UNPROVEN'},'dispatch':'UNPROVEN','implementation_language':'CPP'}
    if method in TYPES:self.apis.setdefault((row['source'],method),[]).append(rec)
    else:self.dependencies.setdefault(row['source'],[]).append(rec)
   for m in re.finditer(r'Lua::registerSharedClass\(L,\s*"Tile",\s*"",\s*([\w:]+)\)',text):
    body=cpp_body(text,m.group(1))
    if body:self.constructors[row['source']]={'registration':span_witness(row,text,m.start(),m.end()),'implementation':span_witness(row,text,body[0],body[1]),'semantics':{'operation':'MAP_TILE_LOOKUP','argument_modes':['POSITION_TABLE_AT_STACK_2','X_UINT16_STACK_2_Y_UINT16_STACK_3_Z_UINT8_STACK_4'],'return':'TILE_USERDATA_OR_NIL','live_value':'UNKNOWN'},'dispatch':'UNPROVEN'}
   for marshal_name in ['Lua::pushOutfit','Lua::pushPosition','KVFunctions::pushValueWrapper']:
    body=cpp_body(text,marshal_name)
    if body:self.marshal.setdefault(row['source'],[]).append({'cpp_symbol':marshal_name,'implementation':span_witness(row,text,body[0],body[1]),'dispatch':'UNPROVEN'})
   for m in re.finditer(r'\bluaL_openlibs\(L\)',text):self.bootstrap.setdefault(row['source'],[]).append(span_witness(row,text,m.start(),m.end()))
  # Player extension is not a storage read: it uses scoped KV and Lua `or 0`.
  for row in self.manifest['files']:
   if row['source'] not in PINS or row['revision']!=PINS[row['source']] or not row['path'].endswith('/lib/quests/soul_war.lua'):continue
   text=read_source(self.path,row).decode('utf-8')
   patterns={'getGoshnarSymbolTormentCounter':r'function Player:getGoshnarSymbolTormentCounter\(\)\s+local soulWarKV = self:soulWarQuestKV\(\)\s+return soulWarKV:get\("goshnars-hatred-torment-count"\) or 0\s+end','soulWarQuestKV':r'function Player:soulWarQuestKV\(\)\s+return self:kv\(\):scoped\("quest"\):scoped\("soul-war"\)\s+end'}
   matches={k:re.search(v,text) for k,v in patterns.items()}
   if not all(matches.values()):continue
   self.lua_methods[row['source']]={'class':'Player','method':'getGoshnarSymbolTormentCounter','implementation_language':'LUA_EXTENSION','definition':span_witness(row,text,matches['getGoshnarSymbolTormentCounter'].start(),matches['getGoshnarSymbolTormentCounter'].end()),'scope_definition':span_witness(row,text,matches['soulWarQuestKV'].start(),matches['soulWarQuestKV'].end()),'semantics':{'operation':'SCOPED_PLAYER_KV_READ_LUA_OR_ZERO','scope_order':['quest','soul-war'],'key':'goshnars-hatred-torment-count','default':0,'false_values_causing_fallback':['NIL','BOOLEAN_FALSE'],'zero_and_empty_string_are_truthy':True,'return_value_type':'KV_STORED_VALUE_OR_NUMERIC_ZERO','not_implicitly_numeric':True,'live_value':'UNKNOWN'},'dispatch':'UNPROVEN','dependency_definitions':self.dependencies.get(row['source'],[]),'value_wrapper_source':self.marshal.get(row['source'],[])}

class Engine:
 def __init__(self,context,corpus_manifest=None,ast_root=None,operand_projector=None,catalog=None):
  self.context=context;self.catalog=catalog or Catalog(corpus_manifest,ast_root);self.source=context.get('provenance',context).get('source');self.full_ast=context.get('full_ast');self.raw=context.get('raw');self.project_operand=operand_projector
  if self.source not in PINS:raise ValueError('Unknown Source donor')
  prov=context.get('provenance',context)
  if prov.get('revision')!=PINS[self.source]:raise ValueError('Unexpected Source pin')
  source_row=self.catalog.rows.get((self.source,prov.get('revision'),prov.get('path')))
  if not source_row or any(source_row.get(k)!=prov.get(k) for k in ['sha256','git_blob_sha1','byte_count']):raise ValueError('Source context provenance mismatch')
  if self.raw is not None and sha(self.raw)!=prov.get('sha256'):raise ValueError('Context raw Source mismatch')
 def ast_ref(self,node,pointer):return {'ast_pointer':pointer,'node_sha256':sha(stable(node)),'span':node.get('span')}
 def operand(self,node,pointer):
  result=self.project(node,pointer)
  if result is not None:return result
  if self.project_operand:
   result=self.project_operand(node,pointer)
   if result is not None:return result
  t=node.get('node_type');f=node.get('fields',{})
  if t=='Number':return {'kind':'literal','type':'NUMBER','value':f['n']}
  if t in ('TrueExpr','FalseExpr'):return {'kind':'literal','type':'BOOLEAN','value':t=='TrueExpr'}
  if t=='String':
   raw=f.get('raw','');delimiter=f.get('delimiter',{}).get('name')
   if delimiter not in ('SINGLE_QUOTE','DOUBLE_QUOTE') or chr(92) in raw:return {'kind':'operand_dependency_unresolved','ast_ref':self.ast_ref(node,pointer),'reason':'ROOT_EXACT_LUA_STRING_LITERAL_PROJECTOR_REQUIRED','source_spec_complete':False}
   return {'kind':'literal','type':'STRING','value':base64.b64decode(f['s']['bytes_base64']).decode('utf-8')}
  if t=='Name':return {'kind':'bound_source_name_read','name':f['id'],'binding':self.binding(node,pointer),'live_value':'UNKNOWN','ast_ref':self.ast_ref(node,pointer)}
  return {'kind':'operand_dependency_unresolved','ast_ref':self.ast_ref(node,pointer),'reason':'OTHER_LANE_OPERAND_REQUIRED','source_spec_complete':False}
 def binding(self,node,pointer):
  if not self.full_ast:return {'status':'LEXICAL_CONTEXT_NOT_SUPPLIED','runtime_binding':'UNPROVEN'}
  name=node['fields']['id'];scopes=[]
  for p,n in walk(self.full_ast):
   f=n.get('fields',{})
   if n.get('node_type') in ('Function','LocalFunction','AnonymousFunction','Method') and pointer.startswith(p+'/fields/body'):
    for i,arg in enumerate(f['args']):
     if arg.get('node_type')=='Name' and arg['fields']['id']==name:scopes.append({'kind':'formal_parameter','declaration_pointer':p+'/fields/args/'+str(i),'name':name,'scope_pointer':p+'/fields/body','receiver_type':'NOT_INFERRED_FROM_PARAMETER_NAME'})
   if n.get('node_type')=='LocalAssign':
    scope=p.rsplit('/fields/body/',1)[0]+'/fields/body' if '/fields/body/' in p else ''
    for i,target in enumerate(f['targets']):
     if target.get('node_type')=='Name' and target['fields']['id']==name and scope and pointer.startswith(scope+'/'):
      sp=n.get('span');use=node.get('span')
      if sp and use and sp['end_char_exclusive']<=use['start_char']:
       scopes.append({'kind':'local_declaration','name':name,'declaration_pointer':p+'/fields/targets/'+str(i),'initializer_ast_ref':self.ast_ref(f['values'][i],p+'/fields/values/'+str(i)) if i<len(f['values']) else None,'scope_pointer':scope,'receiver_type':'INITIALIZER_CANDIDATE_ONLY'})
  scopes.sort(key=lambda x:len(x['scope_pointer']),reverse=True)
  return {'candidates':scopes,'status':'LEXICAL_DECLARATIONS_RETAINED_NOT_UNIQUE_REACHING_VALUE','runtime_binding':'UNPROVEN','global_metatable_or_reassignment':'NOT_ASSESSED'}
 def project(self,node,pointer):
  if self.full_ast is not None and get(self.full_ast,pointer)!=node:raise ValueError('Caller AST pointer mismatch')
  if node.get('node_type')=='Call':
   f=node['fields'];fn=f.get('func',{})
   if fn.get('node_type')!='Name' or fn['fields']['id']!='Tile':return None
   args=[self.operand(a,pointer+'/fields/args/'+str(i)) for i,a in enumerate(f['args'])];api=self.catalog.constructors.get(self.source)
   complete=bool(api) and all(a.get('source_spec_complete',True) for a in args)
   return {'kind':'source_tile_constructor','method':'Tile','arguments':args,'api_definitions':[api] if api else [],'status':'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN' if api else 'SOURCE_API_DEFINITION_MISSING','source_spec_complete':complete,'chain_order':'GLOBAL_CALLEE_LOOKUP_THEN_ARGUMENTS_LEFT_TO_RIGHT_THEN_CALL','dispatch':'UNPROVEN','global_shadowing':'UNPROVEN','live_value':'UNKNOWN','native_admission':False,'runtime_activation':False,'ast_ref':self.ast_ref(node,pointer)}
  if node.get('node_type')!='Invoke':return None
  f=node['fields'];fn=f.get('func',{})
  if fn.get('node_type')!='Name' or fn['fields']['id']not in METHODS:return None
  method=fn['fields']['id'];receiver=self.operand(f['source'],pointer+'/fields/source');args=[self.operand(n,pointer+'/fields/args/'+str(i)) for i,n in enumerate(f['args'])]
  apis=self.catalog.apis.get((self.source,method),[])
  if method=='getGoshnarSymbolTormentCounter':apis=[self.catalog.lua_methods[self.source]] if self.source in self.catalog.lua_methods else []
  status='SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN' if apis else 'SOURCE_API_DEFINITION_MISSING'
  dependency=[]
  if method=='lower':
   apis=[self.catalog.external[self.source]['lower']];status='SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN'
  if receiver.get('source_spec_complete',True) is False or any(a.get('source_spec_complete',True) is False for a in args):dependency.append('OPERAND_PROJECTOR_REQUIRED')
  return {'kind':'source_getter_value','method':method,'receiver':receiver,'arguments':args,'chain_order':'RECEIVER_THEN_METHOD_LOOKUP_THEN_ARGUMENTS_LEFT_TO_RIGHT_THEN_CALL','api_definitions':apis,'status':status,'source_spec_complete':status=='SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN' and not dependency,'dispatch':'UNPROVEN','live_value':'UNKNOWN','native_admission':False,'runtime_activation':False,'ast_ref':self.ast_ref(node,pointer),'remaining_dependencies':dependency,'receiver_dynamic_type':'UNPROVEN','method_lookup_and_global_or_metatable_shadowing':'NOT_PROVEN','nil_error_outcomes_preserved':True,'marshal_definition_refs':self.catalog.marshal.get(self.source,[]) if method in ('getPosition','getOutfit') else []}

def build(partial_path,corpus_manifest,ast_root):
 rows=json.loads(pathlib.Path(partial_path).read_bytes());catalog=Catalog(corpus_manifest,ast_root);indexp=pathlib.Path(ast_root)/'index.json';index=json.loads(indexp.read_bytes() if indexp.exists() else gzip.decompress((pathlib.Path(ast_root)/'index.json.gz').read_bytes()));captures={c['sha256']:c for c in index['captures']};result=[]
 for row in rows:
  prov=row['provenance'];cp=pathlib.Path(ast_root)/'captures'/(prov['sha256']+'.json.gz')
  if not cp.exists():raise ValueError('Producer requires existing loose AST cache; Engine API does not require cache')
  rawcap=cp.read_bytes()
  if sha(rawcap)!=captures[prov['sha256']]['container_sha256']:raise ValueError('AST capture witness mismatch')
  cap=json.loads(gzip.decompress(rawcap));raw=base64.b64decode(cap['raw_bytes_base64']);source_row=catalog.rows.get((prov['source'],prov['revision'],prov['path']))
  if not source_row or any(source_row[k]!=prov[k] for k in ['sha256','git_blob_sha1','byte_count']):raise ValueError('Condition Source identity mismatch')
  if raw!=read_source(catalog.path,source_row):raise ValueError('AST raw bytes mismatch')
  ast=cap['ast'];engine=Engine(row|{'full_ast':ast,'raw':raw},catalog=catalog);condition=row['source_ref']['condition_ast_pointer'];n=get(ast,condition)
  sp=n['span'];text=raw.decode('utf-8-sig')
  if text[sp['start_char']:sp['end_char_exclusive']]!=row['source_ref']['expression']:raise ValueError('Condition AST/source expression mismatch')
  for p,atom in walk(n,condition):
   projected=engine.project(atom,p)
   if projected is not None:result.append({'source_component_id':row['source_component_id'],'condition_ast_pointer':condition,'atom_ast_pointer':p,'baseline_status':row['status'],'canonical_gap_replaced':False,'source_ref':row['source_ref'],'provenance':prov,'operation':projected,'native_admission':False,'runtime_activation':False})
 return {'schema':'OTERYN_ENTITY_GETTER_SOURCE_VALUES/v1','scope':'TYPED_GETTER_OPERANDS_NOT_ORIGINAL_FULL_GUARDS','input_sha256':sha(pathlib.Path(partial_path).read_bytes()),'records':result,'summary':{'getter_operations':len(result),'methods':sorted({r['operation']['method']for r in result}),'complete_source_operation_specs':sum(r['operation']['source_spec_complete']for r in result),'external_library_operations':sum(r['operation']['method']=='lower'for r in result),'external_library_definitions_missing':sum(r['operation']['method']=='lower' and r['operation']['status']!='SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN' for r in result),'native_admission':False}}
def main():
 p=argparse.ArgumentParser()
 for k in ('partial','corpus-manifest','ast-root','out'):p.add_argument('--'+k,type=pathlib.Path,required=True)
 a=p.parse_args();data=build(a.partial,a.corpus_manifest,a.ast_root);a.out.write_bytes(stable(data)+b'\n');print(json.dumps(data['summary']))
if __name__=='__main__':main()
