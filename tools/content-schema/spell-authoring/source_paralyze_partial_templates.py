#!/usr/bin/env python3
"""R45 source-only partial templates. Both full rune registrations remain BLOCKED."""
import argparse
import copy
import gzip
import hashlib
import json
import os
import pathlib
import re
import subprocess

import blocked_completions
import import_source_player_bundles as base
import source_mechanics_inventory as mechanics
import validate_spell

HERE = pathlib.Path(__file__).resolve().parent
SOURCE_PATH = 'data/scripts/runes/paralyze_rune.lua'
SOURCE_SHA = '40b12d7797b1cc0448f1b8020543a977486db671c0be501408de0b15f0bd6781'
PINS = {'canary-main-current': '04b83b512114bfd888000d6e1433ed8ecaec7c5b', 'crystal-summer-current': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
OLD = blocked_completions.REVISIONS
ORIGINAL = pathlib.Path('docs/reference/spells/r28-source-closure/player-source-bundles')
OUT = pathlib.Path('docs/reference/spells/r45-source-closure')
SCHEMA_PATH = HERE / 'source-paralyze-partial-templates.schema.json'
REVISION = 'source-partial-r45'
FLAGS = {'model_projection_equivalence': False, 'source_program_fully_represented': False,
         'runtime_blocked': True, 'runtime_activation': False, 'native_execution_qualified': False,
         'canonical_selection_changed': False, 'native_identity_allocation': False}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def source_file(root, source, revision, path):
    return subprocess.check_output(['git', '-C', str(pathlib.Path(root) / source), 'show', revision + ':' + path], env=dict(os.environ, GIT_NO_LAZY_FETCH='1'))


def scope_fact(data, path, symbol, anchors):
    body = base.source_cpp_function(data, symbol)
    text = data.decode(); body_text = body.decode(); start = text.index(body_text)
    evidence = []
    positions = []
    for marker in anchors:
        if body_text.count(marker) != 1:
            raise ValueError('owning C++ anchor not unique: ' + symbol + ' ' + marker)
        offset = start + body_text.index(marker); positions.append(offset)
        evidence.append({'line': text.count('\n', 0, offset) + 1, 'anchor_sha256': sha(marker.encode())})
    return {'path': path, 'file_sha256': sha(data), 'scope': symbol, 'scope_sha256': sha(body), 'evidence': evidence}, positions


def qualify_cpp(current, before):
    combat_path = 'src/creatures/combat/combat.cpp'; bridge_path = 'src/lua/functions/creatures/combat/combat_functions.cpp'
    facts = []
    fact, positions = scope_fact(current[combat_path], combat_path, 'Combat::CombatHealthFunc', ['g_game().combatBlockHit(', 'if (g_game().combatChangeHealth(caster, target, damage)) {', 'CombatConditionFunc(caster, target, params, &damage);', 'CombatDispelFunc(caster, target, params, nullptr);'])
    if positions != sorted(positions):
        raise ValueError('health-before-condition owning order changed')
    facts.append(fact)
    fact, _ = scope_fact(current[combat_path], combat_path, 'Combat::getCombatDamage', ['CombatDamage damage;', 'if (formulaType == COMBAT_FORMULA_DAMAGE)', 'if (creature->getCombatValues(min, max))', 'if (params.valueCallback)'])
    facts.append(fact)
    bridge = base.source_cpp_function(current[bridge_path], 'CombatFunctions::luaCombatExecute').decode()
    number = bridge.split('case VARIANT_NUMBER: {', 1)[1].split('case VARIANT_POSITION:', 1)[0]
    if 'bool result = true;' not in bridge or 'Lua::pushBoolean(L, result);' not in bridge or 'combat->doCombat(creature, target);' not in number or 'result =' in number:
        raise ValueError('Lua number-variant execute boolean scope changed')
    fact, _ = scope_fact(current[bridge_path], bridge_path, 'CombatFunctions::luaCombatExecute', ['bool result = true;', 'case VARIANT_NUMBER:', 'Lua::pushBoolean(L, result);'])
    facts.append(fact)
    checks = {
        'src/creatures/creatures_definitions.hpp': ['struct CombatDamage {', 'int32_t value = 0;', '} primary, secondary;'],
        'src/creatures/combat/combat.hpp': ['formulaType_t formulaType = COMBAT_FORMULA_UNDEFINED;'],
        'src/creatures/creature.hpp': ['virtual bool getCombatValues(int32_t &, int32_t &) {\n\t\treturn false;', 'uint16_t getBaseSpeed() const'],
        'src/creatures/players/player.hpp': ['public Creature'],
    }
    for path, markers in checks.items():
        text = current[path].decode()
        if any(marker not in text for marker in markers):
            raise ValueError('owning default initializer/base player assumption changed: ' + path)
        if path.endswith('players/player.hpp') and 'getCombatValues(' in text:
            raise ValueError('Player combat-value override requires new qualification')
        facts.append({'path': path, 'file_sha256': sha(current[path]), 'scope': 'bounded_declaration_anchors',
                      'scope_sha256': sha('\0'.join(markers).encode()), 'evidence': [{'line': text.count('\n', 0, text.index(marker)) + 1, 'anchor_sha256': sha(marker.encode())} for marker in markers]})
    route = re.sub(r'\s+', ' ', current[combat_path].decode())
    required = 'if (params.combatType != COMBAT_NONE) { CombatDamage damage = getCombatDamage(caster, target);'
    if required not in route or 'doCombatHealth(caster, target, origin, damage, params);' not in route:
        raise ValueError('undefined-damage health route changed')
    condition_path = 'src/creatures/combat/condition.cpp'
    for symbol, anchors in [('ConditionSpeed::getFormulaValues', ['int32_t difference = var - 40;', 'min = mina * difference + minb;', 'max = maxa * difference + maxb;']),
                            ('ConditionSpeed::startCondition', ['speedDelta = uniform_random(min, max) - baseSpeed;', 'speedDelta < 40 - baseSpeed', 'speedDelta = 40 - baseSpeed;']),
                            ('ConditionSpeed::addCondition', ['speedDelta = uniform_random(min, max) - baseSpeed;', 'speedDelta < 40 - baseSpeed', 'speedDelta = 40 - baseSpeed;'])]:
        fact, _ = scope_fact(current[condition_path], condition_path, symbol, anchors)
        facts.append(fact)
    condition_bridge_path = 'src/lua/functions/creatures/combat/condition_functions.cpp'
    fact, _ = scope_fact(current[condition_bridge_path], condition_bridge_path, 'ConditionFunctions::luaConditionCreate', ['Condition::createCondition(conditionId, conditionType, 0, 0, false, subId, isPersistent);'])
    facts.append(fact)
    fact, _ = scope_fact(current[condition_path], condition_path, 'ConditionSpeed::ConditionSpeed', ['speedDelta(initChangeSpeed)'])
    facts.append(fact)
    factory_text = current[condition_path].decode()
    factory_markers = [marker for marker in ['ObjectPool<ConditionSpeed, 1024>::allocateShared(id, type, ticks, buff, subId, param);', 'std::make_shared<ConditionSpeed>(id, type, ticks, buff, subId, param);'] if factory_text.count(marker) == 1]
    if len(factory_markers) != 1:
        raise ValueError('ConditionSpeed factory parameter forwarding differs')
    factory_marker = factory_markers[0]
    facts.append({'path': condition_path, 'file_sha256': sha(current[condition_path]), 'scope': 'bounded_ConditionSpeed_factory_forwarding',
                  'scope_sha256': sha(factory_marker.encode()), 'evidence': [{'line': factory_text.count('\n', 0, factory_text.index(factory_marker)) + 1, 'anchor_sha256': sha(factory_marker.encode())}]})
    comparisons = []
    for path, symbol in [(combat_path, 'Combat::getCombatDamage'), (combat_path, 'Combat::CombatHealthFunc'), (bridge_path, 'CombatFunctions::luaCombatExecute'), (condition_path, 'ConditionSpeed::getFormulaValues'), (condition_path, 'ConditionSpeed::startCondition'), (condition_path, 'ConditionSpeed::addCondition')]:
        now = base.source_cpp_function(current[path], symbol); old = base.source_cpp_function(before[path], symbol)
        comparisons.append({'path': path, 'scope': symbol, 'current_scope_sha256': sha(now), 'old_scope_sha256': sha(old), 'bytes_equal': now == old})
    return {'owning_source_facts': facts, 'old_current_function_comparisons': comparisons,
            'baseline_primary_secondary_values': [0, 0], 'baseline_scope': 'player_no_explicit_formula_no_value_callback_before_external_combat_providers',
            'source_combat_type': 'COMBAT_UNDEFINEDDAMAGE', 'route': 'non_COMBAT_NONE_to_health_path',
            'owning_order': ['combatBlockHit', 'combatChangeHealth', 'conditional_CombatConditionFunc', 'conditional_CombatDispelFunc'],
            'caster_success_scope': 'Lua_Combat_execute_boolean_not_health_or_condition_application',
            'number_variant_result': 'initialized_true_doCombat_return_not_assigned',
            'external_event_and_buff_mutation_qualified': False, 'final_damage_value_qualified': False,
            'cpp_execution_qualified': False,
            'speed_normalization': {'source_coefficients': [-1, 0, -1, 0], 'source_formula_variable': 'base_speed_minus_40',
                                    'source_delta': 'uniform_source_min_max_minus_base_speed', 'source_paralyze_delta_floor': '40_minus_base_speed',
                                    'base_speed_type': 'uint16_t', 'normalized_base_domain_target_speed': 40, 'condition_delta_projection': '40_minus_base_speed', 'initial_condition_speed_delta': 0, 'final_creature_speed_qualified': False, 'stacking_and_var_speed_provider_qualified': False,
                                    'scope': 'accepted_condition_start_or_refresh_negative_formula_path_only', 'provider_execution_qualified': False}}


def condition_from_source(data):
    if sha(data) != SOURCE_SHA or blocked_completions.cast_body(data.decode()) != blocked_completions.PARALYZE_BODY:
        raise ValueError('exact paralyze Lua identity/CAST mismatch')
    calls = mechanics.scan(data.decode())['calls']
    def one(name):
        result = [call for call in calls if call['call_identity'] == name]
        if len(result) != 1:
            raise ValueError('source condition call not unique: ' + name)
        return result[0]
    constructor = one('Condition'); ticks = one('condition:setParameter'); formula = one('condition:setFormula'); addition = one('combat:addCondition')
    if constructor['arguments'] != [{'kind': 'symbol', 'name': 'CONDITION_PARALYZE'}] or ticks['arguments'] != [{'kind': 'symbol', 'name': 'CONDITION_PARAM_TICKS'}, {'kind': 'number_literal', 'value': 6000}] or [a.get('value') for a in formula['arguments']] != [-1, 0, -1, 0] or addition['arguments'] != [{'kind': 'symbol', 'name': 'condition'}]:
        raise ValueError('actual source condition declaration differs')
    condition = {'type': 'paralyze', 'fixed_duration': 6000, 'speed_formula': dict(zip(['mina', 'minb', 'maxa', 'maxb'], ['-1', '0', '-1', '0']))}
    evidence = [{'call_identity': call['call_identity'], 'line': call['line'], 'argument_fact_sha256': sha(canonical(call['arguments']))} for call in [constructor, ticks, formula, addition]]
    return condition, evidence


def build(repo, source_root):
    repo = pathlib.Path(repo); capture_path = repo / ORIGINAL / 'source-callback-facts.jsonl.gz'
    capture = capture_path.read_bytes(); rows = {row['registration_key']: row for row in map(json.loads, gzip.decompress(capture).splitlines())}
    summary = json.loads((repo / ORIGINAL / 'import-summary.json').read_text()); results = []
    cpp_paths = ['src/creatures/combat/combat.cpp', 'src/lua/functions/creatures/combat/combat_functions.cpp', 'src/creatures/creatures_definitions.hpp', 'src/creatures/combat/combat.hpp', 'src/creatures/creature.hpp', 'src/creatures/players/player.hpp', 'src/creatures/combat/condition.cpp', 'src/lua/functions/creatures/combat/condition_functions.cpp']
    for snapshot, revision in PINS.items():
        source = snapshot.split('-')[0]; registration = snapshot + '/' + SOURCE_PATH + '#1'; row_id = sha(registration.encode())[:16]
        fact = rows[registration]
        selected = [row for row in summary['records_index'] if row['registration_key'] == registration]
        if len(selected) != 1 or selected[0]['status'] != 'BLOCKED' or fact['source_revision'] != revision or fact['source_sha256'] != SOURCE_SHA:
            raise ValueError('exact blocked registration identity mismatch')
        data = source_file(source_root, source, revision, SOURCE_PATH); condition, condition_evidence = condition_from_source(data)
        if data != source_file(source_root, source, OLD[source], SOURCE_PATH):
            raise ValueError('old/current paralyze Lua declaration differs')
        current = {path: source_file(source_root, source, revision, path) for path in cpp_paths}
        before = {path: source_file(source_root, source, OLD[source], path) for path in [cpp_paths[0], cpp_paths[1], cpp_paths[-2]]}
        cpp = qualify_cpp(current, before)
        cpp['old_source_revision'] = OLD[source]
        prefix = 'candidate:source-template/' + snapshot + '/' + row_id
        def identity(suffix):return {'key': prefix + suffix, 'revision': REVISION}
        dependencies = {'abilities': [{'identity': identity('/ability'), 'kind': 'spell', 'needs_target': True, 'needs_direction': False, 'range_tiles': 0,
                                      'effects': [{'family': 'Effect', **identity('/effect-condition')}]}],
                        'effects': [{'identity': identity('/effect-condition'), 'operation': 'condition', 'duration_ms': 6000, 'condition': {'type': 'paralyze', 'lifetime': 'fixed_duration', 'speed_formula': {'family': 'Formula', **identity('/formula-speed')}},
                                     'presentation': {'impact_asset_binding': source + '.appearance:effect/magic_red'}}], 'formulas': [{'identity': identity('/formula-speed'), 'kind': 'speed_modifier', 'speed': {'minimum_multiplier': {'numerator': 0, 'denominator': 1}, 'minimum_offset': 40, 'maximum_multiplier': {'numerator': 0, 'denominator': 1}, 'maximum_offset': 40}}]}
        spec = copy.deepcopy(blocked_completions.SPECS[('rune', 'paralyze rune', source)])
        spec['caster_effect_asset_binding'] = source + '.appearance:effect/magic_green'
        blocked_completions.apply_completion(spec, dependencies)
        errors = validate_spell.structural('spell-dependencies.schema.json', dependencies)
        if errors:raise ValueError('existing partial template schema mismatch: ' + ' | '.join(errors))
        folder = repo / ORIGINAL / snapshot / row_id
        header = json.loads((folder / 'source-header.json').read_text())['spell']
        item = {'family': 'Item', 'key': 'candidate:item/source/' + snapshot + '/3165', 'revision': REVISION}
        result = {'schema': 'OTERYN_SOURCE_PARALYZE_PARTIAL_TEMPLATE/v1', 'registration_key': registration,
                  'source_identity': {'source': source, 'snapshot': snapshot, 'revision': revision, 'path': SOURCE_PATH, 'sha256': SOURCE_SHA, 'git_blob': fact['source_callback_facts']['blob']},
                  'full_spell_status': 'BLOCKED', 'candidate_count': 0, 'executable_spell_exported': False,
                  'source_header': header, 'source_rune_item_reference': item, 'dependency_templates': dependencies,
                  'source_condition_evidence': condition_evidence, 'source_condition_declaration': condition,
                  'source_cast_program': [{'order': 0, 'operation': 'execute_combat', 'receiver': 'combat', 'arguments': ['creature', 'var'], 'return_binding': 'execute_result'},
                                          {'order': 1, 'operation': 'return_false_if_execute_false', 'binding': 'execute_result'},
                                          {'order': 2, 'operation': 'caster_position_magic_effect_if_execute_true', 'constant': 'CONST_ME_MAGIC_GREEN'},
                                          {'order': 3, 'operation': 'return_true'}],
                  'source_cast_body_sha256': sha(blocked_completions.PARALYZE_BODY.encode()), 'cpp_control_proof': cpp,
                  'contract_correction_needed': {'contract_path': 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md',
                     'section': 'D.5.3 item 4', 'contract_sha256': sha((repo / 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md').read_bytes()), 'expectation_quote': 'A failed cast (target immune, out of range) gives no caster-tile effect.',
                     'actual_source': 'For a valid creature-number variant, Lua execute keeps result=true regardless of the doCombat return; health/condition refusal does not establish execute=false.',
                     'projection_gap': 'Existing after_success field lacks qualified Lua-return-stage semantics; cannot claim equivalence to health/condition success.'},
                  'remaining_mechanics': ['Lua execute-return stage must be represented by the owning contract before full Spell qualification.', 'Zero-damage health/provider hooks and conditional condition/dispel execution remain runtime-blocked.', 'External event, buff, target legality, Item/asset bindings and final damage values remain unqualified.'],
                  'source_capture_gzip_sha256': sha(capture), 'source_header_sha256': sha((folder / 'source-header.json').read_bytes()),
                  'historical_receipt_sha256': sha((folder / 'receipt.json').read_bytes()), **FLAGS}
        results.append(result)
    return results


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources'); args = parser.parse_args()
    repo = pathlib.Path(args.repo).resolve(); records = build(repo, args.source_root)
    validate = validate_spell.Draft202012Validator(json.loads(SCHEMA_PATH.read_text()), registry=validate_spell.REGISTRY)
    for row in records:validate.validate(row)
    out = repo / OUT; out.mkdir(parents=True, exist_ok=True); payload = b''.join(canonical(row) + b'\n' for row in records); compressed = gzip.compress(payload, mtime=0)
    (out / 'source-paralyze-partial-templates.jsonl.gz').write_bytes(compressed)
    proof = {'schema': 'OTERYN_SOURCE_PARALYZE_PARTIAL_RECEIPT/v1', 'records': 2, 'partial_abilities': 2, 'partial_condition_effects': 2, 'source_backed_speed_formulas': 2,
             'full_spell_status_counts': {'BLOCKED': 2}, 'candidate_count': 0, 'executable_spell_count': 0,
             'gzip_sha256': sha(compressed), 'payload_sha256': sha(payload), 'schema_sha256': sha(SCHEMA_PATH.read_bytes()),
             'producer_sha256': sha(pathlib.Path(__file__).read_bytes()), 'source_revisions': PINS,
             'existing_schema_proofs': {path.name: sha(path.read_bytes()) for path in [HERE / 'spell-dependencies.schema.json', HERE.parent / 'monster-authoring/monster.schema.json']},
             'existing_adapter_sha256': sha(pathlib.Path(blocked_completions.__file__).read_bytes()), **FLAGS}
    (out / 'source-paralyze-partial-receipt.json').write_text(json.dumps(proof, indent=2) + '\n'); print(json.dumps(proof, indent=2))


if __name__ == '__main__':main()
