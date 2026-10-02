"""Independent captured JS outputs and source-shape fences for GuildStats.

Golden outputs were run from the unchanged hitsCalc function captured on
2026-10-01, HTML SHA256
0eb3e19200d4e76162df361ff43dcb7d44f9ff2571204799cf914893f444c98f.
They qualify the external calculator comparison, not Game formula acceptance.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import formula_corrections_magic
from guildstats_spells import (compare_bundles, compare_formula, effective_magic_level,
                               extract_snapshot, source_bounds)


# Minimal public numerical facts and helper shapes, not a copy of the webpage.
FIXTURE = """<span x-text="s.min + ' - ' + s.max + ' hp'"></span><script>
function hitsCalc() {
 const mlvlLegsVal = this.mlvlLegs ? 1 : 0;
 const mlvlPotionVal = this.mlvlPotion ? 3 : 0;
 const mlvlCakeVal = this.mlvlCake ? 5 : 0;
 const mlvlSpellVal = this.mlvlSpell ? 1 : 0;
 const ml = mlvlSpellVal + mlvlLegsVal + this.mlvlHelm + this.magic +
 this.mlvlShield + mlvlPotionVal + mlvlCakeVal + this.mlvlArm;
 const r = (a, b) => Math.round((lvl * 0.2) + (ml * a) + b);
 this.spells.energy = [
 { name: 'Energy Strike', words: 'exori vis', min: r(1.403, 8), max: r(2.203, 13) },
 ];
 this.spells.flame = [
 { name: 'Hell\\'s Core', words: 'exevo gran mas flam', min: r(7, 0), max: r(14, 0) },
 ];
 this.spells.other = [
 { name: 'Explosion', words: 'adevo mas hur', min: 0, max: r(4.8, 0) },
 { name: 'Berserk', words: 'exori', min: Math.round((ms + this.weaponAtk) * 0.5 + lvl / 5), max: Math.round((ms + this.weaponAtk) * 1.5 + lvl / 5) },
 ];
}
</script>"""


def corrected_formula():
    original = {'identity': {'key': 'candidate:formula/spell/energy_strike/combat-1',
                             'revision': 'test'},
                'kind': 'player_expression', 'inputs': 'level_magic',
                'minimum': {'const': '1'}, 'maximum': {'const': '2'}}
    return formula_corrections_magic.correct('Energy Strike', 'instant', original, 45)[0]


class GuildStatsComparisonTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = extract_snapshot(FIXTURE)
        self.row = self.snapshot['spells'][0]

    def test_outputs_from_unmodified_independent_javascript(self):
        self.assertEqual(source_bounds(self.row, 200, 13), [66, 82])
        self.assertEqual(source_bounds(self.row, 1101, 50), [298, 343])
        self.assertEqual(source_bounds(self.snapshot['spells'][1], 200, 13), [131, 222])
        self.assertEqual(self.snapshot['spells'][1]['name'], "Hell's Core")
        self.assertEqual(source_bounds(self.snapshot['spells'][2], 1101, 50), [0, 460])

    def test_effective_ml_equipment_and_extras_count_once(self):
        self.assertEqual(effective_magic_level(13, spellbook=4, helmet=2, armor=2,
                                               legs=True, potion=True, cake=True, spell=True), 31)
        self.assertEqual(effective_magic_level(13), 13)
        for invalid in (True, -1, float('nan')):
            with self.assertRaises(ValueError):
                effective_magic_level(invalid)
        with self.assertRaises(ValueError):
            effective_magic_level(13, cake=5)

    def test_js_half_ties_are_not_python_round_or_runtime_truncation(self):
        half = deepcopy(self.row)
        half['minimum'] = half['maximum'] = {'magic_coefficient': 0, 'offset': 0}
        self.assertEqual(source_bounds(half, 2.5, 0), [1, 1])
        self.assertEqual(round(0.5), 0)

    def test_unchanged_snapshot_hash_and_changed_helpers_fail_closed(self):
        digest = hashlib.sha256(FIXTURE.encode()).hexdigest()
        self.assertEqual(extract_snapshot(FIXTURE, digest)['html_sha256'], digest)
        with self.assertRaises(ValueError):
            extract_snapshot(FIXTURE, '0' * 64)
        for old, new in [('lvl * 0.2', 'lvl / 6'), ('Math.round', 'Math.floor'),
                         ('mlvlPotion ? 3', 'mlvlPotion ? 4')]:
            with self.subTest(change=old), self.assertRaises(ValueError):
                extract_snapshot(FIXTURE.replace(old, new))

    def test_skill_maxima_are_explicitly_excluded_from_magic_bounds(self):
        self.assertEqual(len(self.snapshot['spells']), 3)
        self.assertEqual(self.snapshot['unsupported'][0]['name'], 'Berserk')
        self.assertEqual(self.snapshot['semantics']['measurement'], 'spell minimum and maximum')
        self.assertIn('not published', self.snapshot['semantics']['pvp'])

    def test_actual_r18_ast_compared_without_normalizing_external_results(self):
        formula = corrected_formula()
        before = deepcopy(formula)
        rows = compare_formula(self.row, formula, 45, [(200, 13), (1101, 50)])
        self.assertTrue(rows[0]['agrees'])
        self.assertEqual(rows[1]['guildstats'], [298, 343])
        self.assertEqual(rows[1]['authoring'], [278, 323])
        self.assertFalse(rows[1]['agrees'])
        self.assertEqual(formula, before)

    def test_multiple_components_are_skipped_instead_of_selected_implicitly(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / 'instant-energy_strike'
            bundle.mkdir()
            (bundle / 'spell.json').write_text(json.dumps({'spell': {
                'name': 'Energy Strike', 'words': 'exori vis', 'base_power': 45}}))
            dependencies = bundle / 'dependencies.json'
            dependencies.write_text(json.dumps({'formulas': [corrected_formula()]}))
            report = compare_bundles(self.snapshot, directory, [(1101, 50)])
            self.assertEqual(report['total_comparisons'], 1)
            self.assertEqual(report['differences'], 1)
            self.assertIn('html_sha256', report['source'])
            dependencies.write_text(json.dumps({'formulas': [corrected_formula(), corrected_formula()]}))
            report = compare_bundles(self.snapshot, directory)
            self.assertEqual(report['total_comparisons'], 0)
            self.assertIn('multiple', report['skipped_bundles'][0]['reason'])

    def test_extra_authoring_inputs_are_not_guessed(self):
        formula = corrected_formula()
        formula['minimum'] = {'var': 'shielding_skill'}
        with self.assertRaisesRegex(ValueError, 'additional inputs'):
            compare_formula(self.row, formula, 45)


if __name__ == '__main__':
    unittest.main()
