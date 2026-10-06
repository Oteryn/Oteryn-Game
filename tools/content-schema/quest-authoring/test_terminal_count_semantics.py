import importlib.util,pathlib,unittest
ROOT=pathlib.Path(__file__).resolve().parents[3]
BUILDER=ROOT/"tools/content-schema/quest-authoring/samples/server-completion/terminal-count-semantics/builder.py"
def mod():
 s=importlib.util.spec_from_file_location("terminal_count_semantics",BUILDER);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);return m
class TestTerminalCountSemantics(unittest.TestCase):
 @classmethod
 def setUpClass(cls): cls.audit=mod().build(ROOT);cls.by={r["quest"]["key"]:r for r in cls.audit["records"]}
 def test_closed_set(self):
  self.assertEqual(9,self.audit["summary"]["held_quests"]);self.assertEqual({"EXACT_CLAIM_SET_CARDINALITY_CANDIDATE":4,"CLAIM_SET_CARDINALITY_MISMATCH":2,"NO_EXACT_REWARD_CLAIM_SET":3},self.audit["summary"]["classifications"])
 def test_exact_candidates(self):
  expected={"oteryn:quest.behemoth_quest":4,"oteryn:quest.demon_helmet_quest":3,"oteryn:quest.dragon_tower_quest":2,"oteryn:quest.edron_goblin_quest":2}
  actual={k:r["terminal_stage"]["chosen_count"] for k,r in self.by.items() if r["classification"]=="EXACT_CLAIM_SET_CARDINALITY_CANDIDATE"}
  self.assertEqual(expected,actual)
  for k,n in expected.items(): self.assertEqual(n,self.by[k]["claim_set"]["count"]);self.assertTrue(self.by[k]["claim_set"]["all_source_claims_found"])
 def test_mismatches(self):
  self.assertEqual("CLAIM_SET_CARDINALITY_MISMATCH",self.by["oteryn:quest.bear_room_quest"]["classification"]);self.assertEqual("CLAIM_SET_CARDINALITY_MISMATCH",self.by["oteryn:quest.barbarian_arena_quest"]["classification"])
  for k in ("oteryn:quest.opticording_sphere_quest","oteryn:quest.rift_warrior_outfits_quest","oteryn:quest.the_ancient_tombs_quest"): self.assertEqual("NO_EXACT_REWARD_CLAIM_SET",self.by[k]["classification"]);self.assertEqual(0,self.by[k]["claim_set"]["count"])
 def test_no_promotion(self):
  self.assertEqual(0,self.audit["summary"]["runtime_admitted"])
  for r in self.audit["records"]: self.assertFalse(r["runtime_admitted"]);self.assertIsNone(r["native_dispatch_binding"]);self.assertFalse(r["source_equivalence"])
if __name__=="__main__":unittest.main()
