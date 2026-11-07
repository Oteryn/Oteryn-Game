"""Negative identity/provenance guards and real accepted Item API preparation."""
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest

import prepare_noncorpse_item_candidates as prep


class NoncorpseCandidates(unittest.TestCase):
    def setUp(self):
        self.binding = {'family': 'Item', 'key': 'oteryn:item.tibia.i1', 'revision': 'definition-r1'}
        self.packet = {'source_item_id': 1, 'scope_is_corpse': False, 'canonical_binding': self.binding,
                       'admission_authorized': False, 'runtime_qualified': False}
        self.native = {self.binding['key']: {'kind': 'Item', 'identity': self.binding, 'materializable': False}}
        self.item = {'identity': {'key': self.binding['key'], 'revision': 'definition-r1'}}

    def candidate(self, packet=None, item=None, native=None, report=None):
        return prep.candidate(packet or self.packet, 'a' * 64, self.item if item is None else item,
                              {}, report or {'item_id': 1, 'key': self.binding['key'],
                                             'blockers': ['sprite_atlas_not_admitted']},
                              native or self.native, lambda *_: ([], ['source-only']))

    def test_valid_authoring_keeps_admission_blockers_and_unknown_units(self):
        result = self.candidate()
        self.assertEqual('AUTHORING_SCHEMA_VALID', result['status'])
        self.assertEqual(['sprite_atlas_not_admitted'], result['report']['blockers'])
        for flag in ('native_applied', 'admission_authorized', 'runtime_qualified', 'current_native_materializable'):
            self.assertIs(False, result[flag])
        self.assertEqual('UNKNOWN', result['native_weight_unit'])
        self.assertEqual('UNKNOWN', result['native_temporal_consumption_mode'])

    def test_source_canonical_family_and_revision_mismatch_refuse(self):
        for field, value in (('family', 'Creature'), ('revision', 'wrong-revision')):
            packet = copy.deepcopy(self.packet)
            packet['canonical_binding'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.candidate(packet=packet)

    def test_converter_other_item_or_epoch_refuses(self):
        for field, value in (('key', 'oteryn:item.tibia.i2'), ('revision', 'wrong-revision')):
            item = copy.deepcopy(self.item)
            item['identity'][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.candidate(item=item)

    def test_native_duplicate_or_wrong_family_is_not_silently_indexed(self):
        record = self.native[self.binding['key']]
        for records in ([record, record], [{**record, 'kind': 'Creature'}],
                        [{**record, 'identity': {**self.binding, 'family': 'Creature'}}]):
            with self.subTest(records=records), self.assertRaises(ValueError):
                prep.native_items({'records': records})

    def test_forged_proof_alias_cannot_rebind_to_another_valid_native_item(self):
        packet = copy.deepcopy(self.packet)
        packet['canonical_binding'].update(key='oteryn:item.tibia.i2', proof_resolved_key=self.binding['key'])
        native = {'oteryn:item.tibia.i2': {'kind': 'Item', 'identity': packet['canonical_binding'],
                                        'materializable': False}}
        with self.assertRaises(ValueError):
            self.candidate(packet=packet, native=native)

    def test_routed_none_cannot_bypass_engine_source_identity(self):
        good = {'item_id': 1, 'key': self.binding['key'],
                'routed_non_item': {'owner': 'WorldObject', 'reason': 'corpse'}}
        for mutation in ({'item_id': 2}, {'item_id': True}, {'key': 'oteryn:item.tibia.i2'}):
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                prep.candidate(self.packet, 'a' * 64, None, None, {**good, **mutation},
                               self.native, lambda *_: ([], []))
        result = prep.candidate(self.packet, 'a' * 64, None, None,
                                {k: v for k, v in good.items() if k != 'key'},
                                self.native, lambda *_: ([], []))
        self.assertEqual('SOURCE_AUTHORING_UNRESOLVED', result['status'])
        self.assertIs(False, result['source_identity_binding_proven'])

    def test_authored_item_without_converter_key_is_not_binding_proof(self):
        with self.assertRaises(ValueError):
            self.candidate(report={'item_id': 1, 'converted': True})

    def scope(self, root):
        prep.write(root / 'items/1.json', self.packet)
        summary = {'schema': prep.SCOPE_SCHEMA, 'admission_authorized': False, 'runtime_qualified': False,
                   'output_sha256': {'items/1.json': prep.sha(root / 'items/1.json')}}
        prep.write(root / 'summary.json', summary)
        return prep.sha(root / 'summary.json')

    def test_modified_packet_or_summary_fails_exact_input_hash(self):
        for target in ('items/1.json', 'summary.json'):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                expected = self.scope(root)
                (root / target).write_text('{}')
                with self.assertRaises(ValueError):
                    prep.scope_packets(root, expected, 1)

    def test_scope_count_or_nonboolean_classification_drift_refuses(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            expected = self.scope(root)
            with self.assertRaises(ValueError):
                prep.scope_packets(root, expected, 2)
            self.packet['scope_is_corpse'] = 0
            expected = self.scope(root)
            with self.assertRaises(ValueError):
                prep.scope_packets(root, expected, 1)

    def test_scope_cannot_follow_other_paths_or_claim_admission(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            expected = self.scope(root)
            summary = json.loads((root / 'summary.json').read_text())
            summary['output_sha256'] = {'../unrelated.json': 'a' * 64}
            prep.write(root / 'summary.json', summary)
            with self.assertRaises(ValueError):
                prep.scope_packets(root, prep.sha(root / 'summary.json'), 1)
            self.packet['admission_authorized'] = True
            expected = self.scope(root)
            with self.assertRaises(ValueError):
                prep.scope_packets(root, expected, 1)

    @unittest.skipUnless(os.environ.get('OTERYN_NONCORPSE_PACKETS'), 'set pinned packet paths for real source test')
    def test_real_46_candidates_with_unchanged_authoring_apis(self):
        repo = Path(__file__).resolve().parents[2]
        packets = Path(os.environ['OTERYN_NONCORPSE_PACKETS'])
        native = Path(os.environ['OTERYN_NONCORPSE_NATIVE'])
        original = prep.sha(native)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            summary = prep.prepare(repo, Path(os.environ['OTERYN_CRYSTAL']), packets,
                                   prep.sha(packets / 'summary.json'), native, original, out)
            self.assertEqual({'AUTHORING_SCHEMA_VALID': 25, 'ROUTED_TO_ACCEPTED_OTHER_OWNER': 18,
                              'SOURCE_AUTHORING_UNRESOLVED': 3}, summary['counts'])
            self.assertEqual(46, len(summary['output_sha256']))
            for relative, digest in summary['output_sha256'].items():
                result = prep.read_verified(out / relative, digest)
                self.assertIs(False, result['admission_authorized'])
                if result['item']:
                    self.assertFalse(result['validation_errors'])
                    self.assertIn('sprite_atlas_not_admitted', result['report']['blockers'])
            self.assertEqual(original, prep.sha(native))


if __name__ == '__main__':
    unittest.main()
