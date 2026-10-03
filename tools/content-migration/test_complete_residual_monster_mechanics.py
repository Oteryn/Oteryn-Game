"""Bounded source-proxy checks, distinct from native gameplay qualification."""
import hashlib
import tempfile
import unittest
from pathlib import Path

import complete_remaining_monster_mechanics as core
import complete_residual_monster_mechanics as residual

REPO = Path(__file__).resolve().parents[2]
BASE = Path('/workspace/monster-mitigation-estimate-output/population-v2')
PREVIOUS = Path('/workspace/monster-final-output/mechanics')
CANARY = Path('/workspace/monster-reference-sources/canary')


@unittest.skipUnless(PREVIOUS.exists() and BASE.exists() and CANARY.exists(), 'pinned source fixture unavailable')
class ResidualCoreTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.out = Path(cls.temp.name) / 'candidate'
        cls.old_hash = hashlib.sha256((PREVIOUS / 'completion.json').read_bytes()).hexdigest()
        cls.receipt = residual.extend(REPO, BASE, PREVIOUS, CANARY, cls.out)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def encounter(self, name):
        return core.read(self.out / 'encounters' / name / 'encounter.json')

    def test_previous_candidate_and_global_stats_preserved(self):
        self.assertEqual(self.old_hash, hashlib.sha256((PREVIOUS / 'completion.json').read_bytes()).hexdigest())
        for actor in self.receipt['actors']:
            name = actor['monster']
            old = core.read(BASE / 'bundles' / name / 'monster.json')
            new = core.read(self.out / 'bundles' / name / 'monster.json')
            self.assertEqual(old['creature']['stats'], new['creature']['stats'])
        self.assertEqual(self.receipt['counts']['restored_components'] + len(self.receipt['remaining']), 50)

    def test_gaz_growth_cannot_exceed_seven_owned_minions(self):
        rules = [r for r in self.encounter('gaz_haragoth')['rules'] if r['key'].startswith('gaz_summons_')]
        for count in range(9):
            matched = []
            for r in rules:
                valid = all((count == c['value'] if c['op'] == '==' else
                             count >= c['value'] if c['op'] == '>=' else count < c['value'])
                            for c in r['conditions'] if c['kind'] == 'summon_count')
                if valid:
                    matched.append(r)
            self.assertLessEqual(len(matched), 1)
            for r in matched:
                self.assertLessEqual(count + r['actions'][0]['count'], 7)

    def test_phase_proxy_preserves_health_and_has_return(self):
        e = self.encounter('the_time_guardian')
        phases = [r for r in e['rules'] if r['key'].endswith('_proxy_phase')]
        returns = [r for r in e['rules'] if r['key'].endswith('_proxy_returns')]
        self.assertEqual(len(phases), 2)
        self.assertEqual([r['delay_ms'] for r in returns], [30000, 30000])
        for r in phases:
            self.assertEqual([b['actions'][0]['health'] for b in r['actions'][0]['branches']],
                             ['keep_absolute', 'keep_absolute'])

    def test_all_encounters_translate_with_exact_native_ppm_and_registered_items(self):
        import creature_admission_stage as stage
        cache = core.read(Path('/workspace/monster-round7-output/native-item-map-rust.json'))
        mapper = stage.Mapper({r['source_item_id']: r['native_key'] for r in cache['records']})
        translated = {row['encounter']: stage.encounter_details(self.encounter(row['encounter']), mapper)
                      for row in self.receipt['encounters']}
        self.assertEqual(len(translated), 22)
        growth = next(r for r in translated['gaz_haragoth']['rules'] if r['key'] == 'gaz_summons_grows')
        chance = next(c['value_ppm'] for c in growth['conditions'] if c['kind'] == 'chance_percent')
        self.assertEqual(chance, 247525)
        self.assertEqual(self.receipt['probability_quantization'][0]['source_percent_ratio'],
                         {'numerator': 2500, 'denominator': 101})

    def test_exact_debt_classified_and_zero_damage_not_assumed_noop(self):
        seacrest = self.receipt['remaining_classification']['SEACREST_DEFENSIVE_MELEE']
        self.assertIn('shield block budget', seacrest['reason'])
        self.assertEqual(len(seacrest['rows']), 1)
        self.assertEqual(sum(len(g['rows']) for g in self.receipt['remaining_classification'].values()), 14)
        self.assertEqual(self.receipt['source_noop_components'], 0)
        self.assertTrue(self.receipt['exact_parity_debt_retained'])


if __name__ == '__main__':
    unittest.main()
