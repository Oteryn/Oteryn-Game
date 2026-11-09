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
