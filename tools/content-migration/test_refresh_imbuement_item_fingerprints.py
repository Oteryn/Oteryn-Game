"""Only derived fingerprints can move; source identities and semantics stay fixed."""
import copy
import unittest
from refresh_imbuement_item_fingerprints import verify_fingerprints_only

class FingerprintGuardTests(unittest.TestCase):
    def fixture(self, eligibility=False):
        key='oteryn:item.tibia.i5954'
        field='canonical_definition_sha256' if eligibility else 'definition_sha256'
        leaf={field:'a'*64,'name':'demon horn'}
        record={'item_ref':{'family':'Item','key':key,'revision':'definition-r1'},
                'slots':2,'source_observations':[{'sha256':'preserved'}],
                'evidence':leaf,'runtime_admission':'BLOCKED'}
        changed=copy.deepcopy(record)
        changed['evidence'][field]='b'*64
        return record,changed,{key:{'definition_sha256':'b'*64}}

    def test_both_existing_fingerprint_shapes_keep_every_other_field(self):
        for eligibility in (False,True):
            old,new,definitions=self.fixture(eligibility)
            old_copy,new_copy=copy.deepcopy(old),copy.deepcopy(new)
            verify_fingerprints_only(old,new,definitions)
            self.assertEqual((old,new),(old_copy,new_copy))

    def test_slot_identity_observation_or_runtime_change_refuses(self):
        for change in ('slots','identity','observation','runtime'):
            old,new,definitions=self.fixture()
            if change=='slots':new['slots']=3
            elif change=='identity':new['item_ref']['key']='oteryn:item.tibia.i9999'
            elif change=='observation':new['source_observations'][0]['sha256']='changed'
            else:new['runtime_admission']='ENABLED'
            with self.assertRaisesRegex(ValueError,'SEMANTIC_CHANGE'):
                verify_fingerprints_only(old,new,definitions)

    def test_unresolved_identity_wrong_target_hash_and_extra_row_refuse(self):
        old,new,definitions=self.fixture()
        with self.assertRaisesRegex(ValueError,'UNQUALIFIED_FINGERPRINT'):
            verify_fingerprints_only(old,new,{})
        new['evidence']['definition_sha256']='c'*64
        with self.assertRaisesRegex(ValueError,'UNQUALIFIED_FINGERPRINT'):
            verify_fingerprints_only(old,new,definitions)
        with self.assertRaisesRegex(ValueError,'SEMANTIC_CHANGE'):
            verify_fingerprints_only([old],[old,old],definitions)

if __name__=='__main__':
    unittest.main()
