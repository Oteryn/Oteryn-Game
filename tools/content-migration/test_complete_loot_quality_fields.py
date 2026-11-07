import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
import complete_loot_quality_fields as loot


class LootQualityTests(unittest.TestCase):
    def test_wiki_item_variant_does_not_choose_an_arbitrary_id(self):
        page = {'page_title': 'Pearl', 'fields': {'itemid': '100, 101'}}
        numbers, _, kind = loot.resolve_item('Pearl', {'pearl': page}, {}, {100, 101})
        self.assertEqual(numbers, {100, 101})
        self.assertEqual(kind, 'WIKI_ITEMID')

    def test_unadmitted_wiki_item_is_not_allocated(self):
        page = {'page_title': 'Pearl', 'fields': {'itemid': '777'}}
        numbers, _, _ = loot.resolve_item('Pearl', {'pearl': page}, {}, {100})
        self.assertEqual(numbers, set())

    def test_ambiguous_donor_name_is_unresolved(self):
        numbers, _, kind = loot.resolve_item('Book', {}, {'book': {100, 101}}, {100, 101})
        self.assertEqual(numbers, set())
        self.assertEqual(kind, 'UNRESOLVED_ITEM_ID')

    def test_zero_quantity_means_no_drop_not_zero_sized_item(self):
        observation, = loot.observations({'lootcomum': '0-4 [[Gold Coin]]s.'})
        self.assertEqual(observation[2], (1, 4))
        self.assertIn('NO_DROP_ZERO', observation[3])

    def test_first_time_quest_grant_is_marked_conditional(self):
        values = list(loot.observations({'lootcomum': '[[Idol]] (apenas na primeira vez), [[Gold Coin]].'}))
        self.assertTrue(values[0][4])
        self.assertFalse(values[1][4])

    def test_small_bucket_cannot_produce_a_balance_rate(self):
        row = {'monster': 'rat', 'loot_fields_raw': json.dumps({'lootraro': '[[Gem]].'})}
        monsters = {'rat': {'loot': {'entries': [{'item': {'key': 'canary:item/1'}, 'probability_percent': 1.0}]}}}
        model = loot.calibrate([row] * 19, monsters, {1: 'gem'}, {'gem': {1}})
        self.assertIsNone(model['buckets']['lootraro']['estimated_ppm'])

    def test_median_does_not_follow_one_extreme_drop_rate(self):
        rows, monsters = [], {}
        for i in range(21):
            name = str(i)
            rows.append({'monster': name, 'loot_fields_raw': json.dumps({'lootraro': '[[Gem]].'})})
            monsters[name] = {'loot': {'entries': [{'item': {'key': 'canary:item/1'},
                'probability_percent': 100 if i == 20 else 1}]}}
        model = loot.calibrate(rows, monsters, {1: 'gem'}, {'gem': {1}})
        self.assertEqual(model['buckets']['lootraro']['estimated_ppm'], 10000)
        self.assertFalse(model['global_parity'])

    def test_changed_source_bytes_cannot_resolve_an_item(self):
        page = {'page_title': 'Gem', 'content': 'changed', 'content_sha256': '0' * 64}
        with self.assertRaisesRegex(ValueError, 'digest'):
            loot.item_index({'pages': [page]})

    def test_output_cannot_be_created_inside_source_population(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'input population'):
                loot.make_packet(root, root, root, root, root, root / 'new-output')


if __name__ == '__main__':
    unittest.main()
