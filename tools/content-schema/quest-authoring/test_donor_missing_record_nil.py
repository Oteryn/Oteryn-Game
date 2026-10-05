"""Bounded missing-field scalar reads preserve guarded reward Source semantics."""
import unittest
import copy
import test_donor_semantic_rewards as originals

class MissingRecordNilTests(unittest.TestCase):
 def convert(self,extra='',prefix=None):
  return originals.RewardRecordTests().convert(extra,prefix=prefix)
 def test_universally_absent_scalar_read_does_not_change_outer_guard(self):
  old,(new,changes)=self.convert('if player:getStorageValue(100) == 1 then\nplayer:sendTextMessage(1, getItemName(reward.absent))\nreturn true\nend')
  self.assertEqual(len(changes),1)
  target=new;parts=changes[0]['baseline_child_pointer'].split('/')[1:]
  for part in parts[:-1]:target=target[int(part)]if isinstance(target,list)else target[part]
  target[int(parts[-1])]=copy.deepcopy(target[int(parts[-1])]['otherwise'][0])
  self.assertEqual(new,old)
 def test_universally_absent_field_write_or_compound_write_reject(self):
  for extra in ['reward.absent=3389','reward.absent, other=3389,1','reward["absent"]=3389']:
   with self.subTest(extra=extra):self.assertEqual(self.convert(extra)[1][1],[])
 def test_metatable_rebinding_escape_and_nested_missing_read_reject(self):
  for extra in ['setmetatable(reward,{__index=function() return 3389 end})','rawset(reward,"absent",3389)','evil(reward)','local second=reward','reward=other','rewards[1002]={itemid=999,count=1}','evil(rewards)','getItemName(reward.absent.nested)']:
   with self.subTest(extra=extra):self.assertEqual(self.convert(extra)[1][1],[])
 def test_unknown_record_expression_remains_rejected(self):
  self.assertEqual(self.convert('getItemName(reward.absent)',prefix='local rewards={[1002]={itemid=pick(),count=1}}')[1][1],[])

if __name__=='__main__':unittest.main()
