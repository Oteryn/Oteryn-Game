import copy,json,unittest
from npc_visual_invisible_refine import ROOT,TARGETS,build,values
class VisualInvisibleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline=json.loads((ROOT/'content/world/definitions/declarations.json').read_text())
        e=ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r27'
        cls.packet=json.loads((e/'native-enrichment.json').read_text())
        cls.selections=json.loads((e/'source-facts.json').read_text())['selections']
        records={r['identity']['key']:r for r in cls.baseline['records']}
        profiles={r['target']['key']:r for r in cls.baseline['authoring_profiles']}
        quest_dialogue=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-enrichment-r28/native-enrichment.json').read_text())
        for repair in quest_dialogue['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in quest_dialogue['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        for repair in cls.packet['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in cls.packet['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        cls.baseline['records']=list(records.values());cls.baseline['authoring_profiles']=list(profiles.values())
    def test_actual_derived_visual_packet_replays_exactly(self):
        self.assertEqual(build(self.baseline,self.selections),self.packet)
    def test_closed_two_actor_scope_rejects_foreign_or_missing_choice(self):
        for wrong in ['foreign','missing']:
            selected=copy.deepcopy(self.selections)
            if wrong=='foreign':selected[0]['key']='oteryn:npc.opticorder_forge_npc'
            else:selected.pop()
            with self.assertRaises(ValueError):build(self.baseline,selected)
    def test_only_two_presentations_change_and_all_other_authoring_is_preserved(self):
        self.assertEqual(len(self.packet['repairs']),133);self.assertEqual(len(self.packet['profile_repairs']),2)
        targets={key.replace('oteryn:npc.','oteryn:presentation.npc.') for key in TARGETS}
        self.assertEqual({r['after']['target']['key'] for r in self.packet['profile_repairs']},targets)
        for repair in self.packet['repairs']:
            b,a=repair['before'],repair['after'];self.assertEqual(b['kind'],'NPC')
            for key in ['identity','presentation','behavior','dialogue','services']:self.assertEqual(b[key],a[key])
            if a['identity']['key'] not in TARGETS:self.assertEqual(b,a)
            else:
                before,after=values(b),values(a)
                for key in ['profession_selection','wiki_profession','dialogue_source','completion']:
                    self.assertEqual(before.get(key),after.get(key))
    def test_invisible_selection_flags_are_explicit_inference_not_source_truth(self):
        for repair in self.packet['profile_repairs']:
            self.assertEqual(repair['after']['data']['profile'],{'selection':'Invisible','light_level':0})
        for repair in self.packet['repairs']:
            if repair['after']['identity']['key'] not in TARGETS:continue
            a=json.loads(values(repair['after'])['appearance_selection'])
            self.assertEqual(a['classification'],'APPROXIMATE_WIKI_VISUAL_INVISIBLE_MAPPING')
            self.assertEqual(a['source_visibility_classification'],'derived_visual_reference')
            self.assertIsNone(a['outfit'])
            for key in ['actor_exact_match','target_native_appearance_verified','canonical_tibia_fidelity_claim',
                        'native_runtime_visibility_qualified','literal_source_invisibility_claim']:
                self.assertFalse(a[key])
            self.assertEqual(a['previous_project_choice']['classification'],'PROJECT_DEFAULT_NO_SOURCE_SPRITE')
        selected=copy.deepcopy(self.selections);selected[0]['source_visibility_classification']='source_fact'
        with self.assertRaises(ValueError):build(self.baseline,selected)
if __name__=='__main__':unittest.main()
