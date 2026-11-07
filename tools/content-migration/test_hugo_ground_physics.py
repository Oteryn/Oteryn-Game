"""The captured ground facts cannot widen identity, geometry, or projectile semantics."""
import copy,importlib.util,json,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('hugo_ground_physics',Path(__file__).with_name('hugo_ground_physics.py'));owner=importlib.util.module_from_spec(spec);spec.loader.exec_module(owner)
PACKET=ROOT/'tools/content-schema/world-authoring/samples/hugo-ladder-ground-physics.json'
class GroundQualification(unittest.TestCase):
 def setUp(self):self.packet=json.loads(PACKET.read_bytes())
 def test_exact_two_source_ground_facts_have_no_unknown_default(self):
  code=owner.render(ROOT,self.packet);self.assertIn('i103',code);self.assertIn('i351',code);self.assertIn('_ => None',code)
 def test_projectile_conflict_is_rejected(self):
  packet=copy.deepcopy(self.packet);packet['donors']['crystal']['policies'][0]['block_projectile']=True
  with self.assertRaises(ValueError):owner.render(ROOT,packet)
 def test_foreign_identity_or_landing_is_rejected(self):
  for field in ['identity','landing']:
   packet=copy.deepcopy(self.packet)
   if field=='identity':packet['donors']['canary']['policies'][0]['server_item_id']=352
   else:packet['landings'][0]['position'][0]+=1
   with self.assertRaises(ValueError):owner.render(ROOT,packet)
if __name__=='__main__':unittest.main()
