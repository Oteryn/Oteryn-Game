import copy,json,unittest
from npc_quest_dialogue_followup import ROOT,build,literal_input,eligible,values
class QuestDialogueFollowupTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline=json.loads((ROOT/'content/world/definitions/declarations.json').read_text())
        e=ROOT/'docs/agents/evidence/OTV2-20261002-npc-enrichment-r28'
        cls.packet=json.loads((e/'native-enrichment.json').read_text());facts=json.loads((e/'source-facts.json').read_text())
        cls.upgrades=facts['upgrades'];cls.exchange=facts.get('exchange_reference')
        records={r['identity']['key']:r for r in cls.baseline['records']}
        for repair in cls.packet['repairs']:records[repair['before']['identity']['key']]=repair['before']
        cls.baseline['records']=list(records.values())
    def test_actual_quest_dialogue_packet_replays_exactly(self):
        self.assertEqual(build(self.baseline,self.upgrades,self.exchange),self.packet)
    def test_player_normalization_is_only_explicit_exact_substitution(self):
        actor=next(r for r in self.upgrades if any(q.get('normalization',{}).get('from')=='Player' for q in r['dialogue'].values()))
        quote=next(q for q in actor['dialogue'].values() if q.get('normalization',{}).get('from')=='Player')
        literal=literal_input(quote,actor['name'])
        self.assertEqual(literal['reply'],[p['original_exact_text'] for p in quote['quote_evidence']])
        broken=copy.deepcopy(quote);broken['reply']=['Invented greeting.']
        with self.assertRaises(ValueError):literal_input(broken,actor['name'])
    def test_exact_speaker_scope_and_no_actions_are_enforced(self):
        quote=copy.deepcopy(next(iter(self.upgrades[0]['dialogue'].values())))
        quote['quote_evidence'][0]['speaker_identity']='Airclairebear'
        with self.assertRaises(ValueError):literal_input(quote,self.upgrades[0]['name'])
        upgrade=copy.deepcopy(self.upgrades[0]);upgrade['appearance']={'look_type':128}
        with self.assertRaises(ValueError):build(self.baseline,[upgrade])
        upgrade=copy.deepcopy(self.upgrades[0]);upgrade['key']='oteryn:npc.candis'
        with self.assertRaises(ValueError):build(self.baseline,[upgrade])
    def test_all_profiles_services_roles_movement_and_previous_history_are_preserved(self):
        self.assertEqual(len(eligible()),33);self.assertFalse(self.packet['profile_repairs'])
        for repair in self.packet['repairs']:
            b,a=repair['before'],repair['after']
            if a['kind']!='NPC':continue
            for k in ['identity','presentation','behavior','dialogue','services']:self.assertEqual(b[k],a[k])
            bv,av=values(b),values(a)
            for k in ['profession_selection','wiki_profession','appearance_selection','completion']:
                self.assertEqual(bv.get(k),av.get(k))
            metadata=json.loads(av['source_metadata']);previous=json.loads(bv['source_metadata'])
            metadata.pop('r28_source_upgrades',None);metadata.pop('r28_exchange_reference',None)
            self.assertEqual(metadata,previous)
    def test_blue_stone_prices_are_documentary_only(self):
        self.assertIsNotNone(self.exchange)
        packet=build(self.baseline,[],self.exchange)
        self.assertFalse(packet['profile_repairs']);self.assertEqual(len(packet['repairs']),133)
        for repair in packet['repairs']:
            if repair['after']['identity']['key']!='oteryn:npc.a_blue_stone':self.assertEqual(repair['before'],repair['after'])
            else:
                self.assertEqual(repair['before']['services'],repair['after']['services'])
                reference=json.loads(values(repair['after'])['source_metadata'])['r28_exchange_reference']
                self.assertFalse(reference['trade_enabled']);self.assertFalse(reference['runtime_enabled'])
                self.assertEqual(len(reference['prices']),5)
if __name__=='__main__':unittest.main()
