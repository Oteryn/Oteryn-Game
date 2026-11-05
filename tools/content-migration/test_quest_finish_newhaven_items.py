"""Named Newhaven rewards qualify through the existing Item import boundaries."""
import copy
import json
import unittest

import quest_finish_newhaven_items as gear


class NewhavenItems(unittest.TestCase):
    def setUp(self):
        self.packet = json.loads((gear.ROOT / gear.PATH).read_bytes())

    def test_exact_named_rewards_reconcile_stale_callback_ids(self):
        gear.validate_packet(self.packet)
        self.assertEqual(self.packet['accepted_difference']['old_to_correct'],
                         [[10321, 3571], [8871, 7991], [48847, 50195],
                          [48845, 51119], [48843, 51760]])
        for witness in self.packet['identity_witnesses']:
            self.assertTrue(witness['target_flags']['flags.take'])
            self.assertFalse(witness['target_flags']['flags.unmove'])
            self.assertFalse(witness['target_flags']['flags.cumulative'])
            self.assertFalse(witness['target_flags']['flags.container'])

    def test_actual_definitions_become_portable_without_replacing_known_facts(self):
        definitions = list(gear.definitions().values())
        before = copy.deepcopy(definitions)
        gear.apply_admissions(definitions)
        for old, actual in zip(before, definitions):
            self.assertTrue(actual['materializable'])
            self.assertEqual(actual['stack_class'], 'NonStackable')
            self.assertEqual(actual['identity'], old['identity'])
            self.assert_known_preserved(old['semantics'], actual['semantics'])
            self.assertEqual(actual['semantics']['physical']['value']['movable'], gear.source.K(True))
            self.assertEqual(actual['semantics']['container'], gear.source.NA)
        after = copy.deepcopy(definitions)
        gear.apply_admissions(definitions)
        self.assertEqual(definitions, after)

    def assert_known_preserved(self, old, actual):
        if isinstance(old, dict):
            if old.get('state') == 'UNKNOWN':
                return
            for key, value in old.items():
                self.assert_known_preserved(value, actual[key])
        elif isinstance(old, list):
            self.assertEqual(len(old), len(actual))
            for left, right in zip(old, actual):
                self.assert_known_preserved(left, right)
        else:
            self.assertEqual(old, actual)

    def test_monk_staff_gets_source_equipment_weapon_and_weight(self):
        definitions = list(gear.definitions().values())
        gear.apply_admissions(definitions)
        staff = next(d for d in definitions if d['identity']['key'] == gear.canonical_key(51119))
        pattern = staff['semantics']['equipment']['value']['patterns']['value'][0]
        self.assertEqual(pattern['primary_slot'], gear.source.K('WEAPON'))
        self.assertEqual(pattern['additional_reserved_slots'], gear.source.K(['SHIELD']))
        self.assertEqual(pattern['vocations'], gear.source.K(['MONK']))
        self.assertEqual(staff['semantics']['weapon']['value']['weapon_type'], gear.source.K('FIST'))
        self.assertEqual(staff['semantics']['physical']['value']['weight'], gear.source.K(1800))

    def test_native_admission_is_complete_sorted_and_hashes_actual_bytes(self):
        generated = {'profiles': gear.encode({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': []}),
                     'manifest': gear.encode({'item_profiles': {'sha256': 'before'}})}
        gear.admit(gear.ROOT, generated, 'profiles', 'manifest',
                   lambda row: int(row['production_binding']['external_id']), gear.encode)
        actual = json.loads(generated['profiles'])
        self.assertEqual([int(r['production_binding']['external_id']) for r in actual['records']], list(gear.IDS))
        self.assertEqual(json.loads(generated['manifest'])['item_profiles']['sha256'],
                         gear.digest(generated['profiles']))
        before = copy.deepcopy(generated)
        gear.admit(gear.ROOT, generated, 'profiles', 'manifest',
                   lambda row: int(row['production_binding']['external_id']), gear.encode)
        self.assertEqual(generated, before)

    def test_source_revision_identity_and_admission_tampering_refuse(self):
        changed = copy.deepcopy(self.packet)
        changed['source_revision'] = 'other'
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_CAPTURE_SEMANTIC_PIN'):
            gear.validate_packet(changed)
        changed = copy.deepcopy(self.packet)
        changed['records'][0]['production_binding']['target']['key'] = gear.canonical_key(10321)
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_CAPTURE_SEMANTIC_PIN'):
            gear.validate_packet(changed)
        changed = copy.deepcopy(self.packet)
        changed['records'][0]['admission']['materializable'] = False
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_CAPTURE_SEMANTIC_PIN'):
            gear.validate_packet(changed)

    def test_packet_cannot_invent_unknown_weapon_equipment_or_evidence_values(self):
        for target in ('attack', 'equipment', 'protection', 'xml_line'):
            changed = copy.deepcopy(self.packet)
            staff = next(r for r in changed['records']
                         if r['production_binding']['external_id'] == '51119')
            if target == 'attack':
                staff['semantics']['weapon']['value']['attack'] = gear.source.K(9999)
            elif target == 'equipment':
                staff['semantics']['equipment']['value']['patterns']['value'][0]['vocations'] = gear.source.K([])
            elif target == 'protection':
                staff['semantics']['protection'] = gear.source.K({'armor': gear.source.K(9999)})
            else:
                changed['identity_witnesses'][0]['xml_line'] += 1
            with self.subTest(target=target), self.assertRaisesRegex(ValueError, 'NEWHAVEN_CAPTURE_SEMANTIC_PIN'):
                gear.validate_packet(changed)

    def test_semantic_pin_ignores_json_indentation_and_object_key_order(self):
        reformatted = json.loads(json.dumps(self.packet, sort_keys=True, indent=4))
        gear.validate_packet(reformatted)

    def test_incomplete_cohort_does_not_partially_admit(self):
        definitions = list(gear.definitions().values())[:-1]
        before = copy.deepcopy(definitions)
        with self.assertRaisesRegex(ValueError, 'NEWHAVEN_ADMISSION_INCOMPLETE'):
            gear.apply_admissions(definitions)
        self.assertEqual(definitions, before)


if __name__ == '__main__':
    unittest.main()
