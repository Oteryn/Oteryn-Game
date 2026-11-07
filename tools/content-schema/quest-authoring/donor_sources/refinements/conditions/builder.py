"""Concrete donor guards: lexical truthiness and bounded proven-value comparisons."""
import argparse,base64,gzip,hashlib,json,sys,tempfile
from collections import Counter
from pathlib import Path

def sha(b):return hashlib.sha256(b).hexdigest()
def walk(v,p=''):
 if isinstance(v,dict):
  if 'node_type'in v:yield p,v
  for k,x in v.items():yield from walk(x,p+'/'+k)
 elif isinstance(v,list):
  for i,x in enumerate(v):yield from walk(x,p+'/'+str(i))
def name(n):return n['fields']['id'] if isinstance(n,dict) and n.get('node_type')=='Name' else None

class Bindings:
 def __init__(self,ast,semantic=None,source_text=None):self.nodes=list(walk(ast));self.semantic=semantic;self.source_text=source_text
 def resolve(self,symbol,use_pointer):
  candidates=[]
  for ptr,n in self.nodes:
   f=n['fields'];kind=n['node_type']
   if kind in {'Function','LocalFunction','AnonymousFunction','Method'}:
    scope=ptr+'/fields/body'
    if use_pointer.startswith(scope+'/'):
     for i,a in enumerate(f.get('args',[])):
      if name(a)==symbol:candidates.append((len(scope),-1,{'kind':'function_argument','name':symbol,'declaration_pointer':ptr+'/fields/args/'+str(i),'scope_pointer':scope}))
   if kind=='LocalAssign' and ptr.rsplit('/',1)[-1].isdigit():
    scope,index=ptr.rsplit('/',1);prefix=scope+'/'
    if use_pointer.startswith(prefix):
     use=use_pointer[len(prefix):].split('/',1)[0]
     if use.isdigit() and int(use)>int(index):
      for i,a in enumerate(f['targets']):
       if name(a)==symbol:candidates.append((len(scope),int(index),{'kind':'local_declaration','name':symbol,'declaration_pointer':ptr+'/fields/targets/'+str(i),'scope_pointer':scope}))
   if kind in {'Forin','Fornum'}:
    scope=ptr+'/fields/body'
    if use_pointer.startswith(scope+'/'):
     targets=f.get('targets',[f.get('target')])
     for i,a in enumerate(targets):
      if name(a)==symbol:candidates.append((len(scope),-1,{'kind':'loop_binding','name':symbol,'declaration_pointer':ptr+'/fields/'+('targets/'+str(i) if 'targets'in f else 'target'),'scope_pointer':scope}))
  return max(candidates,key=lambda x:(x[0],x[1]))[2] if candidates else None
 def value_operand(self,n,p,line):
  kind=n['node_type'];f=n['fields']
  if kind=='Name':
   binding=self.resolve(f['id'],p)
   if binding:return {'kind':'bound_value_read','binding':binding,'value_type_not_inferred':True}
  if kind=='Number' and type(f['n'])is int and abs(f['n'])<=2**53-1:return {'kind':'literal_value','literal_type':'integer','value':f['n']}
  if kind=='UMinusOp' and f['operand']['node_type']=='Number' and type(f['operand']['fields']['n'])is int and abs(f['operand']['fields']['n'])<=2**53-1:return {'kind':'literal_value','literal_type':'integer','value':-f['operand']['fields']['n']}
  if kind=='String':
   # The pinned upstream AST does not apply Lua's long-bracket initial-newline
   # rule. Keep all long-bracket forms opaque instead of altering the parser.
   if f.get('delimiter',{}).get('name')not in {'SINGLE_QUOTE','DOUBLE_QUOTE'}:return None
   try:value=base64.b64decode(f['s']['bytes_base64']).decode('utf-8')
   except (KeyError,UnicodeDecodeError,ValueError):return None
   return {'kind':'literal_value','literal_type':'string','value':value}
  if kind in {'TrueExpr','FalseExpr'}:return {'kind':'literal_value','literal_type':'boolean','value':kind=='TrueExpr'}
  if kind=='Nil':return {'kind':'literal_value','literal_type':'nil','value':None}
  if self.semantic and line is not None:
   proven=self.semantic.operand(n,line)
   if proven and proven['kind']in {'quest_progress_read','donor_wall_clock'}:return {'kind':'existing_proven_source_operand','operand':proven,'ast_pointer':p}
  return None
 def normalize(self,n,p):
  kind=n['node_type'];f=n['fields']
  if kind=='Name':
   binding=self.resolve(f['id'],p)
   return {'kind':'bound_value_truthiness','binding':binding,'false_values':['NIL','BOOLEAN_FALSE'],'zero_and_empty_string_are_truthy':True} if binding else {'kind':'opaque','reason':'NO_PROVEN_LEXICAL_BINDING','ast_pointer':p}
  if kind in {'TrueExpr','FalseExpr','Nil'}:return {'kind':'literal_truthiness','literal_kind':kind,'truthy':kind=='TrueExpr'}
  if kind=='ULNotOp':return {'kind':'not','child':self.normalize(f['operand'],p+'/fields/operand')}
  if kind in {'AndLoOp','OrLoOp'}:return {'kind':'boolean_truthiness','operator':'and' if kind=='AndLoOp' else 'or','children':[self.normalize(f[k],p+'/fields/'+k) for k in ['left','right']],'source_order_preserved':True,'lua_short_circuit_preserved':True}
  comparison_ops={'EqToOp':'==','NotEqToOp':'~=','LessThanOp':'<','LessOrEqThanOp':'<=','GreaterThanOp':'>','GreaterOrEqThanOp':'>='}
  line=n.get('span',{}).get('line') if n.get('span')else None
  if kind in comparison_ops:
   left=self.value_operand(f['left'],p+'/fields/left',line);right=self.value_operand(f['right'],p+'/fields/right',line)
   if left is not None and right is not None:
    return {'kind':'source_value_comparison','operator':comparison_ops[kind],'left':left,'right':right,
            'lua_coercion_errors_and_metamethod_behavior_preserved':True,'value_types_not_inferred':True,'native_admission':False}
  return {'kind':'opaque','reason':'OUTSIDE_BOUND_TRUTHINESS_VOCABULARY','ast_pointer':p}

