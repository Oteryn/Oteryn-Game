import copy
import unittest

import qengine_reward_item_profiles as q


class NativeFactJoin(unittest.TestCase):
    def record(self):
        return dict(production_definition={'production_key': 'item'},
                    production_binding={'exact': True}, admission={'materializable': True},
                    authoring={'item': 'established'}, attributes={'blocks': None},
                    semantics={'charges': {'state': 'UNKNOWN'}})

    def test_daily_roots_source_profile_is_portable_and_bounded(self):
        records = q.capture()['records']
        record = next((r for r in records if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21291'), None)
        self.assertIsNotNone(record, 'Oramond harvest needs an admitted existing Juicy Roots identity')
        self.assertEqual(record['production_binding']['external_id'], '21291')
        self.assertEqual(record['admission'], dict(materializable=True, stack_class='StackCapable', legal_destinations=['CharacterInventory', 'Ground']))
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 200)
        self.assertEqual(record['semantics']['stack']['value']['stack_max']['value'], 100)

    def test_lion_holy_water_source_profile_has_exact_portable_custody(self):
        record = next((r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21466'), None)
        self.assertIsNotNone(record, 'Lion Tears needs its exact holy-water vessel')
        self.assertEqual(record['production_binding']['external_id'], '21466')
        self.assertTrue(record['admission']['materializable'])
        self.assertEqual(record['admission']['legal_destinations'], ['CharacterInventory', 'Ground'])
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 1750)

    def test_lion_flower_preserves_inactive_source_duration_without_inventing_expiry(self):
        record = next((r for r in q.capture()['records']
                       if r['production_definition']['production_key'] == 'oteryn:item.tibia.i21389'), None)
        self.assertIsNotNone(record, 'Lion Beauty requires its exact portable flower')
        self.assertEqual(record['production_binding']['external_id'], '21389')
        self.assertTrue(record['admission']['materializable'])
        self.assertEqual(record['semantics']['physical']['value']['weight']['value'], 100)
        temporal = record['semantics']['temporal']['value']
        self.assertEqual(temporal['duration'], {'state': 'KNOWN', 'value': 3600000})
        self.assertEqual(temporal['consumption_mode'], {'state': 'NOT_APPLICABLE'})
        self.assertEqual(temporal['decay_target'], {'state': 'NOT_APPLICABLE'})

    def test_lion_flower_exception_is_finite_and_rejects_new_instance_mechanisms(self):
        attributes = {'duration': '3600', 'weight': '100', 'description': 'source'}
        self.assertTrue(q.lion_flower.admitted(21389, attributes))
        self.assertFalse(q.lion_flower.admitted(21466, attributes))
        for field, value in [('duration', '3601'), ('weight', '101'), ('charges', '1'),
                             ('decayto', '0'), ('stopduration', '1'), ('transformto', '1')]:
            self.assertFalse(q.lion_flower.admitted(21389, dict(attributes, **{field: value})), field)

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
