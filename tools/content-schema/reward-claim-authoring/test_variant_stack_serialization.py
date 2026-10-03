"""Authoring totals are additive; source/raw execution and native holds stay distinct."""
import copy
import json
import unittest
from unittest.mock import patch
import reward_claim_authoring as plain
import reward_claim_variant_authoring as variants
import reward_claim_variant_migration as migration

class VariantStackSerializationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.items = plain.load_items()
        cls.evidence = json.loads(migration.metadata_path(plain.ROOT, migration.EVIDENCE).read_text())
        cls.packet = migration.derive_packet(plain.ROOT, cls.evidence)
        cls.records = variants.derive_definitions(cls.packet, cls.items, plain.stack_problem)
        # Compare the same source packet with the explicit adaptation disabled,
        # so this regression also runs after the generated content is committed.
        with patch.object(variants, 'stack_normalization_proof', return_value={'cases': []}):
            cls.old = variants.derive_definitions(cls.packet, cls.items, plain.stack_problem)

    def test_two_actual_totals_source_identity_and_native_boundary(self):
        changed = []
        for row, old in zip(self.records, self.old):
            d = row['definition']; variants.validator(plain.ROOT).validate(d)
            self.assertEqual(d['source_variant'], old['definition']['source_variant'])
            self.assertEqual(d['native_admission'], 'WAITING_IMPLEMENTATION')
            if row != old:
                changed.append(d)
                quantities = d['placements'][0]['reward']['items']
                item_id = '3450' if 'scrapper_bp' in d['identity']['key'] else '3031'
                self.assertEqual([q['count'] for q in quantities if q['item']['key'].endswith('.i'+item_id)], [100,100])
                self.assertEqual(d['readiness'], 'waiting_implementation')
                self.assertEqual(d['data_holds'], [])
        self.assertEqual(len(changed), 2)
        self.assertEqual(len(variants.native_lowering_errors(self.records)), 105)

    def test_every_actual_item_total_is_preserved(self):
        for row, old in zip(self.records, self.old):
            for p, before in zip(row['definition']['placements'], old['definition']['placements']):
                def totals(items):
                    out = {}
                    for q in items: out[q['item']['key']] = out.get(q['item']['key'], 0)+q['count']
                    return out
                self.assertEqual(totals(p['reward']['items']), totals(before['reward']['items']))

    def test_known_smaller_max_and_remainder(self):
        items = copy.deepcopy(self.items); key = 'oteryn:item.tibia.i3031'
        items[key]['semantics']['stack']['value']['stack_max']['value'] = 60
        reward = {'items':[{'item':items[key]['identity'], 'count':200}]}
        variants.serialized_item_stacks(reward, items, plain.stack_problem)
        self.assertEqual([q['count'] for q in reward['items']], [60,60,60,20])

    def test_unknown_invalid_nonmaterializable_maximum_is_not_split(self):
        key = 'oteryn:item.tibia.i3031'
        for change in ('unknown','invalid','not_admitted','contradiction'):
            items = copy.deepcopy(self.items); item = items[key]
            if change == 'unknown': item['semantics']['stack'] = {'state':'UNKNOWN'}
            elif change == 'invalid': item['semantics']['stack']['value']['stack_max']['value'] = 101
            elif change == 'not_admitted': item['materializable'] = False
            else: item['semantics']['stack']['value']['stackable']['value'] = False
            reward = {'items':[{'item':item['identity'], 'count':200}]}; before = copy.deepcopy(reward)
            variants.serialized_item_stacks(reward, items, plain.stack_problem)
            self.assertEqual(reward, before)
            self.assertTrue(plain.stack_problem(item,200))

    def test_nonstack_and_random_weights_never_split(self):
        items = self.items; key = 'oteryn:item.tibia.i3081'; stack = 'oteryn:item.tibia.i3031'
        reward = {'items':[{'item':items[key]['identity'], 'count':5}],
                  'random_one_of':[{'item':items[stack]['identity'], 'count':200},
                                   {'item':items[key]['identity'], 'count':1}]}
        before = copy.deepcopy(reward)
        variants.serialized_item_stacks(reward,items,plain.stack_problem)
        self.assertEqual(reward,before)

    def test_selected_source_or_stack_semantics_mutation_rejected(self):
        key = variants.stack_normalization_proof()['cases'][0]['source_identity']['key']
        for change in ('source', 'item'):
            packet, items = copy.deepcopy(self.packet), copy.deepcopy(self.items)
            if change == 'source': next(r for r in packet['records'] if r['source_claim']['identity']['key'] == key)['source_claim']['placements'][0]['reward']['items'][1]['count'] = 199
            else: items['oteryn:item.tibia.i3450']['semantics']['stack']['value']['stack_max']['value'] = 99
            with self.assertRaisesRegex(ValueError, 'normalization inputs changed'):
                variants.derive_definitions(packet, items, plain.stack_problem)


if __name__ == '__main__': unittest.main()
