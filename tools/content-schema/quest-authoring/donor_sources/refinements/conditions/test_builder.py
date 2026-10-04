import importlib.util,json,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('truthiness_builder',Path(__file__).with_name('builder.py'));b=importlib.util.module_from_spec(spec);spec.loader.exec_module(b)
def node(kind,**f):return {'node_type':kind,'fields':f,'span':None}
def local(name):return node('LocalAssign',targets=[node('Name',id=name)],values=[node('Nil')])
def guard(name):return node('If',test=node('Name',id=name),body=node('Block',body=[]),orelse=None)
def chunk(items):return node('Chunk',body=node('Block',body=items))
class TruthinessTests(unittest.TestCase):
 def test_prior_local_is_proven(self):
  ast=chunk([local('tile'),guard('tile')]);bindings=b.Bindings(ast);r=bindings.normalize(ast['fields']['body']['fields']['body'][1]['fields']['test'],'/fields/body/fields/body/1/fields/test')
  self.assertEqual(r['kind'],'bound_value_truthiness');self.assertEqual(r['false_values'],['NIL','BOOLEAN_FALSE']);self.assertTrue(r['zero_and_empty_string_are_truthy'])
 def test_future_declaration_not_visible(self):
  ast=chunk([guard('tile'),local('tile')]);self.assertIsNone(b.Bindings(ast).resolve('tile','/fields/body/fields/body/0/fields/test'))
 def test_sibling_branch_declaration_not_visible(self):
  first=node('If',test=node('TrueExpr'),body=node('Block',body=[local('tile')]),orelse=node('Block',body=[guard('tile')]))
  ast=chunk([first]);self.assertIsNone(b.Bindings(ast).resolve('tile','/fields/body/fields/body/0/fields/orelse/fields/body/0/fields/test'))
 def test_local_initializer_cannot_bind_itself(self):
  ast=chunk([node('LocalAssign',targets=[node('Name',id='tile')],values=[node('Name',id='tile')])]);self.assertIsNone(b.Bindings(ast).resolve('tile','/fields/body/fields/body/0/fields/values/0'))
 def test_argument_visible_only_in_its_body(self):
  fn=node('Function',args=[node('Name',id='creature')],body=node('Block',body=[guard('creature')]))
  ast=chunk([fn,guard('creature')]);bs=b.Bindings(ast);self.assertEqual(bs.resolve('creature','/fields/body/fields/body/0/fields/body/fields/body/0/fields/test')['kind'],'function_argument');self.assertIsNone(bs.resolve('creature','/fields/body/fields/body/1/fields/test'))
 def test_shadowing_chooses_inner_declaration(self):
  ast=chunk([local('tile'),node('If',test=node('TrueExpr'),body=node('Block',body=[local('tile'),guard('tile')]),orelse=None)]);bs=b.Bindings(ast);binding=bs.resolve('tile','/fields/body/fields/body/1/fields/body/fields/body/1/fields/test');self.assertEqual(binding['declaration_pointer'],'/fields/body/fields/body/1/fields/body/fields/body/0/fields/targets/0')
 def test_short_circuit_unknown_sibling_retained(self):
  ast=chunk([local('tile')]);bs=b.Bindings(ast);n=node('AndLoOp',left=node('Name',id='tile'),right=node('Call',func=node('Name',id='unknown'),args=[]));r=bs.normalize(n,'/fields/body/fields/body/1/fields/test');self.assertEqual(b.count(r,'opaque'),1);self.assertEqual(b.count(r,'bound_value_truthiness'),1);self.assertTrue(r['lua_short_circuit_preserved'])
 def test_global_name_not_assumed(self):
  r=b.Bindings(chunk([])).normalize(node('Name',id='globalThing'),'/fields/body/fields/body/0');self.assertEqual(r['kind'],'opaque')
 def test_zero_not_false_constant(self):
  r=b.Bindings(chunk([])).normalize(node('Number',n=0),'/x');self.assertEqual(r['kind'],'opaque')
 def test_schema_blocks_native_promotion(self):
  import jsonschema
  p=Path(__file__).with_name('conditions.json')
  if not p.exists():p=Path(__file__).resolve().parents[3] / 'samples/donor-source/refinements/conditions.json'
  if not p.exists():self.skipTest('Lane packet absent')
  d=json.loads(p.read_text());d['records'][0]['native_admission']=True
  schema=json.loads(Path(__file__).with_name('schema.json').read_text())
  with self.assertRaises(jsonschema.ValidationError):jsonschema.validate(d,schema)
 def test_packet_owner_and_native_fences(self):
  p=Path(__file__).with_name('conditions.json')
  if not p.exists():p=Path(__file__).resolve().parents[3] / 'samples/donor-source/refinements/conditions.json'
  if not p.exists():self.skipTest('Lane checked packet absent')
  d=json.loads(p.read_text());self.assertEqual(d['canonical_gap_replacements'],0);self.assertFalse(d['native_admission'])
  for r in d['records']:
   self.assertFalse(r['native_admission']);self.assertFalse(r['runtime_activation']);self.assertFalse(r['canonical_gap_replaced']);self.assertEqual(r['status']=='FULL_SOURCE_GUARD',b.count(r['normalized'],'opaque')==0)

