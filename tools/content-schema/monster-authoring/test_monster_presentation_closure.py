"""Meaningful negative and donor-helper regression cases for presentation capture."""
import unittest
from pathlib import Path
import os

from build_monster_presentation_closure import HelperCapture, enum, resolve, source_file


class ClosureTests(unittest.TestCase):
    def test_enum_preserves_alias_hex_and_implicit_value(self):
        self.assertEqual(enum('enum T : uint16_t { ZERO=0, HEX=0x10, NEXT, ALIAS=HEX };', 'T'),
                         {'ZERO': 0, 'HEX': 16, 'NEXT': 17, 'ALIAS': 16})

    def test_enum_rejects_unhandled_expression(self):
        with self.assertRaises(ValueError):
            enum('enum T { BAD = 1 << 2 };', 'T')

    def test_missing_is_not_suppression_or_unknown(self):
        self.assertEqual(resolve(None, {})['status'], 'source_absent')
        self.assertEqual(resolve(False, {})['status'], 'source_explicitly_suppressed')
        self.assertEqual(resolve('@UNKNOWN', {})['status'], 'source_unresolved')
        self.assertEqual(resolve('@SOUND', {'SOUND': 42})['donor_numeric_id'], 42)

    def test_numeric_id_requires_known_header_value(self):
        self.assertEqual(resolve(99, {'SOUND': 42})['status'], 'source_unresolved')
        self.assertEqual(resolve(42, {'SOUND': 42})['status'], 'source_resolved_numeric')

    def test_sandbox_denies_filesystem_and_python(self):
        with self.assertRaises(Exception):
            HelperCapture("os.execute('false')")
        with self.assertRaises(Exception):
            HelperCapture("python.eval('1+1')")

    def test_sandbox_bounds_runaway_helper(self):
        with self.assertRaises(Exception):
            HelperCapture('while true do end')

    def test_actual_setter_separate_from_default_intent(self):
        helper = HelperCapture('''
local function loadSpellSoundType() return {cast='@CAST', impact='@IMPACT'} end
function readSpell() local s=MonsterSpell(); s:castSound('@CAST'); s:castSound('@IMPACT'); return s end
''')
        captured = helper.capture({}, 1)
        self.assertEqual(captured['default_sound_variants'], [{'cast': '@CAST', 'impact': '@IMPACT'}])
        self.assertEqual(captured['actual_helper_call_variants'][0],
                         [['castSound', ['@CAST']], ['castSound', ['@IMPACT']]])

    def test_random_melee_variants_are_not_collapsed(self):
        helper = HelperCapture('''
local function loadSpellSoundType() return {impact=math.random(1,3)} end
function readSpell() local s=MonsterSpell(); s:castSound(math.random(1,3)); return s end
''')
        captured = helper.capture({'name': 'melee'}, 1)
        self.assertEqual(len(captured['actual_helper_call_variants']), 3)


@unittest.skipUnless(os.environ.get('SPELL_SOURCE_ROOT'), 'set SPELL_SOURCE_ROOT for pinned donor regression')
class PinnedHelperTests(unittest.TestCase):
    def test_both_pinned_helpers_derive_sounds_and_default_visual(self):
        for source in ('canary', 'crystal'):
            with self.subTest(source=source):
                root = Path(os.environ['SPELL_SOURCE_ROOT']) / source
                helper = HelperCapture((root / 'data/scripts/lib/register_monster_type.lua').read_text())
                capture = helper.capture({'name': 'combat', 'type': '@COMBAT_FIREDAMAGE',
                                          'radius': 4, 'shootEffect': '@CONST_ANI_FIRE'}, 1)
                default = capture['default_sound_variants'][0]
                self.assertEqual(default['cast'], '@SOUND_EFFECT_TYPE_MAGICAL_RANGE_ATK')
                self.assertEqual(default['impact'], '@SOUND_EFFECT_TYPE_MONSTER_SPELL_LARGE_AREA_FIRE')
                calls = capture['actual_helper_call_variants'][0]
                self.assertIn(['setCombatEffect', ['@CONST_ME_POFF']], calls)
                self.assertEqual([c for c in calls if c[0] == 'castSound'][-1][1], [default['impact']])
                self.assertFalse(any(c[0] == 'impactSound' for c in calls))

    def test_registered_lookup_uses_damage_not_condition(self):
        for source in ('canary', 'crystal'):
            with self.subTest(source=source):
                root = Path(os.environ['SPELL_SOURCE_ROOT']) / source
                helper = HelperCapture((root / 'data/scripts/lib/register_monster_type.lua').read_text())
                parameters = {'name': 'registered attack', 'minDamage': -10, 'maxDamage': -30}
                captured = helper.capture(parameters, 1, registered=True)
                calls = captured['actual_helper_call_variants'][0]
                self.assertIn(['setCombatValue', [-10, -30]], calls)
                self.assertFalse(any(c[0] == 'setConditionDamage' for c in calls))
                unknown = helper.capture(parameters, 1, registered=False)
                self.assertIn(['setConditionDamage', [-10, -30, 0]], unknown['actual_helper_call_variants'][0])

    def test_both_helpers_preserve_poison_condition_timing(self):
        for source in ('canary', 'crystal'):
            with self.subTest(source=source):
                root = Path(os.environ['SPELL_SOURCE_ROOT']) / source
                helper = HelperCapture((root / 'data/scripts/lib/register_monster_type.lua').read_text())
                capture = helper.capture({'name': 'melee', 'condition': {'type': '@CONDITION_POISON',
                    'duration': 9000, 'interval': 3000, 'totalDamage': 30}}, 1)
                calls = capture['actual_helper_call_variants'][0]
                self.assertIn(['setConditionTickInterval', [3000]], calls)
                self.assertIn(['setConditionDuration', [9000]], calls)
                self.assertIn(['setConditionDamage', [30, 30, 0]], calls)
                self.assertEqual(len(capture['actual_helper_call_variants']), 3)


if __name__ == '__main__':
    unittest.main()
