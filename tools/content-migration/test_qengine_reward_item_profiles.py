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
