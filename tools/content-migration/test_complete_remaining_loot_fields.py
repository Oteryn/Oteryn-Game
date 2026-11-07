"""Meaningful identity, applicability and non-mutating batch tests."""
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0,str(Path(__file__).parent))
import complete_remaining_loot_fields as lane

class RemainingLootTests(unittest.TestCase):
    def test_unequipped_variant_requires_reciprocal_relationship(self):
        attrs={1:{'transformequipto':'2'},2:{'transformdeequipto':'1','duration':'1800'}}
        self.assertEqual(lane.stable_variant({1,2},attrs),1)
        self.assertIsNone(lane.stable_variant({1,2},{1:{'transformequipto':'2'},2:{}}))
    def test_visual_variants_are_not_arbitrarily_selected(self):
        self.assertIsNone(lane.stable_variant({281,282},{281:{},282:{}}))
    def test_transient_doll_selects_stable_decay_destination(self):
        self.assertEqual(lane.stable_variant({5791,6566},{5791:{},6566:{'duration':'3','decayto':'5791'}}),5791)
    def test_bare_loot_keeps_quantity_and_marks_conditions(self):
        rows=list(lane.parsed_observations({'loot':'0-35 [[Gold Coin]]s, [[Crown]] (apenas na primeira vez).'}))
        self.assertEqual(rows[0],('gold coin','loot',(1,35),False,True))
        self.assertTrue(rows[1][3])
    def test_numeric_uses_latest_compatible_version_and_rejects_low_n(self):
        page=dict(page_title='Loot Statistics:Example',page_id=1,revision_id=2,url='https://tibia.fandom.com/wiki/Loot_Statistics:Example',content_sha256='abc',
                  content='{{Loot2\n|version=8.54\n|kills=100\n|Thing, times:40, amount:1-3\n}}\n{{Loot2\n|version=8.6\n|kills=100\n|Thing, times:20, amount:1-5\n}}')
        result,status=lane.numeric_observation(page,'Thing','8.54')
        self.assertEqual(result['ppm'],200000)
        self.assertEqual(result['quantity'],(1,5))
        self.assertIsNone(lane.numeric_observation(page,'Thing','9.1')[0])
        page['content']='{{Loot2\n|version=15.0\n|kills=100\n|Thing, times:9, amount:1\n}}'
        self.assertIsNone(lane.numeric_observation(page,'Thing','8.6')[0])
    def test_wrong_baseline_rejected_before_any_output(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d);(root/'population-index.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'baseline differs'):
                lane.build(root,root.parent/'unexpected-loot-output')
    def test_output_cannot_modify_source(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(ValueError,'output may not modify'):
                lane.build(Path(d),Path(d)/'output')
    def test_actual_batch_all_ids_admitted_and_no_duplicate_drops(self):
        packet=json.loads(Path('/workspace/monster-field-next-20261002/loot-final-v5/field-patches.json').read_text())
        registry=json.loads(Path('/workspace/monster-round7-output/native-item-map-rust.json').read_text())
        admitted={r['source_item_id'] for r in registry['records']}
        seen={}
        for p in packet['patches']:
            if p['pointer']!='/loot/entries/-':continue
            n=p['monster']
            if n not in seen:
                m=json.loads((Path('/workspace/monster-field-fill-20261002/population/bundles')/n/'monster.json').read_text())
                seen[n]={e['item']['key'] for e in m.get('loot',{}).get('entries',[])}
            e=p['value'];key=e['item']['key']
            self.assertIn(int(key.split('/')[-1]),admitted);self.assertNotIn(key,seen[n]);seen[n].add(key)
            self.assertGreater(e['min_count'],0);self.assertLessEqual(e['min_count'],e['max_count'])
            self.assertFalse(p['source']['global_parity']);self.assertIn('rate_observation_audit',p['source']);self.assertIn('donor_rate_audit',p['source']);self.assertTrue(p['source']['item_presence_source']['sha'])
        for p in packet['patches']:
            if p['pointer']=='/loot':
                self.assertIn(p['monster'],{'heoni','memory_of_a_wolf','shiversleep','the_keeper','phosphorus'})
                self.assertGreater(len(p['value']['entries']),0)
                self.assertEqual(len(p['value']['entries']),len(p['source']['entry_sources']))
                for e in p['value']['entries']:
                    self.assertIn(int(e['item']['key'].split('/')[-1]),admitted)
                self.assertFalse(any(other['monster']==p['monster'] and other['pointer']=='/loot/entries/-' for other in packet['patches']))
        self.assertEqual(packet['counts']['comparisons_examined'],len(packet['resolved_comparisons'])+len(packet['unresolved']))

if __name__=='__main__':unittest.main()
