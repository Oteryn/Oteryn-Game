"""Pinned actor-state authoring, with explicit operations and no live admission claim.

S21 selects Canary for script mechanics; S27 candidate data keeps later official
numbers where the candidate explicitly supersedes Canary. D145 stance retention
is an existing owner decision. This module does not change the Harmony formula,
Ascetic placement, Formula input authority or cast commit ownership.
"""
from copy import deepcopy
import hashlib
from pathlib import Path
import subprocess

from formula_corrections_monk import center_expression

PINS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
        'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
SOURCE_SPECS = {'blood rage': {'canary': {'blob': '544afe1cf1f129fd040b18bf725a221bbec0dd4f',
                           'file': 'data/scripts/spells/support/blood_rage.lua',
                           'sha256': 'b4301ab28386dde40ca0b8ecfb81efc1e77a2ab69bb7a2fbde31f2d67110e483'},
                'crystal': {'blob': '20cddc26c49d01afae82f682004a628cd8c35200',
                            'file': 'data/scripts/spells/support/blood_rage.lua',
                            'sha256': '080706389fa40f3659ffa2bed9cb577780918ba405e95cb585d37b43f154b690'}},
 'flurry of blows': {'canary': {'blob': '95c24a1c3b5a096bff47070b2a9287c2a8a37c35',
                                'file': 'data/scripts/spells/attack/flurry_of_blows.lua',
                                'sha256': '2e491e42e80d7eccfb72b0c5cc8560c550257f021a6bd70065fdff6dfc086e76'},
                     'crystal': {'blob': 'e0f64393b937022cf8c5c30433a8444d7ca7ec55',
                                 'file': 'data/scripts/spells/attack/flurry_of_blows.lua',
                                 'sha256': '28c8f732b9a9b6a3e809e6ceb6a2d98d7e9950a2ffa63520abb65667fc232842'}},
 'focus harmony': {'canary': {'blob': '733a7f1376f6bbbde3f894cc7a6b398385338b21',
                              'file': 'data/scripts/spells/support/focus_harmony.lua',
                              'sha256': '11dd6cf2d4f028831a8396790a6bb4be3e9939abcaa4c3c3425e8612575673c1'},
                   'crystal': {'blob': '367aa383d64d556b3760394d990f11f5c487da4b',
                               'file': 'data/scripts/spells/support/focus_harmony.lua',
                               'sha256': '008b5685dfe41dbfeaf4768efe1340c825626c7e71a995d424ecf604449d3d05'}},
 'focus serenity': {'canary': {'blob': 'd73714fc69d1b9c88f9af42e2857b8cc7c1d2540',
                               'file': 'data/scripts/spells/support/focus_serenity.lua',
                               'sha256': '78a7e2bcff918a4505aa4883ef6fefca6738313c534147f89951e04a58b2b8bd'},
                    'crystal': {'blob': 'd2cc1aa2ee8eb035fb1e9042e0c52f5ed10eb561',
                                'file': 'data/scripts/spells/support/focus_serenity.lua',
                                'sha256': 'ba2c886c142e07226d00da8cda8f262ae8492a97d3699466d831690680519b90'}},
 'protector': {'canary': {'blob': '4823566597af625fd0bd6c46441c64a134e7e0da',
                          'file': 'data/scripts/spells/support/protector.lua',
                          'sha256': '05d442dc129410586ef1604e31ddec89f37de156d8b0a7d7a3ea662802b157df'},
               'crystal': {'blob': '8188e2f19f07b5e390d363f853bacbb013a759cf',
                           'file': 'data/scripts/spells/support/protector.lua',
                           'sha256': 'ffd89e7dec660b5a5289a89a8a4443be7d0ecc8661b0b99b1a5ebaa149be79e5'}},
 'sharpshooter': {'canary': {'blob': '1190e025dae975315d20bb4f1eb4f33e71094d6d',
                             'file': 'data/scripts/spells/support/sharpshooter.lua',
                             'sha256': '293134a33a30129c96615be22a8976c96db5eb3c029b3fb0f9f74da02390ab91'},
                  'crystal': {'blob': '7c581b8d81f95f0fa43a688e9ca7d608c3d46fda',
                              'file': 'data/scripts/spells/support/sharpshooter.lua',
                              'sha256': '631f1e057c77b43af7217c842ee01c5522c26a13dfaa36a2bb98237b5796e527'}},
 'shield bash': {'canary': {'blob': 'ed55d71ff17407c1d61f9b85d843d484e65cacef',
                            'file': 'data/scripts/spells/attack/shield_bash.lua',
                            'sha256': 'aef8a7f72401475e8bfb47d40b3c8e0c9d9145048c0fdecf1b77827e10e68a63'},
                 'crystal': {'blob': '11544f9f7fc9f5dae0ca446a06885e25d401f21e',
                             'file': 'data/scripts/spells/attack/shield_bash.lua',
                             'sha256': '832390523bfdbdd7776b8358b324f098f75f918b192e6888b6d9a9a9d6cdf593'}},
 'shield slam': {'canary': {'blob': 'be95f60251f2a5c0c54a1bab89a522e736c4a4ba',
                            'file': 'data/scripts/spells/attack/shield_slam.lua',
                            'sha256': 'f104db392564dd3ca2bd29ec3eeecb0e860640a5103a2701199e063c65056bc6'},
                 'crystal': {'blob': 'a89d44d72c2b1097494264d4a90b4a6bdc5a16db',
                             'file': 'data/scripts/spells/attack/shield_slam.lua',
                             'sha256': '3584e929e6e3687426ab35323928067cfca028c370c462a6674b5175d3bd2dcf'}},
 'sweeping takedown': {'canary': {'blob': '2cbd247a6af2bfb857b05f6bbabc40156baf42d9',
                                  'file': 'data/scripts/spells/attack/sweeping_takedown.lua',
                                  'sha256': '057c8aa5af04ad0eebec5366ec90f51b45e3bea564514e3688c0ee29d5df5dd6'},
                       'crystal': {'blob': '30133f0a982417b1ccb7441b87a8c6bf6a8a1387',
                                   'file': 'data/scripts/spells/attack/sweeping_takedown.lua',
                                   'sha256': '1edf6944482659d885efd966881529849b6f92d8406f0e1f4dbdd88245a7ce4f'}},
 'virtue of harmony': {'canary': {'blob': '6143e04fedb947ebacac36920107709a70c64b9b',
                                  'file': 'data/scripts/spells/support/virtue_of_harmony.lua',
                                  'sha256': 'd7c342b90e99f1c36925cdf5563b698de745ccec7b633f28574b242d67561410'},
                       'crystal': {'blob': 'a0f71d6cf0bf5575195390d67c6cf28dbc190405',
                                   'file': 'data/scripts/spells/support/virtue_of_harmony.lua',
                                   'sha256': 'b21b016c1f734a189734bfa68267469af9a1d3e502eff08dd86fbda7e55da380'}},
 'virtue of justice': {'canary': {'blob': '4dcfbaa19dab27fd660511f369fa15cd98d88798',
                                  'file': 'data/scripts/spells/support/virtue_of_justice.lua',
                                  'sha256': '3deadffa8b7e3967578599f23468f05ced696bacb8af5a0f87f8a725d5215cd4'},
                       'crystal': {'blob': 'b1d96c79575eda8d97cab5425cde90705670b819',
                                   'file': 'data/scripts/spells/support/virtue_of_justice.lua',
                                   'sha256': '6ed0493b70e790bba9763f2b6b41b439ff6ba8d668680c534fbe0d0cd949654a'}},
 'virtue of sustain': {'canary': {'blob': 'e47547662b79026f80802c0362448f9cb876e003',
                                  'file': 'data/scripts/spells/support/virtue_of_sustain.lua',
                                  'sha256': 'e5d0850be5935d9c5a6c718ea3800c3bf14d9661ee770ca078c1348b8711ac7f'},
                       'crystal': {'blob': '8ebda80fcb16b31f28998773f33ba7c4abe91802',
                                   'file': 'data/scripts/spells/support/virtue_of_sustain.lua',
                                   'sha256': 'e9a12e3affd8eb45b0a8ef852eba3e6541a768ef3a07f50a38bd5f32ed56d85b'}}}
