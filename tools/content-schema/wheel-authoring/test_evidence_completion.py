import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from jsonschema.exceptions import ValidationError
import wheel_authoring as wheel
import client_icons


class EvidenceCompletionTests(unittest.TestCase):
    def setUp(self):
        self.previous_bytes = (wheel.ROOT / 'samples/wheel-candidate.json').read_bytes()
        self.candidate = wheel.decode_json(self.previous_bytes)

    def test_failed_replay_and_conflicting_live_claims_are_not_qualified(self):
        evidence = wheel.read(wheel.ROOT / 'samples/verification-evidence.json')
        for section, field, value, code in [
            ('planner_replay', 'result', 'FAIL: source mismatch', 'EVIDENCE_REPLAY_RESULT'),
            ('live_verification', 'tibiapal', False, 'EVIDENCE_LIVE_FLAGS'),
            ('live_verification', 'fandom', False, 'EVIDENCE_LIVE_FLAGS'),
            ('live_verification', 'static_sprite_sheets', False, 'EVIDENCE_SPRITE_COVERAGE'),
            ('live_verification', 'remaining_sprite_categories', ['conviction'], 'EVIDENCE_SPRITE_COVERAGE')]:
            packet = copy.deepcopy(evidence)
            packet[section][field] = value
            with tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / 'evidence.json'
                path.write_text(json.dumps(packet))
                with self.subTest(field=field), self.assertRaisesRegex(ValueError, code):
                    wheel.validate_evidence(self.candidate,
                        (wheel.ROOT / 'samples/wheel-candidate.json').read_bytes(), path)

    def test_successor_refuses_invalid_historical_envelopes(self):
        successor = copy.deepcopy(self.candidate)
        successor.update(revision='r2', release={'kind': 'wheel_reset',
                                               'predecessor': self.candidate['revision'],
            'predecessor_sha256': hashlib.sha256(self.previous_bytes).hexdigest()})
        for field, value, code in [('schema', 'UNRECOGNIZED', 'PREDECESSOR_ENVELOPE'),
                ('runtime_admitted', True, 'PREDECESSOR_ENVELOPE'),
                ('revision', 'r' * 129, 'PREDECESSOR_ENVELOPE'),
                ('revision', None, 'PREDECESSOR_ENVELOPE'),
                ('historical_note', float('nan'), 'NON_FINITE_NUMBER')]:
            previous = copy.deepcopy(self.candidate)
            previous[field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, code):
                wheel.validate(successor, previous, self.previous_bytes)

    def test_historical_perk_enums_are_not_revalidated_against_current_capture(self):
        previous = copy.deepcopy(self.candidate)
        effect = next(slot['conviction']['unique_parameters']['numeric_effects'][0]
            for vocation in previous['vocations'].values() for slot in vocation['slots']
            if slot['conviction']['unique_parameters'] and
                slot['conviction']['unique_parameters']['numeric_effects'])
        effect['kind'] = 'historical_perk_kind'
        previous_bytes=json.dumps(previous,allow_nan=False).encode()
        successor = copy.deepcopy(self.candidate)
        successor.update(revision='r2', release={'kind': 'wheel_reset',
            'predecessor': previous['revision'],
            'predecessor_sha256': hashlib.sha256(previous_bytes).hexdigest()})
        wheel.validate(successor, previous, previous_bytes)

    def test_manifest_byte_entry_point_refuses_duplicate_fields(self):
        payload=json.dumps(self.candidate).replace('"runtime_admitted": false',
            '"runtime_admitted": true, "runtime_admitted": false',1).encode()
        with self.assertRaisesRegex(ValueError,'DUPLICATE_JSON_KEY'):
            client_icons.build_manifest(self.candidate,payload)

    def test_runic_base_rule_and_grade_iv_reference_are_complete(self):
        audit=wheel.read(wheel.ROOT/'samples/browser-source-audit.json')
        limits=audit['catalogue_corroboration']['maximum_grade_iv_points_per_vocation']
        for vocation,data in self.candidate['vocations'].items():
            basic=len(set(data['basic_mods_position_1'])|set(data['basic_mods_position_2']))
            supreme=len(set(data['supreme_mods']))
            self.assertEqual(limits[vocation],basic+supreme)
            self.assertEqual(limits[vocation],69)
            for slot in data['slots']:
                if slot['conviction']['key']=='runic_mastery':
                    self.assertIn('magic_level_bonus_uses_base_magic_level',slot['conviction']['unique_parameters']['behaviors'])

    def test_atelier_operation_guards_cannot_be_disabled(self):
        for field in ('atelier_actions_require_wheel_eligibility','atelier_commands_require_initial_gem_grant',
                      'vessel_requires_character_owned_gem','vessel_requires_current_gem_ruleset',
                      'rejected_operations_write_nothing','non_current_gem_and_grade_effects_zero',
                      'replay_requires_matching_request_binding'):
            candidate=copy.deepcopy(self.candidate)
            candidate['gems']['atelier']['operation_policy'][field]=False
            with self.subTest(field=field),self.assertRaises(ValidationError):
                wheel.validate(candidate)
