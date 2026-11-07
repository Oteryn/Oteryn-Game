"""Source static projection must never replace previous Item authority or known collision."""
import copy,json,pathlib,tempfile,unittest
import spike_corridor_item_profiles as subject
ROOT=subject.ROOT
class Qualification(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.root=pathlib.Path(self.tmp.name)
  self.packet=json.loads((ROOT/subject.PROFILE).read_bytes())
  p=self.root/subject.PROFILE;p.parent.mkdir(parents=True);p.write_text(json.dumps(self.packet))
 def tearDown(self):self.tmp.cleanup()
 def admit(self,rows):
  generated={'profile':json.dumps({'schema':'OTERYN_NATIVE_ITEM_PROFILES/v1','records':rows}).encode(),'manifest':b'{"item_profiles":{}}'}
  subject.admit(self.root,generated,'profile','manifest',lambda r:r['production_definition']['production_key'],lambda v:json.dumps(v).encode())
  return json.loads(generated['profile'])['records']
 def test_all33_preserve_exact_source_flags_and_no_mint(self):
  result=self.admit([]);self.assertEqual(len(result),33)
  source={r['production_definition']['production_key']:r for r in self.packet['records']}
  for r in result:
   self.assertEqual(r['attributes'],source[r['production_definition']['production_key']]['attributes']);self.assertFalse(r['admission']['materializable']);self.assertEqual(r['admission']['legal_destinations'],['Ground'])
  self.assertEqual(self.admit(result),result)
 def test_existing_materializable_admission_is_never_narrowed(self):
  existing=copy.deepcopy(self.packet['records'][0]);self.assertTrue(existing['admission']['materializable'])
  with self.assertRaisesRegex(ValueError,'SPIKE_EXISTING_ITEM_AUTHORITY_MUST_STAY'):self.admit([existing])
 def test_existing_inventory_admission_is_never_removed(self):
  existing=copy.deepcopy(self.packet['records'][0]);existing['admission']['materializable']=False;existing['admission']['legal_destinations']=['CharacterInventory','Ground']
  with self.assertRaisesRegex(ValueError,'SPIKE_EXISTING_ITEM_AUTHORITY_MUST_STAY'):self.admit([existing])
 def test_known_real_wall_cannot_be_overwritten(self):
  existing=next(r for r in self.admit([]) if r['production_binding']['external_id']=='18368');existing['attributes']['blocks_movement']=False
  with self.assertRaisesRegex(ValueError,'SPIKE_EXISTING_KNOWN_COLLISION_CONFLICT'):self.admit([existing])
 def test_only_unknown_flags_of_same_nonmaterializable_source_can_fill(self):
  existing=copy.deepcopy(self.admit([])[0]);expected=copy.deepcopy(existing)
  for field in subject.COLLISION:existing['attributes'][field]=None
  result=self.admit([existing]);self.assertEqual(next(r for r in result if r['production_definition']==expected['production_definition']),expected)
if __name__=='__main__':unittest.main()