HELPERS = {'data/scripts/lib/register_spells.lua': 'a0a6ff2bdfcd82bc4c36978fc57061eab9f1537e8ca966c51300471e3d3313ae',
 'src/creatures/combat/combat.cpp': '3dc306b44b9ea0c04a48cb6dcdca63b3982a4ee74484cb6a9da40265ecc7fc49',
 'src/creatures/players/player.cpp': 'be92f3b797a5692843b1cce09250287ce52f91afbcfd5edaf9c6fddccfbb9232',
 'src/game/game.cpp': 'b337fb7d7ce61d9ccde4f315cb696df0add9a0bf933c9b5c634b9ed79d7f01dd',
 'src/lua/functions/creatures/player/player_functions.cpp': 'c662021b54f61a23e23f3623f1ee6c30133aa31a5b0fe4acf347327f183670ad'}


def _head(root):
    try:
        result = subprocess.run(['git', '-C', str(root), 'rev-parse', 'HEAD'],
                                check=True, capture_output=True, text=True)
        return result.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def _bytes(text):
    return text.encode('utf-8') if isinstance(text, str) else None


def _qualified(name, records, texts):
    if not isinstance(records, dict) or not isinstance(texts, dict) or 'canary' not in records:
        return False
    for source, record in records.items():
        spec = SOURCE_SPECS.get(name, {}).get(source)
        if spec is None or not isinstance(record, dict):
            return False
        root = record.get('source_root')
        if root is None or _head(root) != PINS[source]:
            return False
        if record.get('revision', PINS[source]) != PINS[source]:
            return False
        record_name = record.get('name')
        if not isinstance(record_name, str) or record_name.casefold() != name or record.get('spell_type') != 'instant':
            return False
        if record.get('file') != spec['file'] or record.get('blob') != spec['blob']:
            return False
        data = _bytes(texts.get((source, spec['file'])))
        if data is None or hashlib.sha256(data).hexdigest() != spec['sha256']:
            return False
        if hashlib.sha1(b'blob ' + str(len(data)).encode() + b'\0' + data).hexdigest() != spec['blob']:
            return False
    # Selected engine helpers are also full-file qualified. A matching spell
    # callback alone cannot establish stance final-skill or cooldown semantics.
    root = Path(records['canary']['source_root'])
    for path, expected in HELPERS.items():
        text = texts.get(('canary', path))
        try:
            data = _bytes(text) if text is not None else (root / path).read_bytes()
        except OSError:
            return False
        if data is None or hashlib.sha256(data).hexdigest() != expected:
            return False
    return True


