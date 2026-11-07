import copy
import unittest
from unittest.mock import patch
import verify_item_delivery_reference as delivery


class ItemDeliveryReferenceTests(unittest.TestCase):
    def test_item_metadata_does_not_claim_native_or_client_admission(self):
        data = delivery.build_reference()
        self.assertFalse(data['runtime_admitted'])
        self.assertFalse(data['redistribution_authorized'])
        self.assertEqual(len(data['items']), 18)
        self.assertTrue(all(not row['native_materializable_observed']
                            and row['native_stack_class_observed'] == 'Unknown'
                            for row in data['items'].values()))
        gems = [row for row in data['items'].values()
                if row['client_flags_observed'].get('flags.skillwheel_gem')]
        self.assertEqual(len(gems), 15)
        self.assertEqual({(row['client_flags_observed']['skillwheel_gem.vocation_id'],
                           row['client_flags_observed']['skillwheel_gem.gem_quality_id'])
                          for row in gems}, {(v, q) for v in range(5) for q in range(3)})
        self.assertEqual(sum(row['client_flags_observed'].get('flags.cumulative', False)
                             for row in data['items'].values()), 17)
        self.assertEqual(data['native_trade_coverage']['item_keys_with_declarations'], 18)
        for key, row in data['items'].items():
            declarations = {(offer['direction'], offer['unit_price'])
                            for offer in row['native_trade_declarations_observed']}
            flags = row['client_flags_observed']
            if flags.get('flags.skillwheel_gem'):
                expected = {('BuyFromPlayer', (2500, 5000, 10000)[flags['skillwheel_gem.gem_quality_id']])}
            else:
                expected = {('SellToPlayer', {'oteryn:item.tibia.i46625': 750000,
                                             'oteryn:item.tibia.i46626': 2250000,
                                             'oteryn:item.tibia.i46627': 500}[key])}
            self.assertEqual(declarations, expected)
        coverage = data['client_icon_coverage']
        self.assertEqual([coverage[x] for x in ('reference_crops', 'bindings',
                                              'pixel_equal_fallback_crops',
                                              'blocked_different_fallback_crops')],
                         [205, 520, 189, 16])

    def test_missing_native_identity_is_rejected(self):
        original = delivery.read
        def missing(path):
            value = original(path)
            if path.name == 'items-25500-25999.json':
                value['records'] = [row for row in value['records']
                                     if row['definition']['identity']['key'] != 'oteryn:item.tibia.i44602']
            return value
        with patch.object(delivery, 'read', side_effect=missing):
            with self.assertRaisesRegex(ValueError, 'ITEM_DELIVERY_NATIVE_KEYS'):
                delivery.build_reference()

    def test_icon_coverage_cannot_reuse_drifting_manifest(self):
        original = delivery.read
        def drift(path):
            value = original(path)
            if path.name == 'client-icon-manifest.json':
                value = copy.deepcopy(value)
                value['bindings'].pop(next(iter(value['bindings'])))
            return value
        with patch.object(delivery, 'read', side_effect=drift):
            with self.assertRaisesRegex(ValueError, 'ICON_MANIFEST_DRIFT'):
                delivery.build_reference()
