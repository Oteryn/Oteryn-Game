import copy,json,unittest
from pathlib import Path
from npc_visual_appearance_stage import ROOT,build,valid_looks,valid_objects,values
class VisualStageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline=json.loads((ROOT/'content/world/definitions/declarations.json').read_text())
        packet=json.loads((ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r25/native-enrichment.json').read_text())
        records={r['identity']['key']:r for r in cls.baseline['records']}
        profiles={r['target']['key']:r for r in cls.baseline['authoring_profiles']}
        for repair in packet['repairs']:records[repair['before']['identity']['key']]=repair['before']
        for repair in packet['profile_repairs']:profiles[repair['before']['target']['key']]=repair['before']
        cls.baseline['records']=list(records.values())
        cls.baseline['authoring_profiles']=list(profiles.values())
        cls.packet=packet
        cls.sample={'key':'oteryn:npc.a_blue_stone','name':'a blue stone',
            'source':{'url':'https://example.org/wiki-visual-fixture','sha256':'a'*64},
            'rationale':'Test fixture for explicit approximate visual mapping.',
            'outfit':{'look_type':128,'head':0,'body':0,'legs':0,'feet':0,'addons':0,'mount':None}}
    def test_actual_visual_packet_replays_with_all_other_profiles_preserved(self):
        evidence=ROOT/'docs/agents/evidence/OTV2-20261002-npc-appearance-r25/source-facts.json'
        selections=json.loads(evidence.read_text())['selections']
        self.assertEqual(build(self.baseline,selections),self.packet)
        changed={r['before']['target']['key'] for r in self.packet['profile_repairs']}
        actors={r['before']['identity']['key']:r['before'] for r in self.packet['repairs']}
        self.assertEqual(len(changed),len(selections))
        for selection in selections:
            before=actors[selection['key']]
            self.assertIn(before['presentation']['key'],changed)
            self.assertNotIn(before['behavior']['key'],changed)
    def test_closed_packet_changes_only_presentation_and_its_evidence(self):
        p=build(self.baseline,[self.sample]);self.assertEqual(len(p['repairs']),133)
        self.assertEqual(len(p['profile_repairs']),1)
        for r in p['repairs']:
            self.assertEqual(r['after']['kind'],'NPC')
            for k in ['identity','behavior','presentation','dialogue','services']:
                self.assertEqual(r['before'][k],r['after'][k])
            before=values(r['before']);after=values(r['after'])
            for k in ['profession_selection','wiki_profession','dialogue_source','completion']:
                self.assertEqual(before.get(k),after.get(k))
    def test_approximation_cannot_be_relabelled_actor_exact_or_donor(self):
        s=copy.deepcopy(self.sample);s['classification']='ACTOR_EXACT_PUBLIC_DONOR'
        p=build(self.baseline,[s]);row=next(r['after'] for r in p['repairs'] if r['after']['identity']['key']==s['key'])
        v=values(row);a=json.loads(v['appearance_selection']);q=json.loads(v['quality'])
        self.assertEqual(a['classification'],'APPROXIMATE_WIKI_VISUAL_MAPPING')
        self.assertFalse(a['actor_exact_match']);self.assertFalse(a['canonical_tibia_fidelity_claim'])
        self.assertEqual(q['presentation'],'defaulted');self.assertEqual(q['presentation.look_type'],'defaulted')
    def test_client_membership_and_nonzero_sprite_data_are_required(self):
        looks=valid_looks();self.assertEqual(len(looks),1479);self.assertNotIn(1,looks)
        for bad in [1,999999,True]:
            s=copy.deepcopy(self.sample);s['outfit']['look_type']=bad
            with self.assertRaises(ValueError):build(self.baseline,[s])
    def test_donors_candis_and_dwarven_guard_cannot_be_overwritten(self):
        for key,name in [('blubster','Blubster'),('candis','Candis'),('dwarven_guard','Dwarven Guard')]:
            s=copy.deepcopy(self.sample);s['key']='oteryn:npc.'+key;s['name']=name
            with self.assertRaises(ValueError):build(self.baseline,[s])
    def test_invalid_palette_addon_or_mount_is_rejected(self):
        for part,bad in [('head',133),('body',-1),('addons',4),('mount',999999)]:
            s=copy.deepcopy(self.sample);s['outfit'][part]=bad
            with self.assertRaises(ValueError):build(self.baseline,[s])
    def test_missing_custody_and_foreign_actor_are_rejected(self):
        for bad in ['missing_sha','foreign_actor']:
            s=copy.deepcopy(self.sample)
            if bad=='missing_sha':s['source']['sha256']='z'*64
            else:s['key']='oteryn:npc.foreign_actor'
            with self.assertRaises(ValueError):build(self.baseline,[s])
    def test_static_object_mapping_uses_existing_native_object_profile(self):
        s=copy.deepcopy(self.sample);s['outfit']={'item_look':min(valid_objects())}
        p=build(self.baseline,[s]);profile=p['profile_repairs'][0]['after']['data']['profile']
        self.assertEqual(profile['asset_binding'],'canary.appearance:object/'+str(s['outfit']['item_look']))
        row=next(r['after'] for r in p['repairs'] if r['after']['identity']['key']==s['key'])
        a=json.loads(values(row)['appearance_selection'])
        self.assertEqual(a['classification'],'APPROXIMATE_WIKI_VISUAL_OBJECT_MAPPING')
        self.assertIn('outfit',a['previous_project_choice'])
    def test_object_membership_and_exclusive_family_selection_are_required(self):
        for outfit in [{'item_look':999999},{'item_look':True},{'item_look':100,'look_type':128}]:
            s=copy.deepcopy(self.sample);s['outfit']=outfit
            with self.assertRaises(ValueError):build(self.baseline,[s])
if __name__=='__main__':unittest.main()
