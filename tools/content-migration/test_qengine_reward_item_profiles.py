"""Reused physical qualification must retain existing value and schema owners."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import qengine_reward_item_profiles as profiles


class SharedPhysicalAdmission(unittest.TestCase):
    def test_capture_keeps_the_mining_owned_pick_speed_unknown(self):
        native=json.loads((profiles.ROOT/profiles.PROFILE).read_bytes())
        key='oteryn:item.tibia.i3456'
        existing=next(row for row in native['records'] if row['production_definition']['production_key']==key)
        self.assertNotIn('speed_bonus',existing['attributes'])
        packet=profiles.capture()
        captured=next(row for row in packet['records'] if row['production_definition']['production_key']==key)
        self.assertEqual(captured['attributes'],existing['attributes'])
        self.assertEqual(captured['semantics'],existing['semantics'])

    def record(self):
        return {
            'production_definition': {'production_key': 'oteryn:item.tibia.i11701'},
            'production_binding': {'source': 'exact'},
            'admission': {'materializable': True, 'stack_class': 'NonStackable',
                          'legal_destinations': ['CharacterInventory', 'Ground']},
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

    def test_custody_completion_keeps_existing_unknown_speed(self):
        record = self.record()
        record['attributes'] = {'speed_bonus': 0}
        existing = copy.deepcopy(record)
        existing['attributes'] = {}
        result = self.run_admission(record, existing)
        admitted = json.loads(result['profiles'])['records'][0]
        self.assertEqual(admitted['attributes'], {})
        self.assertEqual(record['attributes'], {'speed_bonus': 0})

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


    def test_purified_soul_retains_only_the_exact_canonical_weight_pair(self):
        definition={'semantics':{'physical':{'value':{'weight':{'state':'KNOWN','value':20}}}}}
        record=self.record();record['semantics']['physical']['value']['weight']['value']=250
        profiles.reconcile_canonical_physical(32228,definition,record)
        self.assertEqual(record['semantics']['physical']['value']['weight'],{'state':'KNOWN','value':20})
        for canonical,donor in [(21,250),(20,249),(20,20)]:
            changed=copy.deepcopy(definition);changed['semantics']['physical']['value']['weight']['value']=canonical
            candidate=self.record();candidate['semantics']['physical']['value']['weight']['value']=donor
            with self.assertRaisesRegex(ValueError,'WEIGHT_DECISION_DRIFT'):
                profiles.reconcile_canonical_physical(32228,changed,candidate)
        other=self.record();before=copy.deepcopy(other)
        profiles.reconcile_canonical_physical(11701,definition,other)
        self.assertEqual(other,before)


    def test_only_exact_existing_purified_soul_weight_is_reconciled(self):
        target=self.record();target['production_definition']['production_key']='oteryn:item.tibia.i32228'
        target['semantics']['physical']['value']['weight']['value']=20
        for before in [250,20]:
            existing=copy.deepcopy(target);existing['semantics']['physical']['value']['weight']['value']=before
            result=self.run_admission(target,existing)
            self.assertEqual(json.loads(result['profiles'])['records'][0]['semantics']['physical']['value']['weight']['value'],20)
        existing=copy.deepcopy(target);existing['semantics']['physical']['value']['weight']['value']=249
        with self.assertRaisesRegex(ValueError,'ACTIVE_WEIGHT_DRIFT'):self.run_admission(target,existing)

import copy
import unittest

import qengine_reward_item_profiles as q


class MainNativeFactJoin(unittest.TestCase):
    def record(self):
        return dict(production_definition={'production_key': 'item'},
                    production_binding={'exact': True}, admission={'materializable': True},
                    authoring={'item': 'established'}, attributes={'blocks': None},
                    semantics={'charges': {'state': 'UNKNOWN'}})

    def test_source_can_fill_unknown_without_changing_owner(self):
        old = self.record()
        new = copy.deepcopy(old)
        new['semantics']['charges'] = {'state': 'NOT_APPLICABLE'}
        new['attributes']['blocks'] = False
        new['authoring'] = {'item': 'candidate'}
        joined = q.reconcile(old, new)
        self.assertEqual(joined['authoring'], old['authoring'])
        self.assertEqual(joined['semantics'], new['semantics'])
        self.assertEqual(joined['attributes'], new['attributes'])
        self.assertEqual(old, self.record())

    def test_known_identity_custody_and_physical_conflicts_fail(self):
        old = self.record()
        old['semantics']['charges'] = {'state': 'NOT_APPLICABLE'}
        old['attributes']['blocks'] = False
        for field in ('production_definition', 'production_binding', 'admission', 'semantics', 'attributes'):
            new = copy.deepcopy(old)
            if field == 'semantics':
                new[field]['charges'] = {'state': 'KNOWN', 'value': 1}
            elif field == 'attributes':
                new[field]['blocks'] = True
            else:
                new[field]['different'] = True
            with self.assertRaises(ValueError, msg=field):
                q.reconcile(old, new)


if __name__ == '__main__':
    unittest.main()
