#!/usr/bin/env python3
"""The committed attack authoring must cover every attacks[] reference of the profile artifact."""
import copy
import json
import unittest

import build_monster_attack_authoring as gen
import validate_monster_attack_authoring as validator


class CommittedAuthoring(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.profiles_raw = (gen.ROOT / gen.PROFILES).read_bytes()
        cls.rows_raw = (gen.ROOT / gen.RECORDS).read_bytes()
        cls.pin = json.loads((gen.ROOT / gen.PIN).read_bytes())
        cls.rows = [json.loads(line) for line in cls.rows_raw.decode().splitlines()]

    def errors(self, rows=None, pin=None, profiles_raw=None):
        raw = self.rows_raw if rows is None else b''.join(gen.canonical(r) + b'\n' for r in rows)
        pin = copy.deepcopy(self.pin) if pin is None else pin
        if rows is not None:
            pin['records']['sha256'] = gen.sha256(raw)
        return validator.validate(profiles_raw or self.profiles_raw, raw, pin)

    def test_every_attack_reference_has_authoring_or_an_explicit_reason(self):
        self.assertEqual(validator.validate_repository(), [])
        references = validator.attack_references(json.loads(self.profiles_raw))
        self.assertEqual({r['ability']['key'] for r in self.rows}, set(references))
        self.assertGreater(self.pin['records']['authored'], 5000)

    def test_missing_reference_fails(self):
        errors = self.errors(rows=self.rows[1:])
        self.assertTrue(any(e.startswith('attack reference lacks authoring') for e in errors))

    def test_corrupted_archive_pin_fails(self):
        pin = copy.deepcopy(self.pin)
        pin['source_archive']['sha256'] = '0' * 64
        archive = (gen.ROOT / gen.ARCHIVE).read_bytes()
        self.assertEqual(validator.validate(self.profiles_raw, self.rows_raw, self.pin, archive), [])
        errors = validator.validate(self.profiles_raw, self.rows_raw, pin, archive)
        self.assertTrue(any('archive digest' in e for e in errors))

    def test_heal_effects_carry_resolved_magnitudes(self):
        heals = [e for r in self.rows if r['status'] == 'authored' for e in r['effects'] if e['operation'] == 'heal']
        self.assertTrue(heals)
        self.assertTrue(all('minimum' in e and 'maximum' in e for e in heals))

    def test_stale_artifact_digest_fails(self):
        errors = self.errors(profiles_raw=self.profiles_raw + b' ')
        self.assertTrue(any('artifact digest' in e for e in errors))

    def test_authored_damage_requires_element_and_ordered_range(self):
        index = next(i for i, r in enumerate(self.rows)
                     if r['status'] == 'authored' and r['effects'][0]['operation'] == 'damage')
        for mutate in (lambda e: e.update(minimum=e['maximum'] + 1), lambda e: e.pop('damage_type'),
                       lambda e: e.update(maximum=-1)):
            rows = copy.deepcopy(self.rows)
            mutate(rows[index]['effects'][0])
            self.assertTrue(self.errors(rows=rows), mutate)

    def test_authored_requires_evidence_and_matching_schedule(self):
        index = next(i for i, r in enumerate(self.rows) if r['status'] == 'authored')
        for mutate in (lambda r: r.pop('evidence'), lambda r: r['evidence'].update(source_revision='x'),
                       lambda r: r['schedules'][0].update(interval_ms=r['schedules'][0]['interval_ms'] + 1),
                       lambda r: r.update(creatures=[])):
            rows = copy.deepcopy(self.rows)
            mutate(rows[index])
            self.assertTrue(self.errors(rows=rows), mutate)

    def test_unavailable_stays_explicit(self):
        index = next(i for i, r in enumerate(self.rows) if r['status'] == 'unavailable')
        rows = copy.deepcopy(self.rows)
        del rows[index]['reason']
        self.assertTrue(self.errors(rows=rows))
        rows = copy.deepcopy(self.rows)
        rows[index]['effects'] = []
        self.assertTrue(self.errors(rows=rows))

    def test_unavailable_reasons_are_closed(self):
        allowed = {'attack_count_differs_from_donor', 'conflicting_creature_bindings', 'donor_ability_has_no_effects',
                   'donor_health_conflict', 'no_unique_donor_bundle', 'source_provenance_ambiguous',
                   'donor_ability_absent', 'donor_effect_absent', 'schedule_differs_from_donor',
                   'attack_slot_suffix_differs'}
        self.assertLessEqual({r['reason'] for r in self.rows if r['status'] == 'unavailable'}, allowed)

    @unittest.skipUnless((gen.ROOT / gen.ARCHIVE).exists(), 'prepared population archive absent')
    def test_committed_files_equal_regeneration(self):
        records, pin = gen.generate()
        self.assertEqual(records, self.rows_raw)
        self.assertEqual(pin, (gen.ROOT / gen.PIN).read_bytes())


class DerivationRules(unittest.TestCase):
    def fixture(self, formula, schedule_extra=None):
        ref = {'family': 'Ability', 'key': 'canary:ability/x/attack-1', 'revision': 'r'}
        effect_ref = {'family': 'Effect', 'key': 'canary:effect/x/attack-1', 'revision': 'r'}
        formula_ref = {'family': 'Formula', 'key': 'canary:formula/x/attack-1-1', 'revision': 'r'}
        monster = {'behavior': {'attacks': [dict({'ability': ref, 'interval_ms': 2000, 'chance_percent': 50},
                                                 **(schedule_extra or {}))]}}
        deps = {'abilities': [{'identity': ref, 'kind': 'spell', 'range_tiles': 5, 'needs_target': True,
                               'needs_direction': False, 'area': {'radius_tiles': 2}, 'effects': [effect_ref]}],
                'effects': [{'identity': effect_ref, 'operation': 'damage', 'damage_type': 'fire',
                             'formula': formula_ref}],
                'formulas': [dict({'identity': formula_ref}, **formula)]}
        manifest = {'sources': [{'repository': 'opentibiabr/canary', 'revision': 'a' * 40}],
                    'entries': [{'destination': '/monster/behavior/attacks/0', 'status': 'mapped',
                                 'source_file': 'data-otservbr-global/monster/x.lua', 'source_line': 3}]}
        bundle = {'monster.json': json.dumps(monster).encode(), 'dependencies.json': json.dumps(deps).encode(),
                  'manifest.json': json.dumps(manifest).encode()}
        attack = {'ability': {'key': 'oteryn:ability.creature.x.attack-1'}, 'interval_ms': 2000, 'chance_ppm': 500000}
        return bundle, attack

    def test_range_area_element_and_evidence_are_exported(self):
        bundle, attack = self.fixture({'kind': 'range', 'magnitude': {'minimum': 10, 'maximum': 20}})
        result, reason = gen.derive(None, 0, bundle, attack)
        self.assertIsNone(reason)
        authoring = result['authoring']
        self.assertEqual((authoring['range_tiles'], authoring['area']), (5, {'radius_tiles': 2}))
        effect = authoring['effects'][0]
        self.assertEqual((effect['damage_type'], effect['minimum'], effect['maximum']), ('fire', 10, 20))
        self.assertEqual(result['evidence']['source_line'], 3)

    def test_caster_magnitude_needs_schedule_magnitude(self):
        bundle, attack = self.fixture({'kind': 'caster_magnitude'})
        self.assertEqual(gen.derive(None, 0, bundle, attack)[1], 'caster_magnitude_without_schedule_magnitude')
        bundle, attack = self.fixture({'kind': 'caster_magnitude'}, {'magnitude': {'minimum': 3, 'maximum': 9}})
        effect = gen.derive(None, 0, bundle, attack)[0]['authoring']['effects'][0]
        self.assertEqual((effect['minimum'], effect['maximum']), (3, 9))

    def test_melee_skill_matches_the_pinned_engine_edge(self):
        bundle, attack = self.fixture({'kind': 'melee_attack_skill', 'melee': {'attack': 61, 'skill': 30}})
        effect = gen.derive(None, 0, bundle, attack)[0]['authoring']['effects'][0]
        self.assertEqual((effect['minimum'], effect['maximum']), (0, 123))

    def test_unknown_formula_and_schedule_drift_stay_unavailable(self):
        bundle, attack = self.fixture({'kind': 'speed_modifier'})
        self.assertTrue(gen.derive(None, 0, bundle, attack)[1].startswith('damage_formula_kind_unsupported'))
        bundle, attack = self.fixture({'kind': 'range', 'magnitude': {'minimum': 1, 'maximum': 2}})
        attack['interval_ms'] = 1000
        self.assertEqual(gen.derive(None, 0, bundle, attack)[1], 'schedule_differs_from_donor')


if __name__ == '__main__':
    unittest.main()
