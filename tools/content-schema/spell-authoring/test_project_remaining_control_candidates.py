import json
from pathlib import Path
import unittest
import project_remaining_control_candidates as producer


class RemainingControlTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root=Path(__file__).resolve().parents[3]
        cls.packet,cls.proof=producer.build(cls.root)
        cls.audit=json.loads(cls.packet['lane-audit.json'])['records']

    def test_complete_lane_and_only_reviewed_eight_candidates(self):
        self.assertEqual(45,len(self.audit));self.assertEqual(45,len({row['registration_key'] for row in self.audit}))
        full=[row for row in self.audit if row['status']=='CANDIDATE_SCHEMA_VALID']
        self.assertEqual(8,len(full));self.assertEqual(8,self.proof['standard_bundle_count'])
        self.assertTrue(all(row['registration_key'].rsplit('/',1)[1].rsplit('#',1)[0] in producer.FULL for row in full))

    def test_native_aliases_and_partial_controllers_never_promoted(self):
        partial=json.loads(self.packet['partial-data.json'])['records']
        self.assertEqual(30,len(partial));self.assertEqual(24,sum(row['disposition']=='PROPOSED_NATIVE_BINDING' for row in partial))
        self.assertTrue(all(not row['full_target_projection_complete'] for row in partial))
        self.assertTrue(all(not row['runtime_activation'] and not row['native_execution_qualified'] for row in self.audit))

    def test_all_source_headers_remain_byte_exact_including_disabled(self):
        for row in self.audit:
            path=producer.subfolder(row['registration_key'])+'/source-header.json'
            self.assertEqual((self.root/producer.BASE/path).read_bytes(),self.packet[path])
        self.assertEqual(4,sum(row['disposition']=='DISABLED_REFERENCE_EXAMPLE' for row in self.audit))
        self.assertEqual(2,sum(row['disposition']=='RETIRED_REFERENCE_ONLY_S24' for row in self.audit))

    def test_healing_guards_and_paralyze_explicit_canonical_projection(self):
        for row in self.audit:
            if row['status']!='CANDIDATE_SCHEMA_VALID':continue
            path=producer.subfolder(row['registration_key']);s=json.loads(self.packet[path+'/spell.json'])['spell']
            deps=json.loads(self.packet[path+'/dependencies.json']);receipt=json.loads(self.packet[path+'/receipt.json'])
            self.assertEqual(producer.REVISION,s['identity']['revision'])
            self.assertFalse(row['source_full_mechanics_1_to_1_complete'])
            if s['carrier']=='rune':self.assertTrue(receipt['item_owner_bindings_required'])
            if s['name'].lower() in ('intense healing rune','ultimate healing rune'):
                self.assertEqual('self_or_own_summons',s['targeting']['allowed_targets'])
                if s['name'].lower()=='ultimate healing rune':
                    self.assertFalse({'monk','exalted_monk'}&set(s['requirements']['vocations']))
                else:
                    self.assertTrue({'monk','exalted_monk','none'}<=set(s['requirements']['vocations']))
            if s['name'].lower()=='paralyze rune':
                self.assertTrue(deps['abilities'][0]['zero_damage_health_path'])
                self.assertEqual(6000,deps['effects'][0]['duration_ms'])
                self.assertEqual('after_success',deps['effects'][0]['presentation']['caster_effect_timing'])

    def test_native_binding_identical_profile_and_no_source_receipt(self):
        for row in self.audit:
            if row['disposition']!='PROPOSED_NATIVE_BINDING':continue
            path=producer.subfolder(row['registration_key'])
            self.assertNotIn(path+'/receipt.json',self.packet)
            self.assertFalse(row['full_target_projection_complete'])
            self.assertTrue(row['blockers'])


if __name__=='__main__':unittest.main()