def _c(value):
    return {'const': str(value)}


def _v(name):
    return {'var': name}


def _op(name, *args):
    return {'op': name, 'args': list(args)}


def _flat():
    return {'fn': 'level_base_damage_healing', 'args': [_v('level')]}


def _bounds(center, low, high):
    return {'minimum': _op('mul', deepcopy(center), _c(low)),
            'maximum': _op('mul', deepcopy(center), _c(high)),
            'quantization': 'truncate_each_bound_before_draw',
            'distribution': 'world_damage_roll'}


WINDOW = {'same_floor': True, 'dx_min': -8, 'dx_max': 9,
          'dy_min': -6, 'dy_max': 7, 'exclude_dead_removed_and_self': True}
PARTY_BONUSES = [
    {'vocation': 'knight', 'kind': 'damage_taken_percent', 'percent': -4},
    {'vocation': 'paladin', 'kind': 'auto_attack_damage_percent', 'percent': 8},
    {'vocation': 'sorcerer', 'kind': 'spell_and_rune_damage_percent', 'percent': 8},
    {'vocation': 'druid', 'kind': 'spell_and_rune_healing_percent', 'percent': 16},
]


def _focus(serenity):
    charge_factor = _op('add', _c(1), _op('mul', _c('0.05'), _v('gained_charges')))
    heal = {}
    for key, factor, floor in [('minimum', '2.0', 10), ('maximum', '2.3', 25)]:
        heal[key] = _op('max', _c(floor), _op('ceil',
                       _op('mul', _op('mul', _flat(), _c(factor)), deepcopy(charge_factor))))
    return {'key': 'monk_focus', 'parameters': {
        'fill_harmony': True, 'harmony_max': 5,
        'serene_ms': 7000 if serenity else None,
        'forced_serene_prevents_automatic_clear': serenity,
        'reset_spender_cooldowns': serenity,
        'reset_group_cooldowns': 'canary_group_spell_lookup' if serenity else 'none',
        'cooldown_reset_filter': {
            'individual': 'spender_only' if serenity else 'none',
            'group_without_resolved_spell': 'clear' if serenity else 'preserve',
            'group_with_resolved_spell': 'spender_only' if serenity else 'preserve',
            'invalid_individual_spell': 'preserve',
        },
        'rearm_cast_cooldowns': True,
        'apply_after': 'primary_commit',
        'harmony_gain_healing': {
            'charges': 'max_harmony_minus_precommit_harmony',
            'when': 'positive_gained_charges_only', 'bounds': heal,
            'distribution': 'world_healing_roll',
            'target_selection': 'lowest_absolute_health_from_self_and_visible_party',
            'visibility': deepcopy(WINDOW),
            'sustain_percent': 35, 'serene_sustain_percent': 70,
            'sustain_application': 'after_world_healing_roll',
            'sustain_quantization': 'truncate_toward_zero',
        },
        'presentation': {'effect_asset_binding': 'appearance:effect/magic_blue'},
    }}


