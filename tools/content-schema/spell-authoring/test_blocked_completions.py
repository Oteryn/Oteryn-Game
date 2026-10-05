"""Qualification of fail-closed completion matching and lossless dependency attachment."""
import copy
import unittest
from pathlib import Path

import blocked_completions as bc


class CompletionTests(unittest.TestCase):
    SOURCES = Path(__file__).resolve().parent / 'fixtures' / 'completion-sources'

    def dependencies(self):
        return {'abilities': [{'identity': {'key': 'ability', 'revision': 'r'},
                              'effects': [{'family': 'Effect', 'key': 'effect', 'revision': 'r'}]}],
                'effects': [{'identity': {'key': 'effect', 'revision': 'r'}, 'operation': 'heal'}], 'formulas': []}

    def test_all_pinned_implementations_identify(self):
        for (kind, name, source), expected in bc.SPECS.items():
            with self.subTest(name=name, source=source):
                text = (self.SOURCES / source / expected['file']).read_text()
                spec = bc.identify_completion(name, kind, source, text)
                self.assertIsNotNone(spec)
                self.assertEqual(spec['revision'], bc.REVISIONS[source])
                self.assertEqual(spec['authority'], 'OtsHypothesisOnly')
                self.assertIn(spec['body'], bc.accepted_cast_bodies(self.SOURCES / source, source)[name])

    def test_changed_formula_or_added_source_operation_fails_closed(self):
        text = (self.SOURCES / 'canary' / bc.SPECS[('instant', 'heal friend', 'canary')]['file']).read_text()
        for changed in [text.replace('SPELL_BASE_POWER = 260', 'SPELL_BASE_POWER = 261'),
                        text.replace('return combat:execute', 'creature:addItem(1)\n\treturn combat:execute')]:
            self.assertIsNone(bc.identify_completion('heal friend', 'instant', 'canary', changed))
        self.assertIsNone(bc.identify_completion('heal friend', 'rune', 'canary', text))

    def test_crystal_heal_friend_with_secondary_stance_is_not_accepted(self):
        text = (self.SOURCES / 'crystal/data/scripts/spells/healing/heal_friend.lua').read_text()
        self.assertIsNone(bc.identify_completion('heal friend', 'instant', 'crystal', text))

    def test_caster_presentation_preserves_when_event_runs(self):
        for name, kind, timing, color in [('heal friend', 'instant', 'before_combat', 'blue'),
                                          ('paralyze rune', 'rune', 'after_success', 'green')]:
            spec = bc.SPECS[(kind, name, 'canary')]
            deps = self.dependencies()
            bc.apply_completion(spec, deps)
            self.assertEqual(deps['effects'][0]['presentation']['caster_effect_timing'], timing)
            self.assertEqual(deps['effects'][0]['presentation']['caster_effect_asset_binding'],
                             'canary.appearance:effect/magic_' + color)
            self.assertEqual(deps['effects'][0]['operation'], 'heal')
            with self.assertRaises(ValueError):
                bc.apply_completion(spec, deps)

    def test_target_selection_is_explicit_on_ability(self):
        deps = self.dependencies()
        before = copy.deepcopy(deps['effects'])
        bc.apply_completion(bc.SPECS[('instant', 'inflict wound', 'canary')], deps)
        self.assertEqual(deps['abilities'][0]['target_selection'], 'caster_or_top_creature')
        self.assertEqual(deps['effects'], before)

    def test_paralyze_projection_preserves_condition_without_fabricated_damage(self):
        spec = bc.SPECS[('rune', 'paralyze rune', 'canary')]
        self.assertEqual(spec['drop_params'], ['COMBAT_PARAM_TYPE'])
        deps = self.dependencies()
        deps['effects'][0].update({'operation': 'condition', 'condition': {'type': 'paralyze',
            'fixed_duration': 6000, 'speed_formula': {'mina': '-1', 'minb': '0', 'maxa': '-1', 'maxb': '0'}}})
        original = copy.deepcopy(deps['effects'][0]['condition'])
        bc.apply_completion(spec, deps)
        self.assertIs(deps['abilities'][0]['zero_damage_health_path'], True)
        self.assertEqual(deps['effects'][0]['condition'], original)
        self.assertEqual(deps['effects'][0]['operation'], 'condition')
        self.assertEqual(deps['formulas'], [])
        self.assertNotIn('formula', deps['effects'][0])
        self.assertNotIn('damage_type', deps['effects'][0])

    def test_zero_health_path_cannot_be_disabled_or_predeclared(self):
        spec = copy.deepcopy(bc.SPECS[('rune', 'paralyze rune', 'canary')])
        spec['zero_damage_health_path'] = False
        with self.assertRaises(ValueError):
            bc.apply_completion(spec, self.dependencies())
        spec['zero_damage_health_path'] = True
        deps = self.dependencies()
        deps['abilities'][0]['zero_damage_health_path'] = False
        with self.assertRaises(ValueError):
            bc.apply_completion(spec, deps)

    def test_missing_or_ambiguous_effect_payload_refuses_attachment(self):
        duplicate = self.dependencies()
        duplicate['effects'].append(copy.deepcopy(duplicate['effects'][0]))
        for deps in [dict(self.dependencies(), abilities=[]), dict(self.dependencies(), effects=[]), duplicate]:
            with self.assertRaises(ValueError):
                bc.apply_completion(bc.SPECS[('instant', 'heal friend', 'canary')], deps)


if __name__ == '__main__':
    unittest.main()
