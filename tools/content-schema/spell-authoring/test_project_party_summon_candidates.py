"""Real-schema and source-identity regressions for the R57 bounded lane."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import project_party_summon_candidates as m

class ProjectionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.packet=m.build()

    def test_exact_partition(self):
        p=self.packet
        self.assertEqual((p['records'],p['party_records'],p['summon_records']),(28,11,17))
        self.assertEqual((p['full_source_registration_candidates'],p['proposed_native_bindings'],p['partial_condition_targets'],p['remaining_without_target']),(2,6,10,12))
        self.assertEqual(sum(r['status']=='PARTIAL_NATIVE_SOURCE_TARGET' for r in p['rows']),4)

    def test_actual_native_definitions_and_unique_identity(self):
        for row in self.packet['rows']:
            proposal=row['proposed_native_binding']
            if proposal:
                self.assertEqual(m.validate_spell.validate(proposal['bundle'],proposal['dependencies'],proposal['catalog']),[])
                self.assertIn(hashlib.sha256(row['registration_key'].encode()).hexdigest()[:16],proposal['bundle']['spell']['identity']['key'])
                self.assertFalse(proposal['native_profile_alias_used'])
                self.assertFalse(row['reader_acceptance_qualified'])
                self.assertTrue(proposal['exact_cast_matches_accepted_recipe'])

    def test_bounded_function_proofs_and_helper_gap(self):
        for row in self.packet['rows']:
            p=row['proposed_native_binding']
            if p:
                self.assertEqual(len(p['bounded_function_proofs']),7)
                self.assertTrue(all(v['function_equal'] for v in p['bounded_function_proofs']))
                self.assertFalse(p['raw_getter_binding_qualified'])
                if row['name'].casefold()!='summon creature':
                    self.assertIn('set_summon_attacked_target_copy',row['required_operations_unrepresented'])
                    self.assertFalse(row['full_source_registration_candidate'])

    def test_party_actual_parameters(self):
        rows=[r for r in self.packet['rows'] if r['partial_target_dependencies']]
        self.assertEqual(len(rows),10)
        self.assertTrue(all(r['source_parameters']['party_distance_bound']==36 for r in rows))
        self.assertTrue(all(not r['full_source_registration_candidate'] for r in rows))
        enlighten=[r for r in rows if r['name'].casefold()=='enlighten party' and r['source_revision']==m.PINS['crystal']]
        self.assertEqual(len(enlighten),1)
        self.assertIn('300000',json.dumps(enlighten[0]['source_parameters']))

    def test_fail_closed_promotion(self):
        for field in ('runtime_activation','native_execution_qualified','reader_acceptance_qualified'):
            p=copy.deepcopy(self.packet);p['rows'][0][field]=True
            with self.assertRaises(ValueError):m.validate(p)

    def test_layout_headers_manifest_and_exact_rebuild(self):
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp);m.write_packet(out)
            summary=json.loads((out/'import-summary.json').read_text())
            receipt_validator=m.validate_spell.Draft202012Validator(json.loads((out/'receipt.schema.json').read_text()))
            for row in summary['records_index']:
                folder=out/row['snapshot']
                receipt_validator.validate(json.loads((folder/'receipt.json').read_text()))
                self.assertEqual(m.sha((folder/'source-header.json').read_bytes()),row['source_header_sha256'])
                self.assertEqual((folder/'spell.json').exists(),row['full_source_registration_candidate'])
            manifest=json.loads((out/'package-manifest.json').read_text())
            self.assertTrue(all(m.sha((out/path).read_bytes())==digest for path,digest in manifest['files'].items()))
        m.validate(self.packet)

if __name__=='__main__':unittest.main()
