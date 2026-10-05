"""Four immutable, source-qualified delayed/owned-field authoring candidates.

These records describe reference semantics, not executable Lua or runtime admission.
The schemas deliberately qualify these four snapshots only: any semantic edit needs
new evidence and a new candidate. Wiki-stated facts outrank S21 source selection;
unknown interaction/timing decisions remain explicit and fail runtime admission.
"""
from copy import deepcopy
import hashlib
from pathlib import Path

REVISIONS = {
    'canary': '99902524e052f37574194466c2949c576e4ab269',
    'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a',
}
FILES = {
    'death echo': 'data/scripts/spells/attack/death_echo.lua',
    'divine empowerment': 'data/scripts/spells/support/divine_empowerment.lua',
    'divine grenade': 'data/scripts/spells/attack/divine_grenade.lua',
    'spiritual outburst': 'data/scripts/spells/attack/spiritual_outburst.lua',
}
HASHES = {
    'death echo': {
        'canary': 'ca7b92f59be6b35b505fde596469ffcfd3c7d6b763849bd9a578dc8cb43966c1',
        'crystal': '4d541cf3c4db8af260d1659ae38e93f25a8e6c88b6a57ec12887cf9f0c026682'},
    'divine empowerment': {
        'canary': '105872cef06f9b9dda720a9eff1af24884c3e0cc6979be6f7a7e21891bf4830b',
        'crystal': '11e48f91f887a49d8ccb4b5ab243943a51405eb13f0dfc5cc870dae3de910965'},
    'divine grenade': {
        'canary': '4db33a343efa3f45ae5ec968fa37916b0934c7d279dcae9bb7a1a79fc7921e00',
        'crystal': '5a4f852926655219e616900e36f91c51ad1dba228def631cd2baeaeafe4a2882'},
    'spiritual outburst': {
        'canary': '5148f5ebd5bdd58e1975733d007e029e29846a64612967f55baf2e742ccc1c51',
        'crystal': '9d871a326fe9b9fae327c82b10fce8304811f550463f8c0049f9757ae06ebd62'},
}
SUPPORT = [
    ('data/scripts/lib/register_spells.lua',
     'a0a6ff2bdfcd82bc4c36978fc57061eab9f1537e8ca966c51300471e3d3313ae'),
    ('src/creatures/players/components/wheel/player_wheel.cpp',
     'e1eef362cce79252cae9f9d469b57ccb5a1b1658e848ed8c8d1881f5c8650717'),
    ('src/utils/utils_definitions.hpp',
     'b66a1389bfecfe11d93338208f7e2e3f8efc14fb0941d6d12dc0d9e62ed4abac'),
    ('src/creatures/combat/combat.cpp',
     '3dc306b44b9ea0c04a48cb6dcdca63b3982a4ee74484cb6a9da40265ecc7fc49'),
    ('src/creatures/players/player.cpp',
     'be92f3b797a5692843b1cce09250287ce52f91afbcfd5edaf9c6fddccfbb9232'),
]
CIRCLE = [[0, 1, 1, 1, 0], [1, 1, 1, 1, 1], [1, 1, 3, 1, 1],
          [1, 1, 1, 1, 1], [0, 1, 1, 1, 0]]
STANCE_FIELDS = [
    'elementalStanceIntrinsicType', 'elementalStanceResolvedType',
    'elementalStanceStateRevision', 'elementalStanceSpellId',
    'elementalStanceDamageMultiplier', 'elementalStanceCriticalChance',
    'elementalStanceCriticalDamage', 'elementalStanceConverted',
]


def _evidence(source, path, sha256):
    return {'source': source, 'revision': REVISIONS[source], 'path': path,
            'sha256': sha256, 'authority': 'OtsHypothesisOnly'}


def _const(value):
    return {'const': str(value)}


def _var(value):
    return {'var': value}


def _op(operation, *arguments):
    return {'op': operation, 'args': list(arguments)}


