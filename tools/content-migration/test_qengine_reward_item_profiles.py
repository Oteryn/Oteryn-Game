import copy
import unittest

import qengine_reward_item_profiles as q


class NativeFactJoin(unittest.TestCase):
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

    def test_equipment_pattern_unknowns_refine_without_changing_identity(self):
        old = self.record()
        old['semantics']['equipment'] = {'state': 'KNOWN', 'value': {
            'patterns': {'state': 'KNOWN', 'value': [
                {'pattern_id': 1, 'primary_slot': {'state': 'KNOWN', 'value': 'TORSO'},
                 'level': {'state': 'UNKNOWN'}}]}}}
        new = copy.deepcopy(old)
        pattern = new['semantics']['equipment']['value']['patterns']['value'][0]
        pattern['level'] = {'state': 'KNOWN', 'value': 0}
        joined = q.reconcile(old, new)
        self.assertEqual(joined['semantics'], new['semantics'])
        self.assertEqual(old['semantics']['equipment']['value']['patterns']['value'][0]
                         ['level'], {'state': 'UNKNOWN'})
        for field, value in [('pattern_id', 2),
                             ('primary_slot', {'state': 'KNOWN', 'value': 'HEAD'})]:
            invalid = copy.deepcopy(new)
            invalid['semantics']['equipment']['value']['patterns']['value'][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                q.reconcile(old, invalid)
        known = copy.deepcopy(new)
        known['semantics']['equipment']['value']['patterns']['value'][0]['level']['value'] = 100
        with self.assertRaises(ValueError):
            q.reconcile(known, new)

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
