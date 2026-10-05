import unittest
from builder import Engine,HELPERS,operand,name

def n(kind,**fields):return {'node_type':kind,'fields':fields,'span':None}
def call(callee,args):
 names=callee.split('.'); f=n('Name',id=names[0])
 for s in names[1:]:f=n('Index',value=f,idx=n('Name',id=s),notation={'name':'DOT'})
 return n('Call',func=f,args=args)
class Cache:
 def dependencies(self,p,c):return [{'callee':c,'contract':HELPERS[c]['contract'],'source_body_verified':True}]
 def builtin_witnesses(self,p):return [{'kind':'SOURCE_HOST_STANDARD_LIBRARY_OPEN','host_builtin_body_in_corpus':False,'execution_proven':False}]
class ClosedHelperTests(unittest.TestCase):
 def setUp(self):self.engine=Engine(Cache(),{})
 def test_type_preserves_type_query_not_userdata_cast(self):
  q=self.engine.project(call('type',[n('Name',id='target')]),'/atom')
  self.assertEqual(q['lowered']['operation'],'SOURCE_LUA_TYPE_QUERY')
  self.assertTrue(q['lowered']['nil_userdata_distinction_preserved']);self.assertFalse(q['binding_witness']['execution_binding_proven'])
 def test_type_arity_unknown_call_and_os_time_table_form_remain_outside(self):
  for c,args in [('type',[]),('type',[n('Name',id='x'),n('Name',id='y')]),('unknown',[]),('os.time',[n('Table',fields=[])])]:
   with self.subTest(c=c,args=args):self.assertIsNone(self.engine.project(call(c,args),'/atom'))
 def test_contains_vs_find_preserve_key_return_nil_false_and_pairs_order(self):
  contains=self.engine.project(call('table.contains',[n('Name',id='a'),n('Name',id='v')]),'/atom')['lowered']['contract']
  find=self.engine.project(call('table.find',[n('Name',id='a'),n('Name',id='v')]),'/atom')['lowered']['contract']
  self.assertIs(contains['miss_return'],False);self.assertEqual(find['miss_return'],'NIL');self.assertEqual(find['match_return'],'MATCHED_ORIGINAL_KEY')
  self.assertEqual(find['iteration_order'],'SOURCE_PAIRS_ORDER_UNSPECIFIED');self.assertTrue(find['zero_key_truthy'])
 def test_cake_defaults_only_nil_and_soul_retains_raw_values(self):
  cake=self.engine.project(call('CakeQuest.getStage',[]),'/atom')['lowered']['contract']
  soul=self.engine.project(call('SoulWarQuest.ebbAndFlow.isActive',[]),'/atom')['lowered']['contract']
  self.assertEqual(cake['default_only_on'],'NIL');self.assertTrue(cake['false_is_not_defaulted']);self.assertEqual(soul['return'],'RAW_KV_VALUE_WITH_NIL_FALSE_DISTINCT')
 def test_literal_array_preserves_allocation_and_original_entries(self):
  array=n('Table',fields=[n('Field',key=None,value=n('Number',n=15710)),n('Field',key=None,value=n('Number',n=15711))])
  q=self.engine.project(call('table.contains',[array,n('Name',id='value')]),'/atom')['lowered']
  self.assertEqual([e['value']['value']for e in q['arguments'][0]['entries']],[15710,15711]);self.assertTrue(q['arguments'][0]['allocation_and_source_evaluation_preserved'])
 def test_live_field_read_not_membership_snapshot(self):
  arr=n('Index',value=n('Name',id='firstStageConfig'),idx=n('Name',id='shallowWaterBorderIds'),notation={'name':'DOT'})
  q=self.engine.project(call('table.contains',[arr,n('Name',id='value')]),'/atom')['lowered']
  self.assertEqual(q['arguments'][0]['operation'],'SOURCE_INDEX_READ');self.assertTrue(q['arguments'][0]['errors_preserved'])
 def test_time_remains_wall_clock_and_subtraction_keeps_operand_order(self):
  subtraction=n('SubOp',left=call('os.time',[]),right=n('Name',id='lastEnd'))
  q=operand(Cache(),{},subtraction,'/atom',None)
  self.assertEqual(q['left']['operation'],'SOURCE_LUA_OS_TIME_NOW');self.assertEqual(q['right']['name'],'lastEnd');self.assertTrue(q['left']['wall_clock_not_monotonic']);self.assertTrue(q['metamethod_coercion_and_errors_preserved'])
 def test_long_bracket_parser_bytes_not_assumed_to_be_decoded_semantics(self):
  text=n('String',s={'bytes_base64':'Cng='},delimiter={'name':'LONG_BRACKET'})
  q=operand(Cache(),{},text,'/text',None)
  self.assertEqual(q['operation'],'SOURCE_LUA_STRING_LITERAL_TOKEN');self.assertNotIn('value',q)
  self.assertTrue(q['value_not_assumed_from_parser_bytes'])
 def test_mixed_constructor_named_keys_do_not_advance_implicit_sequence(self):
  array=n('Table',fields=[n('Field',key=n('String',s={'bytes_base64':'eA=='}),value=n('Number',n=10)),n('Field',key=None,value=n('Number',n=20))])
  q=operand(Cache(),{},array,'/table',None)
  self.assertEqual(q['entries'][1]['key']['ordinal'],1)
if __name__=='__main__':unittest.main()