def count(v,kind):
 if isinstance(v,dict):return int(v.get('kind')==kind)+sum(count(x,kind) for x in v.values())
 if isinstance(v,list):return sum(count(x,kind) for x in v)
 return 0

def build(authoring,corpus,ast_root,product_root):
 sys.path.insert(0,str(authoring));import ots_interactions as oi;import donor_semantic_conditions as base
 manifest=json.loads(corpus.read_text());sources={(r['source'],r['path']):r for r in manifest['files']}
 progress=json.loads((authoring/'samples/questlog/progress.json').read_text())['progress'];declared=oi.declared_progress_paths(progress)
 temp=tempfile.TemporaryDirectory();source_cache=Path(temp.name)
 input_path=authoring/'samples/interactions/interactions.json';graphs=json.loads(input_path.read_text())['interactions'];entries={r['destination']:r for r in json.loads((authoring/'samples/interactions/manifest.json').read_text())['entries']}
 owners={}
 for shard in sorted((product_root/'content/quests/definitions').glob('*.json')):
  for record in json.loads(shard.read_text()).get('records',[]):
   definition=record['definition']
   for graph in definition.get('source_data',{}).get('interactions',[]):owners.setdefault(graph['identity']['key'],set()).add(definition['identity']['key'])
 selected=[];wanted=set()
 for i,g in enumerate(graphs):
  key=g['identity']['key'];entry=entries[key];donor=oi.CONFLICT_DECISIONS['interactions'].get(key,{}).get('decision',key.split(':',1)[0]);s=next((s for s in entry['sources'] if s['source']==donor),entry['sources'][0]);source=sources[(s['source'],s['path'])];gaps=list(base.unresolved_refs(g['rules'],'/interactions/'+str(i)+'/rules'))
  if gaps:selected.append((g,source,gaps));wanted.add(source['sha256'])
 index_raw,index_rows,payloads=base.load_ast_inputs(ast_root,wanted);cache={};out=[];counts=Counter()
 for graph,source,gaps in selected:
  digest=source['sha256'];cache_key=(source['source'],source['revision'],source['path'],digest)
  if cache_key not in cache:
   capture=json.loads(gzip.decompress(payloads[digest]));raw=base64.b64decode(capture['raw_bytes_base64']);assert sha(raw)==digest and capture['native_semantic_admission']is False and capture['status']=='PARSED'
   text=raw.decode('utf-8');source_path=source_cache/source['source']/source['path'];source_path.parent.mkdir(parents=True,exist_ok=True);source_path.write_bytes(raw)
   script=oi.Script(source['source'],source_cache/source['source'],source['path'],{},'source-truthiness');script.declared=declared
   bindings=Bindings(capture['ast'],source_text=text);byline={}
   for ptr,n in bindings.nodes:
    if n['node_type']in {'If','ElseIf'} and n.get('span'):byline.setdefault(n['span']['line'],[]).append((ptr,n))
   cache[cache_key]=(capture,raw,text,bindings,byline,script)
  capture,raw,text,bindings,byline,script=cache[cache_key]
  for pointer,line in gaps:
   if len(byline.get(line,[]))!=1:continue
   ptr,node=byline[line][0];test=node['fields']['test']
   starts=[n for n,l in enumerate(script.code_lines,1) if oi.CALLBACK.match(l) and n<line];callback=max(starts)if starts else None
   bindings.semantic=None
   if callback is not None:
    cb=oi.CALLBACK.match(script.lines[callback-1]);script.bind(callback,cb.group(2));bindings.semantic=base.Normalizer(script,oi,{r['key']for r in progress},text)
   normalized=bindings.normalize(test,ptr+'/fields/test');bound=count(normalized,'bound_value_truthiness')
   if not bound:continue
   opaque=count(normalized,'opaque');span=node['span'];tokens=[t for t in capture['tokens'] if t['channel']==0 and span['start_char']<=t['start_char']<span['end_char_exclusive']]
   if not tokens or tokens[0]['symbol']not in {'IF','ELSEIF'}:continue
   stop=next((i for i,t in enumerate(tokens[1:],1) if t['symbol']=='THEN'),None)
   if stop is None or stop<2:continue
   test_span=test.get('span')
   if test_span:start,end=test_span['start_char'],test_span['end_char_exclusive']
   elif test['node_type']=='Name':
    if stop!=2 or tokens[1]['text']!=test['fields']['id']:continue
    start,end=tokens[1]['start_char'],tokens[1]['end_char_exclusive']
   else:continue
   bs,be=len(text[:start].encode()),len(text[:end].encode())
   out.append({'interaction':graph['identity']['key'],'quest_keys':sorted(owners.get(graph['identity']['key'],[])),
    'baseline_gap':{'file':'samples/interactions/interactions.json','file_sha256':sha(input_path.read_bytes()),'json_pointer':pointer,'line':line},
    'source_component_id':':'.join([source['source'],source['revision'],source['path']]),'provenance':{k:source[k] for k in ['source','repository','revision','path','git_blob_sha1','sha256','byte_count']},
    'source_ref':{'condition_ast_pointer':ptr+'/fields/test','start_char':start,'end_char_exclusive':end,'byte_start':bs,'byte_end_exclusive':be,'expression':text[start:end],'slice_sha256':sha(raw[bs:be])},
    'normalized':normalized,'bound_truthiness_leaves':bound,'opaque_leaves':opaque,'status':'FULL_SOURCE_GUARD' if opaque==0 else 'PARTIAL_SOURCE_GUARD',
    'canonical_gap_replaced':False,'native_admission':False,'runtime_activation':False})
   counts['guards_supplemented']+=1;counts['full_source_guards' if not opaque else 'partial_source_guards']+=1;counts['bound_truthiness_leaves']+=bound
   counts['source_value_comparisons']+=count(normalized,'source_value_comparison');counts['opaque_leaves_remaining']+=opaque
 temp.cleanup()
 return {'schema':'OTERYN_BOUND_SOURCE_GUARD_SUPPLEMENT/v1','scope':'LEXICALLY_PROVEN_LUA_TRUTHINESS_AND_BOUNDED_VALUE_COMPARISONS','input_sha256':sha(input_path.read_bytes()),'ast_index_sha256':sha(index_raw),'decoder_sha256':sha(Path(__file__).read_bytes()),'counts':dict(counts),'quest_count':len({q for r in out for q in r['quest_keys']}),'records':out,'canonical_gap_replacements':0,'native_admission':False}

def main():
 p=argparse.ArgumentParser();p.add_argument('--authoring',type=Path,required=True);p.add_argument('--corpus-manifest',type=Path,required=True);p.add_argument('--ast-root',type=Path,required=True);p.add_argument('--product-root',type=Path,required=True);p.add_argument('--out',type=Path,required=True);a=p.parse_args();d=build(a.authoring,a.corpus_manifest,a.ast_root,a.product_root);a.out.write_text(json.dumps(d,sort_keys=True,indent=2)+'\n');print(json.dumps({'counts':d['counts'],'quest_count':d['quest_count']}))
if __name__=='__main__':main()
