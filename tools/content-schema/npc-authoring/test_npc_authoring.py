"""Offline checks for the NPC authoring validator and source diff (no Canary/Crystal checkout needed).

Run: python -m unittest test_npc_authoring.py (from this directory).
"""
import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator

import source_diff
import validate_npc

ROOT = Path(__file__).resolve().parent
SAMPLES = ROOT / 'samples' / 'bundles'
SCHEMA = Draft202012Validator(json.loads((ROOT / 'npc.schema.json').read_text(encoding='utf-8')))


def load(source, name):
    return json.loads((SAMPLES / source / f'{name}.json').read_text(encoding='utf-8'))


def errors(bundle, allow_text=False):
    found = [e.message for e in SCHEMA.iter_errors(bundle)]
    return found or validate_npc.semantic_errors(bundle, allow_text)


class ValidatorTest(unittest.TestCase):
    def test_samples_are_valid(self):
        paths = sorted(SAMPLES.rglob('*.json'))
        self.assertGreaterEqual(len(paths), 16)
        for path in paths:
            with self.subTest(path=path.name):
                self.assertEqual(errors(json.loads(path.read_text(encoding='utf-8'))), [])

    def test_text_is_rejected_unless_allowed(self):
        bundle = load('canary', 'sam')
        bundle['dialogue']['messages']['greet']['text'] = 'Hello.'
        self.assertTrue(any('carries text' in e for e in errors(bundle)))
        self.assertEqual(errors(bundle, allow_text=True), [])

    def test_gate_needs_unresolved_row(self):
        bundle = load('canary', 'captain_bluebear')
        bundle['unresolved'] = [r for r in bundle['unresolved'] if r['reason'] != 'LUA_PREDICATE']
        self.assertTrue(any('gate without unresolved row' in e for e in errors(bundle)))

    def test_resolved_status_needs_no_unresolved_rows(self):
        bundle = load('canary', 'sam')
        bundle['status'] = 'RESOLVED'
        self.assertTrue(any('RESOLVED exactly when' in e for e in errors(bundle)))

    def test_key_must_match_source(self):
        bundle = load('canary', 'sam')
        bundle['key'] = 'crystal:npc/sam'
        self.assertTrue(any('does not match source' in e for e in errors(bundle)))

    def test_travel_rows_carry_static_destinations(self):
        bundle = load('canary', 'captain_bluebear')
        travel = {row['keyword']: row for row in bundle['services']['travel']}
        self.assertIn('carlin', travel)
        self.assertEqual(travel['carlin']['price'], 110)
        self.assertEqual(travel['carlin']['destination'], {'x': 32387, 'y': 31820, 'z': 6})
        self.assertEqual(travel['yalahar']['gate'], 'LUA_PREDICATE')


class SourceDiffTest(unittest.TestCase):
    def test_identical_bundles_have_only_same_facts(self):
        bundle = load('canary', 'sam')
        facts = source_diff.facts(bundle)
        for section in source_diff.SECTIONS:
            counts, conflicts = source_diff.compare(facts[section], copy.deepcopy(facts[section]))
            self.assertEqual(conflicts, [])
            self.assertEqual(set(counts) - {'SAME'}, set())

    def test_price_change_is_a_conflict(self):
        left = load('canary', 'sam')
        right = copy.deepcopy(left)
        right['services']['trade']['offers'][0]['buy_price'] = 999999
        counts, conflicts = source_diff.compare(source_diff.facts(left)['trade'], source_diff.facts(right)['trade'])
        self.assertEqual(counts['CONFLICT'], 1)
        self.assertEqual(len(conflicts), 1)

    def test_missing_field_is_one_sided(self):
        left = load('canary', 'sam')
        right = copy.deepcopy(left)
        right['definition']['profession'] = None
        counts, _ = source_diff.compare(source_diff.facts(left)['definition'], source_diff.facts(right)['definition'])
        self.assertEqual(counts['CANARY_ONLY'], 1)
        self.assertNotIn('CONFLICT', counts)


if __name__ == '__main__':
    unittest.main()
