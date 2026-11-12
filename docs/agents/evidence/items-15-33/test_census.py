"""Regression cases for the two structural census findings."""
import unittest
from census import census


def rows(records):
    return {row['path']: row for row in census(records, 'case')}


class CensusRegression(unittest.TestCase):
    def test_domain_unknown_preserves_sibling_facts_and_states(self):
        result = rows([{'value': {'state': 'UNKNOWN', 'old_native_offer': {'price': 0},
                                  'reason': 'mapping unresolved', 'disposition': 'held'}}])
        self.assertEqual(result['value.old_native_offer.price']['records_zero'], 1)
        self.assertEqual(result['value.state']['records_present'], 1)
        self.assertEqual(result['value']['UNKNOWN'], 0)

    def test_wrapper_metadata_is_not_discarded(self):
        result = rows([{'a': {'state': 'KNOWN', 'value': 0, 'source': 'receipt', 'reason': ''}}])
        self.assertEqual(result['a']['KNOWN'], 1)
        self.assertEqual(result['a']['records_zero'], 1)
        self.assertEqual(result['a.source']['records_present'], 1)
        self.assertEqual(result['a.reason']['records_empty_string'], 1)
        self.assertNotIn('a.value', result)

    def test_collection_parent_blocks_array_path_and_counts_records_once(self):
        result = rows([{'a': {'state': 'UNKNOWN'}}, {'a': {'state': 'NOT_APPLICABLE'}},
                       {'a': {'state': 'KNOWN', 'value': [{'x': 1}, {'x': 2}]}}])
        for path in ('a[]', 'a[].x'):
            self.assertEqual(result[path]['records_absent_with_unknown_ancestor'], 1)
            self.assertEqual(result[path]['records_absent_with_not_applicable_ancestor'], 1)
            self.assertEqual(result[path]['records_present'], 1)

    def test_nested_collections_and_empty_zero_null_stay_distinct(self):
        result = rows([{'a': {'state': 'UNKNOWN'}}, {'a': {'state': 'KNOWN', 'value': [[0, None, '']] }},
                       {'a': {'state': 'KNOWN', 'value': []}}])
        self.assertEqual(result['a[][]']['records_absent_with_unknown_ancestor'], 1)
        self.assertEqual(result['a[][]']['records_zero'], 1)
        self.assertEqual(result['a[][]']['records_null'], 1)
        self.assertEqual(result['a[][]']['records_empty_string'], 1)
        self.assertEqual(result['a']['records_empty_list'], 1)


if __name__ == '__main__':
    unittest.main()
