import base64,json,unittest
from builder import Composer,reasons

def n(kind,**fields):return {'node_type':kind,'fields':fields,'span':None}
class Fields:
 def project(self,node,p):
  f=node['fields']
  if node['node_type']=='Name':return {'kind':'source_named_operand','name':f['id'],'binding':{'kind':'function_argument','name':f['id']},'native_admission':False,'runtime_activation':False}
  if node['node_type']=='Index':return {'kind':'source_index_read','receiver':{'kind':'operand_reference','ast_pointer':p+'/fields/value'},'key':{'kind':'literal_field_key','value':f['idx']['fields']['id'],'type':'string'},'native_admission':False,'runtime_activation':False}
class Helper:
 def project(self,n,p):
  if n['fields']['func']['fields']['id']!='type':return None
  return {'lowered':{'operation':'SOURCE_LUA_TYPE_QUERY','arguments':[{'operation':'IGNORED_RAW_ARGUMENT'}]},'dependencies':[],'binding_witness':{'execution_binding_proven':False}}
class Getter:
 def project(self,n,p):return {'kind':'source_getter_value','method':'getId','receiver':{'kind':'operand_dependency_unresolved'},'arguments':[],'source_spec_complete':True,'remaining_dependencies':[],'native_admission':False,'runtime_activation':False}

class CompositionTests(unittest.TestCase):
 def make(self,ast,**kwargs):return Composer(ast,b'',{},Fields(),**kwargs)
 def test_short_circuit_keeps_original_values_and_right_conditional(self):
  ast=n('AndLoOp',left=n('Name',id='a'),right=n('Name',id='b'));q=self.make(ast).compose(ast,'')
  self.assertEqual(q['returns'],'ORIGINAL_SELECTED_OPERAND_VALUE_NOT_BOOLEAN_COERCION');self.assertEqual(q['false_values'],['NIL','BOOLEAN_FALSE']);self.assertTrue(q['zero_and_empty_string_truthy'])
 def test_not_does_not_apply_python_truthiness(self):
  ast=n('ULNotOp',operand=n('Number',n=0));q=self.make(ast).compose(ast,'');self.assertEqual(q['child']['value'],0);self.assertEqual(q['false_values'],['NIL','BOOLEAN_FALSE'])
 def test_compare_retains_types_and_metamethod_errors(self):
  ast=n('NotEqToOp',left=n('Number',n=1),right=n('String',raw='1',delimiter={'name':'DOUBLE_QUOTE'},s={'bytes_base64':'MQ=='}));q=self.make(ast).compose(ast,'');self.assertEqual(q['left']['lua_type'],'NUMBER');self.assertEqual(q['right']['lua_type'],'STRING');self.assertTrue(q['errors_and_side_effects_preserved'])
 def test_method_lookup_precedes_argument_evaluation(self):
  ast=n('Invoke',source=n('Name',id='item'),func=n('Name',id='getId'),args=[n('Number',n=1)]);q=self.make(ast,getter_engine=Getter()).compose(ast,'')
  self.assertEqual(q['chain_order'],['READ_RECEIVER_ONCE','LOOKUP_MEMBER','EVALUATE_EXPLICIT_ARGUMENTS','CALL_WITH_RECEIVER_SELF']);self.assertEqual(q['arguments'][0]['value'],1);self.assertFalse(reasons(q))
 def test_helper_arguments_are_centrally_recomposed(self):
  ast=n('Call',func=n('Name',id='type'),args=[n('Name',id='target')]);q=self.make(ast,helper_engine=Helper()).compose(ast,'')
  self.assertEqual(q['lowered']['arguments'][0]['kind'],'source_named_operand');self.assertEqual(q['callee_expression']['name'],'type')
 def test_field_operand_refs_are_replaced_recursively(self):
  ast=n('Index',value=n('Name',id='item'),idx=n('Name',id='itemid'));q=self.make(ast).compose(ast,'');self.assertEqual(q['receiver']['name'],'item');self.assertNotIn('operand_reference',json.dumps(q))
 def test_named_key_and_square_name_key_are_distinct(self):
  ast=n('Table',fields=[n('Field',key=n('Name',id='x'),value=n('Number',n=1),between_brackets=False),n('Field',key=n('Name',id='x'),value=n('Number',n=2),between_brackets=True)]);q=self.make(ast).compose(ast,'')
  self.assertEqual(q['entries'][0]['key']['lua_type'],'STRING');self.assertEqual(q['entries'][1]['key']['kind'],'source_named_operand')
 def test_mixed_constructor_implicit_indices_do_not_count_keyed_fields(self):
  ast=n('Table',fields=[n('Field',key=n('Name',id='x'),value=n('Number',n=1),between_brackets=False),n('Field',key=None,value=n('Number',n=2))]);q=self.make(ast).compose(ast,'');self.assertEqual(q['entries'][1]['key']['value'],1);self.assertTrue(q['nil_assignments_and_duplicate_keys_not_collapsed'])
 def test_long_bracket_initial_newline_not_assumed_from_parser_bytes(self):
  ast=n('String',raw='\nx',delimiter={'name':'LONG_BRACKET'},s={'bytes_base64':'Cng='});q=self.make(ast).compose(ast,'');self.assertEqual(q['kind'],'source_string_literal_token');self.assertNotIn('value',q)
 def test_large_integer_not_silently_converted(self):
  ast=n('Number',n=2**60);q=self.make(ast).compose(ast,'');self.assertEqual(q['kind'],'source_numeric_literal_token')
 def test_unknown_method_and_grammar_retained_unresolved(self):
  for ast in[n('Invoke',source=n('Name',id='item'),func=n('Name',id='unknown'),args=[]),n('AnonymousFunction',args=[],body=n('Block',body=[]))]:
   with self.subTest(ast=ast):self.assertTrue(reasons(self.make(ast).compose(ast,'')))
 def test_incomplete_nested_specs_never_count_complete(self):
  self.assertTrue(reasons({'kind':'source_getter_value','source_spec_complete':False}));self.assertTrue(reasons({'kind':'operand_reference','ast_pointer':'/fake'}))
 def test_subtract_keeps_clock_and_last_end_order(self):
  ast=n('SubOp',left=n('Name',id='clock'),right=n('Name',id='lastEnd'));q=self.make(ast).compose(ast,'');self.assertEqual(q['left']['name'],'clock');self.assertEqual(q['right']['name'],'lastEnd');self.assertTrue(q['no_constant_folding'])

