"""Compose closed Source guard specifications; never promote historical partial guards."""
import argparse,base64,gzip,hashlib,importlib.util,json,math,re,sys
from collections import Counter
from pathlib import Path
HERE=Path(__file__).resolve().parent

def stable(v):return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()
def sha(raw):return hashlib.sha256(raw).hexdigest()
def load(path,label):
 spec=importlib.util.spec_from_file_location(label,path);module=importlib.util.module_from_spec(spec);sys.modules[label]=module
 sys.path.insert(0,str(path.parent))
 try:spec.loader.exec_module(module)
 finally:sys.path.pop(0)
 return module

def get(v,p):
 for part in p.split('/')[1:]:v=v[int(part)]if isinstance(v,list)else v[part]
 return v

def walk(v):
 if isinstance(v,dict):
  yield v
  for x in v.values():yield from walk(x)
 elif isinstance(v,list):
  for x in v:yield from walk(x)

COMPARISONS={'EqToOp':'==','NotEqToOp':'~=','LessThanOp':'<','LessOrEqThanOp':'<=','GreaterThanOp':'>','GreaterOrEqThanOp':'>='}
ARITHMETIC={'AddOp':'+','SubOp':'-','MultOp':'*','FloatDivOp':'/','FloorDivOp':'//','ModOp':'%','ExpoOp':'^','Concat':'..'}

def builtin_specs(dependency_root,external_catalog):
 manifest=json.loads((dependency_root/'luajit-acquisition.json').read_text());out={}
 for mapping in manifest['mapping']:
  donor=mapping['donor'];texts={};rows={}
  for row in manifest['files']:
   if row['donor']!=donor or row.get('path')not in {'src/lib_base.c','src/lib_os.c','src/vm_x64.dasc'}:continue
   raw=(dependency_root/row['local_file']).read_bytes()
   if sha(raw)!=row['sha256']or len(raw)!=row['byte_count']or row['revision']!=mapping['luajit_revision']:raise ValueError('Pinned builtin Source mismatch')
   texts[row['path']]=raw.decode();rows[row['path']]=row
  def witness(path,pattern):
   text=texts[path];match=re.search(pattern,text,re.S)
   if not match:raise ValueError('Pinned builtin operation body absent')
   row=rows[path];start,end=match.span();bs,be=len(text[:start].encode()),len(text[:end].encode())
   return {k:row[k]for k in ('repository','revision','path','sha256','byte_count','url')}|{'byte_start':bs,'byte_end_exclusive':be,'source_text':text[start:end],'slice_sha256':sha(text[start:end].encode()),'execution_binding_proven':False}
  type_defs=[witness('src/lib_base.c',r'/\* ORDER LJ_T \*/.*?LJLIB_ASM_\(type\)[^\n]*\n/\* Recycle.*?\*/'),witness('src/vm_x64.dasc',r'\|\.ffunc_1 type.*?\|  jmp ->fff_res1')]
  time_defs=[witness('src/lib_os.c',r'LJLIB_CF\(os_time\).*?\n\}')]
  if 'lj_lib_checkany'not in type_defs[0]['source_text']or'LJ_TISNUM'not in type_defs[1]['source_text']:raise ValueError('Type operation body drift')
  if any(token not in time_defs[0]['source_text']for token in ['time(NULL)','lua_pushnil(L)','lua_pushnumber']):raise ValueError('Wallclock operation body drift')
  common={'repository':'LuaJIT/LuaJIT','dependency_pin':mapping['luajit_revision'],'port_ref_witness':external_catalog[donor]['lower']['port_ref_witness'],'dispatch_and_deployed_library_version_unproven':True}
  out[donor]={'type':{'kind':'source_pinned_luajit_builtin_spec','builtin':'type','definitions':type_defs,'operation':'RETURN_TYPE_NAME_FROM_TYPE_TAG_UPVALUE_ARRAY','minimum_arguments':1,'no_tostring_or_entity_cast':True,**common},'os.time':{'kind':'source_pinned_luajit_builtin_spec','builtin':'os.time','definitions':time_defs,'operation':'NO_ARGUMENT_WALLCLOCK_TIME_OR_NIL_ON_HOST_TIME_FAILURE','accepted_call_form':'NO_EXPLICIT_ARGUMENTS','nil_failure_predicate':'t == (time_t)-1 && errno != 0'if'errno != 0'in time_defs[0]['source_text']else't == (time_t)-1','else_result':'LUA_NUMBER_CAST_OF_TIME_T','wallclock_may_move_backwards':True,**common}}
 return out

