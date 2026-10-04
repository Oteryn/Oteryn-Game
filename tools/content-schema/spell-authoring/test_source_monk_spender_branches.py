import copy
import json
import pathlib
import unittest
import jsonschema
from source_monk_spender_branches import callback_program, harmony_program, const, binding_metadata


def callback(power=15, clamps=True):
    guards = '\tif min < 5 then\n\t\tmin = 5\n\tend\n\tif max < 10 then\n\t\tmax = 10\n\tend\n' if clamps else ''
    return f'''local SPELL_BASE_POWER = {power}
function onGetFormulaValues(player, skill, attack, factor)
local damageHealing = player:calculateFlatDamageHealing()
local damage = SPELL_BASE_POWER * (skill / 100) * (attack / 10) + damageHealing
local min = damage - (damage / 10)
local max = damage + (damage / 10)
{guards}return player:getHarmonyDamage(min, max)
end
'''


HARMONY = '''double_t Player::getHarmonyBonus() {
if (harmony == 0) { return 1; }
double_t harmonyBaseBonus = 8.;
if (virtue == Virtue_t::Harmony) { harmonyBaseBonus += hasCondition(CONDITION_SERENE) ? 8. : 4.; }
harmonyBaseBonus += m_wheelPlayer.getStage(WheelStage_t::ASCETIC);
const auto rawHarmonyBuff = getBuff(BUFF_HARMONYBONUS);
if (rawHarmonyBuff != 0) { const auto harmonyBuff = rawHarmonyBuff - 100; harmonyBaseBonus += harmonyBuff; }
if (harmonyBaseBonus <= 0) { return 1; }
const double_t bonusPercent = harmonyBaseBonus * (pow(2, harmony - 1)) / 100.;
return 1 + bonusPercent;
}'''


class SpenderBranchesTests(unittest.TestCase):
    def test_tiger_clamps_and_helper_return_are_not_flattened(self):
        p, start, end = callback_program(callback(), 15)
        self.assertEqual(2, start)
        self.assertEqual(14, end)
        self.assertEqual('Player::calculateFlatDamageHealing', p[0]['value']['helper'])
        self.assertEqual(['5','10'], [s['if']['right']['const'] for s in p if 'if' in s])
        self.assertEqual('Player::getHarmonyDamage', p[-1]['return_pair']['helper'])

    def test_greater_and_knockout_no_invented_clamps(self):
        for power in (44,62):
            p,_,_=callback_program(callback(power,False),power)
            self.assertFalse(any('if' in s for s in p))
            self.assertEqual(str(power),p[1]['value']['args'][0]['args'][0]['args'][0]['const'])

    def test_unsupported_callback_or_changed_power_refused(self):
        with self.assertRaises(ValueError):callback_program(callback().replace('min < 5','min < 7'),15)
        with self.assertRaises(ValueError):callback_program(callback(44),15)
        with self.assertRaises(ValueError):callback_program(callback().replace('return player:getHarmonyDamage(min, max)','return world:probe(min,max)'),15)

    def test_harmony_branches_virtue_serene_buff_and_wheel_preserved(self):
        p=harmony_program(HARMONY)
        self.assertEqual('eq',p[0]['if']['comparison'])
        self.assertEqual('virtue_is_harmony',p[2]['if']['bool_input'])
        self.assertEqual('condition_serene_present',p[2]['then'][0]['if']['bool_input'])
        self.assertEqual('pow',p[-2]['value']['args'][0]['args'][1]['op'])
        with self.assertRaises(ValueError):harmony_program(HARMONY.replace('return 1 + bonusPercent;', 'return 2 + bonusPercent;'))

    def test_binding_pair_is_scoped_and_missing_player_branch_stays_unqualified(self):
        body = """int PlayerFunctions::luaPlayerGetHarmonyDamage(lua_State* L) {
const auto &player = Lua::getUserdataShared<Player>(L, 1, "Player");
if (!player) {
Lua::reportErrorFunc(Lua::getErrorDesc(LUA_ERROR_PLAYER_NOT_FOUND));
return 1;
}
const auto baseMin = Lua::getNumber<uint16_t>(L, 2);
const auto baseMax = Lua::getNumber<uint16_t>(L, 3);
const auto [min, max] = player->getHarmonyDamage(baseMin, baseMax);
lua_pushnumber(L, min);
lua_pushnumber(L, max);
return 2;
}"""
        metadata = binding_metadata(body)
        self.assertEqual('valid_player_userdata',metadata['success_precondition'])
        self.assertEqual(2,metadata['success_lua_return_count'])
        self.assertEqual(1,metadata['invalid_player_guard']['lua_return_count'])
        self.assertNotIn('lua_return_count',metadata)
        with self.assertRaises(ValueError):binding_metadata(body.replace('return 1;', 'return 2;'))
        schema=json.loads(pathlib.Path(__file__).with_name('source-monk-spender-branches.schema.json').read_text())
        scoped={'$defs':schema['$defs'],**schema['properties']['helpers']['properties']['lua_harmony_binding']}
        metadata['source_proof']={'path':'binding.cpp','revision':'a'*40,'sha256':'b'*64,'function':'binding','line_start':1}
        jsonschema.Draft202012Validator(scoped).validate(metadata)
        bad=copy.deepcopy(metadata);bad['invalid_player_guard']['lua_return_count']=2
        with self.assertRaises(jsonschema.ValidationError):jsonschema.Draft202012Validator(scoped).validate(bad)
        bad=copy.deepcopy(metadata);bad['lua_return_count']=2
        with self.assertRaises(jsonschema.ValidationError):jsonschema.Draft202012Validator(scoped).validate(bad)

    def test_strict_schema_refuses_fake_runtime_and_invalid_nodes(self):
        schema=json.loads(pathlib.Path(__file__).with_name('source-monk-spender-branches.schema.json').read_text())
        expression_schema={'$defs':schema['$defs'],'$ref':'#/$defs/expression'}
        jsonschema.Draft202012Validator(expression_schema).validate(const('8.'))
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(expression_schema).validate({'world_probe':True})
        self.assertEqual(False,schema['properties']['runtime_activation']['const'])


if __name__=='__main__':unittest.main()
