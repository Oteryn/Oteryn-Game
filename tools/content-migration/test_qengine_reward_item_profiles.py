"""Reused physical qualification must retain existing value and schema owners."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import qengine_reward_item_profiles as profiles


class SharedPhysicalAdmission(unittest.TestCase):
    def record(self):
        return {
            'production_definition': {'production_key': 'oteryn:item.tibia.i11701'},
            'production_binding': {'source': 'exact'},
            'admission': {'materializable': True},
            'semantics': {
                'physical': {'state': 'KNOWN', 'value': {'weight': {'state': 'KNOWN', 'value': 3500}}},
                'stack': {'state': 'NOT_APPLICABLE'},
                'container': {'state': 'NOT_APPLICABLE'},
                'charges': {'state': 'NOT_APPLICABLE'},
            },
        }

    def run_admission(self, record, existing, schema='OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5'):
        generated = {'profiles': json.dumps({'records': [existing]}).encode(),
                     'manifest': json.dumps({'schema': schema, 'item_profiles': {}}).encode()}
        with tempfile.TemporaryDirectory() as directory, patch.object(profiles, 'capture', return_value={'records': [record]}):
            profiles.admit(Path(directory), generated, 'profiles', 'manifest',
                           lambda row: row['production_definition']['production_key'],
                           lambda value: json.dumps(value).encode())
        return generated

    def test_unknown_custody_completion_keeps_known_charge_policy(self):
        record = self.record()
        existing = copy.deepcopy(record)
        existing['semantics']['physical'] = {'state': 'UNKNOWN'}
        existing['semantics']['charges'] = {'state': 'KNOWN', 'value': {'full': 5}}
        generated = self.run_admission(record, existing)
        result = json.loads(generated['profiles'])['records'][0]
        self.assertEqual(result['semantics']['physical'], record['semantics']['physical'])
        self.assertEqual(result['semantics']['charges'], existing['semantics']['charges'])
        self.assertEqual(json.loads(generated['manifest'])['schema'], 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v5')
        self.assertNotIn('cosmetics', json.loads(generated['manifest']))

    def test_conflicting_known_weight_is_never_replaced(self):
        record = self.record()
        existing = copy.deepcopy(record)
        existing['semantics']['physical']['value']['weight']['value'] = 3499
        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_NATIVE_CONFLICT'):
            self.run_admission(record, existing)

    def test_newer_manifest_schema_is_preserved(self):
        record = self.record()
        generated = self.run_admission(record, copy.deepcopy(record), 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v7')
        self.assertEqual(json.loads(generated['manifest'])['schema'], 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v7')


if __name__ == '__main__':
    unittest.main()