class Composer:
 def __init__(self,ast,raw,provenance,field_engine,helper_engine=None,entity_engine=None,getter_engine=None,builtins=None):
  self.ast,self.raw,self.provenance=ast,raw,provenance;self.text=raw.decode('utf-8');self.fields=field_engine;self.helpers=helper_engine;self.entity=entity_engine;self.getters=getter_engine;self.builtins=builtins or {}
 def source(self,node,pointer):
  span=node.get('span');ref={'ast_pointer':pointer,'node_sha256':sha(stable(node)),'span':span}
  if span:
   a,b=span['start_char'],span['end_char_exclusive'];start,end=len(self.text[:a].encode()),len(self.text[:b].encode())
   ref.update(byte_start=start,byte_end_exclusive=end,slice_sha256=sha(self.raw[start:end]),expression=self.text[a:b])
  return ref
 def unresolved(self,node,pointer,reason):return {'kind':'source_operation_unresolved','reason':reason,'source_ref':self.source(node,pointer)}
 def replace_refs(self,v):
  if isinstance(v,list):return [self.replace_refs(x)for x in v]
  if not isinstance(v,dict):return v
  if v.get('kind')=='operand_reference':return self.compose(get(self.ast,v['ast_pointer']),v['ast_pointer'])
  return {k:self.replace_refs(x)for k,x in v.items()}
 def compose(self,node,pointer):
  t,f=node['node_type'],node['fields'];ref=self.source(node,pointer)
  if t in ('AndLoOp','OrLoOp'):
   return {'kind':'source_short_circuit','operator':'and'if t=='AndLoOp'else'or','left':self.compose(f['left'],pointer+'/fields/left'),'right':self.compose(f['right'],pointer+'/fields/right'),'returns':'ORIGINAL_SELECTED_OPERAND_VALUE_NOT_BOOLEAN_COERCION','right_evaluation':'ONLY_WHEN_LUA_LEFT_TRUTHINESS_REQUIRES','false_values':['NIL','BOOLEAN_FALSE'],'zero_and_empty_string_truthy':True,'source_ref':ref}
  if t=='ULNotOp':return {'kind':'source_not','child':self.compose(f['operand'],pointer+'/fields/operand'),'returns':'BOOLEAN','false_values':['NIL','BOOLEAN_FALSE'],'source_ref':ref}
  if t in COMPARISONS:
   return {'kind':'source_comparison','operator':COMPARISONS[t],'left':self.compose(f['left'],pointer+'/fields/left'),'right':self.compose(f['right'],pointer+'/fields/right'),'evaluation':'SOURCE_LUA_OPERAND_EVALUATION_AND_OPERATOR_DISPATCH_NOT_REORDERED','semantics':'LUA_EQUALITY_ORDERING_TYPE_RULES_AND_METAMETHODS','errors_and_side_effects_preserved':True,'no_domain_casts':True,'source_ref':ref}
  if t in ARITHMETIC:
   return {'kind':'source_arithmetic','operator':ARITHMETIC[t],'left':self.compose(f['left'],pointer+'/fields/left'),'right':self.compose(f['right'],pointer+'/fields/right'),'semantics':'SOURCE_LUA_NUMERIC_COERCION_OR_METAMETHOD_DISPATCH_WITH_ERRORS','no_constant_folding':True,'source_ref':ref}
  if t in ('UMinusOp','ULengthOP'):
   return {'kind':'source_unary_operator','operator':'-'if t=='UMinusOp'else'#','child':self.compose(f['operand'],pointer+'/fields/operand'),'semantics':'SOURCE_LUA_OPERATOR_METAMETHOD_AND_ERROR_RULES','source_ref':ref}
  if t in ('TrueExpr','FalseExpr','Nil'):return {'kind':'source_literal','lua_type':'NIL'if t=='Nil'else'BOOLEAN','value':None if t=='Nil'else t=='TrueExpr','source_ref':ref}
  if t=='Number':
   value=f['n']
   if type(value)in(int,float)and math.isfinite(value)and abs(value)<=2**53-1:return {'kind':'source_literal','lua_type':'NUMBER','value':value,'source_ref':ref}
   return {'kind':'source_numeric_literal_token','source_ref':ref,'decode_rule':'PINNED_LUAJIT_NUMERIC_LITERAL_RULES_NO_PYTHON_VALUE_ASSUMPTION'}
  if t=='String':
   raw=f.get('raw','');delimiter=f.get('delimiter',{}).get('name')
   if delimiter in ('SINGLE_QUOTE','DOUBLE_QUOTE')and '\\'not in raw:return {'kind':'source_literal','lua_type':'STRING','value':base64.b64decode(f['s']['bytes_base64']).decode('utf-8'),'source_ref':ref}
   return {'kind':'source_string_literal_token','source_ref':ref,'decode_rule':'LUA_ESCAPES_AND_LONG_BRACKET_INITIAL_NEWLINE_RULE','parser_bytes_not_semantic_value':True}
  if t=='Table':
   entries=[];array_index=0
   for i,field in enumerate(f['fields']):
    ff=field['fields'];key=ff.get('key');fp=pointer+'/fields/fields/'+str(i)
    if key is None:array_index+=1;projected_key={'kind':'source_literal','lua_type':'NUMBER','value':array_index}
    elif key['node_type']=='Name'and not ff.get('between_brackets',False):projected_key={'kind':'source_literal','lua_type':'STRING','value':key['fields']['id']}
    else:projected_key=self.compose(key,fp+'/fields/key')
    entries.append({'source_ordinal':i,'key':projected_key,'value':self.compose(ff['value'],fp+'/fields/value'),'value_multi_return':'FINAL_IMPLICIT_FIELD_USES_LUA_CONSTRUCTOR_MULTIRETURN_RULES'if key is None and i==len(f['fields'])-1 else'SINGLE_VALUE_ADJUSTMENT'})
   return {'kind':'source_table_constructor','entries':entries,'fresh_table_identity':True,'source_field_evaluation_and_assignment_rules_preserved':True,'nil_assignments_and_duplicate_keys_not_collapsed':True,'source_ref':ref}
  if t in ('Name','Index'):
   projected=self.fields.project(node,pointer)
   return self.replace_refs(projected)if projected else self.unresolved(node,pointer,'FIELD_OR_NAME_PROJECTOR_REJECTED')
  if t=='Invoke':
   entity=self.entity.project(node,pointer)if self.entity else None
   if entity:
    entity=self.replace_refs(entity);entity['receiver_expression']=self.compose(f['source'],pointer+'/fields/source');entity['call_evaluation_order']=['READ_RECEIVER_ONCE','LOOKUP_MEMBER','EVALUATE_EXPLICIT_ARGUMENTS','CALL_WITH_RECEIVER_SELF'];return entity
   getter=self.getters.project(node,pointer)if self.getters else None
   if getter:
    getter['receiver']=self.compose(f['source'],pointer+'/fields/source');getter['arguments']=[self.compose(x,pointer+'/fields/args/'+str(i))for i,x in enumerate(f['args'])]
    getter['chain_order']=['READ_RECEIVER_ONCE','LOOKUP_MEMBER','EVALUATE_EXPLICIT_ARGUMENTS','CALL_WITH_RECEIVER_SELF'];return getter
   return self.unresolved(node,pointer,'CALL_METHOD_NO_PINNED_IMPLEMENTATION_SPEC')
  if t=='Call':
   helper=self.helpers.project(node,pointer)if self.helpers else None
   if helper:
    callee='type'if helper['lowered']['operation']=='SOURCE_LUA_TYPE_QUERY'else'os.time'if helper['lowered']['operation']=='SOURCE_LUA_OS_TIME_NOW'else None
    if callee and callee in self.builtins:
     helper['dependencies'].append(self.builtins[callee]);helper['lowered']['external_builtin_body_captured']=True
     if callee=='os.time':helper['lowered']['host_failure_result']='NIL';helper['lowered']['number_result_only_when_no_host_failure']=True
    helper['lowered']['arguments']=[self.compose(x,pointer+'/fields/args/'+str(i))for i,x in enumerate(f['args'])]
    return {'kind':'source_helper_call',**helper,'callee_expression':self.compose(f['func'],pointer+'/fields/func'),'call_evaluation_order':['READ_CALLEE','EVALUATE_ARGUMENTS','CALL_WITH_LUA_MULTIRETURN_ADJUSTMENT'],'source_ref':ref}
   getter=self.getters.project(node,pointer)if self.getters else None
   if getter:
    getter['arguments']=[self.compose(x,pointer+'/fields/args/'+str(i))for i,x in enumerate(f['args'])];getter['callee_expression']=self.compose(f['func'],pointer+'/fields/func');return getter
   return self.unresolved(node,pointer,'CALL_FUNCTION_NO_PINNED_IMPLEMENTATION_SPEC')
  return self.unresolved(node,pointer,'GRAMMAR_OUTSIDE_CLOSED_GUARD_SPEC_VOCABULARY_'+t)


