import unittest
try:
 from donor_sources.npc_dependencies import literal,static_expression
except ModuleNotFoundError:
 from resolve import literal,static_expression
class ResolverTests(unittest.TestCase):
 def test_literal_concat_profile(self):
  self.assertEqual(static_expression('DATA_DIRECTORY .. "/npc/alesar_functions.lua"',{'DATA_DIRECTORY':'data-global'}),'data-global/npc/alesar_functions.lua')
  self.assertEqual(static_expression('CORE_DIRECTORY .. "/npclib/load.lua"',{'CORE_DIRECTORY':'data'}),'data/npclib/load.lua')
 def test_runtime_names_not_guessed(self):
  for raw in ['filePath','namespace .. "." .. moduleName','DATA_DIRECTORY:lower() .. "/x"','name']:
   self.assertIsNone(static_expression(raw,{'name':'DISTRIBUTION_DEFAULT','DATA_DIRECTORY':'data-global'}))
 def test_no_executable_expression(self):
  for raw in ['os.execute("x")','"path" .. tostring(1)','"x";evil()']:
   self.assertIsNone(static_expression(raw,{}))
 def test_literal_path_not_split_dot_suffix(self):
  self.assertEqual(static_expression('"data/file..lua"',{}),'data/file..lua')
 def test_escaped_string_is_unresolved(self):
  self.assertIsNone(literal('"data\\47x.lua"'));self.assertIsNone(literal('"x\\\"y"'))
 def test_member_call_is_not_bare_loader(self):
  try:
   from donor_sources.npc_capture import annotations,lexical
  except ModuleNotFoundError:
   from capture import annotations,lexical
  text='local x = loader.load(DATA_DIRECTORY)';a=annotations(text);inc=a['includes'][0];mask,_,_=lexical(text);self.assertEqual(mask[:inc['start_char']].rstrip()[-1],'.')
 def test_two_quoted_terms_are_concatenation_not_one_literal(self):
  raw='"data/" .. "npclib/load.lua"'
  self.assertIsNone(literal(raw));self.assertEqual(static_expression(raw,{}),'data/npclib/load.lua')
  self.assertEqual(static_expression('"data/" .. "file..lua"',{}),'data/file..lua')
 def test_quoted_executable_suffix_never_literal_or_path(self):
  for raw in ['"data/x.lua"; evil(); "y"', '\'data/x.lua\'; evil(); \'y\'', '"x" .. "y"; evil()']:
   self.assertIsNone(literal(raw));self.assertIsNone(static_expression(raw,{}))
 def test_opposite_quote_inside_literal_is_valid_data(self):
  self.assertEqual(literal('"O\'Brien.lua"'),"O'Brien.lua")
  self.assertEqual(literal("'x\"y.lua'"),'x"y.lua')
  self.assertEqual(static_expression('"data/" .. "O\'Brien.lua"',{}),"data/O'Brien.lua")
 def test_concat_requires_term_and_operator_boundaries(self):
  for raw in ['"a" ..', '.. "a"','"a" ... "b"','"a" "b"','DATA_DIRECTORYfake', 'DATA_DIRECTORY .. name']:
   self.assertIsNone(static_expression(raw,{'DATA_DIRECTORY':'data'}))
if __name__=='__main__':unittest.main()
