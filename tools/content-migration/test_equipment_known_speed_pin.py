"""Positive source qualification must accompany every UNKNOWN-set exclusion."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

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
