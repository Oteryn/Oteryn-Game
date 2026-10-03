"""Acceptance and source-conservation checks for the SW-3/4/5 transcription."""
import copy
import sys
import os
import unittest
from fractions import Fraction
import json
import tempfile
from jsonschema import Draft202012Validator
from pathlib import Path

import prepare_soul_war_accepted as sw
import validate_encounter as ve


@unittest.skipUnless(os.environ.get('OTERYN_CANARY'), 'set OTERYN_CANARY to the pinned checkout')
class AcceptedSoulWar(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.artifacts = sw.prepare(Path(os.environ['OTERYN_CANARY']))

    def test_actual_sample_contains_pinned_transcription_and_repeats_without_growth(self):
        baseline = sw.ROOT / 'samples/soul_war_taint_zones'
        for name in ('encounter.json', 'manifest.json', 'catalog.json'):
            self.assertEqual(self.artifacts[name], json.loads((baseline / name).read_text()))
        second = sw.prepare(Path(os.environ['OTERYN_CANARY']))
        self.assertEqual(self.artifacts, second)
        location = json.loads((baseline / 'encounter.json').read_text())['anchors'][0]['location']
        self.assertEqual(23, len(location['boxes']))
        self.assertEqual(5, len(location['minus']))
        self.assertTrue(any(box['x'] == [33901, 33980] for box in location['boxes']))

    def test_changed_working_registration_is_rejected_before_roster_selection(self):
        with tempfile.TemporaryDirectory() as directory:
            sparse = Path(directory)
            (sparse / '.git').symlink_to(Path(os.environ['OTERYN_CANARY']) / '.git', target_is_directory=True)
            path = sw.MONSTERS + sw.TELEPORTERS['bony_sea_devil']
            content = sw.pinned_bytes(sparse, path)
            self.assertIn(b'FourthTaintBossesPrepareDeath', content)
            local = sparse / path
            local.parent.mkdir(parents=True)
            local.write_bytes(content.replace(b'FourthTaintBossesPrepareDeath', b'RemovedEvent'))
            with self.assertRaisesRegex(ValueError, 'source working bytes differ.*bony_sea_devil'):
                sw.prepare(sparse)

    def test_absent_working_monsters_keep_the_complete_pinned_roster(self):
        with tempfile.TemporaryDirectory() as directory:
            sparse = Path(directory)
            (sparse / '.git').symlink_to(Path(os.environ['OTERYN_CANARY']) / '.git', target_is_directory=True)
            self.assertFalse((sparse / sw.MONSTERS).exists())
            actual = sw.prepare(sparse)
            self.assertEqual(self.artifacts, actual)
            self.assertIn('canary:creature/bony_sea_devil',
                          actual['manifest.json']['covers']['FourthTaintBossesPrepareDeath'])

    def test_actual_sample_maps_through_existing_native_bridge_but_sw6_blocks_admission(self):
        sys.path.insert(0, str(sw.ROOT.parents[1] / 'content-migration'))
        import creature_admission_stage as stage
        e = json.loads((sw.ROOT / 'samples/soul_war_taint_zones/encounter.json').read_text())
        profile = stage.encounter_details(e, stage.Mapper({}))
        self.assertEqual(23, len(profile['anchors'][0]['location']['boxes']))
        self.assertEqual(5, len(profile['anchors'][0]['location']['minus']))
        teleport = next(rule for rule in profile['rules'] if rule['key'] == 'bony_sea_devil_taint_teleport')
        self.assertEqual('picked_position', teleport['actions'][0]['to']['kind'])
        self.assertEqual(2000, teleport['actions'][0]['after_ms'])
        self.assertEqual(10000, teleport['actions'][0]['picked_cooldown_ms'])
        self.assertEqual({'kind': 'candidate'}, teleport['conditions'][0]['where'][0]['subject'])
        floor = next(rule for rule in profile['rules'] if rule['key'] == 'mirror_image_floor')
        self.assertFalse(floor['conditions'][0]['value'])
        _, waiting = stage.load_encounters({})
        self.assertEqual('unresolved_semantics', waiting['soul_war_taint_zones'])

    def test_complete_transcription_validates_but_sw6_and_e4_still_block_readiness(self):
        e, m, c, r = (self.artifacts[name] for name in
                      ('encounter.json', 'manifest.json', 'catalog.json', 'receipt.json'))
        self.assertEqual([], ve.validate(e, c, m))
        self.assertTrue(r['schema_valid'] and r['semantic_valid'])
        self.assertFalse(r['admission_authorized'] or r['native_ready'] or r['runtime_qualified'])
        self.assertEqual(1, sum(row['status'] == 'unresolved_semantics' for row in m['entries']))
        self.assertNotIn('CloakOfTerrorHealthLoss', m['covers'])
        self.assertFalse(any(action['kind'] == 'map_item' for rule in e['rules'] for action in rule['actions']))

    def test_mirror_uses_last_registration_and_exact_accepted_vocation_probabilities(self):
        e = self.artifacts['encounter.json']
        hunting = next(p for p in e['participants'] if p['role'] == 'hunting_monster')
        self.assertNotIn('canary:creature/mirror_image', [r['key'] for r in hunting['creatures']])
        m = self.artifacts['manifest.json']
        self.assertNotIn('canary:creature/mirror_image', m['covers']['FourthTaintBossesPrepareDeath'])
        rules = [rule for rule in e['rules'] if 'base_vocation' in rule['trigger']]
        self.assertEqual(set(sw.VOCATIONS), {r['trigger']['base_vocation'] for r in rules})
        for rule in rules:
            branches = rule['actions'][0]['branches']
            total = sum(branch['weight'] for branch in branches)
            selected = rule['trigger']['base_vocation']
            for branch in branches:
                action = branch['actions'][0]
                probability = Fraction(branch['weight'], total)
                self.assertEqual(Fraction(7, 10) if selected + '_s_apparition' in action['into']['key']
                                 else Fraction(3, 40), probability)
                self.assertEqual('full', action['health'])
        floor = next(rule for rule in e['rules'] if rule['key'] == 'mirror_image_floor')
        self.assertEqual([{'kind': 'killer_is_player', 'value': False}], floor['conditions'])

    def test_zone_keeps_full_rotten_rectangle_boss_rooms_and_safe_holes(self):
        location = self.artifacts['encounter.json']['anchors'][0]['location']
        def member(x, y, floor):
            def contains(box):
                return box['floor'] == floor and box['x'][0] <= x <= box['x'][1] and box['y'][0] <= y <= box['y'][1]
            return any(map(contains, location['boxes'])) and not any(map(contains, location['minus']))
        self.assertTrue(member(33901, 30986, 11))  # reversed corner corrected by Q2a
        self.assertTrue(member(33980, 31105, 12))
        self.assertFalse(member(33970, 31040, 11))  # Rotten safe area
        self.assertFalse(member(34005, 31010, 9))   # Inferno safe area
        self.assertTrue(member(33743, 31632, 14))   # Spite boss room
        self.assertTrue(member(33856, 31866, 7))    # Cruelty boss room
        self.assertFalse(member(1, 1, 1))

    def test_teleport_covers_are_complete_source_callbacks_and_owner_selected_checks(self):
        e = self.artifacts['encounter.json']
        timer = e['state']['timers'][0]
        self.assertEqual({'name': 'taint_check', 'duration_ms': 2000, 'repeat': True}, timer)
        rules = [r for r in e['rules'] if r['key'].endswith('_taint_teleport')]
        self.assertEqual(7, len(rules))
        for rule in rules:
            pick = rule['conditions'][0]
            self.assertEqual('farthest', pick['pick'])
            self.assertTrue(pick['where'][0]['value'])
            self.assertEqual('canary:quest-progress/soul_war_taint_1', pick['where'][0]['progress'])
            self.assertEqual(10, rule['conditions'][1]['value'])
            self.assertEqual(2000, rule['actions'][0]['after_ms'])
            self.assertEqual(10000, rule['actions'][0]['picked_cooldown_ms'])
        covers = self.artifacts['manifest.json']['covers']['mType.onThink']
        self.assertEqual(6, len(covers))
        self.assertNotIn('canary:creature/dreadful_harvester', covers)  # added by accepted wiki decision, not source callback

    def test_transcription_rejects_missing_pick_and_incomplete_zone_census(self):
        e = copy.deepcopy(self.artifacts['encounter.json'])
        teleport = next(r for r in e['rules'] if r['key'].endswith('_taint_teleport'))
        teleport['conditions'].clear()
        self.assertTrue(any('earlier creature_present with pick' in error for error in ve.validate(e)))
        with self.assertRaises(ValueError):
            sw.zone_location('no pinned zones')

    def test_sw6_pool_source_data_is_complete_without_choosing_creation_policy(self):
        pool = self.artifacts['pool-source-preparation.json']
        self.assertEqual([33854, 34006, 34007],
                         [int(r['item']['key'].rsplit('/', 1)[1]) for r in pool['source_items']])
        self.assertEqual([600000, 300000, 300000], [r['duration_ms'] for r in pool['source_items']])
        self.assertEqual([34006, 34007, 0], [r['decay_target_source_id'] for r in pool['source_items']])
        shares = [Fraction(r['player_max_health_share']['numerator'], r['player_max_health_share']['denominator'])
                  for r in pool['source_items']]
        self.assertEqual([Fraction(1, 5), Fraction(3, 20), Fraction(1, 10)], shares)
        self.assertEqual(1500, pool['cloak_step_in']['heal_minimum'])
        self.assertEqual(2000, pool['cloak_step_in']['heal_maximum'])
        self.assertTrue(pool['every_creature_step_in']['remove_item'])
        self.assertEqual('UNDECIDED', pool['creation_policy']['unless_present'])
        self.assertFalse(pool['admission_authorized'] or pool['runtime_qualified'])
        self.assertEqual(17, self.artifacts['receipt.json']['e4_cohort']['creature_count'])
        self.assertFalse(self.artifacts['receipt.json']['e4_cohort']['admission_ready'])
        schema = json.loads((sw.ROOT.parent / 'monster-authoring/custom-pattern-preparation.schema.json').read_text())
        validator = Draft202012Validator(schema['$defs']['poolSourcePreparation'])
        self.assertEqual([], list(validator.iter_errors(pool)))
        guessed = copy.deepcopy(pool)
        guessed['creation_policy']['unless_present'] = True
        self.assertTrue(list(validator.iter_errors(guessed)))


if __name__ == '__main__':
    unittest.main()
