import copy
import unittest

import qengine_reward_item_profiles as q


class NativeFactJoin(unittest.TestCase):
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
