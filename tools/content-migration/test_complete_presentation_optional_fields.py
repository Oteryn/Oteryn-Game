"""Positive source absence and registration boundaries for optional presentation."""
import importlib.util
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('presentation_optional',Path(__file__).with_name('complete_presentation_optional_fields.py'))
p=importlib.util.module_from_spec(spec);spec.loader.exec_module(p)


class PresentationSourceTests(unittest.TestCase):
    def test_apostrophe_names_preserve_exact_registered_authority(self):
        text='local spell = Spell("instant")\nspell:name("gaz\'haragoth iceball")\nother:name("unregistered")'
        self.assertEqual(p.names(text),[('spell',"gaz'haragoth iceball")])

    def test_single_quoted_names_are_supported(self):
        self.assertEqual(p.names("local s = Spell('instant')\ns:name('cat wave')"), [('s','cat wave')])

    def test_no_setters_is_positive_source_omission_not_inferred_defaults(self):
        text='local combat = Combat()\ncombat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_SOUND_RED)'
        self.assertEqual(p.source_audio(text, {'SOUND_EFFECT_TYPE_SILENCE':0}), {})

    def test_sound_visual_is_not_an_audio_recording(self):
        self.assertEqual(p.source_audio('pos:sendMagicEffect(CONST_ME_SOUND_YELLOW)', {}), {})

    def test_dynamic_audio_requires_specific_remaining_proof(self):
        ids={'SOUND_EFFECT_TYPE_SPELL':7}
        for text in ['spell:castSound(customCue)', 'spell:castSound(SOUND_EFFECT_TYPE_SPELL)\nspell:impactSound(dynamic)', 'pos:sendSoundEffect(custom)']:
            self.assertIsNone(p.source_audio(text,ids))

    def test_exact_source_audio_and_silence(self):
        self.assertEqual(p.source_audio('spell:castSound(SOUND_EFFECT_TYPE_FAMILIAR)',{'SOUND_EFFECT_TYPE_FAMILIAR':42}),{'cast_cue':42})
        self.assertEqual(p.source_audio('spell:impactSound(SOUND_EFFECT_TYPE_SILENCE)',{'SOUND_EFFECT_TYPE_SILENCE':0}),{})

    def test_familiar_alias_requires_manifest_bound_file(self):
        ability={'identity':{'key':'canary:ability/spell/summon_monk_familiar'}}
        self.assertIsNone(p.source_for_ability(ability,'opentibiabr/canary',{'entries':[]},{}))
        path='data/scripts/spells/familiar/monk_familiar.lua'
        self.assertEqual(p.source_for_ability(ability,'opentibiabr/canary',{'entries':[{'source_file':path}]},{}),(path,False))

    def test_derived_component_uses_registered_parent_not_filename_similarity(self):
        ability={'identity':{'key':'canary:ability/spell/energy_pulse_explosion/component-1'}}
        table={('opentibiabr/canary','energy_pulse_explosion'):[('spell','data/scripts/spells/real.lua')]}
        self.assertEqual(p.source_for_ability(ability,'opentibiabr/canary',{'entries':[]},table),('data/scripts/spells/real.lua',True))
        self.assertIsNone(p.source_for_ability(ability,'zimbadev/crystalserver',{'entries':[]},table))

    def test_rune_registration_wins_over_same_name_conjuring_spell(self):
        ability={'identity':{'key':'canary:ability/spell/stone_shower_rune'}}
        table={('opentibiabr/canary','stone_shower_rune'):[('spell','data/scripts/spells/conjuring/stone_shower_rune.lua'),('rune','data/scripts/runes/stone_shower.lua')]}
        self.assertEqual(p.source_for_ability(ability,'opentibiabr/canary',{'entries':[]},table),('data/scripts/runes/stone_shower.lua',False))

    def test_native_estimate_requires_one_supported_damage_profile(self):
        ability={'effects':[{'key':'effect'}],'area':{'shape':'beam','length_tiles':8,'spread_tiles':2}}
        deps={'effects':[{'identity':{'key':'effect'},'operation':'damage','damage_type':'earth'}]}
        self.assertEqual(p.native_audio_row(ability,deps),{'name':'combat','type':'@COMBAT_EARTHDAMAGE','length':8,'spread':2})
        deps['effects'][0]['damage_type']='unknown'
        self.assertIsNone(p.native_audio_row(ability,deps))


if __name__=='__main__':unittest.main()
