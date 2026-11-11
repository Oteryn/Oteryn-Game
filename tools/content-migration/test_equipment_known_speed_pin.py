"""Physical custody cannot turn an owner's UNKNOWN speed into the XML default."""
import copy
import json
import unittest
from unittest.mock import patch

import equipment_known_speed_pin as pin


class SourceKnownSpeedTests(unittest.TestCase):
    SOURCE = '''    let content = native(&input)?;
    let mut unknown_speed = std::collections::BTreeSet::new();
        .chain([3503, 3504, 3506].map(|id| format!("oteryn:item.tibia.i{id}")))
        .collect()
'''

    @classmethod
    def setUpClass(cls):
        cls.raw_profiles = {}
        profile = pin.producer.source.profile

        def observed_profile(item_id, *args, **kwargs):
            result = profile(item_id, *args, **kwargs)
            cls.raw_profiles[item_id] = copy.deepcopy(result)
            return result

        with patch.object(pin.producer.source, 'profile', side_effect=observed_profile):
            cls.packet = pin.producer.capture(ids=pin.IDS)
        cls.prior = {
            row['production_definition']['production_key']: row
            for row in json.loads((pin.producer.ROOT / pin.producer.PROFILE).read_bytes())['records']
            if row['production_definition']['production_key'] in {
                f'oteryn:item.tibia.i{item}' for item in pin.IDS}
        }

    def test_actual_pinned_sources_preserve_unknown_despite_raw_decoder_zero(self):
        self.assertEqual(set(self.raw_profiles), pin.IDS)
        self.assertEqual(len(self.packet['records']), 3)
        for record in self.packet['records']:
            key = record['production_definition']['production_key']
            item_id = int(record['production_binding']['external_id'])
            with self.subTest(key=key):
                self.assertEqual(self.raw_profiles[item_id]['attributes']['speed_bonus'], 0)
                self.assertNotIn('speed_bonus', self.prior[key]['attributes'])
                self.assertNotIn('speed_bonus', record['attributes'])
                self.assertEqual(record['production_definition'], self.prior[key]['production_definition'])
                self.assertEqual(record['production_binding'], self.prior[key]['production_binding'])
                self.assertEqual(record['authoring'], self.prior[key]['authoring'])
                self.assertEqual(record['admission'], {
                    'materializable': True, 'stack_class': 'NonStackable',
                    'legal_destinations': ['CharacterInventory', 'Ground']})
        result = pin.refresh(self.SOURCE, self.packet)
        self.assertEqual(pin.refresh(result, self.packet), result)
        self.assertIn('assert_eq!(record.attributes.speed_bonus, None', result)
        self.assertNotIn('.filter(|key|', result)
        self.assertIn('.collect()', result)
        for item_id in pin.IDS:
            self.assertEqual(result.count(f'"oteryn:item.tibia.i{item_id}"'), 1)

    def test_known_nonzero_duplicate_or_unpinned_source_cannot_remove_a_hold(self):
        variants = []
        for value in (0, 1, False, '0'):
            packet = copy.deepcopy(self.packet)
            packet['records'][0]['attributes']['speed_bonus'] = value
            variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['records'][1] = copy.deepcopy(packet['records'][0])
        variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['records'][0]['production_definition']['production_key'] = 'oteryn:item.tibia.i3456'
        variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['records'].pop()
        variants.append(packet)
        for source, field, value in (
                (0, 'sha256', '0' * 64), (0, 'revision', '0' * 40),
                (0, 'path', 'unqualified/items.xml'), (1, 'sha256', '0' * 64),
                (1, 'path', 'unqualified/appearances.dat')):
            packet = copy.deepcopy(self.packet)
            packet['sources'][source][field] = value
            variants.append(packet)
        for index, packet in enumerate(variants):
            with self.subTest(variant=index):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_SOURCE'):
                    pin.refresh(self.SOURCE, packet)

    def test_missing_or_duplicated_rust_anchor_refuses_generation(self):
        for source in ('', self.SOURCE + self.SOURCE,
                       self.SOURCE.replace(pin.ANCHOR, ''),
                       self.SOURCE.replace(pin.FILTER_ANCHOR, '')):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                    pin.refresh(source, self.packet)

    def test_existing_rust_replaces_only_the_stale_three_item_qualification(self):
        path = pin.producer.ROOT / ('crates/gameplay/dispatch/src/gameplay_transport/'
                                   'equipment_effects_owner_tests/ability_tests.rs')
        source = path.read_text()
        result = pin.refresh(source, self.packet)
        # Works both before and after normal regeneration. Every byte outside
        # these two generated blocks, including the complete UNKNOWN set, stays.
        def without_blocks(text):
            for begin, end in ((pin.LEGACY_BEGIN, pin.LEGACY_END),
                               (pin.BEGIN, pin.END), (pin.FILTER_BEGIN, pin.FILTER_END)):
                if begin in text:
                    start, stop = text.index(begin), text.index(end) + len(end)
                    text = text[:start] + text[stop:]
            return text

        self.assertEqual(without_blocks(result), without_blocks(source))
        self.assertIn('2827, 2833, 2835', result)
        self.assertIn('5799, 5805, 5806, 5807', result)
        self.assertNotIn(pin.FILTER_BEGIN, result)
        self.assertNotIn(pin.LEGACY_BEGIN, result)
        self.assertEqual(result.count(pin.BEGIN), 1)
        self.assertEqual(result.count(pin.END), 1)
        self.assertEqual(pin.refresh(result, self.packet), result)

    def test_null_representation_of_unknown_remains_unknown(self):
        packet = copy.deepcopy(self.packet)
        for record in packet['records']:
            record['attributes']['speed_bonus'] = None
        self.assertEqual(pin.refresh(self.SOURCE, packet), pin.refresh(self.SOURCE, self.packet))

    def test_partial_duplicate_or_reversed_generated_markers_refuse(self):
        result = pin.refresh(self.SOURCE, self.packet)
        for source in (result.replace(pin.END, ''), result.replace(pin.BEGIN, ''),
                       pin.END + pin.BEGIN + self.SOURCE,
                       result + pin.BEGIN + pin.END,
                       result + pin.LEGACY_BEGIN + pin.LEGACY_END,
                       self.SOURCE + pin.LEGACY_BEGIN + pin.END,
                       result + pin.FILTER_BEGIN,
                       result + pin.FILTER_END):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                    pin.refresh(source, self.packet)

    def test_modified_generated_assertion_or_exclusion_is_not_silently_removed(self):
        result = pin.refresh(self.SOURCE, self.packet)
        bad_assertion = result.replace(
            'assert_eq!(record.attributes.speed_bonus, None',
            'assert_eq!(record.attributes.speed_bonus, Some(1)')
        bad_exclusion = (pin.FILTER_BEGIN
                         + '        .filter(|key| key != "oteryn:item.tibia.i3456")\n'
                         + pin.FILTER_END)
        for source in (bad_assertion, result.replace(pin.FILTER_ANCHOR,
                                                    pin.FILTER_ANCHOR + bad_exclusion)):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                    pin.refresh(source, self.packet)

    def test_refresh_keeps_the_complete_captured_packet_unchanged(self):
        packet = copy.deepcopy(self.packet)
        pin.refresh(self.SOURCE, packet)
        self.assertEqual(packet, self.packet)


if __name__ == '__main__':
    unittest.main()
