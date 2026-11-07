"""Boundary tests for fact preservation, proof bindings and decay closure preparation."""
import copy
import unittest

import prepare_creature_item_fact_packets as prepare


def item(key, action='remove', target=None, duration=10000):
    result = {'identity': {'key': key, 'revision': 'source-r1'},
              'presentation': {'name': 'dead creature'},
              'classification': {'is_corpse': True},
              'physical': {'weight_centioz': 0, 'movable': False, 'pickupable': False},
              'collision': {'block_walk': False},
              'container': {'capacity': 36},
              'temporal': {'decay_action': action, 'duration_ms': duration, 'stop_duration': False}}
    if target:
        result['temporal']['decay_target'] = {'family': 'Item', 'key': target, 'revision': 'source-r1'}
    return result


class ItemFactPreparationTests(unittest.TestCase):
    def test_terminal_chain_preserves_all_stages_and_durations(self):
        a, b, c = 'canary:item/1', 'canary:item/2', 'canary:item/3'
        rows = {a: item(a, 'transform', b), b: item(b, 'transform', c, 300000),
                c: item(c, duration=60000)}
        chain = prepare.decay_chain(a, rows, {k: {'key': k} for k in rows})
        self.assertTrue(chain['complete'])
        self.assertEqual([r['source_temporal']['duration_ms'] for r in chain['steps']], [10000, 300000, 60000])
        self.assertFalse(chain['runtime_execution_qualified'])

    def test_self_decay_normalization_retains_raw_loop_and_duration(self):
        key = 'canary:item/52559'
        row = item(key, 'transform', key, 300000)
        before = copy.deepcopy(row)
        chain = prepare.decay_chain(key, {key: row}, {key: {'key': 'bound'}})
        self.assertTrue(chain['complete'])
        self.assertEqual(chain['steps'][0]['prepared_action'], 'remove')
        self.assertEqual(chain['steps'][0]['source_temporal']['decay_target']['key'], key)
        self.assertEqual(row, before)

    def test_self_decay_cannot_normalize_wrong_family_or_revision(self):
        key = 'canary:item/52559'
        for changed, value in (('family', 'Creature'), ('revision', 'wrong')):
            with self.subTest(changed=changed):
                row = item(key, 'transform', key, 300000)
                row['temporal']['decay_target'][changed] = value
                result = prepare.decay_chain(key, {key: row}, {key: {'key': 'bound'}})
                self.assertFalse(result['complete'])
                self.assertIn('DECAY_TARGET_REFERENCE_MISMATCH',
                              [d['code'] for d in result['diagnostics']])
                self.assertEqual(result['steps'][0]['prepared_action'], 'transform')
                self.assertNotIn('normalization', result['steps'][0])

    def test_multi_item_cycle_is_diagnostic_not_terminal(self):
        a, b = 'canary:item/1', 'canary:item/2'
        rows = {a: item(a, 'transform', b), b: item(b, 'transform', a)}
        result = prepare.decay_chain(a, rows, {k: {'key': k} for k in rows})
        self.assertFalse(result['complete'])
        self.assertEqual(result['diagnostics'][0]['code'], 'NONTERMINAL_DECAY_CYCLE')

    def test_missing_target_source_or_binding_is_incomplete(self):
        a, b = 'canary:item/1', 'canary:item/2'
        result = prepare.decay_chain(a, {a: item(a, 'transform', b)}, {})
        self.assertFalse(result['complete'])
        self.assertEqual({r['code'] for r in result['diagnostics']},
                         {'UNRESOLVED_NATIVE_DECAY_BINDING', 'MISSING_SOURCE_DECAY_TARGET'})

    def test_revision_substitution_is_not_accepted(self):
        a, b = 'canary:item/1', 'canary:item/2'
        rows = {a: item(a, 'transform', b), b: item(b)}
        rows[a]['temporal']['decay_target']['revision'] = 'wrong'
        result = prepare.decay_chain(a, rows, {k: {'key': k} for k in rows})
        self.assertFalse(result['complete'])
        self.assertEqual(result['diagnostics'][0]['code'], 'DECAY_TARGET_REFERENCE_MISMATCH')

    def test_duration_boolean_zero_and_negative_are_not_silently_valid(self):
        key = 'canary:item/1'
        for duration in (False, 0, -1):
            with self.subTest(duration=duration):
                result = prepare.decay_chain(key, {key: item(key, duration=duration)}, {key: {'key': key}})
                self.assertFalse(result['complete'])

    def test_false_zero_unknown_conflict_are_preserved(self):
        for state in ({'state': 'KNOWN', 'value': False}, {'state': 'KNOWN', 'value': 0},
                      {'state': 'UNKNOWN'}, {'state': 'CONFLICT'}):
            semantics = {'physical': {'state': 'KNOWN', 'value': {'movable': state}}}
            self.assertEqual(prepare.field_state(semantics, 'physical.movable'), state)
        self.assertEqual(prepare.field_state({'physical': {'state': 'CONFLICT'}}, 'physical.movable'),
                         {'state': 'CONFLICT'})

    def test_d3_overflow_retains_capacity_and_does_not_admit(self):
        key = 'canary:item/1'
        row = item(key)
        binding = {'key': 'oteryn:item.tibia.i1'}
        packet = prepare.prepare({key: row}, {key: [{'bundle': 'creature'}]}, {key: binding},
                                 {binding['key']: {'materializable': False, 'semantics': {}}})[0]
        self.assertEqual(packet['source_projection']['container']['capacity'], 36)
        self.assertEqual(packet['authoring_fragments']['fields']['container']['capacity'], 36)
        self.assertIn('SOURCE_CAPACITY_EXCEEDS_D3_16', [d['code'] for d in packet['diagnostics']])
        self.assertFalse(packet['admission_authorized'])
        self.assertFalse(packet['runtime_qualified'])

    def test_oz_fragment_is_exact_without_native_unit_guess(self):
        row = item('canary:item/1')
        row['physical']['weight_centioz'] = 4201
        fragment = prepare.authoring_fragments(row)
        self.assertEqual(fragment['fields']['physical']['weight'], {'value': '42.01', 'unit': 'oz'})
        self.assertFalse(fragment['native_weight_unit_conversion_authorized'])
        self.assertIn('temporal.consumption_mode', fragment['unknown_required_or_authority_fields'])
        self.assertNotIn('consumption_mode', fragment['fields']['temporal'])

    def test_source_item_without_binding_is_not_numerically_bound(self):
        key = 'canary:item/48296'
        packet = prepare.prepare({key: item(key)}, {key: []}, {}, {})[0]
        self.assertIsNone(packet['canonical_binding'])
        self.assertFalse(packet['native_definition_exists'])

    def test_unclassified_field_fails_closed(self):
        key = 'canary:item/1'
        row = item(key)
        row['physical']['new_unsupported_flag'] = False
        with self.assertRaisesRegex(ValueError, 'unclassified Item field'):
            prepare.prepare({key: row}, {key: []}, {}, {})

    def test_unclassified_group_fails_closed(self):
        key = 'canary:item/1'
        row = item(key)
        row['new_unrouted_group'] = {'data': 1}
        with self.assertRaisesRegex(ValueError, 'unclassified'):
            prepare.prepare({key: row}, {key: []}, {}, {})


if __name__ == '__main__':
    unittest.main()
