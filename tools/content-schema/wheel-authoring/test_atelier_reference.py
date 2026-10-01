import copy
import unittest
from fractions import Fraction
from unittest.mock import patch
from jsonschema.exceptions import ValidationError
import wheel_authoring as wheel
from verify_atelier_reference import validate_atelier_reference


class AtelierReferenceTests(unittest.TestCase):
    def setUp(self):
        self.candidate = wheel.read(wheel.ROOT / 'samples/wheel-candidate.json')

    def test_selected_snapshot_passes_independent_check(self):
        validate_atelier_reference(self.candidate)

    def test_old_cap_and_cost_cannot_qualify_even_if_builder_drifts(self):
        for field in ('cap', 'cost'):
            candidate = copy.deepcopy(self.candidate)
            if field == 'cap':
                candidate['gems']['atelier']['operation_policy']['revealed_gem_limit'] = 250
            else:
                candidate['gems']['grade_costs'][1]['supreme']['gold'] = 12000000
            with self.subTest(field=field), patch.object(wheel, 'build', return_value=candidate):
                with self.assertRaisesRegex(ValueError, 'ATELIER_REFERENCE'):
                    validate_atelier_reference(candidate)

    def test_old_trials_or_missing_termination_are_rejected(self):
        for field in ('legacy', 'missing', 'disabled', 'zero'):
            candidate = copy.deepcopy(self.candidate)
            row = candidate['gems']['loot_reference']['per_quality'][0]
            if field == 'legacy':
                row['independent_trials'] = row.pop('maximum_trials')
            elif field == 'missing':
                row.pop('stop_on_first_failure')
            elif field == 'disabled':
                row['stop_on_first_failure'] = False
            else:
                row['maximum_trials'] = 0
            with self.subTest(field=field), self.assertRaises((ValueError, ValidationError)):
                wheel.validate(candidate)
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'LOOT_TRIALS'):
                validate_atelier_reference(candidate)

    def test_changed_capture_and_builder_cannot_source_qualify_chance(self):
        self.candidate['gems']['loot_reference']['per_quality'][0]['chance_by_category']['influenced'] = 8000
        with patch.object(wheel, 'build', return_value=self.candidate):
            wheel.validate(self.candidate)
        with self.assertRaisesRegex(ValueError, 'LOOT_TRIALS'):
            validate_atelier_reference(self.candidate)

    def test_reachable_two_trial_outcomes_preserve_stop_on_failure(self):
        # Failure on the first roll precludes the success-on-second path.
        # Independent always-two trials would have an extra F,S outcome.
        for p in (Fraction(9, 100), Fraction(3, 100)):
            paths = {'F': 1 - p, 'SF': p * (1 - p), 'SS': p * p}
            self.assertEqual(sum(paths.values()), 1)
            self.assertEqual(paths['SF'] + paths['SS'], p)
            self.assertNotEqual(paths['SF'] + paths['SS'], 1 - (1 - p) ** 2)
        self.assertEqual([paths for paths in (
            Fraction(91, 100), Fraction(819, 10000), Fraction(81, 10000))],
            [Fraction(91, 100), Fraction(9, 100) * Fraction(91, 100), Fraction(9, 100) ** 2])
