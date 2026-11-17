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
import json
import unittest

import qengine_reward_item_profiles as q


class NativeFactJoin(unittest.TestCase):
    def test_slug_corpse_retains_source_ground_custody_and_complete_decay_chain(self):
        records = {r['production_binding']['external_id']: r for r in q.capture()['records']}
        chain = (12526, 12533, 12534, 12535)
        for stage, (item, duration) in enumerate(zip(chain, (10000, 300000, 300000, 60000))):
            with self.subTest(item=item):
                self.assertIn(str(item), set(records), 'real Slug death needs its native corpse chain')
                record = records[str(item)]
                self.assertEqual(record['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['Ground']))
                temporal = record['semantics']['temporal']['value']
                self.assertEqual(temporal['duration']['value'], duration)
                self.assertEqual(temporal['stop_duration']['value'], False)
                target = temporal['decay_target']
                if stage < 3:
                    self.assertEqual(target['value'], dict(key=f'oteryn:item.tibia.i{chain[stage+1]}', revision='definition-r1'))
                else:
                    self.assertEqual(target, dict(state='NOT_APPLICABLE'))

    def test_m09_flask_matches_source_and_runtime_custody(self):
        key = 'oteryn:item.tibia.i12785'
        observed = next(r for r in q.capture()['records']
                        if r['production_definition']['production_key'] == key)
        document = json.loads((q.ROOT/q.PROFILE).read_bytes())
        admitted = next(r for r in document['records']
                        if r['production_definition']['production_key'] == key)
        self.assertEqual(admitted['production_definition'], observed['production_definition'])
        self.assertEqual(admitted['admission'], observed['admission'])
        self.assertEqual(admitted['semantics']['physical'], observed['semantics']['physical'])
        self.assertEqual(admitted['semantics']['physical']['value']['weight'],
                         {'state': 'KNOWN', 'value': 180})
        self.assertEqual(admitted['admission']['stack_class'], 'NonStackable')
        for field in ('charges', 'temporal', 'container'):
            self.assertEqual(admitted['semantics'][field]['state'], 'NOT_APPLICABLE')


class MainNativeFactJoin(unittest.TestCase):
    def record(self):
        return dict(production_definition={'production_key': 'item'},
                    production_binding={'exact': True}, admission={'materializable': True},
                    authoring={'item': 'established'}, attributes={'blocks': None},
                    semantics={'charges': {'state': 'UNKNOWN'}})

    def test_m44_flower_and_holy_water_keep_exact_portable_source_facts(self):
        rows = {r['production_definition']['production_key']: r
                for r in q.capture(ids={21389, 21466})['records']}
        for item, weight in ((21389, 100), (21466, 1750)):
            row = rows[f'oteryn:item.tibia.i{item}']
            self.assertEqual(row['production_binding']['external_id'], str(item))
            self.assertEqual(row['admission'], dict(materializable=True,
                stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground']))
            self.assertEqual(row['semantics']['physical']['value']['weight']['value'], weight)
        temporal = rows['oteryn:item.tibia.i21389']['semantics']['temporal']['value']
        self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': 3600000})
        self.assertEqual(temporal['decay_target'], {'state': 'NOT_APPLICABLE'})

    def test_daily_roots_source_profile_is_portable_and_bounded(self):
        records = q.capture()['records']
        record = next((r for r in records if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21291'), None)
        self.assertIsNotNone(record, 'Oramond harvest needs an admitted existing Juicy Roots identity')
        self.assertEqual(record['production_binding']['external_id'], '21291')
        self.assertEqual(record['admission'], dict(materializable=True, stack_class='StackCapable', legal_destinations=['CharacterInventory', 'Ground']))
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 200)
        self.assertEqual(record['semantics']['stack']['value']['stack_max']['value'], 100)

    def test_m56_wax_is_an_exact_portable_non_stackable_source(self):
        records = q.capture()['records']
        record = next((r for r in records if r['production_definition']['production_key'] == 'oteryn:item.tibia.i9106'), None)
        self.assertIsNotNone(record, 'M56 requires the real selected Molten Wax instance')
        self.assertEqual(record['production_binding']['external_id'], '9106')
        self.assertEqual(record['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground']))
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 50)

    def test_arena_goblets_promote_only_pinned_portable_custody(self):
        import json
        existing = {r['production_definition']['production_key']: r
                    for r in json.loads((q.ROOT / q.PROFILE).read_text())['records']}
        packet = q.capture()
        observed = {r['production_definition']['production_key']: r for r in packet['custody_promotions']}
        self.assertEqual(len(observed), 2)
        for item in (5805, 5806):
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(key in observed, 'native arena trophies need pinned portable profiles')
            old = copy.deepcopy(existing[key])
            old['admission'] = dict(materializable=False, stack_class='NonStackable', legal_destinations=['Ground'])
            joined = q.reconcile(old, observed[key])
            self.assertTrue(joined['admission']['materializable'])
            self.assertEqual(joined['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
            self.assertEqual(joined['production_binding'], old['production_binding'])
            self.assertFalse(old['admission']['materializable'])
            wrong = copy.deepcopy(observed[key])
            wrong['admission']['legal_destinations'].append('Equipment')
            with self.assertRaises(ValueError):
                q.reconcile(old, wrong)

    def test_goblin_tools_preserve_their_source_identity_and_portable_facts(self):
        records = {r['production_binding']['external_id']: r for r in q.capture()['records']}
        for item in (17669, 17670, 17672, 17673, 17674, 17675):
            with self.subTest(item=item):
                self.assertIn(str(item), set(records), 'carried quest tools need native physical admission')
                record = records[str(item)]
                self.assertEqual(record['production_definition']['production_key'], f'oteryn:item.tibia.i{item}')
                self.assertTrue(record['admission']['materializable'])
                self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
                self.assertEqual(record['semantics']['physical']['state'], 'KNOWN')
                self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 800 if item == 17669 else 350)

    def test_lion_holy_water_source_profile_has_exact_portable_custody(self):
        record = next((r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21466'), None)
        self.assertIsNotNone(record, 'Lion Tears needs its exact holy-water vessel')
        self.assertEqual(record['production_binding']['external_id'], '21466')
        self.assertTrue(record['admission']['materializable'])
        self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 1750)

    def test_lion_flower_preserves_source_duration_as_absolute_deadline(self):
        """Crystal 201189f8 data/items/items.xml:42586-42590 sets duration=3600."""
        record = next((r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21389'), None)
        self.assertIsNotNone(record, 'Lion Beauty requires its exact portable flower')
        self.assertEqual(record['production_binding']['external_id'], '21389')
        self.assertTrue(record['admission']['materializable'])
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 100)
        temporal = record['semantics']['temporal']['value']
        self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': 3600000})
        self.assertEqual(temporal['consumption_mode'],
                         {'state': 'KNOWN', 'value': 'DURABLE_ABSOLUTE_DEADLINE'})
        self.assertEqual(temporal['decay_target'], {'state': 'NOT_APPLICABLE'})

    def test_lion_flower_exception_is_finite_and_rejects_new_instance_mechanisms(self):
        attributes = {'duration': '3600', 'weight': '100', 'description': 'source'}
        self.assertTrue(q.lion_flower.admitted(21389, attributes))
        self.assertFalse(q.lion_flower.admitted(21466, attributes))
        for field, value in [('duration', '3601'), ('weight', '101'), ('charges', '1'),
                             ('decayto', '0'), ('stopduration', '1'), ('transformto', '1')]:
            self.assertFalse(q.lion_flower.admitted(21389, dict(attributes, **{field: value})), field)

    def test_m07_realm_key_and_repair_kit_have_exact_portable_source_facts(self):
        records = {r['production_definition']['production_key']: r for r in q.capture()['records']}
        for item, weight in [(14682, 100), (7785, 1800)]:
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(key in records, f'assigned Realm/repair mechanic needs {key}')
            record = records[key]
            self.assertEqual(record['production_binding']['external_id'], str(item))
            self.assertTrue(record['admission']['materializable'])
            self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
            self.assertEqual(record['semantics']['physical']['value']['weight']['value'], weight)

    def test_m07_fire_bug_has_exact_portable_source_custody(self):
        record = next((r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i5467'), None)
        self.assertIsNotNone(record, 'Machinery burn actions require their exact carried Fire Bug')
        self.assertEqual(record['production_binding']['external_id'], '5467')
        self.assertTrue(record['admission']['materializable'])
        self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 3050)

    def test_m07_too_hot_tools_have_exact_portable_source_custody(self):
        records = {r['production_definition']['production_key']: r for r in q.capture()['records']}
        for item in (36543, 36544, 36545):
            key = f'oteryn:item.tibia.i{item}'
            self.assertTrue(key in records, f'Too Hot native interactions require exact tool {key}')
            record = records[key]
            self.assertEqual(record['production_binding']['external_id'], str(item))
            self.assertTrue(record['admission']['materializable'])
            self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
            self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 850)

    def test_fanfare_copper_key_has_native_portable_nonstackable_facts(self):
        records = q.capture()['records']
        record = next((r for r in records if r['production_definition']['production_key'] == 'oteryn:item.tibia.i2973'), None)
        self.assertIsNotNone(record, 'Fanfare key 3520 needs its exact physical Item owner')
        self.assertEqual(record['production_binding']['external_id'], '2973')
        self.assertEqual(record['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground']))

    def test_grimvale_blossoms_have_native_bounded_stack_facts(self):
        record = next((r for r in q.capture()['records'] if r['production_definition']['production_key'] == 'oteryn:item.tibia.i22082'), None)
        self.assertIsNotNone(record, 'Grimvale needs the source-defined harvested blossoms')
        self.assertEqual(record['admission'], dict(materializable=True, stack_class='StackCapable', legal_destinations=['CharacterInventory','Ground']))
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 200)
        self.assertEqual(record['semantics']['stack']['value']['stack_max']['value'], 100)

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



class SealPaintAdmission(unittest.TestCase):
    def test_source_paint_is_portable_nonstackable_and_keeps_its_weight(self):
        records = q.capture()['records']
        paint = next((r for r in records if r['production_binding']['external_id'] == '7253'), None)
        self.assertIsNotNone(paint)
        self.assertEqual(paint['admission'], dict(materializable=True, stack_class='NonStackable', legal_destinations=['CharacterInventory', 'Ground']))
        self.assertEqual(paint['semantics']['physical']['value']['weight']['value'], 180)
        self.assertEqual(paint['semantics']['temporal'], dict(state='NOT_APPLICABLE'))

if __name__ == '__main__':
    unittest.main()