def _formula(kind):
    flat = {'fn': 'level_base_damage_healing', 'args': [_var('level')]}
    if kind == 'magic':
        center = _op('add', flat, _op('add',
                     _op('mul', _op('div', _var('base_power'), _const(25)),
                         _var('magic_level')),
                     _op('div', _var('base_power'), _const(4))))
        minimum = _op('mul', deepcopy(center), _const('0.9'))
        maximum = _op('mul', deepcopy(center), _const('1.1'))
    elif kind == 'skill':
        center = _op('add', _op('mul',
                     _op('mul', _var('base_power'),
                         _op('div', _var('attack_skill'), _const(100))),
                     _op('div', _var('attack_value'), _const(10))), flat)
        minimum = _op('sub', deepcopy(center), _op('div', deepcopy(center), _const(10)))
        maximum = _op('add', deepcopy(center), _op('div', deepcopy(center), _const(10)))
    else:
        # S5 replaces the linear level/5 only. The 4..6 magic coefficients
        # remain the pinned source baseline, whose base-power model is unresolved.
        minimum = _op('add', deepcopy(flat), _op('mul', _var('magic_level'), _const(4)))
        maximum = _op('add', flat, _op('mul', _var('magic_level'), _const(6)))
    return {'kind': 'player_expression', 'inputs': 'skill' if kind == 'skill' else 'level_magic',
            'minimum': minimum, 'maximum': maximum, 'selection_rule': 'S5/S21'}


def _omission(source, field, value, rule):
    return {'source': source, 'field': field, 'value': value,
            'status': 'approved_omission', 'selection_rule': rule}


def _common(name):
    return {'spell_name': name, 'selection_rule': 'WIKI/S21/S5/S24',
            'runtime_admission': 'rejected_until_native_contract',
            'source_evidence': [_evidence(s, FILES[name], HASHES[name][s]) for s in REVISIONS],
            'support_evidence': [_evidence('canary', path, digest) for path, digest in SUPPORT],
            'commitment': {'cost_occurrences': 1, 'cooldown_occurrences': 1,
                           'refund_after_committed_effect': False,
                           'anchor': 'requires_owned_native_contract'},
            'unresolved_decisions': [], 'approved_omissions': []}


