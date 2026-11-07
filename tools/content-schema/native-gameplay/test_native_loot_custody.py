"""Loot admission follows exact native custody without overwriting accepted facts."""
import copy
import unittest
import build_full_spell_artifact as artifact


class NativeLootCustody(unittest.TestCase):
    def fixture(self):
        item={'family':'Item','key':'oteryn:item.tibia.i6001','revision':'definition-r1'}
        facts={item['key']:{'item':item,'materializable':False,'container_capacity':None}}
        profile={'production_definition':{'family':'Item','production_key':item['key'],'revision_ref':item['revision']},
                 'admission':{'materializable':True},
                 'semantics':{'container':{'state':'KNOWN','value':{'capacity':{'state':'KNOWN','value':10}}}}}
        return facts,{'records':[profile]},item['key']

    def test_exact_native_corpse_custody_refines_inert_declaration(self):
        facts,profiles,key=self.fixture()
        artifact.refine_native_item_facts(facts,profiles)
        self.assertTrue(facts[key]['materializable'])
        self.assertEqual(facts[key]['container_capacity'],10)

    def test_foreign_revision_cannot_promote_custody(self):
        facts,profiles,_=self.fixture()
        profiles['records'][0]['production_definition']['revision_ref']='foreign'
        with self.assertRaises(ValueError):artifact.refine_native_item_facts(facts,profiles)

    def test_accepted_canonical_admission_remains_authoritative(self):
        facts,profiles,key=self.fixture()
        facts[key].update(materializable=True,container_capacity=20)
        before=copy.deepcopy(facts)
        artifact.refine_native_item_facts(facts,profiles)
        self.assertEqual(facts,before)

    def test_inert_native_provider_cannot_authorize_materialization(self):
        facts,profiles,_=self.fixture()
        profiles['records'][0]['admission']['materializable']=False
        before=copy.deepcopy(facts)
        artifact.refine_native_item_facts(facts,profiles)
        self.assertEqual(facts,before)
