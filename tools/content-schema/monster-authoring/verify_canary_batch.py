"""Boundary checks for the pinned-source converter; no third-party Lua is executed."""
import unittest
from canary_batch import Converter, BOSSTIARY_LEVELS, REV


class SpellConversion(unittest.TestCase):
    def convert(self, spell):
        converter = Converter(None, {}, {}, {}, {})
        converter.pending_definitions = set()
        deps = {'abilities': [], 'effects': [], 'formulas': []}
        assets = set()
        result = converter.spell(spell, 'canary:{}/boundary', deps, lambda key: assets.add(key) or key)
        return result, deps, assets, converter.pending_definitions

    def test_haste_source_formula_and_duration(self):
        _, deps, _, _ = self.convert({'name': 'speed', 'speedChange': 400, 'duration': 8000})
        self.assertEqual(deps['effects'][0]['condition']['type'], 'haste')
        self.assertEqual(deps['effects'][0]['duration_ms'], 8000)
        self.assertEqual(deps['formulas'][0]['speed'], {
            'minimum_multiplier': {'numerator': 7, 'denominator': 10}, 'minimum_offset': 40,
            'maximum_multiplier': {'numerator': 7, 'denominator': 5}, 'maximum_offset': 40})

    def test_slowdown_source_formula(self):
        _, deps, _, _ = self.convert({'name': 'speed', 'speedChange': -700})
        self.assertEqual(deps['effects'][0]['condition']['type'], 'paralyze')
        self.assertEqual(deps['formulas'][0]['speed']['minimum_multiplier'], {'numerator': 3, 'denominator': 20})
        self.assertEqual(deps['formulas'][0]['speed']['maximum_multiplier'], {'numerator': 3, 'denominator': 10})

    def test_slowdown_clamps_at_minus_1000(self):
        for change in (-1000, -1500):
            with self.subTest(change=change):
                _, deps, _, _ = self.convert({'name': 'speed', 'speedChange': change})
                self.assertEqual(deps['formulas'][0]['speed']['maximum_multiplier'], {'numerator': 0, 'denominator': 1})

    def test_zero_change_is_paralyze_with_identity_maximum(self):
        _, deps, _, _ = self.convert({'name': 'speed', 'speedChange': 0})
        self.assertEqual(deps['effects'][0]['condition']['type'], 'paralyze')
        self.assertEqual(deps['formulas'][0]['speed']['maximum_multiplier'], {'numerator': 1, 'denominator': 1})

    def test_zero_and_missing_durations_use_engine_default(self):
        for name in ('speed', 'invisible', 'drunk', 'outfit'):
            for duration in ({}, {'duration': 0}):
                with self.subTest(name=name, duration=duration):
                    _, deps, _, _ = self.convert({'name': name, 'outfitItem': 7172, **duration})
                    self.assertEqual(deps['effects'][0]['duration_ms'], 10000)

    def test_outfit_item_is_a_declared_item_reference(self):
        _, deps, _, definitions = self.convert({'name': 'outfit', 'outfitItem': 7172, 'duration': 4000})
        self.assertEqual(deps['effects'][0]['appearance_transform']['item'],
                         {'family': 'Item', 'key': 'canary:item/7172', 'revision': REV})
        self.assertIn(('Item', 'canary:item/7172'), definitions)

    def test_outfit_creature_takes_precedence_over_item(self):
        _, deps, _, _ = self.convert({'name': 'outfit', 'outfitMonster': 'Rat', 'outfitItem': 7172})
        self.assertEqual(set(deps['effects'][0]['appearance_transform']), {'creature'})

    def test_outfit_without_source_target_stays_unresolved(self):
        result, deps, _, _ = self.convert({'name': 'outfit'})
        self.assertIsNone(result)
        self.assertEqual(deps['effects'], [])

    def test_visual_only_has_no_damage_or_condition(self):
        _, deps, assets, _ = self.convert({'name': 'effect', 'effect': '@CONST_ME_MAGIC_BLUE'})
        self.assertEqual(deps['effects'][0]['operation'], 'presentation_only')
        self.assertNotIn('formula', deps['effects'][0])
        self.assertNotIn('condition', deps['effects'][0])
        self.assertIn('canary.appearance:effect/magic_blue', assets)

    def test_unknown_registered_spell_is_not_fabricated(self):
        result, deps, _, _ = self.convert({'name': 'wyrm wave'})
        self.assertIsNone(result)
        self.assertEqual(deps['abilities'], [])

    def test_bosstiary_source_awards_are_incremental(self):
        self.assertEqual(BOSSTIARY_LEVELS['nemesis'], [(1, 10), (3, 30), (5, 60)])
        self.assertEqual(sum(points for _, points in BOSSTIARY_LEVELS['bane']), 50)
        self.assertEqual(sum(points for _, points in BOSSTIARY_LEVELS['archfoe']), 100)


if __name__ == '__main__':
    unittest.main()
