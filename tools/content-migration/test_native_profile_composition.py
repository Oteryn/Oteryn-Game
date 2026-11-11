"""Native composition preserves facts and final bytes across intermediate encodings."""
import copy
import hashlib
import json
import unittest
from contextvars import Context

from native_profile_composition import (
    compact_native_profile_scope,
    encode_native_profile,
    merge_fact,
)


class NativeProfileComposition(unittest.TestCase):
    def test_nested_ordered_patterns_only_enrich_unknown_facts(self):
        old = {'state': 'KNOWN', 'value': [
            {'pattern_id': 1, 'compatibility_rule': {'state': 'UNKNOWN'}}]}
        source = {'state': 'KNOWN', 'value': [
            {'pattern_id': 1, 'compatibility_rule': {'state': 'NOT_APPLICABLE'}}]}
        self.assertEqual(merge_fact(old, source, 'patterns'), source)
        self.assertEqual(old['value'][0]['compatibility_rule'], {'state': 'UNKNOWN'})
        self.assertEqual(merge_fact(source, old, 'patterns'), source)

    def test_conflicting_values_types_states_and_pattern_identities_are_refused(self):
        for old, new in [(1, 2), (1, True),
                         ({'state': 'KNOWN', 'value': 1},
                          {'state': 'KNOWN', 'value': True}),
                         ({'state': 'NOT_APPLICABLE'}, {'state': 'KNOWN', 'value': 1}),
                         ([{'pattern_id': 1}], [{'pattern_id': 2}]),
                         ([1], [1, 2]), ({'a': 1}, {'b': 1})]:
            with self.subTest(old=old, new=new):
                with self.assertRaisesRegex(ValueError, 'NATIVE_PROFILE_FACT_CONFLICT'):
                    merge_fact(old, new, 'profile')


class NativeProfileEncoding(unittest.TestCase):
    def setUp(self):
        self.document = {
            'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
            'records': [{
                'semantics': {'state': 'KNOWN', 'value': {'name': 'żółw', 'weight': 1.5}},
                'attributes': {'movable': True, 'unknown': None},
                'production_definition': {'revision_ref': 'r1', 'production_key': 'item.a'},
            }],
        }
        self.pretty = (json.dumps(self.document, indent=2, ensure_ascii=False) + '\n').encode()

    def test_standalone_encoding_keeps_exact_pretty_bytes_and_input(self):
        before = copy.deepcopy(self.document)
        self.assertEqual(encode_native_profile(self.document), self.pretty)
        self.assertIn('żółw'.encode(), self.pretty)
        self.assertEqual(self.document, before)

    def test_compact_encoding_preserves_order_values_and_final_digest(self):
        before = copy.deepcopy(self.document)
        with compact_native_profile_scope():
            compact = encode_native_profile(self.document)
        self.assertEqual(compact.count(b'\n'), 1)
        self.assertLess(len(compact), len(self.pretty))
        self.assertEqual(json.loads(compact), self.document)
        # A premature sort could change later insertion-order-sensitive work.
        self.assertLess(compact.index(b'"schema"'), compact.index(b'"records"'))
        self.assertLess(compact.index(b'"semantics"'), compact.index(b'"attributes"'))
        self.assertLess(compact.index(b'"revision_ref"'), compact.index(b'"production_key"'))
        def final(raw):
            return (json.dumps(json.loads(raw), ensure_ascii=False,
                               sort_keys=True, separators=(',', ':')) + '\n').encode()
        self.assertEqual(final(compact), final(self.pretty))
        self.assertEqual(hashlib.sha256(final(compact)).hexdigest(),
                         hashlib.sha256(final(self.pretty)).hexdigest())
        self.assertEqual(self.document, before)
        self.assertEqual(encode_native_profile(self.document), self.pretty)

    def test_nested_scope_restores_outer_and_standalone_encodings(self):
        with compact_native_profile_scope():
            compact = encode_native_profile(self.document)
            with compact_native_profile_scope():
                self.assertEqual(encode_native_profile(self.document), compact)
            self.assertEqual(encode_native_profile(self.document), compact)
        self.assertEqual(encode_native_profile(self.document), self.pretty)

    def test_exception_restores_encoding_and_unrelated_context_is_isolated(self):
        with self.assertRaisesRegex(ValueError, 'admission rejected'):
            with compact_native_profile_scope():
                self.assertNotEqual(encode_native_profile(self.document), self.pretty)
                self.assertEqual(Context().run(encode_native_profile, self.document), self.pretty)
                raise ValueError('admission rejected')
        self.assertEqual(encode_native_profile(self.document), self.pretty)


if __name__ == '__main__':
    unittest.main()
