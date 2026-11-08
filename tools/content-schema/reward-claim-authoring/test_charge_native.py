"""Exact source charge arguments must survive native reward admission."""
import copy
import json
import unittest

import charge_native
import reward_claim_authoring as plain


class ChargeNativeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        # Definition admission is independently proved by item-semantics tests.
        for number in (3048, 3081):
            cls.items[f'oteryn:item.tibia.i{number}']['materializable'] = True
        cls.rows = {r['definition']['identity']['key']: r['definition']
                    for path in (plain.ROOT / plain.CONTENT_DIR).glob('*.json')
                    for r in json.loads(path.read_text()).get('records', [])}

    def test_every_partial_claim_keeps_one_charge_and_all_siblings(self):
        partial = {key: spec for key, spec in charge_native.CASES.items()
                   if key != 'oteryn:reward-claim.kv.ornamentedshield'}
        for key, spec in partial.items():
            definition = self.rows[key]
            before = copy.deepcopy(definition)
            native = charge_native.derive(definition, self.items, plain.stack_problem)
            self.assertIsNotNone(native, key)
            placement, = native['placements']
            self.assertEqual(placement['unique_id'], spec['uid'])
            if spec['container'] is None:
                self.assertEqual(placement['root_charges_remaining'], 1)
                self.assertEqual(placement['contents'], [])
            else:
                self.assertEqual(len(placement['contents']), len(spec['reward']))
                self.assertEqual([(i, child['charges_remaining'])
                                  for i, child in enumerate(placement['contents'])
                                  if 'charges_remaining' in child],
                                 [(i, 1) for i in spec['charged']])
            self.assertEqual(definition, before, 'retain historical SOURCE holds')

    def test_ornamented_full_default_has_all_siblings_and_no_partial_override(self):
        definition = self.rows['oteryn:reward-claim.kv.ornamentedshield']
        before = copy.deepcopy(definition)
        self.assertIsNone(charge_native.derive(definition, self.items, plain.stack_problem))
        native = charge_native.containers.derive(definition, self.items, plain.stack_problem)
        self.assertIsNotNone(native)
        placement, = native['placements']
        self.assertEqual(placement['unique_id'], 6063)
        self.assertEqual([(child['item']['key'], child['count']) for child in placement['contents']],
                         [(f'oteryn:item.tibia.i{item}', count) for item, count in
                          [(2949, 1), (3059, 1), (3083, 1), (3035, 5), (3053, 1)]])
        self.assertNotIn('charges_remaining', placement['contents'][2])
        self.assertEqual(self.items['oteryn:item.tibia.i3083']['semantics']['charges']['value']['count']['value'], 150)
        self.assertEqual(definition, before)
        changed = copy.deepcopy(definition)
        changed['source_variant']['charge_dispositions'][0]['source_raw_argument'] = 2
        self.assertIsNone(charge_native.containers.derive(changed, self.items, plain.stack_problem))

    def test_partial_admission_fails_closed_on_changed_source_or_sibling(self):
        for key, spec in charge_native.CASES.items():
            for mutation in ('position', 'uid', 'count', 'identity', 'default', 'hold', 'sibling'):
                definition = copy.deepcopy(self.rows[key])
                items = copy.deepcopy(self.items)
                placement = definition['placements'][0]
                index = next(iter(spec['charged']))
                item_key = placement['reward']['items'][index]['item']['key']
                if mutation == 'position':
                    placement['source_binding']['project_position']['x'] += 1
                elif mutation == 'uid':
                    placement['source_binding']['legacy_unique_ids'][-1]['unique_id'] += 1
                elif mutation == 'count':
                    placement['reward']['items'][index]['count'] = 2
                elif mutation == 'identity':
                    definition['identity']['key'] += '.invented'
                elif mutation == 'default':
                    items[item_key]['semantics']['charges']['value']['count']['value'] += 1
                elif mutation == 'hold':
                    definition.setdefault('data_holds', []).append({'code': 'UNRESOLVED'})
                else:
                    sibling = placement['reward']['items'][0]['item']['key']
                    items[sibling]['materializable'] = False
                with self.subTest(key=key, mutation=mutation):
                    derive = (charge_native.containers.derive
                              if key == 'oteryn:reward-claim.kv.ornamentedshield'
                              else charge_native.derive)
                    self.assertIsNone(derive(definition, items, plain.stack_problem))


if __name__ == '__main__':
    unittest.main()