def _stance(name, stance_id, modifiers, virtue=None):
    params = {'stance': stance_id, 'slot': 'standard',
              'eligible_vocations': ['monk', 'exalted_monk'] if virtue else (
                  ['royal_paladin'] if name == 'sharpshooter' else ['elite_knight']),
              'toggle_same_stance_off': True, 'replace_existing': True,
              'persist_across_sessions': True, 'keep_on_death': True,
              'toggle_off_costs_mana': True, 'toggle_off_starts_cooldowns': True,
              'prune_on_ineligible_vocation': True,
              'apply_after': 'primary_commit', 'rounding': 'truncate_toward_zero',
              'modifiers': modifiers}
    if virtue:
        params['virtue'] = virtue
        params['party_bonus'] = {
            'requires_any_active_virtue': True,
            'visibility': deepcopy(WINDOW), 'bonuses': deepcopy(PARTY_BONUSES),
            'caster_receives_visible_vocation_union_only_when_serene': True,
            'multiple_monks_stack': False,
        }
    return {'key': 'stance_toggle', 'parameters': params}


def _weapon_input():
    return {'kind': 'wielded_weapon', 'hand_order': ['left', 'right'],
            'exclude_types': ['shield', 'ammunition', 'non_weapon'],
            'skill': 'effective_weapon_skill', 'attack': 'weapon_attack',
            'unarmed': {'attack_skill': 0, 'attack_value': 7},
            'snapshot': 'precommit_owned_equipment_and_skills',
            'damage_type': 'equipped_weapon_elemental_bond_or_physical'}


FLURRY = ['.....', '..x..', '.xxx.', '.xCx.', '.x.x.', '.....']
FLURRY_WHEEL = ['..x..', '.xxx.', '.xxx.', 'xxCxx', '.x.x.', '.....']
SWEEP_INNER = ['.....', '.....', '.xxx.', 'xxCxx', 'xx.xx', '.....']
SWEEP_OUTER = ['.xxx.', 'xxxxx', 'x...x', '..c..', '.....', '.....']