def _raw_snapshot(name):
    p = _common(name)
    if name == 'divine empowerment':
        p.update({
            'wheel_stage_required': True,
            'field_item': {'family': 'Item', 'key': 'candidate:item/40450', 'revision': 'spell-p2-r21'},
            'position_source': 'caster_at_cast', 'radius_tiles': 1, 'duration_ms': 5000,
            'maximum_fields': 9, 'field_count_per_tile': 1,
            'skip_tile_flags': ['missing_tile', 'immovable_block_solid', 'floor_change'],
            'item_owner': 'caster', 'owner_only': True, 'same_floor_required': True,
            'bonus_damage_percent_by_stage': [8, 10, 12],
            'cooldown_ms_by_stage': [32000, 28000, 24000],
            'evaluate': 'cast_and_periodic_poll', 'evaluation_interval_ms': 1000,
            'cleanup': 'only_created_item_instances',
            'expiry_requires_caster_online': False,
            'unresolved_decisions': ['Overlap/stacking across recasts requires the Item owner contract.',
                                     'Owner identity mapping and per-event evaluation require the native contract.'],
            'approved_omissions': [
                _omission('canary', 'cleanup', 'remove_first_matching_type_regardless_of_owner', 'owner_only_cleanup'),
                _omission('crystal', 'cleanup', 'remove_first_matching_type_regardless_of_owner', 'owner_only_cleanup'),
                _omission('crystal', 'ownerless_cleanup', True, 'owner_only_cleanup'),
                _omission('crystal', 'placement', 'no_explicit_tile_flag_filter', 'S21')],
        })
        return {'key': 'owned_field_buff', 'parameters': p}
    p.update({'delay_ms': 3000 if name == 'divine grenade' else 1000,
              'requires_caster_online': True, 'base_power': {'death echo': 75, 'divine grenade': 190,
                                                           'spiritual outburst': 42}[name],
              'schedule_bound': 1, 'repeat_resource_cost': False})
    if name == 'death echo':
        p.update({
            'position_source': 'cast_position', 'position_timing': 'at_cast',
            'target_resolution': 'area_creatures_at_each_occurrence', 'area': deepcopy(CIRCLE),
            'damage_element': 'death', 'effect_asset_binding': 'canary.appearance:effect/death_echo',
            'formula': _formula('magic'), 'immediate_strike': True,
            'delay_multiplier': '0.5', 'value_timing': 'at_expiry',
            'source_value_timing': 'at_expiry', 'suppress_charms': True,
            'immediate_suppress_charms': False, 'ignore_caster_floor': True,
            'elemental_stance_snapshot': list(STANCE_FIELDS),
            'identity_gate': 'player_id_and_guid', 'schedule_after_successful_immediate_strike': True,
            'unresolved_decisions': [
                'Wiki initial damage versus Canary re-evaluated stats/roll remains unresolved.',
                'Official door fix does not specify placement versus area propagation legality.',
                'Modifier snapshot order and session-generation fencing require the native contract.'],
            'approved_omissions': [
                _omission('crystal', 'base_power', 85, 'WIKI/S24:official_8872'),
                _omission('crystal', 'mana', 155, 'S24:official_spell_list_150'),
                _omission('crystal', 'area', '25_tile_square', 'S21:wiki_5x5_does_not_specify_corners'),
                _omission('crystal', 'identity_gate', 'player_id_only', 'S21'),
                _omission('crystal', 'schedule_after_successful_immediate_strike', False, 'S21')],
        })
    elif name == 'divine grenade':
        p.update({
            'wheel_stage_required': True, 'position_source': 'cast_position', 'position_timing': 'at_cast',
            'target_resolution': 'area_creatures_at_explosion', 'area': deepcopy(CIRCLE),
            'damage_element': 'holy', 'effect_asset_binding': 'canary.appearance:effect/holydamage',
            'formula': _formula('grenade'), 'immediate_strike': False,
            'value_timing': 'raw_at_cast_modifiers_at_expiry', 'suppress_charms': False,
            'ignore_caster_floor': False, 'identity_gate': 'player_id_source_baseline',
            'marker_asset_binding': 'canary.appearance:effect/divine_grenade', 'marker_ms': 3000,
            'marker_removal_requires_caster_online': False,
            'cooldown_ms_by_stage': [26000, 20000, 14000],
            'base_damage_bonus_percent_by_stage': [0, 16, 32],
            'snapshot_inputs': ['magic_level', 'flat_damage'],
            'expiry_inputs': ['critical_chance', 'extra_critical_damage', 'divine_empowerment',
                              'elemental_pierce', 'prey', 'bounty_talisman'],
            'late_damage_buffs_apply': False,
            'placement_gate': 'existing_tile_and_no_pz_from_outside_pz',
            'unresolved_decisions': [
                'Cumulative interpretation of wiki +16% per additional stage requires acceptance.',
                'Base power 190 versus source magic coefficients 4..6 requires a qualified formula.',
                'Wiki excludes late buffs but lists expiry modifiers; modifier snapshot split needs resolution.',
                'Out-of-range target fallback and door/placement legality need target owner contract.',
                'Player-id reuse and session-generation fence require the native contract.'],
            'approved_omissions': [
                _omission('canary', 'value_timing', 'at_expiry', 'WIKI:raw_damage_at_cast'),
                _omission('crystal', 'value_timing', 'at_expiry', 'WIKI:raw_damage_at_cast'),
                _omission('canary', 'formula_stage_multiplier', ['1.3', '1.6', '2.0'], 'WIKI:0_16_32_percent'),
                _omission('crystal', 'formula_stage_multiplier', ['1.3', '1.6', '2.0'], 'WIKI:0_16_32_percent'),
                _omission('canary', 'engine_stage_bonus_percent', [30, 60, 100], 'WIKI:0_16_32_percent'),
                _omission('crystal', 'placement_gate', 'walkable_non_house_non_pz', 'S21'),
                _omission('crystal', 'range', 7, 'WIKI/S21:4')],
        })
    else:
        p.update({
            'wheel_stage_required': True, 'position_source': 'original_cast_variant',
            'position_timing': 'variant_at_cast', 'target_resolution': 'chain_reacquired_at_occurrence',
            'damage_element': 'physical', 'chain_effect_asset_binding': 'canary.appearance:effect/blow_white',
            'formula': _formula('skill'), 'immediate_strike': True,
            'value_timing': 'at_expiry_source_baseline', 'suppress_charms': False,
            'ignore_caster_floor': False, 'identity_gate': 'guid_lookup_source_baseline',
            'requires_full_harmony': True, 'full_harmony_value': 5, 'harmony_gate_timing': 'at_cast',
            'harmony_role': 'spender', 'repeat_harmony_multiplier': 'current_harmony_at_expiry',
            'delay_damage_percent_by_stage': ['37.5', '50', '62.5'],
            'cooldown_ms_by_stage': [24000, 20000, 16000],
            'chain': {'additional_targets': 7, 'jump_range_tiles': 4, 'backtracking': False,
                      'exclude': ['npc', 'caster', 'protection_zone_target'],
                      'selection': 'nearest_euclidean', 'tie_break': 'first_in_source_spectator_iteration'},
            'source_schedule_order': 'before_immediate_combat_result',
            'unresolved_decisions': [
                'Wiki original damage versus post-spend Harmony/re-evaluated stats remains unresolved.',
                'Scheduled repeat after failed immediate combat requires explicit commitment policy.',
                'Guid lookup, target ties, equipment/stat timing and session fence need native contract.'],
            'approved_omissions': [
                _omission('crystal', 'delay_ms', 1500, 'WIKI/S21:1000'),
                _omission('crystal', 'additional_targets', 4, 'WIKI/S21:7'),
                _omission('crystal', 'jump_range_tiles', 3, 'WIKI/S21:4'),
                _omission('crystal', 'weapon_element_conversion', True, 'S21:physical'),
                _omission('crystal', 'harmony_role', 'untyped_harmony_boolean', 'S21:spender')],
        })
    return {'key': 'delayed_strike', 'parameters': p}


