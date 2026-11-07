"""Cue IDs must come from the owning enum and actual Lua registration."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import build_spell_cues as producer


class CueQualificationTests(unittest.TestCase):
    def build(self, aliases, utils=None, creatures=None, lua=None):
        utils = utils or b'''enum MagicEffectClasses : uint16_t {
            CONST_ME_NONE, CONST_ME_POFF = 3 };
            enum ShootType_t : uint8_t { CONST_ANI_NONE, CONST_ANI_ARROW = 3 };
            enum Unrelated : uint16_t { CONST_ME_FOREIGN = 99 };'''
        creatures = creatures or b'''enum SoundEffect_t : uint16_t {
            SILENCE = 0, SPELL_OR_RUNE = 10, SPELL_TEST = 100 };
            enum OtherType : uint16_t { SPELL_FOREIGN = 123 };'''
        lua = lua or b'''constexpr const char* soundNamespace = "SOUND_EFFECT_TYPE_";
            for (auto value : magic_enum::enum_values<MagicEffectClasses>()) {
                registerMagicEnum(L, value);
            }
            registerEnum(L, CONST_ANI_ARROW);
            registerEnumNamespace(L, soundNamespace, SoundEffect_t::SPELL_OR_RUNE);
            registerEnumNamespace(L, soundNamespace, SoundEffect_t::SPELL_TEST);'''
        outputs = [producer.PIN, utils, creatures, lua]
        with tempfile.TemporaryDirectory() as temporary:
            catalog = Path(temporary) / 'catalog.json'
            catalog.write_text(json.dumps(aliases))
            with patch.object(producer.subprocess, 'check_output', side_effect=outputs):
                return producer.build(Path(temporary), catalog)

    def test_real_enum_kinds_registered_values_and_lua_provenance(self):
        result = self.build(['appearance:effect/poff', 'canary.appearance:missile/arrow',
                             'canary.sound:spell_or_rune'])
        self.assertEqual([row['value'] for row in result['records']], [3, 3, 10])
        self.assertEqual(result['source_files'][-1]['path'],
                         'src/lua/functions/core/game/lua_enums.cpp')

    def test_unrelated_enum_names_cannot_supply_effect_or_sound_ids(self):
        for alias in ['appearance:effect/foreign', 'canary.sound:spell_foreign']:
            with self.subTest(alias=alias), self.assertRaises(ValueError):
                self.build([alias])

    def test_unregistered_or_comment_only_lua_sound_is_refused(self):
        for lua in [b'''constexpr const char* soundNamespace = "SOUND_EFFECT_TYPE_";
                       for (auto value : magic_enum::enum_values<MagicEffectClasses>()) {
                           registerMagicEnum(L, value); }
                       // registerEnumNamespace(L, soundNamespace, SoundEffect_t::SPELL_TEST);''',
                    b'''constexpr const char* soundNamespace = "OTHER_";
                       for (auto value : magic_enum::enum_values<MagicEffectClasses>()) {
                           registerMagicEnum(L, value); }
                       registerEnumNamespace(L, soundNamespace, SoundEffect_t::SPELL_TEST);''']:
            with self.subTest(lua=lua), self.assertRaises(ValueError):
                self.build(['canary.sound:spell_test'], lua=lua)

    def test_enum_expressions_duplicates_and_width_overflow_refused(self):
        for text in ['enum SoundEffect_t : uint16_t { A = 1, A = 2 };',
                     'enum SoundEffect_t : uint16_t { A = 65536 };',
                     'enum SoundEffect_t : uint16_t { A = helper() };']:
            with self.subTest(text=text), self.assertRaises(ValueError):
                producer.enum_values(text, 'SoundEffect_t')
        self.assertEqual(producer.enum_values(
            'enum ShootType_t : uint8_t { NONE, A = 3, B, LAST = B, DYNAMIC = 0xFE };',
            'ShootType_t'), {'NONE': 0, 'A': 3, 'B': 4, 'LAST': 4, 'DYNAMIC': 254})


if __name__ == '__main__':
    unittest.main()