def _equipment(name):
    shield = name in ('shield bash', 'shield slam')
    params = {'spell': name.replace(' ', '_'), 'base_power': {
        'flurry of blows': 55, 'sweeping takedown': 48,
        'shield bash': 55, 'shield slam': 52}[name],
        'apply_after': 'primary_commit', 'all_components_share_input_snapshot': True}
    if shield:
        center = _op('add', _op('mul', _op('mul', _v('base_power'),
                    _op('div', _v('shielding_skill'), _c(100))),
                    _op('div', _v('shield_defense'), _c(10))), _flat())
        params.update({
            'input': {'kind': 'wielded_shield', 'hand_order': ['left', 'right'],
                      'selection': 'first_shield', 'skill': 'effective_shielding',
                      'defense': 'shield_item_defense',
                      'snapshot': 'precommit_owned_equipment_and_skills'},
            'refusal': {'when': 'no_shield',
                        'message': 'You need to equip a shield to cast this spell.',
                        'effect_asset_binding': 'appearance:effect/poff'},
            'damage_type': 'physical', 'blocked_by_armor': True,
            'formula': _bounds(center, '0.9', '1.1'),
            'area': {'kind': 'target'} if name == 'shield bash' else {
                'kind': 'direction_independent_matrix', 'rows': ['xxx', 'xcx', 'xxx']},
            'next_auto_attack_reduction': {
                'duration_ms': 10000, 'percent': 50,
                'wheel_grade_2_additional_percent': 25 if name == 'shield slam' else 0,
                'origins': ['melee', 'ranged', 'fist'],
                'affects_primary_and_secondary': True, 'affects_players': True,
                'consume_at': 'first_auto_attack_damage_step',
                'consume_on_spell': False, 'replacement': 'refresh_same_condition',
            },
        })
    elif name == 'flurry of blows':
        params.update({'input': _weapon_input(),
                       'formula': _bounds(center_expression(), '0.9', '1.1'),
                       'area': {'kind': 'directional_matrix', 'rows': deepcopy(FLURRY)},
                       'wheel_enlarged_area': {'when': 'flurry_additional_area',
                            'rows': deepcopy(FLURRY_WHEEL)},
                       'harmony_role': 'builder', 'harmony_owner': 'cast_actor'})
    else:
        # Preserve the piecewise source term and IEEE-754 order. Runtime needs
        # the existing whole-bound Harmony snapshot, never a BP-only multiplier.
        skill_term = _op('div', _op('mul', _op('mul', _v('attack_skill'),
                        _v('attack_value')), _v('base_power')), _c(1000))
        center = _op('add', _op('add', skill_term, _flat()), _v('sweeping_skill_bonus'))
        params.update({'input': _weapon_input(), 'formula': _bounds(center, '1.3', '1.7'),
                       'skill_bonus': {'threshold_basis': 'attack_skill',
                           'delta_from': 110, 'exponent': 2, 'comparison': 'strictly_greater',
                           'steps': [{'above': k, 'coefficient': v} for k, v in (
                               (250, '0.043'), (230, '0.039'), (210, '0.037'),
                               (190, '0.035'), (160, '0.033'), (140, '0.029'),
                               (130, '0.026'), (120, '0.024'), (110, '0.022'))],
                           'otherwise': '0'},
                       'area': {'kind': 'directional_matrix', 'rows': deepcopy(SWEEP_INNER)},
                       'outer_area': {'rows': deepcopy(SWEEP_OUTER), 'factor': '0.75',
                           'input_bounds': 'inner_harmony_scaled_truncated_bounds',
                           'draw': 'separate_world_roll'},
                       'harmony_role': 'spender', 'harmony_owner': 'cast_actor',
                       'harmony_scaling': 'existing_whole_bound_multiplier_once',
                       'cast_cache': 'occurrence_local_bounds_not_global_caster_cache'})
    return {'key': 'equipment_attack', 'parameters': params}