METADATA_FIELDS = ('selection_rule', 'runtime_admission', 'source_evidence',
                   'support_evidence', 'commitment', 'unresolved_decisions',
                   'approved_omissions')


def _snapshot(name):
    value = _raw_snapshot(name)
    for field in METADATA_FIELDS:
        value['parameters'].pop(field)
    return value


def build(name, spell_type, records, texts):
    """Return a fresh candidate only if both source scripts match their full pins."""
    if not isinstance(name, str) or spell_type != 'instant':
        return None
    name = ' '.join(name.casefold().split())
    if name not in FILES or not isinstance(records, dict) or not isinstance(texts, dict):
        return None
    for source in REVISIONS:
        record = records.get(source)
        if not isinstance(record, dict) or record.get('file') != FILES[name]:
            return None
        if record.get('revision', REVISIONS[source]) != REVISIONS[source]:
            return None
        if record.get('spell_type', spell_type) != spell_type:
            return None
        if str(record.get('name', name)).casefold() != name:
            return None
        text = texts.get((source, FILES[name]))
        if not isinstance(text, str):
            return None
        raw = text.encode('utf-8')
        blob = hashlib.sha1(b'blob ' + str(len(raw)).encode('ascii') + b'\0' + raw).hexdigest()
        if hashlib.sha256(raw).hexdigest() != HASHES[name][source] or record.get('blob') != blob:
            return None
    source_root = records['canary'].get('source_root')
    if source_root is None:
        return None
    for path, digest in SUPPORT:
        try:
            if hashlib.sha256((Path(source_root) / path).read_bytes()).hexdigest() != digest:
                return None
        except (OSError, TypeError, ValueError):
            return None
    return _snapshot(name)


def evidence(name, spell_type, records, texts):
    """Provenance and still-open decisions, kept outside execution parameters."""
    result = build(name, spell_type, records, texts)
    if result is None:
        return None
    name = result['parameters']['spell_name']
    raw = _raw_snapshot(name)['parameters']
    return {field: deepcopy(raw[field]) for field in METADATA_FIELDS}


def _closed_snapshot_schema(value):
    """Exact closed types for a bounded immutable reference snapshot."""
    if isinstance(value, dict):
        return {'type': 'object', 'additionalProperties': False,
                'required': list(value),
                'properties': {k: _closed_snapshot_schema(v) for k, v in value.items()}}
    if isinstance(value, list):
        result = {'type': 'array', 'minItems': len(value), 'maxItems': len(value)}
        if value:
            result.update({'prefixItems': [_closed_snapshot_schema(v) for v in value], 'items': False})
        return result
    return {'const': value}


def schemas():
    """Strict parameter snapshots, keyed by their two candidate discriminants."""
    return {key: {'oneOf': [_closed_snapshot_schema(_snapshot(name)['parameters']) for name in FILES
                           if _snapshot(name)['key'] == key]}
            for key in ('delayed_strike', 'owned_field_buff')}
