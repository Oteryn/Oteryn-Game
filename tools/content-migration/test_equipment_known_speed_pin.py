"""Positive source qualification must accompany every UNKNOWN-set exclusion."""
import copy
import unittest

import equipment_known_speed_pin as pin


class SourceKnownSpeedTests(unittest.TestCase):
    SOURCE = '''    let content = native(&input)?;
    let mut unknown_speed = std::collections::BTreeSet::new();
        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))
        .collect()
'''

    @classmethod
    def setUpClass(cls):
        cls.packet = pin.producer.capture(ids=pin.IDS)

    def test_actual_pinned_sources_generate_positive_and_negative_set_guards(self):
        result = pin.refresh(self.SOURCE, self.packet)
        self.assertEqual(pin.refresh(result, self.packet), result)
        self.assertIn('assert_eq!(record.attributes.speed_bonus, Some(0)', result)
        self.assertIn('.filter(|key|', result)
        self.assertIn('.collect()', result)
        for key in pin.IDS:
            self.assertEqual(result.count(f'"oteryn:item.tibia.i{key}"'), 2)

    def test_unknown_nonzero_duplicate_or_unpinned_source_cannot_remove_a_hold(self):
        variants = []
        for value in (None, 1, False):
            packet = copy.deepcopy(self.packet)
            packet['records'][0]['attributes']['speed_bonus'] = value
            variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['records'][1] = copy.deepcopy(packet['records'][0])
        variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['sources'][0]['sha256'] = '0' * 64
        variants.append(packet)
        for packet in variants:
            with self.subTest(packet=packet['sources']):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_SOURCE'):
                    pin.refresh(self.SOURCE, packet)

    def test_missing_or_duplicated_rust_anchor_refuses_generation(self):
        for source in ('', self.SOURCE + self.SOURCE):
            with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                pin.refresh(source, self.packet)


if __name__ == '__main__':
    unittest.main()