MODELS = {
    'focus harmony': _focus(False), 'focus serenity': _focus(True),
    'blood rage': _stance('blood rage', 'blood_rage', [
        {'kind': 'skill_percent_of_final', 'skills': ['sword', 'axe', 'club'], 'percent': 25},
        {'kind': 'damage_taken_percent', 'percent': 15, 'exclude_healing': True}]),
    'protector': _stance('protector', 'protector', [
        {'kind': 'skill_percent_of_final', 'skills': ['shielding'], 'percent': 30},
        {'kind': 'damage_taken_percent', 'percent': -15, 'exclude_healing': True},
        {'kind': 'damage_dealt_percent', 'percent': -15, 'exclude_healing': True}]),
    'sharpshooter': _stance('sharpshooter', 'sharpshooter', [
        {'kind': 'skill_percent_of_final', 'skills': ['distance'], 'percent': 32}]),
    'virtue of harmony': _stance('virtue of harmony', 'virtue_of_harmony', [
        {'kind': 'harmony_base_bonus_scale', 'percent': 50, 'serene_percent': 100,
         'basis': 'level_scaled_base_bonus', 'spender_refund_charges': 1}], 'harmony'),
    'virtue of justice': _stance('virtue of justice', 'virtue_of_justice', [
        {'kind': 'fist_bonus_percent', 'percent': 8, 'serene_percent': 16, 'basis': 'base'}], 'justice'),
    'virtue of sustain': _stance('virtue of sustain', 'virtue_of_sustain', [
        {'kind': 'healing_done_percent', 'percent': 35, 'serene_percent': 70,
         'scope': 'monk_spells_and_harmony_healing'}], 'sustain'),
}
MODELS.update({name: _equipment(name) for name in (
    'flurry of blows', 'sweeping takedown', 'shield bash', 'shield slam')})


def build(name, spell_type, records, texts):
    """Return native data only for an exact named, full-file qualified snapshot."""
    if not isinstance(name, str) or spell_type != 'instant':
        return None
    normalized = name.casefold()
    if normalized not in MODELS or not _qualified(normalized, records, texts):
        return None
    return deepcopy(MODELS[normalized])


def schemas():
    """Closed parameter variants: additions and changed facts require new qualification.

An object-level const includes every recursively nested field and forbids missing
or extra fields. Finite schemas prevent an arbitrary native script from being
smuggled into an otherwise structurally valid nativeBehaviour object.
"""
    return {key: {'type': 'object', 'oneOf': [
        {'const': deepcopy(model['parameters'])} for model in MODELS.values()
        if model['key'] == key]} for key in ('monk_focus', 'stance_toggle', 'equipment_attack')}


def evidence(name=None):
    """Source/decision attribution is separate from executable parameters."""
    names = [name.casefold()] if isinstance(name, str) else list(MODELS)
    return {n: {'source_selection': 'S21 Canary mechanics; S27 authored candidate data',
                'source_pins': deepcopy(PINS), 'source_files': deepcopy(SOURCE_SPECS[n]),
                'helpers': deepcopy(HELPERS),
                'stance_retention': 'D145: logout and death retained; not reopened',
                'contract': 'OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1 A.2/C.4/C.5/D.4',
                'runtime_ready': False, 'conflicts': [
                    'Crystal timed attribute/base-skill stances and gates differ from Canary persistent final-skill stances.',
                    'Canary fixed Harmony8+4/8 differs from official level scaling; no Harmony formula/Ascetic change here.',
                    'Official8944 party4/8/8/16 supersedes Canary3/6/6/12; Guiding Presence33% composition remains Wheel-owned and unqualified.',
                    'Focus individual reset is spender-only; Canary helper resolves group subId as spellId and clears unresolved groups. Crystal clears all CDs; group scope needs owner qualification.',
                    'Focus heal target uses Canary absolute HP selection; official monk-and-ally count and magic-level coefficients remain unqualified.',
                    'Sustain passive35/70 follows Fandom/BR; Canary excludes ORIGIN_HARMONY.',
                    'Sweeping Canary12inner+10outer differs from wiki areas and the candidate text11; caches endpoints, not a shared RNG draw. Outer has a separate draw.',
                    'Canary standard knight/paladin stances require promotion; virtue script permits monk and exalted_monk, unlike wiki promotion-only wording. Admission needs reconciled vocation eligibility.',
                    'Shield Canary first hand/player inclusion/one auto-attack differs from Crystal highest defense/no players/timed all-damage buff.',
                    'Monk unarmed source0skill/7attack remains hypothesis; live admission requires owned equipment facts and owner-qualified unarmed policy.',
                ]} for n in names if n in MODELS}
