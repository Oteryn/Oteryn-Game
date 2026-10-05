"""Qualification, strict ABI rejection and independent reference fact assertions."""
from copy import deepcopy
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator, ValidationError
import native_delayed as nd


class DelayedTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.texts = {}
        self.hashes = deepcopy(nd.HASHES)
        self.records = {}
        for name, file in nd.FILES.items():
            self.records[name] = {}
            for source in nd.REVISIONS:
                # Source qualification tests use synthetic full inputs, not copied Lua.
                text = f'qualified fixture {source} {name}\n'
                self.texts[source, file] = text
                self.hashes[name][source] = hashlib.sha256(text.encode()).hexdigest()
                self.records[name][source] = {
                    'name': name, 'spell_type': 'instant', 'file': file,
                    'revision': nd.REVISIONS[source], 'source_root': self.root,
                    'blob': hashlib.sha1(b'blob ' + str(len(text.encode())).encode() + b'\0' + text.encode()).hexdigest()}
        self.support = [('support.txt', hashlib.sha256(b'pinned support').hexdigest())]
        (self.root / 'support.txt').write_bytes(b'pinned support')
        self.hash_patch = patch.object(nd, 'HASHES', self.hashes)
        self.support_patch = patch.object(nd, 'SUPPORT', self.support)
        self.hash_patch.start()
        self.support_patch.start()
        self.addCleanup(self.hash_patch.stop)
        self.addCleanup(self.support_patch.stop)

    def build(self, name):
        return nd.build(name, 'instant', self.records[name], self.texts)

    def test_exact_four_candidates_have_independent_facts(self):
        expected = {'death echo': ('delayed_strike', 1000),
                    'divine grenade': ('delayed_strike', 3000),
                    'spiritual outburst': ('delayed_strike', 1000),
                    'divine empowerment': ('owned_field_buff', 5000)}
        self.assertEqual(set(nd.FILES), set(expected))
        for name, (key, duration) in expected.items():
            result = self.build(name)
            self.assertEqual(set(result), {'key', 'parameters'})
            self.assertEqual(result['key'], key)
            self.assertEqual(result['parameters'].get('delay_ms', result['parameters'].get('duration_ms')), duration)
            Draft202012Validator(nd.schemas()[key]).validate(result['parameters'])

    def test_mutated_missing_wrong_revision_and_wrong_carrier_fail_closed(self):
        for name in nd.FILES:
            file = nd.FILES[name]
            for source in nd.REVISIONS:
                bad = dict(self.texts)
                bad[source, file] += 'new operation'
                self.assertIsNone(nd.build(name, 'instant', self.records[name], bad))
                bad.pop((source, file))
                self.assertIsNone(nd.build(name, 'instant', self.records[name], bad))
                bad_records = deepcopy(self.records[name])
                bad_records[source]['revision'] = 'unqualified'
                self.assertIsNone(nd.build(name, 'instant', bad_records, self.texts))
            self.assertIsNone(nd.build(name, 'rune', self.records[name], self.texts))
        self.assertIsNone(nd.build('sap strength', 'instant', {}, {}))

    def test_missing_malformed_or_substituted_git_blob_fails_closed(self):
        for name in nd.FILES:
            for source in nd.REVISIONS:
                for bad_blob in (None, '0' * 40, 'not-a-blob'):
                    records = deepcopy(self.records[name])
                    if bad_blob is None:
                        records[source].pop('blob')
                    else:
                        records[source]['blob'] = bad_blob
                    self.assertIsNone(nd.build(name, 'instant', records, self.texts))
                    self.assertIsNone(nd.evidence(name, 'instant', records, self.texts))

    def test_support_pin_and_missing_root_fail_closed(self):
        records = deepcopy(self.records['death echo'])
        records['canary'].pop('source_root')
        self.assertIsNone(nd.build('death echo', 'instant', records, self.texts))
        (self.root / 'support.txt').write_bytes(b'changed area or wheel rules')
        self.assertIsNone(self.build('death echo'))
        (self.root / 'support.txt').unlink()
        self.assertIsNone(self.build('death echo'))

    def test_snapshot_schema_rejects_unknown_lua_and_weakened_gates(self):
        for name in nd.FILES:
            original = self.build(name)
            schema = nd.schemas()[original['key']]
            for field, value in [('lua_body', 'execute()'), ('patterns', ['delayed_or_repeated'])]:
                changed = deepcopy(original)
                changed['parameters'][field] = value
                with self.assertRaises(ValidationError):
                    Draft202012Validator(schema).validate(changed['parameters'])
            changed = deepcopy(original)
            changed['parameters']['spell_name'] = 'unresolved'
            with self.assertRaises(ValidationError):
                Draft202012Validator(schema).validate(changed['parameters'])
            changed = deepcopy(original)
            changed['parameters'].pop('spell_name')
            with self.assertRaises(ValidationError):
                Draft202012Validator(schema).validate(changed['parameters'])

    def test_online_snapshot_marker_and_harmony_semantics(self):
        death = self.build('death echo')['parameters']
        self.assertEqual(sum(v > 0 for row in death['area'] for v in row), 21)
        self.assertEqual(death['delay_multiplier'], '0.5')
        self.assertTrue(death['requires_caster_online'])
        self.assertTrue(death['ignore_caster_floor'])
        self.assertTrue(death['suppress_charms'])
        self.assertEqual(death['identity_gate'], 'player_id_and_guid')
        grenade = self.build('divine grenade')['parameters']
        self.assertFalse(grenade['immediate_strike'])
        self.assertEqual(grenade['marker_ms'], 3000)
        self.assertFalse(grenade['marker_removal_requires_caster_online'])
        self.assertFalse(grenade['late_damage_buffs_apply'])
        self.assertEqual(grenade['cooldown_ms_by_stage'], [26000, 20000, 14000])
        self.assertEqual(grenade['base_damage_bonus_percent_by_stage'], [0, 16, 32])
        outburst = self.build('spiritual outburst')['parameters']
        self.assertEqual(outburst['full_harmony_value'], 5)
        self.assertEqual(outburst['harmony_gate_timing'], 'at_cast')
        self.assertEqual(outburst['delay_damage_percent_by_stage'], ['37.5', '50', '62.5'])
        self.assertEqual(outburst['chain']['additional_targets'], 7)
        self.assertEqual(outburst['chain']['jump_range_tiles'], 4)

    def test_owner_cleanup_no_foreign_fields_and_evidence_separation(self):
        field = self.build('divine empowerment')['parameters']
        self.assertTrue(field['owner_only'])
        self.assertEqual(field['cleanup'], 'only_created_item_instances')
        self.assertEqual(field['maximum_fields'], 9)
        self.assertEqual(field['bonus_damage_percent_by_stage'], [8, 10, 12])
        for name in nd.FILES:
            execution = self.build(name)
            metadata = nd.evidence(name, 'instant', self.records[name], self.texts)
            self.assertEqual(metadata['runtime_admission'], 'rejected_until_native_contract')
            self.assertTrue(metadata['unresolved_decisions'])
            self.assertEqual(len(metadata['source_evidence']), 2)
            self.assertTrue(all(e['authority'] == 'OtsHypothesisOnly' for e in metadata['source_evidence']))
            self.assertFalse(set(nd.METADATA_FIELDS).intersection(execution['parameters']))
            execution['parameters']['spell_name'] = 'changed'
            self.assertEqual(self.build(name)['parameters']['spell_name'], name)


if __name__ == '__main__':
    unittest.main()
