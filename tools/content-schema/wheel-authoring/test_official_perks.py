import copy
import unittest
from wheel_authoring import ROOT, read, validate
from verify_official_perks import validate_official_perks


class OfficialPerkRegressionTests(unittest.TestCase):
    def setUp(self):
        self.candidate = copy.deepcopy(read(ROOT / 'samples/wheel-candidate.json'))

    def conviction(self, vocation, key):
        return next(s['conviction'] for s in self.candidate['vocations'][vocation]['slots']
                    if s['conviction']['key'] == key)

    def test_all_five_vocations_qualify(self):
        validate_official_perks(self.candidate)

    def test_correct_description_cannot_hide_wrong_mystic_damage(self):
        perk = self.conviction('monk', 'augmented_mystic_repulse')
        stage = perk['augment_stages'][1]
        self.assertEqual(stage['reference_text'], '+60% Base Damage')
        next(e for e in stage['numeric_effects'] if e['kind'] == 'base_damage_bonus')['value'] = 40
        # Local tuning remains valid authoring, but does not qualify as official parity.
        validate(self.candidate)
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_AUGMENT_VALUE'):
            validate_official_perks(self.candidate)

    def test_superseded_battle_healing_multiplier_is_rejected(self):
        perk = self.conviction('knight', 'battle_healing')
        next(e for e in perk['unique_parameters']['numeric_effects']
             if e['kind'] == 'shield_healing_multiplier')['value'] = 3
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_UNIQUE_VALUE'):
            validate_official_perks(self.candidate)

    def test_obsolete_monk_cooldown_descriptions_are_rejected(self):
        for key, number in [('augmented_mystic_repulse', 0), ('augmented_thousand_fist_blows', 1)]:
            with self.subTest(key=key):
                stage = self.conviction('monk', key)['augment_stages'][number]
                stage['reference_text'] = '-6s Cooldown'
                with self.assertRaisesRegex(ValueError, 'OFFICIAL_AUGMENT_DESCRIPTION'):
                    validate_official_perks(self.candidate)
                stage['reference_text'] = '-4s Cooldown'

    def test_guiding_description_must_include_party_bonus(self):
        self.conviction('monk', 'guiding_presence')['reference_description'] = (
            'Gain an aura that shares your mantra with members of your group.')
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_UNIQUE_DESCRIPTION'):
            validate_official_perks(self.candidate)

    def test_planner_typo_does_not_override_official_lord_value(self):
        rev = next(r for r in self.candidate['vocations']['sorcerer']['revelations']
                   if r['key'] == 'lord_of_destruction')
        next(e for e in rev['stages'][1]['numeric_effects']
             if e['kind'] == 'mastery_decay_critical_extra_damage')['value'] = 25.5
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_REVELATION_VALUE'):
            validate_official_perks(self.candidate)

    def test_flurry_area_does_not_grant_extra_cast_range(self):
        stage = self.conviction('monk', 'augmented_flurry_of_blows')['augment_stages'][0]
        stage['numeric_effects'].append({'kind': 'range_increase', 'value': 1, 'unit': 'tiles'})
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_FLURRY_AREA'):
            validate_official_perks(self.candidate)

    def test_positive_fractional_mitigation_is_not_rounded_down(self):
        slot = next(s for s in self.candidate['vocations']['paladin']['slots']
                    if s['dedication'][0]['stat'] == 'mitigation_multiplier')
        slot['dedication'][0]['value_per_point'] = 0.07
        with self.assertRaisesRegex(ValueError, 'OFFICIAL_DEDICATION_VALUE'):
            validate_official_perks(self.candidate)


if __name__ == '__main__':
    unittest.main()