class BoundedComparisonTests(unittest.TestCase):
 def comparison(self,left,right,kind='GreaterThanOp'):
  ast=chunk([local('storedHealth')]);bs=b.Bindings(ast)
  return bs.normalize(node(kind,left=left,right=right),'/fields/body/fields/body/1/fields/test')
 def test_local_numeric_comparison_retains_actual_operand(self):
  result=self.comparison(node('Name',id='storedHealth'),node('Number',n=0))
  self.assertEqual(result['kind'],'source_value_comparison');self.assertEqual(result['left']['binding']['name'],'storedHealth');self.assertTrue(result['value_types_not_inferred']);self.assertTrue(result['lua_coercion_errors_and_metamethod_behavior_preserved'])
 def test_nil_equality_is_not_false_equality(self):
  nil=self.comparison(node('Name',id='storedHealth'),node('Nil'),'NotEqToOp');boolean=self.comparison(node('Name',id='storedHealth'),node('FalseExpr'),'NotEqToOp')
  self.assertEqual(nil['right']['literal_type'],'nil');self.assertIsNone(nil['right']['value']);self.assertEqual(boolean['right']['literal_type'],'boolean');self.assertFalse(boolean['right']['value']);self.assertNotEqual(nil,boolean)
 def test_long_bracket_initial_newline_remains_opaque(self):
  literal=node('String',s={'bytes_base64':'CmhlbGxv'},raw='\nhello',delimiter={'name':'DOUBLE_SQUARE'})
  self.assertEqual(self.comparison(node('Name',id='storedHealth'),literal,'EqToOp')['kind'],'opaque')
 def test_quoted_escape_keeps_verified_decoded_value(self):
  literal=node('String',s={'bytes_base64':'aGVsbG8Kd29ybGQ='},raw=r'hello\nworld',delimiter={'name':'DOUBLE_QUOTE'})
  result=self.comparison(node('Name',id='storedHealth'),literal,'EqToOp')
  self.assertEqual(result['right']['value'],'hello\nworld')
 def test_unbound_comparison_operand_remains_opaque(self):
  r=self.comparison(node('Name',id='unboundGlobal'),node('Number',n=0));self.assertEqual(r['kind'],'opaque')
 def test_arbitrary_method_getter_not_wrapped(self):
  r=self.comparison(node('Invoke',source=node('Name',id='storedHealth'),func=node('Name',id='getName'),args=[]),node('Number',n=0));self.assertEqual(r['kind'],'opaque')
 def test_unknown_field_and_helper_not_wrapped(self):
  for n in [node('Index',value=node('Name',id='storedHealth'),idx=node('Name',id='value'),notation={'name':'DOT'}),node('Call',func=node('Name',id='helper'),args=[])]:self.assertEqual(self.comparison(n,node('Number',n=0))['kind'],'opaque')
 def test_numeric_literal_outside_exact_common_range_stays_opaque(self):
  r=self.comparison(node('Name',id='storedHealth'),node('Number',n=2**53+1));self.assertEqual(r['kind'],'opaque')
 def test_reversed_operand_order_preserved(self):
  r=self.comparison(node('Number',n=0),node('Name',id='storedHealth'),'LessThanOp');self.assertEqual(r['left']['kind'],'literal_value');self.assertEqual(r['right']['kind'],'bound_value_read');self.assertEqual(r['operator'],'<')
 def context(self,tail='',known=True):
  import tempfile,ots_interactions as oi,donor_semantic_conditions as base
  tmp=tempfile.TemporaryDirectory();self.addCleanup(tmp.cleanup);root=Path(tmp.name)
  (root/'f.lua').write_text('local a=Action()\nfunction a.onUse(player,item,fromPosition,target,toPosition)\nlocal diamondItem=true\nif diamondItem and player:getStorageValue(Storage.Quest.X)<1 then\nend\n'+tail+'\nend\na:id(300)\n')
  script=oi.Script('canary',root,'f.lua',{},'fixture');script.declared={oi.track_of('Storage.Quest.X'):'canary:quest-progress/fixture'};script.bind(2,'onUse');semantic=base.Normalizer(script,oi,{'canary:quest-progress/fixture'}if known else set())
  return b.Bindings(chunk([local('diamondItem')]),semantic=semantic)
 def storage_ast(self):
  storage=node('Index',value=node('Index',value=node('Name',id='Storage'),idx=node('Name',id='Quest'),notation={'name':'DOT'}),idx=node('Name',id='X'),notation={'name':'DOT'})
  invoke=node('Invoke',source=node('Name',id='player'),func=node('Name',id='getStorageValue'),args=[storage]);invoke['span']={'line':4}
  comparison=node('LessThanOp',left=invoke,right=node('Number',n=1));comparison['span']={'line':4};return comparison
 def test_declared_progress_reuses_existing_receiver_proof(self):
  r=self.context().normalize(self.storage_ast(),'/fields/body/fields/body/1/fields/test');self.assertEqual(r['kind'],'source_value_comparison');self.assertEqual(r['left']['operand']['progress'],'canary:quest-progress/fixture')
 def test_undeclared_progress_remains_opaque(self):
  self.assertEqual(self.context(known=False).normalize(self.storage_ast(),'/fields/body/fields/body/1/fields/test')['kind'],'opaque')
 def test_shadowed_receiver_or_storage_remains_opaque(self):
  for tail in ['player=other','Storage.Quest.X=10','debug.getmetatable(player)']:
   with self.subTest(tail=tail):self.assertEqual(self.context(tail).normalize(self.storage_ast(),'/fields/body/fields/body/1/fields/test')['kind'],'opaque')

if __name__=='__main__':unittest.main()
