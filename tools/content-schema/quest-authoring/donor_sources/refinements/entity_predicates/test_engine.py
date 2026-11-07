import copy,importlib.util,json,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('entity_source_engine',Path(__file__).with_name('engine.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)

def packet_views():
 path=Path(__file__).with_name('entity-atoms.json')
 if path.exists():
  data=json.loads(path.read_text());return data,data['api_specs'],[a['projection']for a in data['atoms']],False
 for parent in Path(__file__).resolve().parents:
  path=parent/'samples/donor-source/refinements/guard-specs.json'
  if path.exists():
   data=json.loads(path.read_text());operations=[]
   def walk(value):
    if isinstance(value,dict):
     if value.get('kind')=='source_entity_api_operation':operations.append(value)
     for item in value.values():walk(item)
    elif isinstance(value,list):
     for item in value:walk(item)
   walk(data['records']);return data,data['entity_api_specs'],operations,True
 return None

def projection_view(projection,schema):
 fields=set(schema['$defs']['projection']['properties'])
 extra=set(projection)-fields
 if extra:
  if extra!={'receiver_expression','call_evaluation_order'}:raise AssertionError('Unexpected unvalidated composer fields: '+str(extra))
  receiver=projection['receiver_expression']
  if receiver['kind']!='source_named_operand' or receiver['name']!=projection['receiver']['symbol']:raise AssertionError('Composer receiver differs from actual lexical entity receiver')
  if receiver['binding']!=projection['receiver']['lexical_binding']:raise AssertionError('Composer receiver binding differs from engine proof')
  if receiver['native_admission']is not False or receiver['runtime_activation']is not False:raise AssertionError('Receiver falsely admitted')
  if projection['call_evaluation_order']!=['READ_RECEIVER_ONCE','LOOKUP_MEMBER','EVALUATE_EXPLICIT_ARGUMENTS','CALL_WITH_RECEIVER_SELF']:raise AssertionError('Composer call evaluation order changed')
 return {key:value for key,value in projection.items()if key in fields}

class BindingFixture:
 def __init__(self,bound=True):self.bound=bound
 def resolve(self,symbol,pointer):return {'kind':'function_argument','name':symbol,'declaration_pointer':'/argument','scope_pointer':''}if self.bound else None
class EntityProjectionTests(unittest.TestCase):
 def engine(self,method='isPlayer',args=None,bound=True):
  raw=('x:'+method+'()').encode();node={'node_type':'Invoke','fields':{'source':{'node_type':'Name','fields':{'id':'x'}},'func':{'node_type':'Name','fields':{'id':method}},'args':args or []},'span':{'start_char':0,'end_char_exclusive':len(raw)}}
  value=object.__new__(module.Engine);value.source={'source':'test','revision':'a'*40,'sha256':module.sha(raw)};value.raw=raw;value.text=raw.decode();value.ast={'node':node};value.bindings=BindingFixture(bound);value.api_specs={method:{'api_spec_id':'test:entity-api/'+method}};value.corpus=None
  return value,node
 def test_projection_preserves_lookup_errors_and_unknown_runtime_class(self):
  engine,node=self.engine();p=engine.project(node,'/node');self.assertTrue(p['dispatch_unknown']);self.assertTrue(p['receiver']['runtime_class_not_inferred']);self.assertTrue(p['call_semantics']['member_lookup_may_error']);self.assertFalse(p['native_admission']);self.assertEqual(p['status'],'SOURCE_SPECIFIED_EXECUTION_UNPROVEN')
 def test_no_lexical_binding_not_projected(self):
  engine,node=self.engine(bound=False);self.assertIsNone(engine.project(node,'/node'))
 def test_wrong_arity_not_projected(self):
  engine,node=self.engine(args=[{'node_type':'Number','fields':{'n':1}}]);self.assertIsNone(engine.project(node,'/node'))
 def test_unknown_method_not_projected(self):
  engine,node=self.engine(method='arbitraryGetter');self.assertIsNone(engine.project(node,'/node'))
 def test_expression_receiver_not_guessed(self):
  engine,node=self.engine();node['fields']['source']={'node_type':'Invoke','fields':{}};self.assertIsNone(engine.project(node,'/node'))
 def test_wrong_ast_pointer_rejected(self):
  engine,node=self.engine()
  with self.assertRaises(ValueError):engine.project(node,'/fake')
 def test_changed_node_rejected(self):
  engine,node=self.engine();changed=copy.deepcopy(node);changed['fields']['func']['fields']['id']='isMonster'
  with self.assertRaises(ValueError):engine.project(changed,'/node')
 def test_cpp_brace_mask_ignores_comments_strings(self):
  text='int f() { const char*s="}"; /* { */ return 1; }';start,end=module.brace_region(text,0);self.assertEqual(text[start:end],text)
 def test_closed_packet_schema_and_predicate_defaults(self):
  import jsonschema
  views=packet_views()
  if views is None:self.skipTest('Replayed donor guard specification fixture pending')
  d,apis,operations,central=views;schema=json.loads(Path(__file__).with_name('schema.json').read_text())
  if central:
   self.assertEqual(len(d['records']),125);self.assertFalse(d['native_admission'])
   for projection in operations:jsonschema.validate(projection_view(projection,schema),{'$ref':'#/$defs/projection','$defs':schema['$defs']})
   for api in apis:jsonschema.validate(api,{'$ref':'#/$defs/api_spec','$defs':schema['$defs']})
  else:
   jsonschema.validate(d,schema);self.assertEqual(d['original_partial_guards_retained'],125);self.assertEqual(d['original_opaque_guards_replaced'],0)
  self.assertEqual(len(operations),78);self.assertEqual(len(apis),13)
  by_id={a['api_spec_id']:a for a in apis}
  for api in apis:
   if api['method']in {'isPlayer','isMonster','isItem','isTeleport'}:
    creature=next(c for c in api['cases']if c['declaring_class']=='Creature');self.assertIs(creature['operation']['value'],False)
   if api['method']=='isTeleport':self.assertIs(next(c for c in api['cases']if c['declaring_class']=='Teleport')['operation']['value'],True)
   if api['method']in {'getPlayer','getMonster'}:
    operation=api['cases'][0]['operation'];self.assertEqual(operation['true_result'],'SAME_RECEIVER_IDENTITY');self.assertEqual(operation['false_result'],'NIL');self.assertIn(operation['nested_api_spec_ref'],by_id)
  kinds={api['api_spec_id'].split(':')[0]:api['userdata_conversion']for api in apis};self.assertEqual(kinds['canary'],'METATABLE_TEST_AND_INHERITANCE_CHECK');self.assertEqual(kinds['crystalserver'],'TYPED_SHARED_PAYLOAD_CAST_WITHOUT_METATABLE_CHECK')
 def test_schema_prevents_native_or_proven_dispatch_promotion(self):
  import jsonschema
  views=packet_views()
  if views is None:self.skipTest('Replayed donor guard specification fixture pending')
  _,_,operations,_=views;schema=json.loads(Path(__file__).with_name('schema.json').read_text());projection=copy.deepcopy(operations[0]);projection['dispatch_unknown']=False
  with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(projection_view(projection,schema),{'$ref':'#/$defs/projection','$defs':schema['$defs']})
if __name__=='__main__':unittest.main()
