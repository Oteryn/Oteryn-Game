"""Concrete donor regressions and rejected identity/timer shortcuts; no content adoption."""
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from jsonschema import Draft202012Validator

import verify_spyrat_corpse_source_boundaries as proof


class SourceBoundaryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.report = proof.verify()
        path = proof.ROOT / 'tools/content-schema/monster-authoring/monster.schema.json'
        schema = json.loads(path.read_text())
        cls.temporal = Draft202012Validator({'$schema': schema['$schema'], '$defs': schema['$defs'],
                                               '$ref': '#/$defs/temporal'})

    def test_exact_donor_pipelines_compile_and_preserve_absent_target(self):
        for source in self.report['sources'].values():
            self.assertEqual(source['compiled_missing_target_witness']['result'],
                             'ABSENT_TARGET_RETURNS_ORIGINAL;VALID_TARGET_PASSES_GUARD')
            self.assertFalse(source['compiled_missing_target_witness']['runtime_engine_executed'])
            self.assertEqual(source['corpse_48267_xml_attributes']['duration'], '5')

    def test_forged_server_to_client_translation_is_rejected(self):
        for source in self.report['sources'].values():
            direct = source['outfit_wire']['text']
            self.assertTrue(proof.direct_object_wire(direct))
            changed = direct.replace('msg.add<uint16_t>(outfit.lookTypeEx);',
                                     'msg.add<uint16_t>(Item::items[outfit.lookTypeEx].clientId);')
            self.assertFalse(proof.direct_object_wire(changed))

    def test_numeric_spyrat_and_decay_targets_are_not_admitted_objects(self):
        for rows in self.report['appearance_membership'].values():
            for number in (30375, 30376, 30377, 30378, 48296):
                self.assertIsNone(rows[str(number)])
            self.assertIsNotNone(rows['48267'])
        self.assertEqual(self.report['sources']['canary']['candy_horror_declared_corpse'], 48267)
        self.assertEqual(self.report['sources']['crystal']['candy_horror_declared_corpse'], 48268)
        self.assertEqual(len(self.report['spyrat']['existing_registry_tombstones']), 4)
        self.assertTrue(all(row['state'] == 'RETIRED_WITHOUT_SUCCESSOR'
                            for row in self.report['spyrat']['existing_registry_tombstones']))

    def test_reversed_cpp_guard_cannot_pass_the_behavior_witness(self):
        function = self.report['sources']['canary']['item_transform_guard']['text']
        changed = function.replace('if (newType.id == 0)', 'if (newType.id != 0)')
        with self.assertRaises(subprocess.CalledProcessError):
            proof.compile_transform_witness(changed)

    def test_source_pin_rejects_a_changed_local_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source.cpp').write_bytes(b'forged declaration')
            with patch.object(proof.subprocess, 'check_output', return_value=b'pinned declaration'):
                with self.assertRaises(AssertionError):
                    proof.pinned(root, 'exact-revision', 'source.cpp')

    def test_existing_temporal_none_cannot_preserve_five_second_timer(self):
        existing = {'decay_action': 'none', 'stop_duration': False}
        self.assertFalse(list(self.temporal.iter_errors(existing)))
        proposed = {**existing, 'duration_ms': 5000}
        self.assertTrue(list(self.temporal.iter_errors(proposed)))

    def test_timer_removal_would_be_valid_shape_but_wrong_source_action(self):
        remove = {'decay_action': 'remove', 'stop_duration': False, 'duration_ms': 5000}
        self.assertFalse(list(self.temporal.iter_errors(remove)))
        self.assertIn('original corpse', self.report['candy_horror']['effective_donor_decay'])
        self.assertIsNone(self.report['spyrat']['accepted_normalization_candidate'])
        self.assertFalse(self.report['product_data_adopted'])


if __name__ == '__main__':
    unittest.main()
