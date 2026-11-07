import tempfile,unittest
from pathlib import Path
import ots_interactions as oi
class UIDIntervals(unittest.TestCase):
 def script(self,body,receiver='item'):
  tmp=tempfile.TemporaryDirectory();self.addCleanup(tmp.cleanup);p=Path(tmp.name)
  text='local a = Action()\nfunction a.onUse(player, '+receiver+', fromPosition, target, toPosition)\n'+body+'\nend\na:uid(3148)\n';(p/'f.lua').write_text(text)
  return oi.Script('canary',p,'f.lua',{},'fixture').interactions()[0]
 def predicate(self,body):return self.script(body)['rules'][-1]['branch'][0]['when']
 def test_exact_open_and_closed_sets(self):
  for expr,values in [('item.uid > 3147 and item.uid < 3151',[3148,3149,3150]),('item.uid >= 3148 and item.uid <= 3150',[3148,3149,3150]),('item.uid > 2049 and item.uid < 2065',list(range(2050,2065)))]:
   pred=self.predicate('if '+expr+' then\nplayer:addItem(100, 1)\nend');self.assertEqual([v['object']['value'] for v in pred['any']],values)
 def test_singleton(self):self.assertEqual(self.predicate('if item.uid >= 3 and item.uid <= 3 then\nreturn true\nend')['object']['value'],3)
 def test_rejects_other_receiver_wide_empty_or_negative(self):
  for expr in ['item.uid > 1 and target.uid < 4','target.uid > 1 and target.uid < 4','item.uid > 1 and item.uid < 1000','item.uid > 5 and item.uid < 5','item.uid > -2 and item.uid < 2','item.uid >= 0 and item.uid <= 65536']:
   with self.subTest(expr=expr):self.assertIn('unresolved',self.predicate('if '+expr+' then\nreturn true\nend'))
 def test_rejects_mutation_shadow_escape_reflection(self):
  for before in ['local item = target','item = target','item.uid = 3148','local copy = item','inspect(item)','_G.item = target']:
   with self.subTest(before=before):self.assertIn('unresolved',self.predicate(before+'\nif item.uid > 3147 and item.uid < 3151 then\nreturn true\nend'))
 def test_not_and_outer_boolean_guard_preserved(self):
  pred=self.predicate('if not (item.uid > 3147 and item.uid < 3151) then\nreturn true\nend');self.assertTrue(all(x['negate'] for x in pred['all']))
 def test_existing_effect_exact(self):
  rules=self.script('if item.uid > 3147 and item.uid < 3151 then\nplayer:addItem(100, 2)\nelse\nplayer:addItem(101, 3)\nend')['rules'][0]
  self.assertEqual(rules['branch'][0]['then'][0]['count'],2);self.assertEqual(rules['otherwise'][0]['count'],3)
if __name__=='__main__':unittest.main()
