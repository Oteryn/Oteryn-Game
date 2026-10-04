#!/usr/bin/env python3
"""Bounded current Canary monk spender source programs, not executable Game formulae."""
import argparse
import copy
import gzip
import hashlib
import json
import os
import pathlib
import re
import subprocess
from source_formula_evidence import expression, flat_helper

PIN = '04b83b512114bfd888000d6e1433ed8ecaec7c5b'
REPO = pathlib.Path('/workspace/spell-sources/canary')
SPELLS = {'devastating_knockout': 62, 'greater_tiger_clash': 44, 'tiger_clash': 15}


def const(v): return {'const': str(v)}
def var(v): return {'var': v}
def op(name, *args): return {'op': name, 'args': list(args)}
def comparison(name, left, right): return {'comparison': name, 'left': left, 'right': right}
def branch(test, yes, no=None): return {'if': test, 'then': yes, 'else': no or []}
def assign(name, value): return {'assign': name, 'value': value}


def source(path):
    raw = subprocess.check_output(['git', '-C', str(REPO), 'show', PIN+':'+path], env={**os.environ, 'GIT_NO_LAZY_FETCH':'1'})
    blob = subprocess.check_output(['git', '-C', str(REPO), 'rev-parse', PIN+':'+path], text=True).strip()
    return raw, {'path':path, 'revision':PIN, 'git_blob':blob, 'sha256':hashlib.sha256(raw).hexdigest()}


def function_proof(raw, base, signature):
    text = raw.decode(); start = text.index(signature); end = text.index('\n}', start)+2
    body = text[start:end]
    return {**base, 'function':signature.split('(')[0].split()[-1],
            'line_start':text[:start].count('\n')+1,'line_end':text[:end].count('\n')+1,
            'body_sha256':hashlib.sha256(body.encode()).hexdigest()}, body


def callback_program(text, expected_power):
    power = re.search(r'^local SPELL_BASE_POWER = (\d+)\s*$',text,re.M)
    if not power or int(power[1]) != expected_power:
        raise ValueError('bounded spender source power differs')
    match = re.search(r'^function onGetFormulaValues\(player, skill, attack, factor\)\n(.*?)^end\s*$',text,re.M|re.S)
    if not match:
        raise ValueError('bounded spender callback declaration differs')
    lines = [line.split('--')[0].strip() for line in match[1].splitlines()]; lines = [line for line in lines if line]
    statements=[]; i=0
    while i<len(lines):
        line=lines[i]
        if line == 'local damageHealing = player:calculateFlatDamageHealing()':
            statements.append(assign('damageHealing', {'helper':'Player::calculateFlatDamageHealing','arguments':[]}))
        elif line.startswith('local '):
            m=re.fullmatch(r'local (damage|min|max) = (.+)',line)
            if not m: raise ValueError('unsupported callback assignment')
            tree=expression(m[2], {'SPELL_BASE_POWER':const(expected_power)})
            statements.append(assign(m[1],tree))
        elif line.startswith('if '):
            m=re.fullmatch(r'if (min|max) < (5|10) then',line)
            if not m or i+2 >= len(lines) or lines[i+1] != f'{m[1]} = {m[2]}' or lines[i+2] != 'end':
                raise ValueError('unsupported source callback branch')
            statements.append(branch(comparison('lt',var(m[1]),const(m[2])),[assign(m[1],const(m[2]))]));i+=2
        elif line == 'return player:getHarmonyDamage(min, max)':
            statements.append({'return_pair':{'helper':'Player::getHarmonyDamage','arguments':[var('min'),var('max')]}})
        else: raise ValueError('unsupported spender source statement: '+line)
        i+=1
    if not statements or 'return_pair' not in statements[-1]: raise ValueError('source pair return absent')
    return statements, text[:match.start()].count('\n')+1, text[:match.start(1)+len(match[1])].count('\n')+1


def harmony_program(body):
    clean=re.sub(r'//[^\n]*','',body); compact=re.sub(r'\s+','',clean)
    expected=['if(harmony==0){return1;}', 'double_tharmonyBaseBonus=8.;',
              'if(virtue==Virtue_t::Harmony){harmonyBaseBonus+=hasCondition(CONDITION_SERENE)?8.:4.;}',
              'harmonyBaseBonus+=m_wheelPlayer.getStage(WheelStage_t::ASCETIC);',
              'constautorawHarmonyBuff=getBuff(BUFF_HARMONYBONUS);',
              'if(rawHarmonyBuff!=0){constautoharmonyBuff=rawHarmonyBuff-100;harmonyBaseBonus+=harmonyBuff;}',
              'if(harmonyBaseBonus<=0){return1;}',
              'constdouble_tbonusPercent=harmonyBaseBonus*(pow(2,harmony-1))/100.;', 'return1+bonusPercent;']
    # Exact helper shape guard: no omitted unseen branch or extra arithmetic accepted.
    prefix='double_tPlayer::getHarmonyBonus(){'
    if compact != prefix+''.join(expected)+'}': raise ValueError('Harmony helper shape differs')
    return [branch(comparison('eq',var('harmony'),const(0)),[{'return':const(1)}]),
            assign('harmonyBaseBonus',const('8.')),
            branch({'bool_input':'virtue_is_harmony'},[
                branch({'bool_input':'condition_serene_present'},[assign('harmonyBaseBonus',op('add',var('harmonyBaseBonus'),const('8.')))],
                       [assign('harmonyBaseBonus',op('add',var('harmonyBaseBonus'),const('4.')))])]),
            assign('harmonyBaseBonus',op('add',var('harmonyBaseBonus'),var('wheel_ascetic_stage'))),
            assign('rawHarmonyBuff',var('raw_harmony_buff')),
            branch(comparison('neq',var('rawHarmonyBuff'),const(0)),[
                assign('harmonyBuff',op('sub',var('rawHarmonyBuff'),const(100))),
                assign('harmonyBaseBonus',op('add',var('harmonyBaseBonus'),var('harmonyBuff')))]),
            branch(comparison('lte',var('harmonyBaseBonus'),const(0)),[{'return':const(1)}]),
            assign('bonusPercent',op('div',op('mul',var('harmonyBaseBonus'),op('pow',const(2),op('sub',var('harmony'),const(1)))),const('100.'))),
            {'return':op('add',const(1),var('bonusPercent'))}]


