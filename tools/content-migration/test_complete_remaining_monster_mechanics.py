"""Checks the selected source mechanics without claiming native combat parity."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import complete_remaining_monster_mechanics as mechanics

REPO = Path(__file__).resolve().parents[2]
BASE = Path('/workspace/monster-mitigation-estimate-output/population-v2')
CANARY = Path('/workspace/monster-reference-sources/canary')
INVENTORY = BASE.parent / 'remaining-readiness-mechanics.json'


@unittest.skipUnless(BASE.exists() and CANARY.exists(), 'pinned research input fixture unavailable')
class SourceCoreTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.output = Path(cls.temp.name) / 'candidate'
        cls.before = hashlib.sha256((BASE / 'population-index.json').read_bytes()).hexdigest()
        cls.receipt = mechanics.prepare(REPO, BASE, CANARY, INVENTORY, cls.output)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def encounter(self, name):
        return mechanics.read(self.output / 'encounters' / name / 'encounter.json')

    def test_audit_is_accounted_and_input_is_immutable(self):
        self.assertEqual(self.receipt['counts']['restored_components'] + len(self.receipt['remaining']), 50)
        self.assertEqual(self.before, hashlib.sha256((BASE / 'population-index.json').read_bytes()).hexdigest())
        for row in self.receipt['actors']:
            old = mechanics.read(BASE / 'bundles' / row['monster'] / 'monster.json')
            new = mechanics.read(self.output / 'bundles' / row['monster'] / 'monster.json')
            self.assertEqual(old['creature']['stats'], new['creature']['stats'])
            self.assertEqual(old.get('loot'), new.get('loot'))

    def test_heal_cooldown_blocks_rescheduling(self):
        e = self.encounter('professor_maxxen')
        starts = next(r for r in e['rules'] if r['key'] == 'fairy_heal_starts')
        self.assertIn({'kind': 'flag', 'flag': 'fairy_regeneration_cooldown', 'value': False}, starts['conditions'])
        self.assertEqual({t['name']: t['duration_ms'] for t in e['state']['timers'] if t['name'].startswith('fairy_')},
                         {'fairy_regeneration_reset': 30000, 'fairy_delayed_heal': 10000})

    def test_death_blast_targets_players_and_keeps_death_position(self):
        e = self.encounter('wormling')
        self.assertEqual(e['abilities'][0]['damage'], {'damage_type': 'earth', 'min': 750, 'max': 750})
        self.assertEqual(e['rules'][0]['actions'][0]['at'], 'death_position')
        self.assertFalse(e['abilities'][0]['affects']['creatures'])

    def test_pool_policy_is_explicit_and_source_sign_bug_not_copied(self):
        e = self.encounter('soul_war_taint_zones')
        r = next(r for r in e['rules'] if r['key'] == 'cloak_of_terror_bleeds')
        self.assertEqual(r['trigger']['kind'], 'damage_taken')
        self.assertTrue(r['actions'][0]['unless_present'])
        actor = next(a for a in self.receipt['actors'] if a['monster'] == 'cloak_of_terror')
        self.assertIn('GLOBAL_POOL_STACKING_UNVERIFIED', actor['completion_flags'])

    def test_cannot_overwrite_candidate_or_source(self):
        for output in (self.output, BASE):
            with self.assertRaises(ValueError):
                mechanics.prepare(REPO, BASE, CANARY, INVENTORY, output)


if __name__ == '__main__':
    unittest.main()
