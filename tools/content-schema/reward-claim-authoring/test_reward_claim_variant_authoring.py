"""Offline regressions for typed canonical variant DATA, never native readiness."""
import copy
import json
from pathlib import Path
import unittest
import jsonschema
import reward_claim_authoring as plain
import reward_claim_variant_authoring as variants
import reward_claim_variant_migration as migration


class CanonicalVariantTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root = plain.ROOT; cls.items = plain.load_items()
        evidence = json.loads(migration.metadata_path(cls.root, migration.EVIDENCE).read_text())
        cls.packet = migration.derive_packet(cls.root, evidence)
        cls.records = variants.derive_definitions(cls.packet, cls.items, plain.stack_problem)
        cls.schema = variants.validator(cls.root)

    def test_all_actual_variants_have_real_canonical_rewards_and_native_block(self):
        self.assertEqual(len(self.records), 105)
        for row in self.records:
            d = row['definition']; self.schema.validate(d)
            self.assertNotEqual(d['readiness'], 'ready')
            self.assertEqual(d['native_admission'], 'WAITING_IMPLEMENTATION')
            self.assertEqual(d['source_variant']['source_claim']['identity']['key'], d['provenance']['pilot_key'])
            for p in d['placements']:
                for field in ('items', 'random_one_of'):
                    for q in p['reward'].get(field, []):
                        self.assertTrue(q['item']['key'].startswith('oteryn:item.tibia.i'))
                        self.assertEqual(q['item'], self.items[q['item']['key']]['identity'])

    def test_plain_231_definition_records_remain_exact_and_readiness_219_12(self):
        outputs = plain.generate(); index = json.loads(outputs[plain.INDEX_PATH])
        self.assertEqual(index['plain_record_count'], 231)
        self.assertEqual(index['record_count'], 336)
        self.assertEqual(index['readiness']['ready'], 219)
        self.assertEqual(index['readiness']['waiting_item_semantics'], 12)
        for path in index['shards'][:3]:
            self.assertEqual(outputs[path], (self.root / path).read_text())
        self.assertEqual(index['native_admission']['variants'], 'WAITING_IMPLEMENTATION')

    def test_source_complete_is_not_native_or_item_complete(self):
        self.assertTrue(any(r['definition']['readiness'] == 'waiting_implementation' for r in self.records))
        held = [r['definition'] for r in self.records if any(g['category'] == 'item' for g in r['definition']['data_holds'])]
        self.assertTrue(held)
        self.assertTrue(all(d['readiness'] == 'waiting_data' for d in held))

    def test_medusa_real_unknown_carrier_is_held_not_invented(self):
        d = next(r['definition'] for r in self.records if any(g['code'] == 'SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED' for g in r['definition']['data_holds']))
        self.assertEqual(d['readiness'], 'waiting_data')
        self.assertTrue(any(p['reward'].get('written_text', {}).get('item', True) is None for p in d['placements']))

    def test_charge_quantities_one_instance_and_raw_conflicts_survive(self):
        found = 0
        for row in self.records:
            d = row['definition']
            for charge in d['source_variant']['charge_dispositions']:
                p = d['placements'][charge['placement_index']]
                if charge['normalized_quantity']['state'] == 'KNOWN':
                    self.assertEqual(p['reward'][charge['reward_field']][charge['reward_index']]['count'], 1)
                if charge['data_status'] == 'CONFLICT':
                    found += 1
                    self.assertTrue(any(g['code'] == 'SOURCE_CHARGE_MISMATCH' for g in d['data_holds']))
                    self.assertNotEqual(charge['source_raw_argument'], charge['definition_charges']['value'])
        self.assertEqual(found, 3)

    def test_covered_fields_text_key_random_container_cooldown_retained(self):
        seen = set()
        for row in self.records:
            d = row['definition']; original = d['source_variant']['source_claim']
            self.assertEqual(d['claim'], original['claim'])
            for p, old in zip(d['placements'], original['placements']):
                self.assertEqual(p['reward'].keys(), old['reward'].keys())
                for field in ('key_binding', 'written_text'):
                    if field in old['reward']:
                        seen.add(field)
                        if field == 'key_binding': self.assertEqual(p['reward'][field], old['reward'][field])
                        else: self.assertEqual(p['reward'][field]['text_ref'], old['reward'][field]['text_ref'])
                seen.update(p['reward'].keys())
        self.assertTrue({'key_binding', 'written_text', 'random_one_of', 'container'} <= seen)
        self.assertTrue(any(r['definition']['claim']['repeat']['kind'] == 'cooldown' for r in self.records))

    def test_achievement_request_is_typed_exact_existing_grant(self):
        found = []
        for row in self.records:
            d = row['definition']
            for grant in d['source_variant']['achievement_grants']:
                p = d['placements'][grant['placement_index']]
                self.assertEqual(p['achievement_grant'], grant['grant_request']); found.append(grant)
        self.assertTrue(found)

    def test_schema_rejects_fake_ready_native_admission_and_unknown_field(self):
        for field, value in [('readiness', 'ready'), ('native_admission', 'READY'), ('unexpected', True)]:
            d = copy.deepcopy(self.records[0]['definition']); d[field] = value
            with self.subTest(field=field), self.assertRaises(jsonschema.ValidationError): self.schema.validate(d)

    def test_semantic_validation_rejects_payload_hold_revision_uid_and_omission(self):
        for field in ('reward', 'holds', 'uid', 'revision', 'omission'):
            broken = copy.deepcopy(self.records)
            if field == 'reward': broken[0]['definition']['placements'][0]['reward']['items'][0]['count'] += 1
            elif field == 'holds': broken[0]['definition']['data_holds'].append({'category':'item', 'code':'invented'})
            elif field == 'uid': broken[0]['definition']['placements'][0]['source_binding']['legacy_unique_ids'][0]['unique_id'] += 1
            elif field == 'revision': broken[0]['definition']['identity']['revision'] = 'invented'
            else: broken.pop()
            with self.subTest(field=field): self.assertTrue(variants.validate(broken, self.root, self.items, plain.stack_problem))

    def test_missing_or_changed_item_identity_is_rejected(self):
        key = self.packet['records'][0]['item_bindings'][0]['canonical_item']['key']
        for change in ('missing', 'revision'):
            items = copy.deepcopy(self.items)
            if change == 'missing': items.pop(key)
            else: items[key]['identity']['revision'] = 'invented'
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'identity/revision'):
                variants.derive_definitions(self.packet, items, plain.stack_problem)

    def test_duplicate_binding_and_identity_are_rejected(self):
        for change in ('binding', 'identity'):
            packet = copy.deepcopy(self.packet)
            if change == 'binding': packet['records'][0]['item_bindings'].append(copy.deepcopy(packet['records'][0]['item_bindings'][0]))
            else: packet['records'].append(copy.deepcopy(packet['records'][0]))
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'duplicate|collision'):
                variants.derive_definitions(packet, self.items, plain.stack_problem)

    def test_compiler_boundary_lists_all_variants_as_unsupported(self):
        errors = variants.native_lowering_errors(self.records)
        self.assertEqual(len(errors), 105)
        self.assertTrue(all(e['code'] == 'REWARD_CLAIM_NATIVE_LOWERING_NOT_IMPLEMENTED' and e['pending'] for e in errors))

    def test_committed_family_reproduces_with_separate_holds(self):
        self.assertEqual(plain.committed_errors(), [])
        for path, text in plain.generate().items():
            self.assertEqual((self.root / path).read_text(), text, path)


if __name__ == '__main__':
    unittest.main()
