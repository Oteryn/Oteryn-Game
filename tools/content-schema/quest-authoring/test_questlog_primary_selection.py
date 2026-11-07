"""Configured primary selection and exact conditional fallback safety."""
import copy,hashlib,json,tempfile,unittest
from unittest.mock import patch
from pathlib import Path
import ots_questlog as q
FIXTURE=Path(__file__).parent/'samples/primary-selection/primary-selection.json'
class QuestlogPrimarySelectionTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.fixture=json.loads(FIXTURE.read_text());cls.temp=tempfile.TemporaryDirectory()
  cls.base=Path(cls.temp.name);cls.body=cls.fixture['fallback_body']
  guard=patch.object(q,'ARENA_FALLBACK_SHA256',hashlib.sha256(cls.body.encode()).hexdigest());guard.start();cls.addClassCleanup(guard.stop)
  dest=cls.base/'data-crystal/lib/core/quests.lua';dest.parent.mkdir(parents=True);dest.write_text(cls.body)
  cls.primary=cls.fixture['primary'];cls.fallback=cls.fixture['fallback']
  cls.selected=q.select_questlog_primary('crystalserver',cls.base,cls.fixture['other_primary_entries']+[cls.primary,cls.fallback])
  cls.arena=next(r for r in cls.selected if r['name']=='The Ultimate Challenges')
 @classmethod
 def tearDownClass(cls):cls.temp.cleanup()
 def test_exact_actual_primary_and_fallback_loop_retained(self):
  self.assertEqual(len(self.selected),58);self.assertEqual(self.arena['path'],'data-global/lib/core/quests.lua');self.assertEqual(len(self.arena['missions']),3)
  f=self.arena['conditional_fallbacks'][0];self.assertEqual(f['condition']['expression'],'not Quests');self.assertEqual(f['dynamic_missions']['assignment_line'],18)
  self.assertEqual([m['missionId']for m in f['dynamic_missions']['declared_modes']],[10312,10313,10314]);self.assertEqual(f['missions'],[])
 def test_configured_selection_is_not_input_order(self):
  for rows in ([self.primary,self.fallback],[self.fallback,self.primary]):self.assertEqual(q.select_questlog_primary('crystalserver',self.base,rows),[self.arena])
 def test_other57_selected_names_unchanged(self):
  self.assertEqual(self.fixture['other_primary_entries'],[r for r in self.selected if r['name']!='The Ultimate Challenges'])
 def test_unconditional_duplicate_rejected(self):
  with tempfile.TemporaryDirectory()as folder:
   dest=Path(folder)/'data-crystal/lib/core/quests.lua';dest.parent.mkdir(parents=True);dest.write_text(self.body.replace('if not Quests then','if true then'))
   with self.assertRaisesRegex(ValueError,'UNKNOWN'):q.select_questlog_primary('crystalserver',Path(folder),[self.primary,self.fallback])
 def test_stale_dynamic_initializer_rejected(self):
  with tempfile.TemporaryDirectory()as folder:
   dest=Path(folder)/'data-crystal/lib/core/quests.lua';dest.parent.mkdir(parents=True);dest.write_text(self.body.replace('endValue = 2','endValue = 3'))
   with self.assertRaisesRegex(ValueError,'UNKNOWN'):q.select_questlog_primary('crystalserver',Path(folder),[self.primary,self.fallback])
 def test_missing_primary_is_unknown(self):
  with self.assertRaisesRegex(ValueError,'configured primary missing'):q.select_questlog_primary('crystalserver',self.base,[self.fallback])
 def test_two_primary_entries_not_first_wins(self):
  with self.assertRaisesRegex(ValueError,'duplicate'):q.select_questlog_primary('crystalserver',self.base,[self.primary,copy.deepcopy(self.primary)])
 def test_unknown_alternative_and_forged_parsed_shape_rejected(self):
  for field,value in [('path','data/lib/quests.lua'),('missions',[{'name':'invented'}]),('line',4)]:
   altered=copy.deepcopy(self.fallback);altered[field]=value
   with self.subTest(field=field),self.assertRaisesRegex(ValueError,'UNKNOWN'):q.select_questlog_primary('crystalserver',self.base,[self.primary,altered])
if __name__=='__main__':unittest.main()
