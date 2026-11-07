import copy
import importlib.util
import json
import os
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).parent
ROOT = Path(os.environ.get('QUEST_PRODUCT_ROOT', str(HERE.parents[2])))
sys.path.insert(0,str(ROOT/'tools/content-schema/quest-authoring'))
MODULE = HERE/'quest_chosen_journal.py' if (HERE/'quest_chosen_journal.py').exists() else HERE/'builder.py'
spec = importlib.util.spec_from_file_location('chosen', MODULE)
b = importlib.util.module_from_spec(spec)
spec.loader.exec_module(b)

class ChosenTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.evidence = HERE/'corrections.json'
        if not cls.evidence.exists(): cls.evidence = ROOT/b.PACKET
        cls.packet = b.read(cls.evidence)
        defs = b.definitions(ROOT)
        for value in defs.values():
            value['oteryn_recipe']['runtime_enabled'] = False
        cls.records = [{'definition': dict(d, identity={'key':key})} for key,d in defs.items()]

    def test_finite_schema_and_raw_source_replay(self):
        b.check_packet(self.packet)
        rebuilt = b.build(ROOT,self.evidence.parent,ROOT/'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz')
        self.assertEqual(rebuilt,self.packet)
        self.assertEqual(rebuilt['summary']['exact_source_spans'],57)

    def test_source_rewards_and_title_map_preserved(self):
        result = b.apply_records(self.records,self.packet,b.EXPECTED_PACKET_SHA256)
        before = {r['definition']['identity']['key']:r['definition'] for r in self.records}
        after = {r['definition']['identity']['key']:r['definition'] for r in result}
        for key in before:
            self.assertEqual(before[key]['source_data'],after[key]['source_data'])
            self.assertEqual(before[key]['oteryn_recipe']['payload']['recipe']['reward_intents'],after[key]['oteryn_recipe']['payload']['recipe']['reward_intents'])
            if key != 'oteryn:quest.tibia_tales':
                self.assertEqual(before[key]['oteryn_recipe']['payload']['covered_wiki_titles'],after[key]['oteryn_recipe']['payload']['covered_wiki_titles'])
        fox = after['oteryn:quest.tibia_tales']['oteryn_recipe']['payload']
        self.assertEqual([s['key'] for s in fox['recipe']['stages']],['s'+str(i) for i in range(1,21)])
        self.assertEqual(fox['title_stage_keys']['Tibia Tales'],['s'+str(i) for i in range(1,21)])
        self.assertEqual(fox['title_stage_keys']['To Outfox a Fox Quest'],['s17','s18','s19'])
        self.assertEqual(fox['covered_wiki_titles'].count('To Outfox a Fox Quest'),1)

    def test_mission_and_stage_drift_rejected(self):
        for kind in ('mission','stages'):
            records = copy.deepcopy(self.records)
            d = next(r['definition'] for r in records if r['definition']['identity']['key']=='oteryn:quest.tibia_tales')
            if kind=='mission':
                next(m for m in d['source_data']['quest']['missions'] if m['key']=='to_outfox_a_fox')['end_value']=99
            else:
                d['oteryn_recipe']['payload']['recipe']['stages'][0]['objective']='tampered'
            with self.assertRaises(ValueError): b.apply_records(records,self.packet,b.EXPECTED_PACKET_SHA256)

    def test_packet_unknown_promotion_choice_rejected(self):
        for field,value in [('native_admission',True),('unapproved_field',1)]:
            packet=copy.deepcopy(self.packet);packet[field]=value
            with self.assertRaises(Exception): b.check_packet(packet)
        packet=copy.deepcopy(self.packet);packet['records'][0]['chosen_mapping']['goal']=100
        with self.assertRaises(Exception): b.check_packet(packet)

    def test_missing_packet_fail_closed(self):
        import tempfile
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError): b.apply(Path(directory),self.records)

    def test_distinct_task_and_new_death_fences(self):
        for r in self.packet['records'][:3]:
            c=r['chosen_mapping']
            self.assertEqual(len(c['source_task_stage_keys']),4)
            self.assertFalse(c['repeat_handin_advances_chosen_stage'])
        middle=self.packet['records'][1]['chosen_mapping']['source_task_stage_keys']
        self.assertEqual(next(stage for task,stage in middle.items() if '_Nest_' in task),'s7')
        self.assertEqual(next(stage for task,stage in middle.items() if '_Charge_' in task),'s9')
        c=self.packet['records'][-1]['chosen_mapping']
        self.assertTrue(c['fresh_authenticated_death_required'])
        self.assertFalse(c['prior_kill_history_completes_stage'])

if __name__=='__main__': unittest.main()
