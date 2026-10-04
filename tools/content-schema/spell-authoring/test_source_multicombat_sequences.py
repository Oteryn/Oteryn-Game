import copy
import gzip
import json
from pathlib import Path
import unittest
import jsonschema
import source_multicombat_sequences as producer


class OrderedSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.packet = producer.build()

    def test_strict_schema_and_no_candidates(self):
        producer.validate(self.packet)
        self.assertEqual(0, self.packet['candidate_count'])
        self.assertTrue(all(not r['complete_spell_candidate'] for r in self.packet['records']))

    def test_actual_current_forked_single_combat_chain(self):
        for row, count in zip(self.packet['records'][:2], [7, 6]):
            self.assertEqual(['combat'], row['combat_identities'])
            self.assertEqual(count, row['facts']['base_target_count'])
            self.assertEqual(5, row['facts']['jump_distance'])
            self.assertEqual(0, row['facts']['independent_probability_draw_count'])
            self.assertEqual('combat_execute_result', row['facts']['cast_return'])
            self.assertEqual(['onGetFormulaValues', 'getChainValue', 'spell.onCastSpell'],
                             [f['name'] for f in row['function_programs']])

    def test_sweeping_execute_order_then_cleanup(self):
        row = self.packet['records'][2]
        self.assertEqual(['capture_player_id', 'combat_execute', 'combat_execute', 'clear_cache', 'return_true'],
                         [r['operation'] for r in row['cast_chronology']])
        self.assertEqual(['combatInner', 'combatOuter'],
                         [r['combat'] for r in row['cast_chronology'] if r['operation'] == 'combat_execute'])
        self.assertEqual([0, 0], row['facts']['cache']['cache_miss_pair'])
        self.assertEqual('0.75', row['facts']['cache']['outer_scale'])
        self.assertTrue(row['facts']['execute_returns_ignored'])
        self.assertFalse(row['facts']['cache']['cleanup_on_exception_qualified'])

    def test_reordered_or_missing_cast_rejected(self):
        for mode in ['reorder', 'omit']:
            packet = copy.deepcopy(self.packet)
            chronology = packet['records'][2]['cast_chronology']
            if mode == 'reorder':
                chronology[1], chronology[2] = chronology[2], chronology[1]
            else:
                chronology.pop()
            with self.assertRaises(ValueError):
                producer.validate(packet)

    def test_omitted_function_or_branch_link_rejected(self):
        packet = copy.deepcopy(self.packet)
        packet['records'][2]['function_programs'][2]['statement_refs'].pop()
        with self.assertRaises(ValueError):
            producer.validate(packet)

    def test_unexpected_source_bytes_rejected(self):
        with self.assertRaisesRegex(ValueError, 'source bytes differ'):
            producer.build_record(*producer.SPECS[0], raw=b'return true\n')

    def test_activation_or_untyped_fields_rejected(self):
        for key, value in [('runtime_activation', True), ('extra_unknown_field', False)]:
            packet = copy.deepcopy(self.packet)
            packet[key] = value
            with self.assertRaises(jsonschema.ValidationError):
                producer.validate(packet)

    def test_complete_ast_matches_cached_r38(self):
        expected = {(r['source_syntax']['source'], r['source_syntax']['path']): r['source_syntax']
                    for r in self.packet['records']}
        root = Path(__file__).resolve().parents[3]
        found = {}
        with gzip.open(root / 'docs/reference/spells/r38-source-closure/source-syntax.jsonl.gz', 'rt') as stream:
            for line in stream:
                row = json.loads(line)
                key = (row['source'], row['path'])
                if key in expected:
                    found[key] = row
        self.assertEqual(expected, found)


if __name__ == '__main__':
    unittest.main()
