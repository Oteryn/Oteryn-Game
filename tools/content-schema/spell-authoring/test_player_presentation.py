"""Source default and malformed/unregistered cue regression checks."""
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from build_player_presentation import constants, cue
from convert_spells import Bundle

class PlayerPresentationTests(unittest.TestCase):
    def test_unregistered_sound_is_not_a_guessed_numeric_binding(self):
        values, registered = constants('enum SoundEffect_t : uint16_t { SILENCE = 0, SPELL_OR_RUNE = 1 }; enum MagicEffectClasses : uint16_t { CONST_ME_NONE }; enum ShootType_t : uint8_t { CONST_ANI_NONE }; registerEnumNamespace(L, soundNamespace, SoundEffect_t::SPELL_OR_RUNE)')
        self.assertEqual(cue('SOUND_EFFECT_TYPE_SPELL_OR_RUNE', values, registered)['source_id'], 1)
        self.assertNotIn('source_id', cue('SOUND_EFFECT_TYPE_UNKNOWN', values, registered))
        self.assertNotIn('source_id', cue('SOUND_EFFECT_TYPE_SILENCE', values, registered))

    def test_weapon_projectile_requires_equipment_not_constant_id(self):
        self.assertNotIn('source_id', cue('CONST_ANI_WEAPONTYPE', {'CONST_ANI_WEAPONTYPE': 254}, set()))

    def bundle(self, registrar):
        fake = SimpleNamespace(records={'canary': {'registrar': registrar}}, rows=[])
        fake.executions = {'canary': SimpleNamespace(
            sound_defaults={'castSound': 'SOUND_EFFECT_TYPE_SPELL_OR_RUNE', 'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'},
            sound=lambda value: {'SOUND_EFFECT_TYPE_SPELL_OR_RUNE': 'canary.sound:spell_or_rune',
                                 'SOUND_EFFECT_TYPE_SILENCE': 'canary.sound:silence'}.get(value))}
        fake.row = lambda *args, **kwargs: fake.rows.append({})
        return fake

    def test_absent_registrar_inherits_cast_default_only(self):
        fake = self.bundle({})
        with patch('convert_spells.source_text', return_value='SoundEffect_t soundCastEffect = SoundEffect_t::SPELL_OR_RUNE;\nSoundEffect_t soundImpactEffect = SoundEffect_t::SILENCE;'):
            fake.executions['canary'].root = '/qualified-source'
            self.assertEqual(Bundle.sound_cues(fake, '/spell'), {'cast_cue': 'canary.sound:spell_or_rune'})
            self.assertEqual(fake.rows[0]['source_file'], 'src/creatures/combat/spells.hpp')

    def test_enum_reference_without_lua_registration_is_not_bound(self):
        values, registered = constants('enum SoundEffect_t : uint16_t { SILENCE = 0, SPELL_OR_RUNE = 10 }; enum MagicEffectClasses : uint16_t { CONST_ME_NONE }; enum ShootType_t : uint8_t { CONST_ANI_NONE }; SoundEffect_t::SPELL_OR_RUNE')
        self.assertNotIn('source_id', cue('SOUND_EFFECT_TYPE_SPELL_OR_RUNE', values, registered))

    def test_explicit_silence_is_preserved_not_replaced_by_default(self):
        self.assertEqual(Bundle.sound_cues(self.bundle({'castSound': 'SOUND_EFFECT_TYPE_SILENCE',
                                                      'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'}), '/spell'), {})

    def test_canary_silence_wins_over_crystal_audible_sound(self):
        fake = self.bundle({'castSound': 'SOUND_EFFECT_TYPE_SILENCE',
                            'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'})
        fake.records['crystal'] = {'registrar': {'castSound': 'SOUND_EFFECT_TYPE_SPELL_OR_RUNE',
                                              'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'}}
        fake.executions['crystal'] = fake.executions['canary']
        self.assertEqual(Bundle.sound_cues(fake, '/spell'), {})

    def test_canary_unregistered_constant_silence_wins_over_crystal_sound(self):
        fake = self.bundle({'castSound': 'SOUND_EFFECT_TYPE_UNKNOWN',
                            'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'})
        fake.records['crystal'] = {'registrar': {'castSound': 'SOUND_EFFECT_TYPE_SPELL_OR_RUNE',
                                              'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'}}
        fake.executions['crystal'] = fake.executions['canary']
        self.assertEqual(Bundle.sound_cues(fake, '/spell'), {})

    def test_unknown_explicit_sound_does_not_inherit_default(self):
        self.assertEqual(Bundle.sound_cues(self.bundle({'castSound': 'SOUND_EFFECT_TYPE_UNKNOWN',
                                                      'impactSound': 'SOUND_EFFECT_TYPE_SILENCE'}), '/spell'), {})

if __name__ == '__main__': unittest.main()