class RecursiveSchemaTests(unittest.TestCase):
 def test_nested_native_promotion_and_operand_reference_rejected(self):
  from pathlib import Path
  from jsonschema import Draft202012Validator,ValidationError
  schema=json.loads(Path(__file__).with_name('schema.json').read_text())
  schema={**schema,'$ref':'#/$defs/expression'}
  schema.pop('required');schema.pop('additionalProperties');schema.pop('properties');schema.pop('type')
  validator=Draft202012Validator(schema)
  correct={'kind':'source_not','child':{'kind':'source_named_operand','name':'x','binding':{},'native_admission':False,'runtime_activation':False},'source_ref':{}}
  validator.validate(correct)
  mutant=json.loads(json.dumps(correct));mutant['child']['native_admission']=True
  with self.assertRaises(ValidationError):validator.validate(mutant)
  mutant=json.loads(json.dumps(correct));mutant['child']={'kind':'operand_reference','ast_pointer':'/missing'}
  with self.assertRaises(ValidationError):validator.validate(mutant)
 def test_closed_unknown_expression_property_rejected(self):
  from pathlib import Path
  from jsonschema import Draft202012Validator,ValidationError
  schema=json.loads(Path(__file__).with_name('schema.json').read_text());nested={'$schema':schema['$schema'],'$defs':schema['$defs'],'$ref':'#/$defs/expression'}
  with self.assertRaises(ValidationError):Draft202012Validator(nested).validate({'kind':'source_literal','lua_type':'NUMBER','value':1,'invented_property':True})
 def test_complete_expression_cannot_hide_unresolved_operation(self):
  from pathlib import Path
  from jsonschema import Draft202012Validator,ValidationError
  schema=json.loads(Path(__file__).with_name('schema.json').read_text());nested={'$schema':schema['$schema'],'$defs':schema['$defs'],'$ref':'#/$defs/complete_expression'}
  with self.assertRaises(ValidationError):Draft202012Validator(nested).validate({'kind':'source_not','child':{'kind':'source_operation_unresolved','reason':'unknown','source_ref':{}},'source_ref':{}})
 def test_child_expression_must_be_an_object(self):
  from pathlib import Path
  from jsonschema import Draft202012Validator,ValidationError
  schema=json.loads(Path(__file__).with_name('schema.json').read_text());nested={'$schema':schema['$schema'],'$defs':schema['$defs'],'$ref':'#/$defs/expression'}
  with self.assertRaises(ValidationError):Draft202012Validator(nested).validate({'kind':'source_not','child':'opaque','source_ref':{}})

if __name__=='__main__':unittest.main()
