"""Exact source epochs, explicit UNKNOWN states and decoder boundary tests."""
import copy
import unittest
import prepare_canonical_corpse_item_candidates as c


class CanonicalCorpseCandidatesTests(unittest.TestCase):
    def fixture(self, xml=None, state=None):
        data = xml or b'<items><item id="10" name="dead rat"><attribute key="containersize" value="36"/><attribute key="duration" value="60"/><attribute key="decayto" value="10"/><attribute key="movable" value="0"/></item></items>'
        binding = {'family': 'Item', 'key': 'oteryn:item.rat', 'revision': 'definition-r1'}
        native = {binding['key']: {'identity': binding, 'semantics': state or {}}}
        tables = {e: c.parse_items(data) for e in c.EPOCHS}
        rows = {(c.EPOCHS[0], 10): binding}
        return binding, tables, rows, native

    def prepare(self, fixture, appearances=None):
        return c.prepare_item(10, *fixture, appearances)

    def test_exact_epoch_preserves_capacity_and_false_without_promoting(self):
        r = self.prepare(self.fixture())
        self.assertEqual(r['selected_source_revision'], c.EPOCHS[0])
        s = r['native_fragment']['semantics']
        self.assertEqual(s['container']['value']['capacity']['value'], 36)
        self.assertEqual(s['physical']['value']['movable']['value'], False)
        self.assertEqual(s['temporal']['value']['duration']['value'], 60000)
        self.assertEqual(s['temporal']['value']['consumption_mode'], c.UNKNOWN)
        self.assertEqual(s['temporal']['value']['decay_target'], c.UNKNOWN)
        self.assertFalse(r['admission_authorized']); self.assertFalse(r['canonical_item_contract_resolved'])
        self.assertIn('SOURCE_CAPACITY_EXCEEDS_D3_16', [d['code'] for d in r['diagnostics']])
        self.assertNotIn('source_terminal_action', r)
        self.assertEqual(r['accepted_item_add_1_3a_preparation_action'], 'remove')
        self.assertTrue(all(set(row['decoder_row']) == {'field_path', 'native_key', 'source_item_id', 'source_value', 'typed_value'} for row in r['promotion_candidates']))

    def test_no_numeric_identity_shortcut_or_cross_epoch_binding(self):
        for replacement in ({'key': 'oteryn:item.wrong'}, {'revision': 'wrong'}, {'family': 'Creature'}):
            b, t, rows, n = self.fixture()
            rows[(c.EPOCHS[0], 10)] = dict(b, **replacement)
            r = self.prepare((b, t, rows, n))
            self.assertIsNone(r['native_fragment']); self.assertFalse(r['promotion_candidates'])

    def test_known_and_conflict_fields_preserved_and_input_not_mutated(self):
        state = {'container': {'state': 'CONFLICT'}, 'physical': {'state': 'KNOWN', 'value':
                 {'weight': {'state': 'KNOWN', 'value': 777}, 'movable': {'state': 'KNOWN', 'value': True}, 'pickupable': c.UNKNOWN}}}
        fixture = self.fixture(state=state); before = copy.deepcopy(fixture)
        r = self.prepare(fixture)
        self.assertEqual(fixture, before)
        s = r['native_fragment']['semantics']
        self.assertEqual(s['container'], state['container']); self.assertEqual(s['physical'], state['physical'])

    def test_missing_xml_and_duplicate_xml_remain_unqualified(self):
        for data in (b'<items/>', b'<items><item id="10" name="a"/><item id="10" name="b"/></items>'):
            self.assertIsNone(self.prepare(self.fixture(data))['native_fragment'])

    def test_unbound_appearance_only_retains_unknown_semantics(self):
        b, t, rows, n = self.fixture(); rows.clear()
        r = self.prepare((b, t, rows, n), {c.EPOCHS[1]: {10: {'name': 'appearance only'}}})
        self.assertIsNone(r['native_fragment']); self.assertFalse(r['promotion_candidates'])

    def test_native_decoder_bounds_and_utf8_bytes(self):
        for path, value in [('presentation.name', 'ż'*1025), ('presentation.name', ''),
                            ('container.capacity', True), ('container.capacity', 65536), ('temporal.duration', 1)]:
            with self.assertRaises(ValueError): c.promotion_row('x', 10, path, value)
        self.assertEqual(c.promotion_row('x', 10, 'container.capacity', 0)['typed_value']['value'], 0)

    def test_transform_requires_same_epoch_native_target_revision(self):
        f = self.fixture(b'<items><item id="10" name="a"><attribute key="decayto" value="11"/></item></items>')
        b, t, rows, n = f; target = dict(b, key='oteryn:item.target')
        rows[(c.EPOCHS[1], 11)] = target; n[target['key']] = {'identity': target, 'semantics': {}}
        self.assertNotIn('temporal', self.prepare(f)['native_fragment']['semantics'])
        rows[(c.EPOCHS[0], 11)] = target
        self.assertEqual(self.prepare(f)['native_fragment']['semantics']['temporal']['value']['decay_target']['value'], {'key': target['key'], 'revision': target['revision']})

    def test_appearance_flags_false_and_xml_override(self):
        appearance = {c.EPOCHS[0]: {10: {'id': 10, 'flags': {'unmove': True, 'take': False}}}}
        r = self.prepare(self.fixture(), appearance)
        self.assertEqual(r['native_fragment']['semantics']['physical']['value']['pickupable']['value'], False)
        self.assertEqual(r['native_fragment']['semantics']['physical']['value']['weight'], c.UNKNOWN)

    def test_negative_duration_not_coerced_and_repeated_scalars_not_collapsed(self):
        f = self.fixture(b'<items><item id="10" name="a"><attribute key="duration" value="-1"/><attribute key="containersize" value="4"/><attribute key="containersize" value="5"/></item></items>')
        r = self.prepare(f)
        self.assertNotIn('temporal', r['native_fragment']['semantics'])
        self.assertNotIn('container', r['native_fragment']['semantics'])
        self.assertEqual(len(r['source_candidates'][0]['xml_records'][0]['fields']['containersize']), 2)

    def test_invalid_source_range_preserved_diagnostic_without_invented_items(self):
        findings = []
        self.assertEqual(c.parse_items(b'<items><item fromid="10" toid="9" name="a"/></items>', findings), {})
        self.assertEqual(findings[0]['attributes']['fromid'], '10')

    def test_authoritative_decay_closure_self_removal_and_cycle_fail_closed(self):
        b, tables, bindings, n = self.fixture()
        closure = c.canonical_decay_closure(10, c.EPOCHS[0], tables, bindings)
        self.assertFalse(closure['source_closure_complete'])
        self.assertEqual(closure['steps'][0]['source_action'], 'transform')
        self.assertEqual(closure['steps'][0]['duration_ms'], 60000)
        tables[c.EPOCHS[0]][10][0]['fields']['decayto'][0]['value'] = '0'
        closure = c.canonical_decay_closure(10, c.EPOCHS[0], tables, bindings)
        self.assertTrue(closure['source_closure_complete'])
        self.assertEqual(closure['steps'][0]['terminal_guard'], {'is_loaded_from_map': 'UNKNOWN_CONTEXT'})
        tables[c.EPOCHS[0]] = c.parse_items(b'<items><item id="10"><attribute key="decayto" value="11"/><attribute key="duration" value="5"/></item><item id="11"><attribute key="decayto" value="10"/><attribute key="duration" value="6"/></item></items>')
        bindings[(c.EPOCHS[0], 11)] = dict(b, key='target')
        closure = c.canonical_decay_closure(10, c.EPOCHS[0], tables, bindings)
        self.assertFalse(closure['source_closure_complete'])
        self.assertEqual(len(closure['steps']), 2)
        self.assertEqual(closure['diagnostics'][-1]['code'], 'NONTERMINAL_SOURCE_DECAY_CYCLE')

    def test_exact_binding_rejects_wrong_family(self):
        row = {'source_key': 'oteryn:source.crystalserver', 'source_revision': c.EPOCHS[0], 'identity_namespace': 'ots/item_server_id', 'disposition': 'EXACT', 'external_id': '10', 'target': {'family': 'Creature', 'key': 'x', 'revision': 'r'}}
        with self.assertRaises(ValueError): c.exact_bindings([row])


if __name__ == '__main__': unittest.main()
