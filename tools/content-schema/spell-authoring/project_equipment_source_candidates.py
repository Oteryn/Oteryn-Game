"""R60 concrete source equipment/native parameter extension; no runtime admission.

Private extension uses the accepted keys and standard Formula/Ability/Effect data.
It preserves source variants rather than aliasing a canonical native profile.
"""
import argparse
import copy
import json
import re
from pathlib import Path
from referencing import Registry, Resource
from jsonschema import Draft202012Validator
import project_equipment_candidates as prior
import project_player_control_candidates as reader
import validate_spell as validation

REVISION = 'source-equipment-r60'
OUTPUT = Path('docs/reference/spells/r60-source-closure')
SCHEMA_FILE = Path(__file__).with_name('source-equipment-native-extensions.schema.json')
sha, canonical, base = prior.sha, prior.canonical, prior.base


def normalize_crystal_formula(callback):
    """Exact captured arithmetic; accepted S5 names the identical Crystal curve."""
    def normalize(tree):
        if isinstance(tree, list):
            return [normalize(value) for value in tree]
        if not isinstance(tree, dict):
            return tree
        if tree.get('fn') == 'base_damage_healing':
            return {'fn': 'level_base_damage_healing', 'args': normalize(tree['args'])}
        if tree == {'var': 'skill:SKILL_SHIELD'}:
            return {'var': 'shielding_skill'}
        return {key: normalize(value) for key, value in tree.items()}
    source = callback['formula']
    if source.get('status') != 'resolved':
        raise ValueError('Formula unresolved')
    bounds = [{'op': 'abs', 'args': [normalize(source[key])]} for key in ('minimum', 'maximum')]
    return {'kind': 'player_expression', 'inputs': 'skill',
            'minimum': {'op': 'min', 'args': copy.deepcopy(bounds)},
            'maximum': {'op': 'max', 'args': copy.deepcopy(bounds)}}


def const(value):
    return {'const': str(value)}


def op(name, *args):
    return {'op': name, 'args': list(args)}


def chain_formula(donor, text):
    flat = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    skill, attack, power = ({'var': key} for key in ('attack_skill', 'attack_value', 'base_power'))
    if donor == 'crystal':
        factor = re.search(r'local spellFactor = ([0-9.]+)', text).group(1)
        value = op('add', flat, op('mul', op('floor', op('mul', const('1.2'), attack)),
                                          op('div', op('add', skill, const(4)), const(28))))
        center = op('add', op('div', op('mul', power, value), const(100)), op('mul', const(factor), value))
    else:
        center = op('add', op('mul', op('mul', const(42), op('div', skill, const(100))), op('div', attack, const(10))), flat)
    return {'kind': 'player_expression', 'inputs': 'skill',
            'minimum': op('sub', center, op('div', center, const(10))) if donor == 'canary' else op('mul', center, const('0.9')),
            'maximum': op('add', center, op('div', center, const(10))) if donor == 'canary' else op('mul', center, const('1.1'))}


def ref(family, identity):
    return {'family': family, **identity}


