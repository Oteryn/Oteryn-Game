import copy,json,unittest
from npc_visual_appearance_followup import ROOT,build,values
class VisualFollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline=json.loads((ROOT/'content/world/definitions/declarations.json').read_text())
        evidence=ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r26'
        cls.packet=json.loads((evidence/'native-enrichment.json').read_text())
        cls.selections=json.loads((evidence/'source-facts.json').read_text())['selections']
        records={r['identity']['key']:r for r in cls.baseline['records']}
        profiles={r['target']['key']:r for r in cls.baseline['authoring_profiles']}
        invisible = json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r27/native-enrichment.json').read_text())
        quest_dialogue=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-enrichment-r28/native-enrichment.json').read_text())
        for repair in quest_dialogue['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in quest_dialogue['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        for repair in invisible['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in invisible['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        for repair in cls.packet['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in cls.packet['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        cls.baseline['records']=list(records.values());cls.baseline['authoring_profiles']=list(profiles.values())
        parent=ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r25/source-facts.json'
        cls.previous=json.loads(parent.read_text())
    def test_actual_followup_packet_replays_exactly(self):
        self.assertEqual(build(self.baseline,self.selections),self.packet)
        self.assertEqual(self.packet['from_project_revision'],'g4-npc-provisional-enrichment-r25')
        self.assertEqual(self.packet['project_revision'],'g4-npc-provisional-enrichment-r26')
    def test_prior_98_visual_selections_cannot_be_overwritten(self):
        self.assertEqual(len(self.previous['selections']),98)
        with self.assertRaises(ValueError):build(self.baseline,[self.previous['selections'][0]])
    def test_held_scope_preserves_all_unselected_profiles_and_other_authoring(self):
        held={x['key'] for x in self.previous['held']};selected={x['key'] for x in self.selections}
        self.assertEqual(len(held),12);self.assertTrue(selected<=held)
        for repair in self.packet['repairs']:
            before,after=repair['before'],repair['after'];self.assertEqual(before['kind'],'NPC')
            for k in ['identity','presentation','behavior','dialogue','services']:
                self.assertEqual(before[k],after[k])
            b,a=values(before),values(after)
            for k in ['profession_selection','wiki_profession','dialogue_source','completion']:
                self.assertEqual(b.get(k),a.get(k))
            if before['identity']['key'] not in selected:self.assertEqual(before,after)
            else:
                metadata=json.loads(a['source_metadata'])
                self.assertIn('r26_visual_mapping',metadata);self.assertNotIn('r25_visual_mapping',metadata)
        targets={r['before']['target']['key'] for r in self.packet['profile_repairs']}
        visual={s['key'] for s in self.selections if s.get('choice_kind')!='project_default_no_source_sprite'}
        self.assertEqual(targets,{k.replace('oteryn:npc.','oteryn:presentation.npc.') for k in visual})
        # No Behavior or source-protected/previously selected presentation appears in repairs.
        self.assertTrue(all(k.startswith('oteryn:presentation.npc.') for k in targets))
    def test_documented_invisible_opticorder_uses_empty_presentation(self):
        selected=next(s for s in self.selections if s.get('visibility')=='invisible')
        packet=build(self.baseline,[selected])
        profile=packet['profile_repairs'][0]['after']
        self.assertEqual(profile['target']['key'],'oteryn:presentation.npc.opticorder_forge_npc')
        self.assertEqual(profile['data']['profile'],{'selection':'Invisible','light_level':0})
        row=next(r['after'] for r in packet['repairs'] if r['after']['identity']['key']==selected['key'])
        selection=json.loads(values(row)['appearance_selection'])
        self.assertEqual(selection['classification'],'APPROXIMATE_WIKI_DOCUMENTED_INVISIBLE_MAPPING')
        self.assertIsNone(selection['outfit']);self.assertFalse(selection['canonical_tibia_fidelity_claim'])
    def test_invisibility_requires_correct_actor_and_literal_documentary_proof(self):
        selected=next(s for s in self.selections if s.get('visibility')=='invisible')
        for wrong in ['actor','literal_proof']:
            sample=copy.deepcopy(selected)
            if wrong=='actor':sample['key']='oteryn:npc.dread_guardian';sample['name']='Dread Guardian'
            else:sample.pop('visibility_evidence',None)
            with self.assertRaises(ValueError):build(self.baseline,[sample])
    def test_source_unknown_neutral_choices_preserve_profiles_and_unknown_status(self):
        defaults=[s for s in self.selections if s.get('choice_kind')=='project_default_no_source_sprite']
        self.assertEqual({s['key'] for s in defaults},{'oteryn:npc.mud','oteryn:npc.planestrider_npc'})
        packet=build(self.baseline,defaults);self.assertFalse(packet['profile_repairs'])
        for repair in packet['repairs']:
            if repair['after']['identity']['key'] not in {s['key'] for s in defaults}:continue
            selection=json.loads(values(repair['after'])['appearance_selection'])
            self.assertEqual(selection['classification'],'PROJECT_DEFAULT_NO_SOURCE_SPRITE')
            self.assertEqual(selection['source_completeness'],'unknown')
            self.assertEqual(selection['visual_correspondence'],'unknown')
            self.assertFalse(selection['actor_exact_match'])
    def test_neutral_source_unknown_choices_cannot_change_actor_or_outfit(self):
        default=next(s for s in self.selections if s.get('choice_kind')=='project_default_no_source_sprite')
        for wrong in ['actor','outfit']:
            sample=copy.deepcopy(default)
            if wrong=='actor':sample['key']='oteryn:npc.dread_guardian';sample['name']='Dread Guardian'
            else:sample['outfit']['look_type']=129
            with self.assertRaises(ValueError):build(self.baseline,[sample])
if __name__=='__main__':unittest.main()