def binding_metadata(body):
    compact = re.sub(r'\s+', '', re.sub(r'//[^\n]*', '', body))
    guard = 'if(!player){Lua::reportErrorFunc(Lua::getErrorDesc(LUA_ERROR_PLAYER_NOT_FOUND));return1;}'
    if guard not in compact or compact.index(guard) > compact.index('constautobaseMin='):
        raise ValueError('Invalid-player binding refusal branch differs')
    if ('constautobaseMin=Lua::getNumber<uint16_t>(L,2);' not in compact
            or 'constautobaseMax=Lua::getNumber<uint16_t>(L,3);' not in compact
            or 'lua_pushnumber(L,min);lua_pushnumber(L,max);return2;}' not in compact):
        raise ValueError('Valid-player binding numeric/pair branch differs')
    return {'input_types':['uint16_t','uint16_t'],
            'cpp_result_type':'std::pair<uint64_t,uint64_t>',
            'success_precondition':'valid_player_userdata',
            'success_lua_return_count':2,
            'invalid_player_guard':{'condition':'player_userdata_missing',
                'diagnostic_operation':'Lua::reportErrorFunc(Lua::getErrorDesc(LUA_ERROR_PLAYER_NOT_FOUND))',
                'lua_return_count':1,
                'return_stack_value_status':'unqualified_missing_player_branch_no_pair_claim'}}


def build():
    cpp,cpp_proof=source('src/creatures/players/player.cpp'); binding,binding_proof=source('src/lua/functions/creatures/player/player_functions.cpp')
    flat,flat_proof=flat_helper('canary-main-current',PIN)
    bonus_proof,bonus_body=function_proof(cpp,cpp_proof,'double_t Player::getHarmonyBonus() {')
    harmony=harmony_program(bonus_body)
    damage_proof,damage_body=function_proof(cpp,cpp_proof,'std::pair<uint64_t, uint64_t> Player::getHarmonyDamage(double min, double max) {')
    compact=re.sub(r'\s+','',damage_body)
    if compact!='std::pair<uint64_t,uint64_t>Player::getHarmonyDamage(doublemin,doublemax){autoharmonyBonus=getHarmonyBonus();returnstd::make_pair(min*harmonyBonus,max*harmonyBonus);}':
        raise ValueError('Harmony damage helper differs')
    binding_fn_proof,binding_body=function_proof(binding,binding_proof,'int PlayerFunctions::luaPlayerGetHarmonyDamage(lua_State* L) {')
    binding_contract = binding_metadata(binding_body)
    records=[]
    for slug,power in SPELLS.items():
        path=f'data/scripts/spells/attack/{slug}.lua';raw,proof=source(path)
        statements,start,end=callback_program(raw.decode(),power)
        records.append({'registration_key':f'canary-main-current/{path}#1','source_proof':{**proof,'function':'onGetFormulaValues','line_start':start,'line_end':end},
                        'base_power':power,'callback_kind':'CALLBACK_PARAM_SKILLVALUE','source_inputs':['player','skill','attack','factor'],
                        'unused_inputs':['factor'],'program':statements,'runtime_activation':False,
                        'status':'source_branch_and_helpers_preserved_native_consumer_unimplemented'})
    return {'schema':'OTERYN_CURRENT_MONK_SPENDER_SOURCE_PROGRAM/v1','runtime_activation':False,'record_count':3,'records':records,
            'helpers':{'flat_damage_healing':{'source_proof':flat_proof,'program':flat},
                       'harmony_bonus':{'source_proof':bonus_proof,'inputs':['harmony','virtue_is_harmony','condition_serene_present','wheel_ascetic_stage','raw_harmony_buff'],'program':harmony},
                       'harmony_damage':{'source_proof':damage_proof,'minimum':op('mul',var('min'),var('harmonyBonus')),'maximum':op('mul',var('max'),var('harmonyBonus'))},
                       'lua_harmony_binding':{'source_proof':binding_fn_proof,**binding_contract}},
            'runtime_gaps':['Current flat-damage recurrence requires a source-program consumer; no replacement world curve applied.',
                            'Harmony live state, virtue, Serene, Ascetic and buff input production/ownership remain unqualified.',
                            'Lua uint16 argument conversion and C++ uint64 result conversion are recorded; edge-case conversion parity not qualified.',
                            'Program preserves positive callback pair; no fabricated negative sign, native identity allocation or live cohort rewrite.']}


def main():
    p=argparse.ArgumentParser();p.add_argument('--out',required=True);a=p.parse_args();data=build()
    import jsonschema
    schema=json.loads(pathlib.Path(__file__).with_name('source-monk-spender-branches.schema.json').read_text());jsonschema.Draft202012Validator(schema).validate(data)
    payload=(json.dumps(data,indent=2)+'\n').encode();out=pathlib.Path(a.out);out.write_bytes(gzip.compress(payload,mtime=0) if out.suffix=='.gz' else payload)


if __name__=='__main__':main()
