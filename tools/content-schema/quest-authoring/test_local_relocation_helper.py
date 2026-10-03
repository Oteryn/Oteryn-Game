"""Pure local relocation helpers preserve branches and fail closed."""
import tempfile,unittest
from pathlib import Path
import ots_interactions as c
class RelocationHelperTests(unittest.TestCase):
 def run_source(self, prefix='', body='jump(player, Position(100,200,7))', tail='', helper=None, callback='onUse'):
  helper=helper or 'local function jump(p, dest)\np:teleportTo(dest)\np:getPosition():sendMagicEffect(CONST_ME_POFF)\np:sendTextMessage(19, "Synthetic message.")\nend'
  with tempfile.TemporaryDirectory()as folder:
   repo=Path(folder);(repo/'fixture.lua').write_text(prefix+helper+'\nlocal a=Action()\nfunction a.'+callback+'(player,item,fromPosition,target,toPosition)\n'+body+'\nend\n'+tail)
   return c.Script('canary',repo,'fixture.lua',{},'fixture').interactions()[0]
 def test_literal_helper_retains_effects_and_exact_anchor(self):
  d=self.run_source();self.assertEqual([x['owner']for x in d['rules']],['Movement','Presentation','Presentation']);self.assertEqual(d['anchors'][0]['source_position'],{'x':100,'y':200,'z':7});self.assertEqual(d['unresolved'],[])
 def test_previous_position_and_caller_branch_preserved(self):
  d=self.run_source(body='if item.uid == 100 then\njump(player, fromPosition)\nend');b=d['rules'][0]['branch'][0];self.assertEqual(b['when']['object']['value'],100);self.assertEqual(b['then'][0]['target'],{'kind':'previous_position'})
 def test_unknown_effects_control_flow_and_arguments_fail_closed(self):
  for h,b in [('local function jump(p,d)\np:teleportTo(d)\np:addItem(300,1)\nend','jump(player, Position(1,2,7))'),('local function jump(p,d)\nif p then\np:teleportTo(d)\nend\nend','jump(player, Position(1,2,7))'),(None,'jump(player, unknownPosition)'),(None,'jump(other, Position(1,2,7))')]:
   with self.subTest(helper=h,body=b):self.assertTrue(self.run_source(helper=h,body=b)['unresolved'])
 def test_shadow_mutation_escape_and_reflection_fail_closed(self):
  for suffix in ['jump = other','local alias = jump','take(jump)','local function later(jump)\nend','_G.jump = other','local a,b=jump,nil']:
   with self.subTest(suffix=suffix):self.assertTrue(self.run_source(tail=suffix)['unresolved'])
 def test_actor_reassignment_and_wrong_callback_fail_closed(self):
  self.assertTrue(self.run_source(body='player = other\njump(player, Position(1,2,7))')['unresolved']);self.assertTrue(self.run_source(callback='onStepIn')['unresolved'])
 def test_conditional_or_loop_local_helper_is_not_global(self):
  self.assertTrue(self.run_source(prefix='if unknown then\n',tail='end')['unresolved'])
 def test_argument_count_and_previous_position_mutation_fail_closed(self):
  for body in ['jump(player)', 'jump(player, fromPosition, 1)', 'fromPosition = unknown\njump(player, fromPosition)']:
   with self.subTest(body=body):self.assertTrue(self.run_source(body=body)['unresolved'])
 def test_proven_get_player_alias_identity_guard_remains_supported(self):
  d=self.run_source(callback='onStepIn',body='local p=player:getPlayer()\nif not p then\nreturn true\nend\njump(p, fromPosition)')
  self.assertFalse(d['unresolved']);self.assertTrue(any(x.get('owner')=='Movement' for x in c.walk(d['rules'])))
 def test_position_and_actor_alias_or_argument_escape_fail_closed(self):
  for setup in ['local alias=fromPosition\n alias.x=999', 'mutate(fromPosition)', 'local alias=player\nalias.teleportTo=evil', 'mutate(player)']:
   with self.subTest(setup=setup):self.assertTrue(self.run_source(body=setup+'\njump(player, fromPosition)')['unresolved'])
 def test_message_argument_is_literal_not_computed(self):
  h='local function jump(p,d,msg)\np:teleportTo(d)\np:sendTextMessage(19, msg)\nend'
  self.assertEqual(self.run_source(helper=h,body='jump(player, fromPosition, "Synthetic.")')['unresolved'],[])
  self.assertTrue(self.run_source(helper=h,body='jump(player, fromPosition, effectful())')['unresolved'])
if __name__=='__main__':unittest.main()
