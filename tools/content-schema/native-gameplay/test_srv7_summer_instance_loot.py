import copy,json,pathlib,sys,tempfile,unittest
ROOT=pathlib.Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tools/content-schema/native-gameplay'))
import build_full_spell_artifact as build
CAPTURE=ROOT/'tools/content-schema/native-gameplay/summer-instance-item-profiles.json'
BODIES={43758,54346,54352,54433,54670}
class SummerFacts(unittest.TestCase):
 def facts(self, mutate=None):
  captured=json.loads(CAPTURE.read_bytes())['records']
  wanted={int(r['production_definition']['production_key'].rsplit('i',1)[1]) for r in captured}
  records=[]
  for path in (ROOT/'content/items/definitions').glob('items-*.json'):
   document=json.loads(path.read_bytes())
   records.extend(r for r in document['records'] if r['definition']['identity']['key']in{f'oteryn:item.tibia.i{i}'for i in wanted})
  native=json.loads((ROOT/'content/items/definitions/spell-native-profiles.json').read_bytes())
  bykey={r['production_definition']['production_key']:r for r in native['records']}
  bykey.update({r['production_definition']['production_key']:r for r in captured})
  if mutate: mutate(bykey)
  native['records']=list(bykey.values())
  with tempfile.TemporaryDirectory(prefix='srv7-summer-facts-')as d:
   d=pathlib.Path(d);(d/'items-00000-00000.json').write_text(json.dumps({'records':records}));(d/'spell-native-profiles.json').write_text(json.dumps(native))
   return build.item_facts(d)
 def test_actual_builder_admits_five_exact_source_bodies(self):
  facts=self.facts()
  for i in sorted(BODIES):self.assertEqual((facts[f'oteryn:item.tibia.i{i}']['materializable'],facts[f'oteryn:item.tibia.i{i}']['container_capacity']),(True,30))
 def test_actual_builder_admits_source_rakesh_werepanther_trophy(self):
  self.assertTrue(self.facts()['oteryn:item.tibia.i43917']['materializable'])
 def test_all_fifteen_xml_loot_operands_are_admitted_by_actual_builder(self):
  facts=self.facts()
  for row in json.loads(CAPTURE.read_bytes())['records']:
   key=row['production_definition']['production_key']
   self.assertTrue(facts[key]['materializable'],key)
 def test_ground_only_inventory_operand_is_not_promoted(self):
  def change(rows): rows['oteryn:item.tibia.i43917']['admission']['legal_destinations']=['Ground']
  self.assertFalse(self.facts(change)['oteryn:item.tibia.i43917']['materializable'])
 def test_wrong_source_binding_is_refused(self):
  def change(rows): rows['oteryn:item.tibia.i43917']['production_binding']['disposition']='UNBOUND'
  with self.assertRaisesRegex(ValueError,'exact source-qualified binding'):self.facts(change)
 def test_body_capacity_is_not_clamped_or_guessed(self):
  def change(rows):rows['oteryn:item.tibia.i43758']['semantics']['container']['value']['capacity']['value']=31
  self.assertFalse(self.facts(change)['oteryn:item.tibia.i43758']['materializable'])
if __name__=='__main__':unittest.main()
