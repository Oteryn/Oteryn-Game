import unittest,hashlib,json
try:from . import quest_dialogue_links_all as q
except ImportError:import quest_dialogue_links_all as q

def N(t,**fields):return {'node_type':t,'fields':fields,'span':None}
def name(s):return N('Name',id=s)
def path(s):
 x=name(s.split('.')[0])
 for member in s.split('.')[1:]:x=N('Index',value=x,idx=name(member),notation={'name':'DOT'})
 return x
class DS:
 @staticmethod
 def name(n):return q.qualified_symbol(n,{})
 @staticmethod
 def walk(x,p='$'):
  if isinstance(x,dict):
   if 'node_type' in x:yield p,x,()
   for k,v in x.items():yield from DS.walk(v,p+'.'+k)
  elif isinstance(x,list):
   for i,v in enumerate(x):yield from DS.walk(v,p+'['+str(i)+']')
 @staticmethod
 def witness(row,p,n):return {'ast_path':p,'span':n.get('span'),'node_sha256':q.sha(q.stable(n))}
def declaration(symbol='S',value=None):
 d=N('LocalAssign',targets=[name(symbol)],values=[value or path('Storage.Quest.A')]);d['span']={'start_char':10,'end_char_exclusive':20};return d

def fn(body,args=()):return N('LocalFunction',name=name('callback'),args=[name(s) for s in args],body=N('Block',body=body))
def tree(functions):return N('Chunk',body=N('Block',body=functions))
class ScopedControls(unittest.TestCase):
 def test_literal_direct_callback_alias(self):
  ps=q.function_storage_aliases(tree([fn([declaration()])]),{},DS);self.assertEqual(len(ps),1);self.assertEqual(ps[0]['exact_storage_expression'],'Storage.Quest.A')
 def test_conditional_competing_assignment_rejected(self):
  branch=N('If',test=name('condition'),body=N('Block',body=[N('Assign',targets=[name('S')],values=[path('Storage.Quest.B')])]),orelse=None)
  self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),branch])]),{},DS),[])
 def test_parameter_shadow_rejected(self):self.assertEqual(q.function_storage_aliases(tree([fn([declaration()],args=['S'])]),{},DS),[])
 def test_global_storage_shadow_rejected(self):self.assertEqual(q.function_storage_aliases(tree([fn([declaration()],args=['Storage'])]),{},DS),[])
 def test_redefinition_rejected(self):
  self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),N('Assign',targets=[name('S')],values=[name('other')])])]),{},DS),[])
 def test_nested_callback_shadow_rejected_conservatively(self):
  self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),fn([],args=['S'])])]),{},DS),[])
 def test_branch_local_not_lifted_into_callback(self):
  branch=N('If',test=name('condition'),body=N('Block',body=[declaration()]),orelse=None);self.assertEqual(q.function_storage_aliases(tree([fn([branch])]),{},DS),[])
 def test_dynamic_rhs_not_fabricated(self):
  d=declaration(value=N('Call',func=name('lookup'),args=[]));self.assertEqual(q.function_storage_aliases(tree([fn([d])]),{},DS),[])
 def test_declaration_before_use_required(self):
  ps=q.function_storage_aliases(tree([fn([declaration()])]),{},DS);fpath=ps[0]['function_witness']['ast_path'];e={'witness':{'ast_path':fpath+'.fields.body.fields.body[0]','span':{'start_char':15}}};self.assertEqual(q.event_storage_aliases(e,{},ps),({},[]))
 def test_sibling_callback_not_bound(self):
  ps=q.function_storage_aliases(tree([fn([declaration()]),fn([])]),{},DS);e={'witness':{'ast_path':'$.fields.body.fields.body[1].fields.body.fields.body[0]','span':{'start_char':50}}};self.assertEqual(q.event_storage_aliases(e,{},ps),({},[]))
 def test_bound_source_value_is_not_arbitrary(self):
  self.assertEqual(q.literal_number(N('Number',n=3)),3);self.assertIsNone(q.literal_number(N('Call',func=name('getStorage'),args=[])))
 def test_generic_for_target_shadow_rejected(self):
  loop=N('Forin',targets=[name('S')],iter=[name('iterator')],body=N('Block',body=[]));self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),loop])]),{},DS),[])
 def test_numeric_for_target_shadow_rejected(self):
  loop=N('Fornum',target=name('S'),start=N('Number',n=1),stop=N('Number',n=3),body=N('Block',body=[]));self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),loop])]),{},DS),[])
 def test_file_alias_generic_for_shadow_rejected(self):
  loop=N('Forin',targets=[name('S')],iter=[name('iterator')],body=N('Block',body=[]));self.assertEqual(q.storage_aliases(tree([declaration(),fn([loop])]),{},DS)[0],{})
 def test_file_alias_numeric_for_shadow_rejected(self):
  loop=N('Fornum',target=name('S'),start=N('Number',n=1),stop=N('Number',n=3),body=N('Block',body=[]));self.assertEqual(q.storage_aliases(tree([declaration(),fn([loop])]),{},DS)[0],{})
 def test_nested_local_declaration_shadow_rejected(self):
  nested=N('If',test=name('condition'),body=N('Block',body=[declaration()]),orelse=None);self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),nested])]),{},DS),[])
 def test_nested_method_parameter_shadow_rejected(self):
  method=N('Method',name=name('other'),args=[name('S')],body=N('Block',body=[]));self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),method])]),{},DS),[])
 def test_file_storage_root_shadow_rejects_manufactured_global_key(self):
  self.assertEqual(q.storage_aliases(tree([declaration(symbol='Storage',value=name('custom')),declaration()]),{},DS),({},[]))
 def test_function_alias_storage_property_mutation_rejected(self):
  mutation=N('Assign',targets=[path('Storage.Quest.A')],values=[name('other')]);self.assertEqual(q.function_storage_aliases(tree([fn([declaration(),mutation])]),{},DS),[])
 def test_file_alias_storage_property_mutation_rejected(self):
  mutation=N('Assign',targets=[path('Storage.Quest.A')],values=[name('other')]);self.assertEqual(q.storage_aliases(tree([declaration(),mutation]),{},DS),({},[]))
if __name__=='__main__':unittest.main()
