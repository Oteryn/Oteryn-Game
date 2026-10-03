"""Reject false cross-wiki item aliases and qualify bounded positive matches."""
from pathlib import Path
import sys
import tempfile
import json
import unittest
sys.path.insert(0,str(Path(__file__).parent))
import complete_captured_loot_alias_delta as lane

class CapturedAliasTests(unittest.TestCase):
    def test_real_actor_item_link_and_weight_required(self):
        br={'fields':{'weight':'1.60','droppedby':'[[Werewolf]], [[Darkfang]]'}}
        en={'fields':{'weight':'1.60'}}
        stats={'content':'{{Loot2\n|Werewolf Fangs, times:30\n}}'}
        self.assertTrue(lane.corroborated(br,en,'Werewolf',stats,'Werewolf Fangs')[0])
        en['fields']['weight']='2.50'
        self.assertFalse(lane.corroborated(br,en,'Werewolf',stats,'Werewolf Fangs')[0])
    def test_matching_name_without_exact_actor_context_is_insufficient(self):
        br={'fields':{'weight':'0.90','droppedby':'[[Brachiodemon]]'}}
        en={'fields':{'weight':'0.90'}}
        self.assertFalse(lane.corroborated(br,en,'Unrelated phase',{'content':'|Hand, times:50'},'Hand')[0])
        self.assertFalse(lane.corroborated(br,en,'Brachiodemon',{'content':'|Other hand, times:50'},'Hand')[0])
    def test_unknown_weight_cannot_prove_identity(self):
        br={'fields':{'weight':'?','droppedby':'[[Example]]'}}
        en={'fields':{'weight':'?'}}
        self.assertFalse(lane.corroborated(br,en,'Example',{'content':'|Thing, times:50'},'Thing')[0])
    def test_empty_field_does_not_swallow_following_weight(self):
        page=lane.exact_fields({'content':'| flavortext =\n| weight = 0.90\n| droppedby = [[Brachiodemon]]'})
        self.assertEqual(page['fields']['weight'],'0.90')
        self.assertEqual(page['fields']['flavortext'],'')
    def test_final_delta_has_no_duplicate_data_patches_and_flag_guards(self):
        packet=json.loads(Path('/workspace/monster-field-next-20261002/loot-alias-delta-final/field-patches.json').read_text())
        self.assertEqual(packet['patches'],[])
        outstanding={r['monster'] for r in packet['unresolved']}
        idx=json.loads(Path('/workspace/monster-field-fill-20261002/population/population-index.json').read_text())
        flags={r['monster']:set(r.get('completion_flags',[])) for r in idx['monsters']}
        for actor,retired in packet['resolved_actor_flags'].items():
            self.assertNotIn(actor,outstanding)
            self.assertTrue(set(retired)<=flags[actor])
        phase=[r for r in packet['resolved_comparisons'] if r['status']=='BOSS_PHASE_APPLICABILITY_LINK_NOT_A_LOOT_ITEM']
        self.assertEqual(len(phase),1);self.assertEqual(phase[0]['monster'],'zushuka')
        self.assertIn('Disponível apenas na sua fase',phase[0]['proof']['raw_loot_value'])
        for row in packet['resolved_comparisons']:
            if row['status']!='SOURCE_CORROBORATED_ALIAS_ALREADY_PRESENT':continue
            proof=row['proof'];self.assertEqual(len(proof['existing_item_ids']),1)
            self.assertTrue(proof['item_identity_source']['sha']);self.assertTrue(proof['br_identity_source']['sha'])
        self.assertEqual(294,len(packet['unresolved'])+len(packet['resolved_comparisons']))
    def test_output_cannot_write_to_population(self):
        with tempfile.TemporaryDirectory() as d:
            with self.assertRaisesRegex(ValueError,'cannot modify'):
                lane.build(Path(d),Path(d)/'unused',Path(d)/'out')

if __name__=='__main__':unittest.main()
