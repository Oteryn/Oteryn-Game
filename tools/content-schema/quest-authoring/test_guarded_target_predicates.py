"""Item identity predicates require exact same-target type evidence."""
import tempfile
import unittest
from pathlib import Path
import ots_interactions as c

GUARD = 'if not target or type(target) ~= "userdata" or not target:isItem() then\nreturn false\nend\n'
READ = 'if target:getId() == 300 then\nplayer:addItem(301, 1)\nend\n'

class GuardedTargetTests(unittest.TestCase):
    def graph(self, body, prefix=''):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as directory:
            root = Path(directory)
            (root/'f.lua').write_text(prefix+'\nlocal a=Action()\nfunction a.onUse(player,item,fromPosition,target,toPosition)\n'+body+'\nend\na:id(300)\n')
            return c.Script('canary',root,'f.lua',{},'fixture').interactions()[0]

    def objects(self, graph):
        return [x for x in c.conditions(graph['rules']) if 'object' in x]

    def test_guarded_id_and_membership_keep_guard_and_children(self):
        graph=self.graph(GUARD+READ+'if table.contains({300,301}, target:getId()) then\nend')
        self.assertEqual(len(self.objects(graph)),3)
        self.assertTrue(all(x['object']['role']=='use_target' for x in self.objects(graph)))
        self.assertTrue(any('unresolved' in x for x in c.conditions(graph['rules'])))
        self.assertEqual(list(c.walk(graph['rules']))[0]['count'],1)

    def test_positive_branch_and_negative_else_do_not_leak(self):
        for guard in ('if target:isItem() then\n'+READ+'end\n',
                      'if not target:isItem() then\nelse\n'+READ+'end\n'):
            graph=self.graph(guard+READ)
            self.assertEqual(len(self.objects(graph)),1)
            self.assertTrue(any('unresolved' in x for x in c.conditions(graph['rules'])))

    def test_fallthrough_wrong_boolean_guard_and_wrong_receiver_opaque(self):
        for body in ('if not target:isItem() then\nplayer:addItem(301,1)\nend\n'+READ,
                     'if not target:isItem() and flag then\nreturn false\nend\n'+READ,
                     'if target:isItem() or flag then\n'+READ+'end\n',
                     'if not other:isItem() then\nreturn false\nend\n'+READ, READ):
            with self.subTest(body=body): self.assertEqual(self.objects(self.graph(body)),[])

    def test_mutation_shadow_escape_and_reflection_fail_closed(self):
        for tail in ('target=other', 'local target=other','mutate(target)',
                     'target.getId,other=helper,1','target.isItem=helper',
                     'Item.getId=helper','type=helper','debug.getmetatable(Item(300))',
                     'loadfile("mutator.lua")()', 'local alias=target',
                     'getmetatable(Item(300)).getId=helper'):
            with self.subTest(tail=tail): self.assertEqual(self.objects(self.graph(GUARD+READ+tail)),[])

    def test_transform_is_live_read_and_dynamic_id_still_unknown(self):
        graph=self.graph(GUARD+'target:transform(301)\n'+READ+'if target:getId()==runtimeId then\nend')
        self.assertEqual(len(self.objects(graph)),1)
        self.assertTrue(any('unresolved' in x for x in c.conditions(graph['rules'])))

    def test_quoted_guard_and_nested_helper_do_not_grant_type(self):
        for prefix in ('local text="not target:isItem()"',
                       'local function helper(target)\nif target:isItem() then\nend\nend'):
            self.assertEqual(self.objects(self.graph(READ,prefix)),[])

if __name__=='__main__': unittest.main()