def reasons(value):
 result=[]
 for n in walk(value):
  if n.get('native_admission')is True or n.get('runtime_activation')is True:result.append('RUNTIME_OR_NATIVE_PROMOTION_FORBIDDEN')
  if n.get('kind')in('source_operation_unresolved','operand_dependency_unresolved','operand_reference'):result.append(n.get('reason',n['kind']))
  if n.get('status')in('SOURCE_API_DEFINITION_MISSING','SOURCE_EXTERNAL_LIBRARY_SPEC_UNPROVEN'):result.append(n['status'])
  if n.get('source_spec_complete')is False:result.append('SOURCE_OPERATION_SPEC_INCOMPLETE')
  if n.get('remaining_dependencies'):result.extend(n['remaining_dependencies'])
 return sorted(set(result))


def build(conditions_path,corpus_manifest,ast_root,lane_root=HERE.parent,authoring=None):
 packet=json.loads(conditions_path.read_text());rows=packet if isinstance(packet,list)else packet['records'];rows=[r for r in rows if r['status']=='PARTIAL_SOURCE_GUARD']
 if authoring:sys.path.insert(0,str(authoring))
 else:
  for parent in HERE.parents:
   if(parent/'donor_semantic_conditions.py').exists():sys.path.insert(0,str(parent));break
 from donor_semantic_conditions import load_ast_inputs
 corpus_data=json.loads(corpus_manifest.read_text())
 pins={(r['provenance']['source'],r['provenance']['revision'])for r in rows}
 library_paths={'data-global/scripts/lib/a_piece_of_cake_config.lua','data-otservbr-global/lib/others/soulpit.lua','data-global/lib/others/soulpit.lua'}
 wanted={r['provenance']['sha256']for r in rows}|{r['sha256']for r in corpus_data['files']if(r['source'],r['revision'])in pins and r['path']in library_paths}
 index_bytes,index,payloads=load_ast_inputs(ast_root,wanted)
 fields=load(lane_root/'fields/builder.py','guard_fields');helpers=load(lane_root/'helpers/builder.py','guard_helpers');entity=load(lane_root/'entity_predicates/engine.py','guard_entities');getters=load(lane_root/'entity_values/getter_values.py','guard_getters')
 helper_cache=helpers.Cache(corpus_manifest,ast_root);getter_catalog=getters.Catalog(corpus_manifest,ast_root);external_builtins=builtin_specs(lane_root/'entity_values/dependency',getter_catalog.external);records=[];api_specs={};property_cache={};global_cache={};captures={}
 class Loader:
  def capture(self,ident,raw):
   cap=json.loads(gzip.decompress(payloads[ident]))
   if base64.b64decode(cap['raw_bytes_base64'])!=raw:raise ValueError('AST/source raw mismatch')
   return cap['ast']
 loader=Loader();corpus_rows=list(helper_cache.files.values())
 for row in rows:
  p=row['provenance'];raw=helper_cache.raw(p);ident=p['sha256']
  if ident not in captures:
   cap=json.loads(gzip.decompress(payloads[ident]));decoded=gzip.decompress(payloads[ident])
   if sha(decoded)!=index[ident]['capture_sha256']or cap['status']!='PARSED'or base64.b64decode(cap['raw_bytes_base64'])!=raw:raise ValueError('Qualified AST mismatch')
   captures[ident]=cap['ast']
  ast=captures[ident];pin=(p['source'],p['revision'])
  if pin not in property_cache:property_cache[pin]=fields.property_evidence(*pin,corpus_rows,corpus_manifest)
  if pin not in global_cache:global_cache[pin]=fields.library_evidence(*pin,corpus_rows,corpus_manifest,loader)
  field_engine=fields.Engine(ast,raw,p,property_witnesses=property_cache[pin],global_witnesses=global_cache[pin])
  composer=Composer(ast,raw,p,field_engine,builtins=external_builtins[p['source']])
  composer.helpers=helpers.Engine(helper_cache,p);composer.entity=entity.Engine(p,raw,ast,corpus_manifest,field_engine)
  composer.getters=getters.Engine(row|{'full_ast':ast,'raw':raw},catalog=getter_catalog,operand_projector=composer.compose)
  ptr=row['source_ref']['condition_ast_pointer'];node=get(ast,ptr);ref=composer.source(node,ptr)
  if ref.get('slice_sha256')!=row['source_ref']['slice_sha256']or ref.get('expression')!=row['source_ref']['expression']:raise ValueError('Condition Source witness mismatch')
  typed=composer.compose(node,ptr);unresolved=reasons(typed)
  for spec in composer.entity.api_specs.values():api_specs[spec['api_spec_id']]=spec
  records.append({'interaction':row['interaction'],'quest_keys':row['quest_keys'],'baseline_gap':row['baseline_gap'],'provenance':p,'source_component_id':row['source_component_id'],'source_ref':row['source_ref'],'baseline_record_sha256':sha(stable(row)),'baseline_status':row['status'],'status':'SOURCE_SPEC_COMPLETE_EXECUTION_UNPROVEN'if not unresolved else'SOURCE_SPEC_PARTIAL_UNRESOLVED','typed_operation':typed,'unresolved_dependencies':unresolved,'native_admission':False,'runtime_activation':False,'canonical_gap_replaced':False})
 result={'schema':'OTERYN_COMPOSED_SOURCE_GUARD_SPECS/v1','scope':'OPERATIONAL_SOURCE_SPECIFICATION_NOT_HISTORICAL_GUARD_PROMOTION','conditions_sha256':sha(conditions_path.read_bytes()),'corpus_manifest_sha256':sha(corpus_manifest.read_bytes()),'ast_index_sha256':sha(index_bytes),'records':records,'entity_api_specs':sorted(api_specs.values(),key=lambda s:s['api_spec_id']),'summary':{'original_partial_guards':len(rows),'complete_execution_unproven_specs':sum(not r['unresolved_dependencies']for r in records),'partial_specs':sum(bool(r['unresolved_dependencies'])for r in records),'native_admission':False,'canonical_gap_replacements':0},'native_admission':False}
 return result


def main():
 ap=argparse.ArgumentParser()
 for key in('conditions','corpus-manifest','ast-root','out'):ap.add_argument('--'+key,type=Path,required=True)
 ap.add_argument('--lane-root',type=Path,default=HERE.parent);ap.add_argument('--authoring',type=Path)
 a=ap.parse_args();packet=build(a.conditions,a.corpus_manifest,a.ast_root,a.lane_root,a.authoring);a.out.write_text(json.dumps(packet,ensure_ascii=False,indent=2)+'\n');print(json.dumps(packet['summary']))
if __name__=='__main__':main()
