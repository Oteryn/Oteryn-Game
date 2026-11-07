import copy
import unittest
from jsonschema.exceptions import ValidationError
import wheel_authoring as wheel


class NumericBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.candidate = wheel.read(wheel.ROOT / 'samples/wheel-candidate.json')

    def combat_mastery(self, candidate):
        return next(revelation for revelation in candidate['vocations']['knight']['revelations']
                    if revelation['key'] == 'combat_mastery')

    def test_combat_mastery_refuses_zero_and_negative_health_steps_at_every_stage(self):
        for stage_index in range(3):
            for value in (0, -0.5):
                candidate = copy.deepcopy(self.candidate)
                stage = self.combat_mastery(candidate)['stages'][stage_index]
                next(effect for effect in stage['numeric_effects']
                     if effect['kind'] == 'missing_health_step')['value'] = value
                with self.subTest(stage=stage_index + 1, value=value), self.assertRaises(ValidationError):
                    wheel.validate(candidate)

    def test_fractional_health_steps_preserve_valid_zero_bonus_tuning(self):
        wheel.validate(self.candidate)
        self.assertEqual([next(effect['value'] for effect in stage['numeric_effects']
            if effect['kind'] == 'missing_health_step')
            for stage in self.combat_mastery(self.candidate)['stages']], [14, 12, 10])
        candidate = copy.deepcopy(self.candidate)
        for stage in self.combat_mastery(candidate)['stages']:
            for effect in stage['numeric_effects']:
                if effect['kind'] == 'missing_health_step':
                    effect['value'] = 0.5
                elif effect['kind'] in ('damage_reduction_per_step', 'damage_bonus_per_step',
                                       'flat_damage_bonus', 'flat_healing_bonus'):
                    effect['value'] = 0
        wheel.validate(candidate)
