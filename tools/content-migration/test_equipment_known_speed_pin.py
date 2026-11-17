"""Closed source qualification and exact generated guards must agree."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
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

    def test_actual_pinned_sources_keep_closed_zero_after_custody_projection(self):
        self.assertEqual(set(self.raw_profiles), pin.IDS)
        self.assertEqual(len(self.packet['records']), 3)
        for record in self.packet['records']:
            key = record['production_definition']['production_key']
            item_id = int(record['production_binding']['external_id'])
            with self.subTest(key=key):
                self.assertIs(type(self.raw_profiles[item_id]['attributes']['speed_bonus']), int)
                self.assertEqual(self.raw_profiles[item_id]['attributes']['speed_bonus'], 0)
                prior_speed = self.prior[key]['attributes'].get('speed_bonus')
                if prior_speed is not None:
                    self.assertIs(type(prior_speed), int)
                    self.assertEqual(prior_speed, 0)
                self.assertIs(type(record['attributes']['speed_bonus']), int)
                self.assertEqual(record['attributes']['speed_bonus'], 0)
                self.assertEqual(record['production_definition'], self.prior[key]['production_definition'])
                self.assertEqual(record['production_binding'], self.prior[key]['production_binding'])
                self.assertEqual(record['authoring'], self.prior[key]['authoring'])
                self.assertEqual(record['admission'], {
                    'materializable': True, 'stack_class': 'NonStackable',
                    'legal_destinations': ['CharacterInventory', 'Ground']})
        result = pin.refresh(self.SOURCE, self.packet)
        self.assertEqual(pin.refresh(result, self.packet), result)
        self.assertIn('assert_eq!(record.attributes.speed_bonus, Some(0)', result)
        self.assertIn('.filter(|key|', result)
        self.assertIn('.collect()', result)
        for item_id in pin.IDS:
            self.assertEqual(result.count(f'"oteryn:item.tibia.i{item_id}"'), 2)

    def test_unknown_nonzero_duplicate_or_unpinned_source_cannot_remove_a_hold(self):
        variants = []
        for value in (None, False, True, 1, -1, 0.0, '0'):
            packet = copy.deepcopy(self.packet)
            packet['records'][0]['attributes']['speed_bonus'] = value
            variants.append(packet)
        packet = copy.deepcopy(self.packet)
        packet['records'][0]['attributes'].pop('speed_bonus')
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

    def test_existing_rust_changes_only_the_closed_three_item_qualification(self):
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
        self.assertEqual(result.count(pin.FILTER_BEGIN), 1)
        self.assertEqual(result.count(pin.FILTER_END), 1)
        self.assertNotIn(pin.LEGACY_BEGIN, result)
        self.assertEqual(result.count(pin.BEGIN), 1)
        self.assertEqual(result.count(pin.END), 1)
        self.assertEqual(pin.refresh(result, self.packet), result)

    UNKNOWN_ASSERTION = '''    // BEGIN GENERATED CUSTODY-RETAINED UNKNOWN SPEED
    // Physical custody retains UNKNOWN speed for parchment and the two trophies.
    for key in [
        "oteryn:item.tibia.i2833",
        "oteryn:item.tibia.i5805",
        "oteryn:item.tibia.i5806",
    ] {
        let record = profiles
            .records
            .iter()
            .find(|record| record.production_definition.production_key == key)
            .ok_or("missing custody-owned unknown-speed Item")?;
        assert_eq!(record.attributes.speed_bonus, None, "{key}");
    }
    // END GENERATED CUSTODY-RETAINED UNKNOWN SPEED
'''

    def test_exact_unknown_predecessor_is_qualified_and_idempotent(self):
        source = self.UNKNOWN_ASSERTION + self.SOURCE
        result = pin.refresh(source, self.packet)
        self.assertEqual(result, pin.refresh(self.SOURCE, self.packet))
        self.assertEqual(pin.refresh(result, self.packet), result)
        self.assertNotIn(pin.LEGACY_BEGIN, result)
        self.assertNotIn('assert_eq!(record.attributes.speed_bonus, None', result)

    def test_partial_duplicate_or_reversed_generated_markers_refuse(self):
        result = pin.refresh(self.SOURCE, self.packet)
        for source in (result.replace(pin.END, ''), result.replace(pin.BEGIN, ''),
                       pin.END + pin.BEGIN + self.SOURCE,
                       result + pin.BEGIN + pin.END,
                       result + pin.LEGACY_BEGIN + pin.LEGACY_END,
                       self.SOURCE + pin.LEGACY_BEGIN + pin.END,
                       result + pin.FILTER_BEGIN,
                       result + pin.FILTER_END,
                       result + pin.FILTER_BEGIN + pin.FILTER_END,
                       result.replace(pin.FILTER_BEGIN, '__FILTER_START__')
                             .replace(pin.FILTER_END, pin.FILTER_BEGIN)
                             .replace('__FILTER_START__', pin.FILTER_END)):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                    pin.refresh(source, self.packet)

    def test_modified_generated_assertion_or_exclusion_is_not_silently_replaced(self):
        result = pin.refresh(self.SOURCE, self.packet)
        bad_assertion = result.replace(
            'assert_eq!(record.attributes.speed_bonus, Some(0)',
            'assert_eq!(record.attributes.speed_bonus, Some(1)')
        bad_unknown = result.replace(
            'assert_eq!(record.attributes.speed_bonus, Some(0)',
            'assert_eq!(record.attributes.speed_bonus, None')
        bad_exclusion = (pin.FILTER_BEGIN
                         + '        .filter(|key| key != "oteryn:item.tibia.i3456")\n'
                         + pin.FILTER_END)
        start = result.index(pin.FILTER_BEGIN)
        stop = result.index(pin.FILTER_END) + len(pin.FILTER_END)
        bad_filter = result[:start] + bad_exclusion + result[stop:]
        unmarked_filter = result.replace(pin.FILTER_BEGIN, '').replace(pin.FILTER_END, '')
        bad_legacy = self.UNKNOWN_ASSERTION.replace(
            'assert_eq!(record.attributes.speed_bonus, None',
            'assert_eq!(record.attributes.speed_bonus, Some(1)') + self.SOURCE
        for source in (bad_assertion, bad_unknown, bad_filter, unmarked_filter, bad_legacy):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_ANCHOR'):
                    pin.refresh(source, self.packet)

    def test_refresh_keeps_the_complete_captured_packet_unchanged(self):
        packet = copy.deepcopy(self.packet)
        pin.refresh(self.SOURCE, packet)
        self.assertEqual(packet, self.packet)


class QualifiedKnownSpeedAdmissionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = json.loads((pin.producer.ROOT /
            'tools/content-schema/native-gameplay/questpc5-parchment-profile.json').read_bytes())
        cls.rows = cls.packet['records'] + cls.packet['custody_promotions']
        if (len(cls.rows) != 3 or
                {row['production_binding']['external_id'] for row in cls.rows} != {'2833', '5805', '5806'}):
            raise ValueError('known-speed fixture must contain exactly the three qualified Items')

    @staticmethod
    def canonical(value):
        return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()

    def generated(self, records):
        return {'profiles': self.canonical({'records': records}),
                'manifest': self.canonical({'schema': 'OTERYN_NATIVE_GAMEPLAY_MANIFEST/v7',
                                            'item_profiles': {}})}

    def admit(self, generated, records):
        with tempfile.TemporaryDirectory() as directory:
            pin.producer.admit_records(Path(directory), generated, 'profiles', 'manifest',
                lambda row: row['production_definition']['production_key'], self.canonical, records)

    def prior(self, records, mode):
        previous = copy.deepcopy(records)
        for row in previous:
            if mode == 'null':
                row['attributes']['speed_bonus'] = None
            else:
                row['attributes'].pop('speed_bonus')
            if mode == 'inert':
                row['admission']['materializable'] = False
                row['admission']['legal_destinations'] = ['Ground']
        return previous

    def test_source_zero_qualification_rejects_nonzero_bool_or_unsupported_values(self):
        for row in self.rows:
            item_id = int(row['production_binding']['external_id'])
            self.assertEqual(pin.producer._source_known_speed(item_id, row['attributes']['speed_bonus']), 0)
            for value in (None, False, True, 1, -1, 0.0, '0'):
                with self.subTest(item_id=item_id, value=repr(value)):
                    with self.assertRaisesRegex(ValueError, 'KNOWN_SPEED_SOURCE'):
                        pin.producer._source_known_speed(item_id, value)

    def test_real_closed_profiles_keep_zero_through_portable_and_inert_replay(self):
        for mode in ('absent', 'portable', 'inert', 'null'):
            with self.subTest(mode=mode):
                records = copy.deepcopy(self.rows)
                generated = self.generated([] if mode == 'absent' else self.prior(records, mode))
                self.admit(generated, records)
                admitted = json.loads(generated['profiles'])['records']
                self.assertEqual(admitted, self.rows)
                self.assertEqual(records, self.rows)
                first = dict(generated)
                self.admit(generated, copy.deepcopy(self.rows))
                self.assertEqual(generated, first)
                packet = dict(self.packet, records=admitted)
                result = pin.refresh(SourceKnownSpeedTests.SOURCE, packet)
                self.assertEqual(pin.refresh(result, packet), result)
                for item_id in pin.IDS:
                    self.assertEqual(result.count(f'"oteryn:item.tibia.i{item_id}"'), 2)

    def test_closed_zero_completion_preserves_other_owner_state(self):
        prior = self.prior(self.rows, 'portable')
        observed = []
        expected = []
        for existing, qualified in zip(prior, self.rows):
            existing['attributes'].update(always_on_top=False, always_on_top_order=0)
            for group in ('charges', 'readable_writeable', 'physical', 'stack', 'container'):
                existing['semantics'][group] = {'state': 'UNKNOWN'}
            record = copy.deepcopy(existing)
            record['attributes']['speed_bonus'] = qualified['attributes']['speed_bonus']
            for group in ('physical', 'stack', 'container'):
                record['semantics'][group] = copy.deepcopy(qualified['semantics'][group])
            observed.append(record)
            expected.append(copy.deepcopy(record))
        before = copy.deepcopy(observed)
        generated = self.generated(prior)
        self.admit(generated, observed)
        self.assertEqual(json.loads(generated['profiles'])['records'], expected)
        self.assertEqual(observed, before)
        first = dict(generated)
        self.admit(generated, copy.deepcopy(observed))
        self.assertEqual(generated, first)

    def test_closed_admission_refuses_bad_speed_identity_and_binding_without_writes(self):
        for mode in ('portable', 'inert'):
            for index in range(len(self.rows)):
                variants = []
                for value in (None, False, True, 1, -1, 0.0, '0'):
                    observed = copy.deepcopy(self.rows)
                    observed[index]['attributes']['speed_bonus'] = value
                    variants.append(('observed:' + repr(value), self.prior(self.rows, mode), observed))
                observed = copy.deepcopy(self.rows)
                observed[index]['attributes'].pop('speed_bonus')
                variants.append(('observed:missing', self.prior(self.rows, mode), observed))
                for value in (False, True, 1, -1, 0.0, '0'):
                    prior = self.prior(self.rows, mode)
                    prior[index]['attributes']['speed_bonus'] = value
                    variants.append(('prior:' + repr(value), prior, copy.deepcopy(self.rows)))
                for field, name, value in (
                        ('production_definition', 'family', 'WorldObject'),
                        ('production_definition', 'production_key', 'oteryn:item.tibia.i11701'),
                        ('production_definition', 'revision_ref', 'wrong-revision'),
                        ('production_binding', 'external_id', '11701'),
                        ('production_binding', 'disposition', 'ALIAS'),
                        ('production_binding', 'identity_namespace', 'wrong-namespace'),
                        ('production_binding', 'source_key', 'wrong-source'),
                        ('production_binding', 'source_revision', 'wrong-revision'),
                        ('production_binding', 'target', {'family': 'Item', 'key': 'wrong-key',
                                                          'revision': 'definition-r1'})):
                    for owner in ('observed', 'prior'):
                        observed = copy.deepcopy(self.rows)
                        prior = self.prior(self.rows, mode)
                        target = observed if owner == 'observed' else prior
                        target[index][field][name] = value
                        variants.append((owner + ':' + field + '.' + name, prior, observed))
                for name, prior, observed in variants:
                    with self.subTest(mode=mode, index=index, variant=name):
                        generated = self.generated(prior)
                        before = dict(generated)
                        with self.assertRaisesRegex(ValueError, 'QENGINE_REWARD_ITEM_'):
                            self.admit(generated, observed)
                        self.assertEqual(generated, before)
        for index in range(len(self.rows)):
            observed = copy.deepcopy(self.rows)
            observed[index]['production_binding']['source_revision'] = 'wrong-revision'
            for prior in ([], copy.deepcopy(observed)):
                with self.subTest(index=index, absent_prior=not prior, matching_wrong_revision=True):
                    generated = self.generated(prior)
                    before = dict(generated)
                    with self.assertRaisesRegex(ValueError, 'known_speed_identity'):
                        self.admit(generated, observed)
                    self.assertEqual(generated, before)

    def test_other_owners_keep_their_unknown_speed(self):
        for item_id in (3456, 11701):
            for has_null in (False, True):
                with self.subTest(item_id=item_id, has_null=has_null):
                    observed = copy.deepcopy(self.rows[0])
                    key = f'oteryn:item.tibia.i{item_id}'
                    observed['production_definition']['production_key'] = key
                    observed['production_binding']['external_id'] = str(item_id)
                    observed['production_binding']['target']['key'] = key
                    observed['authoring']['item']['key'] = f'candidate:item/{item_id}'
                    prior = copy.deepcopy(observed)
                    if has_null:
                        prior['attributes']['speed_bonus'] = None
                    else:
                        prior['attributes'].pop('speed_bonus')
                    generated = self.generated([prior])
                    self.admit(generated, [observed])
                    self.assertEqual(json.loads(generated['profiles'])['records'], [prior])


if __name__ == '__main__':
    unittest.main()
