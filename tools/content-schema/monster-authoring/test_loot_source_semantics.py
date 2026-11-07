"""Independent loot contract arithmetic and identity failure cases."""
import copy
import unittest
from decimal import Decimal, localcontext

import verify_loot_source_semantics as proof


class LootSourceSemanticsTests(unittest.TestCase):
    def test_d1_preserves_original_decimal_precision(self):
        for value in ('0.29', '0.290000', '2.9e-1'):
            self.assertEqual(proof.exact_ppm(Decimal(value)), 2900)
        for value in ('0.29000000001', '0.29000000000000000000000000000001', '-1', '100.0001'):
            with self.assertRaises(ValueError):
                proof.exact_ppm(Decimal(value))

    def test_d1_does_not_depend_on_decimal_context(self):
        with localcontext() as context:
            context.prec = 2
            self.assertEqual(proof.exact_ppm(Decimal('99.1234')), 991234)

    def test_half_even_ties_round_to_even_integer(self):
        self.assertEqual([proof.half_even(n, 2) for n in (1, 3, 5, 7)], [0, 2, 2, 4])
        self.assertEqual(proof.half_even(5, 3), 2)
        with self.assertRaises(ValueError):
            proof.half_even(1, 0)

    def test_zero_count_probability_and_retained_nonzero_count(self):
        self.assertEqual(proof.source_values({'chance': 100000, 'minCount': 0, 'maxCount': 1}), (1, 1, 500000))
        self.assertEqual(proof.source_values({'chance': 1, 'minCount': 0, 'maxCount': 3}), (1, 3, 8))
        self.assertEqual(proof.source_values({'chance': 1, 'minCount': 0, 'maxCount': 4}), (1, 4, 8))
        self.assertEqual(proof.source_values({'chance': 42, 'maxCount': 8}), (1, 8, 420))

    def test_guaranteed_source_chance_clamps_at_max_lootchance(self):
        self.assertEqual(proof.source_values({'chance': 200000}), (1, 1, 1000000))

    def test_observed_drops_replace_probability_without_zero_count_rescaling(self):
        source = {'chance': 100000, 'minCount': 0, 'maxCount': 3}
        chance = {'status': 'DIFF', 'confidence': 'estimate', 'times': 10, 'wiki_amount': '1-2'}
        self.assertEqual(proof.wiki_values(source, chance, 16), (1, 3, 625000, 'APPROVED_WIKI_ESTIMATE'))

    def test_sample_range_inside_source_range_is_not_narrowed(self):
        chance = {'status': 'CONSISTENT', 'confidence': 'estimate', 'times': 10, 'wiki_amount': '2-4'}
        self.assertEqual(proof.wiki_values({'chance': 100000, 'maxCount': 8}, chance, 20)[:2], (1, 8))
        chance['wiki_amount'] = '2-10'
        self.assertEqual(proof.wiki_values({'chance': 100000, 'maxCount': 8}, chance, 20)[:2], (2, 10))

    def test_uncertain_low_sample_and_split_entries_keep_source_values(self):
        for status, confidence, times in [('DIFF', 'low_confidence', 9), ('MULTI_ENTRY', None, 100),
                                          ('WIKI_UNCERTAIN', None, 100), ('NOT_OBSERVED', None, 0),
                                          ('INVALID_STATISTICS', None, 101)]:
            chance = {'status': status, 'confidence': confidence, 'times': times, 'wiki_amount': '9-99'}
            self.assertEqual(proof.wiki_values({'chance': 290, 'maxCount': 4}, chance, 100)[:3], (1, 4, 2900))

    def test_item_id_is_literal_and_source_name_has_registrar_priority(self):
        names = {'gold': {3031}, 'ambiguous': {1, 2}}
        self.assertEqual(proof.source_item({'id': 3031}, names), 3031)
        self.assertEqual(proof.source_item({'id': 99, 'name': 'GOLD'}, names), 3031)
        for name in ('ambiguous', 'absent'):
            with self.assertRaises(ValueError):
                proof.source_item({'name': name}, names)

    def test_wiki_identity_uses_exact_page_ids_and_equal_split(self):
        item = {'name': 'pearl', 'item_page': {'item_ids': [281, 282]}}
        self.assertEqual(proof.wiki_items(item, 'Demon', {281: 'pearl', 282: 'pearl'}, {'pearl': {281, 282}}, {}), {281, 282})
        self.assertEqual(proof.half_even(1000000, 3 * 2), 166667)

    def test_equipped_state_and_explicit_dropped_by_variant(self):
        item = {'name': 'armor', 'item_page': {'item_ids': [1, 2]}}
        self.assertEqual(proof.wiki_items(item, 'Demon', {1: 'armor', 2: 'armor'}, {'armor': {1, 2}},
                                         {1: {'transformequipto': '2'}}), {1})
        item['item_page']['variants'] = [{'item_ids': [1], 'dropped_by': ['demon']},
                                        {'item_ids': [2], 'dropped_by': []}]
        self.assertEqual(proof.wiki_items(item, 'Demon', {1: 'armor', 2: 'armor'}, {}, {}), {1})

    def test_unbound_wiki_identity_is_explicit_failure(self):
        with self.assertRaises(ValueError):
            proof.wiki_items({'name': 'unknown'}, 'Demon', {}, {}, {})

    def native_case(self):
        source = [{'item': {'key': 'canary:item/3031'}, 'min_count': 1, 'max_count': 4,
                   'probability_percent': Decimal('0.29'), 'skip_later_same_item_after_success': False}]
        binding = {'source_key': 'oteryn:source.canary', 'source_revision': 'pinned-sha',
                   'identity_namespace': 'canary/monster-file', 'external_id': 'mammals/rat'}
        creature = {'family': 'Creature', 'key': 'creature.rat', 'revision': 'r1'}
        loot = {'family': 'Loot', 'key': 'loot.rat', 'revision': 'r1'}
        item = {'family': 'Item', 'key': 'oteryn:item.currency.gold', 'revision': 'r1'}
        stage = {'source_identity_bindings': [{**binding, 'disposition': 'EXACT', 'target': creature}],
                 'records': [{'kind': 'Creature', 'identity': creature, 'loot': loot},
                             {'kind': 'Loot', 'identity': loot, 'entries': [
                                 {'item': item, 'min_count': 1, 'max_count': 4, 'probability_ppm': 2900}]}],
                 'authoring_profiles': []}
        mapping = {'records': [{'source_item_id': 3031, 'native_key': item['key'], 'native_revision': 'r1'}]}
        return stage, mapping, [], binding, source

    def test_native_projection_requires_exact_canonical_item_and_revision(self):
        case = self.native_case()
        self.assertTrue(proof.native_projection(*case)['status'].startswith('PROVEN'))
        case[0]['records'][1]['entries'][0]['item']['revision'] = 'r2'
        self.assertEqual(proof.native_projection(*case)['status'], 'MISMATCH_NATIVE_PROJECTION')

    def test_native_binding_checks_all_source_coordinates(self):
        for field in ('source_key', 'source_revision', 'identity_namespace', 'external_id'):
            with self.subTest(field=field):
                case = self.native_case()
                case[0]['source_identity_bindings'][0][field] = 'wrong-source'
                self.assertEqual(proof.native_projection(*case)['status'], 'UNKNOWN_NOT_ADMITTED_OR_UNBOUND_SOURCE_ROOT')

    def test_native_creature_and_loot_match_full_identity(self):
        for record_index, field in ((0, 'family'), (0, 'revision'), (1, 'family'), (1, 'revision')):
            with self.subTest(record=record_index, field=field):
                case = self.native_case()
                case[0]['records'][record_index]['identity'] = {**case[0]['records'][record_index]['identity'], field: 'wrong'}
                self.assertFalse(proof.native_projection(*case)['status'].startswith('PROVEN'))
        for field in ('family', 'revision'):
            case = self.native_case()
            case[0]['records'][0]['loot'] = {**case[0]['records'][0]['loot'], field: 'wrong'}
            self.assertFalse(proof.native_projection(*case)['status'].startswith('PROVEN'))

    def test_native_duplicate_records_and_wrong_profile_revision_are_not_proven(self):
        case = self.native_case()
        case[0]['records'].append(copy.deepcopy(case[0]['records'][1]))
        self.assertFalse(proof.native_projection(*case)['status'].startswith('PROVEN'))
        case = self.native_case()
        case[0]['authoring_profiles'] = [{'target': {'family': 'Loot', 'key': 'loot.rat', 'revision': 'wrong'},
            'data': {'kind': 'Loot', 'profile': {'entries': [{'skip_later_same_item_after_success': False}]}}}]
        self.assertEqual(proof.native_projection(*case)['status'], 'MISMATCH_LOOT_PROFILE_IDENTITY')

    def test_protected_rekey_requires_exact_before_binding(self):
        mapping = {'records': [{'source_item_id': 3031, 'native_key': 'opaque', 'native_revision': 'r1'}]}
        fact = {'source_identity': {'source_item_id': 3031, 'current_native_key': 'wrong',
                                    'target_native_key': 'gold', 'target_revision': 'r1'}}
        with self.assertRaises(ValueError):
            proof.native_projection({}, mapping, [fact], 'any', [])


if __name__ == '__main__':
    unittest.main()
