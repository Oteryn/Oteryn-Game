import copy
import unittest
import project_monster_slot_candidates as producer


class RealMonsterProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.packet=producer.build()

    def test_all175_exact_partition_and_schema(self):
        producer.validate(self.packet)
        self.assertEqual((175,10,62,103),tuple(self.packet[k] for k in
                         ['slot_count','full_slot_candidate_count','partial_target_slot_count','blocked_without_target_count']))
        self.assertFalse(self.packet['runtime_activation'])
        self.assertTrue(all(not s['native_execution_qualified'] for s in self.packet['slots']))

    def test_real_target_definitions_all_validate(self):
        fragments=[f for s in self.packet['slots'] for f in s['target_fragments']]
        self.assertEqual(130,sum(len(f['definitions']['abilities']) for f in fragments))
        self.assertEqual(141,sum(len(f['definitions']['effects']) for f in fragments))
        for f in fragments:producer.validate_definitions(f['definitions'])

    def test_d25_9_explicit_normalizations_no_alias_guess(self):
        selected=[s for s in self.packet['slots'] if s['canonical_normalization']]
        self.assertEqual(9,len(selected))
        for s in selected:
            self.assertFalse(s['source_type_equivalence'])
            self.assertEqual('D25',s['canonical_normalization']['decision'])
            self.assertIn(s['canonical_normalization']['damage_type'],['life_drain','mana_drain','physical'])
        held=[s for s in self.packet['slots'] if s['monster']=='Grimeleech' and s['slot_identity']['source_slot_index']==3]
        self.assertEqual(3,len(held));self.assertTrue(all(s['status']=='BLOCKED' and not s['canonical_normalization'] for s in held))

    def test_named_ratmiral_allies_and_no_caster_removal(self):
        s=next(s for s in self.packet['slots'] if s['monster']=='Ratmiral Blackwhiskers' and s['source_parameters']['name']=='ratmiral ball')
        self.assertTrue(s['full_slot_projection_complete'])
        definitions=s['target_fragments'][0]['definitions']
        self.assertEqual(['presentation_only','heal'],[e['operation'] for e in definitions['effects']])
        effect=definitions['effects'][1]
        self.assertTrue(effect['affects']['top_creature_only'])
        self.assertEqual('named_creatures',effect['affects']['kind'])
        self.assertEqual(3,len(effect['affects']['creatures']))
        self.assertEqual({'minimum':0,'maximum':1000},definitions['formulas'][0]['magnitude'])
        self.assertFalse(s['native_execution_qualified'])

    def test_unsupported_controller_never_full(self):
        s=next(s for s in self.packet['slots'] if s['source_parameters']['name']=='foamsplash')
        self.assertEqual(3,len(s['target_fragments']))
        self.assertFalse(s['full_slot_projection_complete'])
        self.assertTrue(any('scheduled_callback' in b['detail'] for b in s['blockers']))

    def test_promotion_omission_or_target_mutation_rejected(self):
        for mode in ['promote','omit','effect']:
            p=copy.deepcopy(self.packet)
            if mode=='promote':p['slots'][0]['full_slot_projection_complete']=True
            elif mode=='omit':p['slots'].pop()
            else:
                f=next(f for s in p['slots'] for f in s['target_fragments'])
                f['definitions']['abilities'][0]['range_tiles']=999
            with self.assertRaises(Exception):producer.validate(p)


if __name__=='__main__':unittest.main()