def make_dependencies(raw, spell, donor, text):
    deps = {'abilities': [], 'effects': [], 'formulas': []}
    routes = []
    for index, captured in enumerate(raw.get('combats', [])):
        callbacks = [cb for cb in captured.get('callbacks', []) if 'formula' in cb]
        if not callbacks:
            continue
        identity = lambda suffix: {'key': spell['identity']['key'] + '/' + str(index) + '/' + suffix, 'revision': REVISION}
        formula = normalize_crystal_formula(callbacks[0]) if donor == 'crystal' else chain_formula(donor, text)
        # Source global castShieldDefense is deliberately absent from the old
        # callback symbolic capture. Reconstruct its actual explicit source term.
        if raw['name'] in ('Shield Bash', 'Shield Slam'):
            coefficients = ('1.5', '0.6', '2.5', '1.0') if raw['name'] == 'Shield Bash' else ('1.4', '0.55', '2.3', '0.95')
            flat = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
            def shield_bound(a, b):
                return op('add', op('add', op('add', flat, {'var': 'base_power'}),
                    op('mul', {'var': 'shield_defense'}, const(a))), op('mul', {'var': 'shielding_skill'}, const(b)))
            formula = {'kind': 'player_expression', 'inputs': 'skill',
                'minimum': shield_bound(*coefficients[:2]), 'maximum': shield_bound(*coefficients[2:])}
        formula['identity'] = identity('formula')
        params = captured['parameters']
        damage = params.get('COMBAT_PARAM_TYPE', 'COMBAT_PHYSICALDAMAGE').replace('COMBAT_', '').replace('DAMAGE', '').lower()
        effect = {'identity': identity('effect'), 'operation': 'damage', 'damage_type': damage,
            'formula': ref('Formula', identity('formula')), 'mitigated_by': ['armor'] if params.get('COMBAT_PARAM_BLOCKARMOR') else [],
            'presentation': {'impact_asset_binding': 'source:effect/' + str(params.get('COMBAT_PARAM_EFFECT', params.get('COMBAT_PARAM_CHAIN_EFFECT', 'CONST_ME_NONE'))).lower()}}
        if effect['operation'] == 'heal':
            effect['damage_type'] = 'healing'
        if not effect['mitigated_by']:
            effect.pop('mitigated_by')
        ability = {'identity': identity('ability'), 'kind': 'spell', 'needs_target': spell['targeting']['needs_target'],
            'needs_direction': spell['targeting']['needs_direction'], 'range_tiles': spell['targeting'].get('range_tiles', 0),
            'effects': [ref('Effect', identity('effect'))]}
        if captured.get('areas'):
            matrix = captured['areas'][0]['north']
            ability['area'] = {'matrix': {'north': [''.join({0: '.', 1: 'x', 2: 'c', 3: 'C'}[value] for value in row) for row in matrix]}}
        deps['abilities'].append(ability); deps['effects'].append(effect); deps['formulas'].append(formula)
        routes.append({'damage_type': damage, 'ability': ref('Ability', identity('ability')), 'source_combat_index': index,
            'source_phase': 'recast' if donor == 'canary' and raw['name'] == 'Spiritual Outburst' and index == 0 else 'primary',
            'area_variant': 'wheel_augmented' if raw['name'] == 'Flurry of Blows' and index >= 3 else 'default',
            'hit_order': index // 3 if raw['name'] == 'Sweeping Takedown' else 0})
    # Lua doTargetCombatHealth chain scripts have no Combat object to capture.
    if not routes and '/attack/' in raw['file']:
        for index, damage in enumerate(('physical', 'energy', 'earth')):
            identity = lambda suffix: {'key': spell['identity']['key'] + '/chain/' + str(index) + '/' + suffix, 'revision': REVISION}
            formula = {'identity': identity('formula'), **chain_formula(donor, text)}
            effect = {'identity': identity('effect'), 'operation': 'damage', 'damage_type': damage,
                'formula': ref('Formula', identity('formula')),
                'presentation': {'impact_asset_binding': 'source:effect/const_me_' + {'physical': 'white', 'energy': 'yellow', 'earth': 'green'}[damage] + '_energyshock'}}
            if effect['operation'] == 'heal':
                effect['damage_type'] = 'healing'
            ability = {'identity': identity('ability'), 'kind': 'spell', 'needs_target': False, 'needs_direction': False,
                'range_tiles': 3, 'effects': [ref('Effect', identity('effect'))]}
            deps['abilities'].append(ability); deps['effects'].append(effect); deps['formulas'].append(formula)
            routes.append({'damage_type': damage, 'ability': ref('Ability', identity('ability')), 'source_combat_index': index})
    return deps, routes


def source_parameters(raw, donor, text, routes):
    name = raw['name']
    common = {'source_model': 'r60_equipment_source_v1', 'default_cost_commit': 'source_cast_return_true_only',
              'consumer_contract_pending': True}
    if name.startswith('Focus '):
        return 'monk_focus', {**common, 'fill_harmony': True, 'harmony_max': 5,
            'harmony_assignment': 'clamp_to_virtue_min_then_set', 'gain_healing': False,
            'serene_cooldown_ms': 7000 if name == 'Focus Serenity' else 0,
            'serene_assignment': 'deadline_only_automatic_state_owner',
            'reset_cooldowns': 'all_spell_and_group' if name == 'Focus Serenity' else 'none',
            'rearm_individual': raw['registrar']['id'] if name == 'Focus Serenity' else None,
            'rearm_group': raw['registrar']['group'] if name == 'Focus Serenity' else None,
            'rearm_duration_source': 'authored_spell_and_group_cooldowns', 'presentation_effect': 'CONST_ME_MAGIC_BLUE'}
    if name.startswith('Virtue of '):
        virtue = name.rsplit(' ', 1)[1].lower()
        return 'stance_toggle', {**common, 'stance': virtue, 'toggle_same_stance_off': False,
            'replace_previous_virtue': True, 'harmony_min': 1 if virtue == 'harmony' else 0,
            'harmony_minimum_policy': 'after_spend_rebuild_only' if donor == 'canary' else 'all_setHarmony_calls_clamped',
            'harmony_zero_to_one_on_cast': virtue == 'harmony' and donor == 'crystal',
            'remove_justice_attribute_condition': donor == 'canary',
            'justice': {'mechanism': 'indefinite_fist_percent_attribute' if donor == 'canary' else 'effective_fist_skill_owner',
                'normal_percent': 115, 'serene_percent': 130, 'buff_spell': True,
                'duration_ms': -1, 'sub_id': 'AttrSubId_VirtueOfJustice',
                'apply_on_cast': donor == 'canary' and virtue == 'justice'},
            'presentation_effect': 'CONST_ME_MAGIC_GREEN' if donor == 'canary' else 'CONST_ME_MAGIC_BLUE',
            'state_apply_when': 'before_combat_result' if donor == 'canary' else 'on_cast_before_true',
            'return_route': 'combat_execute' if donor == 'canary' else 'true',
            'player_guard': donor == 'canary'}
    chain = name in ('Spiritual Outburst', 'Chained Penance')
    builder = 'addHarmony(1)' in text
    spender = bool(raw['registrar'].get('harmony')) or raw['registrar'].get('monkSpellType') == 'MonkSpell_Spender'
    params = {**common, 'routes': routes,
        'equipment': {'slot': 'left', 'damage_input': 'source_skill_callback_equipment',
            'default_damage_type': 'physical', 'bond_case': 'lowercase' if 'getElementalBond():lower()' in text else 'exact',
            'nil_item_type': 'lua_getAttack_error' if donor == 'crystal' and name == 'Spiritual Outburst' else 'physical_fallback',
            'unknown_bond': 'lua_nil_effect_error' if 'effectData = config[elementalBondType]' in text else 'physical_fallback',
            'nil_bond': 'lua_lower_call_error' if 'getElementalBond():lower()' in text else 'physical_fallback',
            'no_weapon_attack': 0 if chain and donor == 'crystal' else 7,
            'no_weapon_skill': 0, 'no_weapon_attack_factor': 0, 'shield_selection': 'none'},
        'harmony': {'gain': 1 if builder else 0,
            'gain_when': 'before_combat_even_on_false' if builder and not chain else 'after_chain_returns_before_true' if builder else 'none',
            'spend_all': spender, 'spend_when': 'successful_spell_postcast_only', 'infinite_flag_preflight_exemption': donor == 'crystal',
            'minimum': 1 if spender else 0, 'maximum': 5, 'virtue_harmony_minimum': 1,
            'damage_bonus_owner': 'crystal_float_spender_finishing' if donor == 'crystal' else 'canary_getHarmonyDamage_uint64',
            'bonus_base_percent': 7 if donor == 'crystal' else 8,
            'virtue_extra_percent': [3, 6] if donor == 'crystal' else [4, 8],
            'ascetic_stage_extra': [0, 1, 2, 3], 'bonus_scale': '2_pow_harmony_minus_1',
            'damage_conversion': 'float_then_int32_truncate' if donor == 'crystal' else 'double_then_uint64_truncate',
            'bonus_when': 'harmony_spell_name_finishing' if donor == 'crystal' else 'explicit_callback_getHarmonyDamage',
            'raw_harmony_buff_percent': donor == 'canary', 'spend_healing': donor == 'canary',
            'virtue_rebuild_healing': donor == 'canary',
            'builder_cooldown_reduction_ms_per_spent_charge': 2000 if donor == 'canary' else 0,
            'sanctuary': {'when': 'wheel_instant_enabled_and_positive_charges_and_no_existing_attribute' if donor == 'canary' else 'none',
                'attribute_sub_id': 'Sanctuary', 'duration_ms': 5000, 'bonus_percent_per_charge': 2,
                'bonus_scopes': ['damage_dealt', 'healing_dealt'], 'create_decaying_item': 'ITEM_SANCTUARY',
                'item_failure_does_not_prevent_attribute': True}},
        'combat': {'execute_order': 'phase_then_wheel_area_choice_then_element_then_hit_order',
            'ignore_combat_return': name == 'Sweeping Takedown',
            'independent_draws': True, 'wheel_area_routes': name == 'Flurry of Blows',
            'wheel_area_predicate': 'getWheelSpellAdditionalArea_truthy',
            'ordered_area_sets': [0, 1] if name == 'Sweeping Takedown' else [0],
            'fail_player_guard': name in ('Shield Bash', 'Shield Slam')}}
    if name in ('Shield Bash', 'Shield Slam'):
        params['equipment'].update(slot='both_hands', shield_selection='highest_defense_left_tie',
            wrong_type_or_empty='reject_if_no_shield', damage_input='snapshot_selected_shield_defense',
            nil_bond='not_used', unknown_bond='not_used')
        params['shield_debuff'] = {'exclude_players': True, 'duration_ms': 10000, 'damage_dealt_percent': 50,
            'wheel_grade_2_damage_dealt_percent': 25 if name == 'Shield Slam' else 50,
            'scope': 'all_outgoing_damage_buff_attribute', 'when': 'source_target_callback_even_if_health_blocked'}
    if chain:
        params['chain'] = {'candidate_kind': 'any_non_npc_except_self_non_protection_tile' if donor == 'canary' else 'unowned_monsters',
            'range_tiles': 2 if donor == 'canary' else 3, 'max_targets': 6 if donor == 'canary' else 5 if name == 'Chained Penance' else 4,
            'initial_anchor': 'caster', 'selection': 'source_engine_chain' if donor == 'canary' else 'shortest_path',
            'tie_break': 'highest_health_percent_then_spectator_order' if name == 'Chained Penance' else 'spectator_order',
            'target_health_sign': 'negative_offensive_delta',
            'canonical_health_sign_normalization': name == 'Spiritual Outburst',
            'path_effect_on_each_step': donor == 'crystal', 'engine_backtracking': False,
            'visited_unique': True, 'path_required': donor == 'crystal', 'sight_clear_before_hit': name == 'Chained Penance',
            'missing_target': 'break' if name == 'Chained Penance' else 'skip',
            'global_chain_storage': donor == 'crystal', 'chain_array_cleared_after': name == 'Chained Penance',
            'damage_decay': '0.5_pow_jump_1_based' if name == 'Chained Penance' else 'none',
            'scaled_bounds_conversion': 'floor_signed_times_multiplier_plus_half' if donor == 'crystal' else 'source_skill_callback',
            'source_direct_health_sign_route': 'positive_skill_callback' if donor == 'canary' else 'negate_and_swap_signed_bounds' if name == 'Spiritual Outburst' else 'signed_bounds',
            'wheel_additional_targets': name == 'Chained Penance', 'item_additional_target': 50146 if name == 'Chained Penance' else None,
            'item_additional_target_count': 1, 'revelation_gate': name == 'Spiritual Outburst',
            'recast_delay_ms': (1000 if donor == 'canary' else 1500) if name == 'Spiritual Outburst' else 0,
            'recast_when': 'harmony_equals_5_before_primary', 'recast_caster': 'guid_reacquired_if_present' if donor == 'canary' else 'captured_userdata_no_guard',
            'recast_chain_constructor_argument': 'captured_creature_userdata' if donor == 'crystal' else 'reacquired_guid_player',
            'recast_targets': 'saved_variant' if donor == 'canary' else 'rebuild_from_captured_caster',
            'recast_damage_inputs': 'recompute_callback_on_event' if donor == 'canary' else 'captured_weapon_attack_then_fresh_skill_level_power', 'recast_grade': 'fresh_callback_revelation_stage' if donor == 'canary' else 'captured_precast',
            'recast_grade_multipliers': ['0.375', '0.5', '0.625'],
            'cooldown_by_grade_seconds': [24, 20, 16] if donor == 'crystal' and name == 'Spiritual Outburst' else None,
            'cooldown_rate_scaled': donor == 'crystal' and name == 'Spiritual Outburst'}
    return 'equipment_attack', params


def closed_schema(value):
    """Closed immutable candidate-native values, with explicit JSON types."""
    if isinstance(value, dict):
        return {'type': 'object', 'additionalProperties': False, 'required': sorted(value),
                'properties': {key: closed_schema(item) for key, item in value.items()}}
    if isinstance(value, list):
        if not value:
            return {'type': 'array', 'items': False, 'minItems': 0, 'maxItems': 0}
        return {'type': 'array', 'prefixItems': [closed_schema(item) for item in value],
                'items': False, 'minItems': len(value), 'maxItems': len(value)}
    kind = 'null' if value is None else 'boolean' if isinstance(value, bool) else 'integer' if isinstance(value, int) else 'number' if isinstance(value, float) else 'string'
    return {'type': kind, 'const': value}


def build(repo, source_root):
    repo, source_root = Path(repo), Path(source_root)
    captures = prior.source_library.rows(repo, prior.source_library.CAPTURE_PATH, prior.source_library.CAPTURE_SHA)
    lane = json.loads((repo / prior.WORKLIST_PATH).read_text())['records']
    lane = [row for row in lane if row['lane'] == 'monk_equipment' and not (row['registration_key'].startswith('canary-') and '/focus_' in row['registration_key'])]
    if len(lane) != 23:
        raise ValueError('R60 membership drift')
    packet, records, variants = {}, [], []
    defaults = {donor: base.default_fields(source_root / donor, pin)[0] for donor, pin in prior.PINS.items()}
    for entry in lane:
        reg = entry['registration_key']; snapshot = reg.split('/')[0]; donor = snapshot.split('-')[0]
        row_id = sha(reg.encode())[:16]; prefix = snapshot + '/' + row_id + '/'
        archived = repo / 'imports/spells/r28/player-source-bundles' / snapshot / row_id
        header_bytes = (archived / 'source-header.json').read_bytes(); header = json.loads(header_bytes)
        old_receipt = json.loads((archived / 'receipt.json').read_text()); fact = captures[reg]; raw = fact['source_callback_facts']
        source = base.source_file(source_root / donor, prior.PINS[donor], raw['file'])
        if sha(source) != fact['source_sha256']:
            raise ValueError('Source source-byte identity changed')
        text = source.decode()
        spell = base.fill_header(header['spell'], defaults[donor], {'key': old_receipt['candidate_key'], 'revision': REVISION}, raw['registrar'])
        spell.pop('harmony_cost', None); spell['requirements'].pop('vocation_display_flags', None)
        spell['requirements']['learning_required'] = False
        if donor == 'canary' and raw['registrar'].get('needLearn'):
            spell['requirements']['wheel_unlock'] = True
        spell['targeting'].update(allow_on_self=defaults[donor]['allowOnSelf'], check_floor=True)
        deps, routes = make_dependencies(raw, spell, donor, text)
        key, params = source_parameters(raw, donor, text, routes)
        spell['execution'] = {'native_behavior': {'key': key, 'parameters': params}}
        if raw['name'] in ('Shield Bash', 'Shield Slam'):
            spell['needs_shield'] = True
        numeric_errors = []
        for formula in deps['formulas']:
            validation.check_formula(formula, spell.get('base_power'), 'formula', numeric_errors, needs_shield=raw['name'] in ('Shield Bash', 'Shield Slam'))
        if numeric_errors:
            raise ValueError('Formula numeric validation: ' + repr(numeric_errors))
        reader.validate_reader_shape(repo, {'spell': spell}, deps)
        dependency_errors = validation.structural('spell-dependencies.schema.json', deps)
        if dependency_errors:
            raise ValueError('Standard dependencies invalid: ' + repr(dependency_errors))
        variants.append(spell['execution']['native_behavior'])
        receipt = copy.deepcopy(old_receipt)
        receipt.update(status='CANDIDATE_SCHEMA_VALID', blockers=[], native_execution_qualified=False,
            dependencies={family: len(values) for family, values in deps.items()}, item_owner_bindings_required=[],
            schema_and_semantic_validation_errors=[], conversion_notes=['Concrete source equipment native parameter extension; private pending consumer contract. Accepted S5 names canonical level contribution; S27 D.4 explicitly projects Spiritual Outburst to offensive damage despite raw source-positive health routes, kept and flagged separately.'],
            remaining_mechanics=[{'source_field': 'source.native_consumer', 'reason': 'Source data model complete; extension contract/runtime admission/input owners/assets not qualified.'}])
        Draft202012Validator(base.receipt_schema(), registry=validation.REGISTRY).validate(receipt)
        packet[prefix + 'source-header.json'] = header_bytes
        for filename, value in {'spell.json': {'spell': spell}, 'dependencies.json': deps, 'catalog.json': {'definitions': []}, 'receipt.json': receipt}.items():
            packet[prefix + filename] = value
        records.append({'registration_key': reg, 'candidate_key': spell['identity']['key'], 'candidate_revision': REVISION,
            'status': 'CANDIDATE_SCHEMA_VALID', 'source_revision': prior.PINS[donor], 'source_sha256': sha(source),
            'source_header_path': prefix + 'source-header.json', 'source_header_sha256': sha(header_bytes),
            'source_cast_sha256': sha(source), 'capture_fact_sha256': sha(canonical(fact)),
            'source_unexpected_healing_route': raw['name'] == 'Spiritual Outburst',
            'source_health_equivalence': False if raw['name'] == 'Spiritual Outburst' else None,
            'canonical_health_sign_normalization_used': raw['name'] == 'Spiritual Outburst',
            'native_data_model_complete': True, 'source_alias_to_existing_native_profile': False,
            'required_operations_unrepresented': [], 'consumer_contract_pending': True, 'authoring_contract_extension_pending': True, 'source_consumer_qualified': False,
            'source_consumer_implemented': False, 'target_schema_family': 'private_source_complete_v2',
            'runtime_capability_blocked': True, 'reader_acceptance_qualified': False,
            'runtime_activation': False, 'native_execution_qualified': False,
            'source_numeric_equivalence': False, 'canonical_normalization_used': True})
    source_spell = copy.deepcopy(validation.SCHEMAS['spell.schema.json']['$defs']['spell'])
    def external_refs(value):
        if isinstance(value, dict):
            return {key: ('urn:oteryn:spell-authoring:candidate:1' + item if key == '$ref' and item.startswith('#/') else external_refs(item)) for key, item in value.items()}
        return [external_refs(item) for item in value] if isinstance(value, list) else value
    source_spell = external_refs(source_spell)
    source_spell['properties']['execution'] = {'type': 'object', 'additionalProperties': False,
        'required': ['native_behavior'], 'properties': {'native_behavior': {'oneOf': [closed_schema(item) for item in variants]}}}
    native_schema = {'oneOf': [closed_schema(item) for item in variants]}
    source_spell['properties']['execution']['properties']['native_behavior'] = {'$ref': '#/$defs/nativeBehavior'}
    schema = {'$defs': {'nativeBehavior': native_schema}, '$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'urn:oteryn:source-equipment-native-extensions:1',
        'title': 'Closed source equipment parameter contract candidates; consumer pending',
        'type': 'object', 'additionalProperties': False, 'required': ['spell'], 'properties': {'spell': source_spell}}
    composed = copy.deepcopy(validation.SCHEMAS['spell.schema.json'])
    composed['$id'] = 'urn:oteryn:spell-authoring:source-complete:candidate:2'
    composed['$defs']['nativeBehavior'] = {'anyOf': [composed['$defs']['nativeBehavior'],
        {'$ref': schema['$id'] + '#/$defs/nativeBehavior'}]}
    registry = validation.REGISTRY.with_resources([(schema['$id'], Resource.from_contents(schema)),
        (composed['$id'], Resource.from_contents(composed))])
    Draft202012Validator.check_schema(schema)
    Draft202012Validator.check_schema(composed)
    validator = Draft202012Validator(composed, registry=registry)
    for path, value in packet.items():
        if path.endswith('/spell.json'):
            validator.validate(value)
    packet['source-spell-complete.schema.json'] = composed
    flags = {'runtime_activation': False, 'native_execution_qualified': False, 'input_provider_equivalence': False,
             'canonical_selection_changed': False, 'native_identity_allocation': False, 'consumer_contract_qualified': False}
    for record in records:
        prefix = record['source_header_path'].rsplit('/', 1)[0] + '/'
        packet[prefix + 'projection-receipt.json'] = {key: value for key, value in record.items() if key not in ('source_header_path',)}
    packet['source-equipment-native-extensions.schema.json'] = schema
    packet['receipt.schema.json'] = base.receipt_schema()
    packet['import-summary.json'] = {'records': 23, 'full_candidates': 23, 'full_source_mechanics_1_to_1_complete': False,
        'status_counts': {'CANDIDATE_SCHEMA_VALID': 23}, 'records_index': records, **flags}
    packet['lane-audit.json'] = {'records': records, 'record_count': 23, 'full_candidate_count': 23, **flags}
    helpers = []
    for donor in ('canary', 'crystal'):
        for path in ('data/scripts/lib/register_spells.lua', 'src/creatures/players/player.cpp',
                     'src/creatures/combat/combat.cpp', 'src/creatures/combat/spells.cpp',
                     'src/lua/functions/creatures/player/player_functions.cpp', 'src/game/game.cpp'):
            content = base.source_file(source_root / donor, prior.PINS[donor], path)
            helpers.append({'donor': donor, 'path': path, 'source_revision': prior.PINS[donor],
                'sha256': sha(content), 'evidence_method': 'local_immutable_git_object',
                'runtime_equivalence_qualified': False})
    packet['projection-proof.json'] = {'records': records, 'schema_sha256': sha(canonical(schema)),
        'schema_file_bytes_sha256': sha((json.dumps(schema, sort_keys=True, indent=2) + '\n').encode()),
        'source_pins': prior.PINS, 'helper_proofs': helpers, 'capture_path': prior.source_library.CAPTURE_PATH, 'capture_sha256': prior.source_library.CAPTURE_SHA,
        'producer_sha256': sha(Path(__file__).read_bytes()),
        'offensive_sign_normalization_proof': {'decision': 'S27',
            'path': 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md',
            'sha256': sha((repo / 'docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md').read_bytes()),
            'section': 'D.4.1 Weapon-skill formula; A.2 Spiritual Outburst spender',
            'source_raw_health_sign_preserved_in_parameters_and_projection_receipts': True,
            'target_offensive_health_delta_negative': True, 'source_health_equivalence': False}, 'target_header_schema_valid': True,
        'actual_reader_shape_valid': True, 'actual_native_reader_admission': False,
        'policy_proofs': [{'path': prior.POLICY_PATH, 'sha256': sha((repo / prior.POLICY_PATH).read_bytes()),
            'exact_rows': [line for line in (repo / prior.POLICY_PATH).read_text().splitlines() if any(line.startswith('| ' + key + ' |') for key in ('S5', 'S16', 'S27'))]}], **flags}
    return packet, schema


def write(repo, source_root):
    repo = Path(repo); packet, schema = build(repo, source_root)
    SCHEMA_FILE.write_text(json.dumps(schema, sort_keys=True, indent=2) + '\n')
    out = repo / OUTPUT; out.mkdir(parents=True, exist_ok=True)
    for relative, value in packet.items():
        path = out / relative; path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value if isinstance(value, bytes) else (json.dumps(value, sort_keys=True, indent=2) + '\n').encode())
    manifest = {'files': {str(path.relative_to(out)): sha(path.read_bytes()) for path in sorted(out.rglob('*')) if path.is_file() and path.name != 'package-manifest.json'}}
    (out / 'package-manifest.json').write_text(json.dumps(manifest, sort_keys=True, indent=2) + '\n')
    return manifest


if __name__ == '__main__':
    parser = argparse.ArgumentParser(); parser.add_argument('--repo', default='.'); parser.add_argument('--source-root', default='/workspace/spell-sources')
    args = parser.parse_args(); write(args.repo, args.source_root)
