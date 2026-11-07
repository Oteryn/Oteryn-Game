import copy
import unittest
from normalize_optional_core_packet import normalize


def packet():
    return {'patches': [{'monster': 'icicle', 'file': 'monster.json', 'pointer': '/behavior/defenses/-',
             'source': {'source_field': 'defenses[1]'},
             'value': {'ability': {'family': 'Ability', 'key': 'core', 'revision': 'r1'}}}],
            'restored_source_rows': [{'monster': 'icicle', 'source_field': 'defenses[1]',
             'target': {'family': 'Creature', 'key': 'dragon_egg', 'revision': 'r1'},
             'remaining_exact_parity_debt': 'HP mutation proxy; retargeting omitted.'}]}


class ReceiptNormalizationTests(unittest.TestCase):
    def test_local_loot_catalog_duplicate_removed_but_external_item_preserved(self):
        identity = {'key': 'canary:loot/heoni', 'revision': 'r1'}
        local = {'monster': 'heoni', 'file': 'monster.json', 'pointer': '/loot',
                 'value': {'identity': identity, 'entries': [{'item': 'existing', 'probability_percent': 0.2232}]}}
        redundant = {'monster': 'heoni', 'file': 'catalog.json', 'pointer': '/definitions/-',
                     'value': dict(identity, family='Loot')}
        external = dict(redundant, value={'family': 'Item', 'key': 'admitted', 'revision': 'r1'})
        before = {'patches': [local, redundant, external]}; saved = copy.deepcopy(before)
        after = normalize(before)
        self.assertEqual(before, saved)
        self.assertEqual(after['patches'], [local, external])
        self.assertEqual(after['redundant_local_loot_catalog_patches'], [redundant])

    def test_preserves_patches_named_target_and_original(self):
        before = packet(); saved = copy.deepcopy(before); after = normalize(before)
        self.assertEqual(before, saved)
        self.assertEqual(after['patches'], saved['patches'])
        row = after['restored_source_rows'][0]
        self.assertEqual(row['target'], saved['restored_source_rows'][0]['target'])
        self.assertEqual(row['limitation'], row['remaining_exact_parity_debt'])
        self.assertEqual(row['direct_typed_core'], saved['patches'][0]['value']['ability'])

    def test_unknown_or_ambiguous_receipt_fails_closed(self):
        for mutation in ('missing_debt', 'missing_schedule', 'duplicate_schedule'):
            value = packet()
            if mutation == 'missing_debt': value['restored_source_rows'][0].pop('remaining_exact_parity_debt')
            if mutation == 'missing_schedule': value['patches'] = []
            if mutation == 'duplicate_schedule': value['patches'] *= 2
            with self.subTest(mutation=mutation), self.assertRaises(ValueError): normalize(value)


if __name__ == '__main__': unittest.main()
