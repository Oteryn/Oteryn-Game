import unittest,types,base64
import getter_values as g
from dependency_model import dependency_catalog
PROV={'source':'canary','revision':g.PINS['canary'],'path':'fake.lua','sha256':'x','git_blob_sha1':'y','byte_count':0}
def name(n):return {'node_type':'Name','fields':{'id':n},'span':None}
def invoke(n,receiver=None,args=None):return {'node_type':'Invoke','fields':{'func':name(n),'source':receiver or name('player'),'args':args or []},'span':None}
class Tests(unittest.TestCase):
 def engine(self):
  cat=types.SimpleNamespace(rows={('canary',g.PINS['canary'],'fake.lua'):PROV},apis={('canary','getName'):[{'semantics':{'return':'string'}}]},lua_methods={},external=dependency_catalog(),constructors={'canary':{'semantics':{'operation':'MAP_TILE_LOOKUP'}}},marshal={})
  return g.Engine({'provenance':PROV},catalog=cat)
 def test_escaped_quote_and_braces(self):
  s='int A::f(lua_State* L) { const char *x="\\\"} notbody"; return 1; } int tail;'
  body=g.cpp_body(s,'A::f');self.assertEqual(body[2],s[:s.index(' int tail;')])
 def test_raw_string(self):
  s='int X::f() { auto s=R"foo( } ")foo"; return 1; }'
  self.assertEqual(g.cpp_body(s,'X::f')[2],s)
 def test_comment_braces(self):
  s='int X::f() { /* } */ return 1; }';self.assertEqual(g.cpp_body(s,'X::f')[2],s)
 def test_order(self):
  x=self.engine().project(invoke('getName'),'/x');self.assertIn('METHOD_LOOKUP_THEN_ARGUMENTS',x['chain_order']);self.assertFalse(x['native_admission'])
 def test_lower_chain(self):
  x=self.engine().project(invoke('lower',invoke('getName')),'/x');self.assertTrue(x['source_spec_complete']);self.assertEqual(x['receiver']['method'],'getName');self.assertEqual(x['api_definitions'][0]['semantics']['operation'],'BYTE_ASCII_CASE_MAP')
 def test_long_string_fenced(self):
  n={'node_type':'String','fields':{'raw':'[[\nA]]','delimiter':{'name':'LONG_BRACKET'},'s':{'bytes_base64':base64.b64encode(b'\nA').decode()}}};self.assertFalse(self.engine().operand(n,'/s')['source_spec_complete'])
 def test_escaped_string_fenced(self):
  n={'node_type':'String','fields':{'raw':'"A\\n"','delimiter':{'name':'DOUBLE_QUOTE'}}};self.assertFalse(self.engine().operand(n,'/s')['source_spec_complete'])
 def test_other_method_not_claimed(self):self.assertIsNone(self.engine().project(invoke('unknown'),'/x'))
 def test_wrong_pin(self):
  e=self.engine()
  with self.assertRaises(ValueError):g.Engine({'provenance':PROV|{'revision':'bad'}},catalog=e.catalog)
 def test_wrong_provenance(self):
  e=self.engine()
  with self.assertRaises(ValueError):g.Engine({'provenance':PROV|{'sha256':'bad'}},catalog=e.catalog)
 def test_wrong_ast_pointer(self):
  e=self.engine();e.full_ast={'x':invoke('getId')}
  with self.assertRaises(ValueError):e.project(invoke('getName'),'/x')
 def test_tile(self):
  x=self.engine().project({'node_type':'Call','fields':{'func':name('Tile'),'args':[name('position')]}},'/x');self.assertTrue(x['source_spec_complete']);self.assertEqual(x['dispatch'],'UNPROVEN')
 def test_item_subtype(self):
  self.assertEqual(g.cpp_semantics('getItemById','', 'Player')['algorithm']['subtype_argument_stack_index'],4);self.assertEqual(g.cpp_semantics('getItemById','', 'Tile')['algorithm']['subtype_argument_stack_index'],3)
 def test_dynamic_storage(self):
  x=g.cpp_semantics('getStorageValue','lua_pushnil(L);');self.assertEqual(x['algorithm']['key_conversion'],'UINT32');self.assertEqual(x['algorithm']['key_domain'],'UNKNOWN_DYNAMIC_OR_SOURCE_EXPRESSION')
 def test_recursive_slots(self):self.assertFalse(g.cpp_semantics('getEmptySlots','')['algorithm']['default'])
 def test_freecap_no_conversion(self):self.assertEqual(g.cpp_semantics('getFreeCapacity','')['algorithm']['units'],'SOURCE_ENGINE_VALUE_NO_CONVERSION_IN_THIS_WRAPPER')
 def test_no_receiver_class_from_name(self):self.assertEqual(self.engine().operand(name('player'),'/x')['binding']['runtime_binding'],'UNPROVEN')
if __name__=='__main__':unittest.main()
