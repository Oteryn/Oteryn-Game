"""Registry ownership and bounded compilation, independent of quest availability."""
import copy
import unittest

import batch3_world as tool


class Batch3WorldTests(unittest.TestCase):
    def test_regeneration_preserves_foreign_rows_and_removes_only_recorded_owned_keys(self):
        foreign = {'key': 'other', 'sources': ['unchanged'], 'private': {'x': 17}}
        existing = {'schema': 1, 'callbacks': [foreign, {'key': 'old-owned'}]}
        previous = {'schema': tool.SCHEMA, 'sources': [{'path': 'old', 'keys': ['old-owned']}]}
        owned = {'key': 'new-owned', 'sources': [tool.PIN + 'file.lua:2']}
        before = copy.deepcopy(existing)
        result, index = tool.compile_callbacks(existing, previous, {'source': {'schema': 1, 'callbacks': [owned]}})
        self.assertEqual(existing, before)
        self.assertEqual(result['callbacks'], [owned, foreign])
        again, same_index = tool.compile_callbacks(result, index, {'source': {'schema': 1, 'callbacks': [owned]}})
        self.assertEqual((again, same_index), (result, index))

    def test_duplicate_foreign_key_and_unpinned_input_are_refused(self):
        existing = {'schema': 1, 'callbacks': [{'key': 'foreign'}]}
        empty = {'schema': tool.SCHEMA, 'sources': []}
        for row in [{'key': 'foreign', 'sources': [tool.PIN + 'file.lua:1']},
                    {'key': 'own', 'sources': ['crystal:unverified:file.lua:1']}]:
            with self.assertRaises(ValueError):
                tool.compile_callbacks(existing, empty, {'source': {'schema': 1, 'callbacks': [row]}})

    def test_existing_runtime_altar_keeps_its_reserved_slot(self):
        empty = {'schema': tool.SCHEMA, 'sources': []}
        for count, valid in [(511, True), (512, False)]:
            existing = {'schema': 1, 'callbacks': [{'key': str(i)} for i in range(count)]}
            if valid:
                self.assertEqual(len(tool.compile_callbacks(existing, empty, {})[0]['callbacks']), count)
            else:
                with self.assertRaisesRegex(ValueError, 'budget'):
                    tool.compile_callbacks(existing, empty, {})

    def test_declined_fields_cannot_enter_the_existing_runtime_contract(self):
        empty = {'schema': tool.SCHEMA, 'sources': []}
        for extension in [
            {'failure_teleport': {'x': 1, 'y': 2, 'floor': -7}},
            {'success_say': 'source monster speech'},
            {'storage': [{'track': 'source', 'minimum': 2, 'maximum': 99,
                          'set': 2, 'unsupported': True}]},
        ]:
            row = {'key': 'own', 'sources': [tool.PIN + 'file.lua:1'], **extension}
            with self.assertRaisesRegex(ValueError, 'held callback contract'):
                tool.compile_callbacks({'schema': 1, 'callbacks': []}, empty,
                                       {'source': {'schema': 1, 'callbacks': [row]}})

    def test_native_read_only_gate_and_success_message_are_preserved(self):
        row = {'key': 'guard', 'sources': [tool.PIN + 'file.lua:1'],
               'storage': [{'track': 'source', 'minimum': 3, 'maximum': 13,
                            'set': 0, 'read_only': True}], 'success_message': 'source text'}
        result, _ = tool.compile_callbacks({'schema': 1, 'callbacks': []},
            {'schema': tool.SCHEMA, 'sources': []}, {'source': {'schema': 1, 'callbacks': [row]}})
        self.assertEqual(result['callbacks'], [row])

    def test_returning_step_preserves_explicit_failure_landing(self):
        row = {'key': 'guard', 'sources': [tool.PIN + 'file.lua:1'],
               'trigger': 'step_in', 'return_on_failure': True,
               'failure_teleport': {'x': 32818, 'y': 31599, 'floor': -9}}
        result, _ = tool.compile_callbacks({'schema': 1, 'callbacks': []},
            {'schema': tool.SCHEMA, 'sources': []}, {'source': {'schema': 1, 'callbacks': [row]}})
        self.assertEqual(result['callbacks'], [row])
        for field, value in [('trigger', 'use'), ('return_on_failure', False),
                             ('failure_teleport', {'x': 1, 'y': 2, 'floor': -16})]:
            invalid = {**row, field: value}
            with self.assertRaisesRegex(ValueError, 'held callback contract'):
                tool.compile_callbacks({'schema': 1, 'callbacks': []},
                    {'schema': tool.SCHEMA, 'sources': []},
                    {'source': {'schema': 1, 'callbacks': [invalid]}})

    def test_conditional_set_is_preserved_and_invalid_conditions_are_rejected(self):
        row = {'key': 'reset', 'sources': [tool.PIN + 'file.lua:1'],
               'storage': [{'track': 'source', 'minimum': -1, 'maximum': 2, 'set': 0, 'set_if': 1}]}
        empty = {'schema': tool.SCHEMA, 'sources': []}
        result, _ = tool.compile_callbacks({'schema': 1, 'callbacks': []}, empty,
                                          {'source': {'schema': 1, 'callbacks': [row]}})
        self.assertEqual(result['callbacks'], [row])
        for patch in [{'set_if': 3}, {'set_if': True}, {'read_only': True}]:
            invalid = copy.deepcopy(row)
            invalid['storage'][0].update(patch)
            with self.assertRaisesRegex(ValueError, 'held callback contract'):
                tool.compile_callbacks({'schema': 1, 'callbacks': []}, empty,
                                       {'source': {'schema': 1, 'callbacks': [invalid]}})
