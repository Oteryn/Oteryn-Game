"""Bounded ordinary Knight/Paladin formula repair, expressed as authoring AST.

The level curve and BP facts use captured wiki evidence, except the explicit
user-selected Strong Ethereal BP25 override; OTS callbacks establish
which actor facts the formulas read. Numeric power spread/rounding is the pinned
independent calculator compatibility model, not a claim that legacy OTS Lua has
those coefficients. No third-party calculation implementation is imported.
"""
import copy
import math

# name: (accepted BP, spread buckets, denominator, actor scaling, rounding)
# Each entry is an ordinary, non-Wheel-stage spell. Scope is deliberately exact.
MODELS = {
    'annihilation': (125, 80, '1000', 'weapon', 'round'),
    'berserk': (44, 24, '1000', 'weapon', 'round'),
    'brutal strike': (39, 20, '1000', 'weapon', 'round'),
    'fierce berserk': (92, 50, '1000', 'weapon', 'round'),
    'groundshaker': (32, 18, '1000', 'weapon', 'round'),
    'whirlwind throw': (32, 18, '1000', 'weapon', 'round'),
    'divine barrage': (130, 20, '25', 'magic', 'floor'),
    'divine caldera': (150, 40, '25', 'magic', 'floor'),
    'ethereal barrage': (40, 6, '17.8571', 'distance', 'floor'),
    'ethereal spear': (25, 10, '30.30303', 'distance', 'floor'),
    'strong ethereal spear': (25, 10, '19.23077', 'distance', 'floor'),
    'divine missile': (60, 30, '25', 'magic', 'floor'),
}
# Source conflict retained as provenance: all captured wikis say38; the user
# selected the calculator's25 for Strong Ethereal Spear. Keep the empty export
# for existing standalone review drivers; this entry is no longer unresolved.
UNRESOLVED = {}
OWNER_DECISIONS = {
    'strong ethereal spear': 'Source conflict: captured Fandom/BR/tibiopedia BP38 versus '
                            'calculator BP25; resolved by the user instruction '
                            '"kalkulator ma rację". Use declared BP25 and the calculator '
                            'distance model; wiki/OTS coefficients are not claimed to agree.',
}


def _constant(value):
    return {'const': str(value)}


def _variable(name):
    return {'var': name}


def _operation(name, *args):
    return {'op': name, 'args': list(args)}


def _bound(model, sign):
    _, buckets, denominator, scaling, rounding = model
    power = _variable('base_power')
    scaled = _operation('mul', _operation('div', power, _constant(denominator)),
                        _variable('magic_level' if scaling == 'magic' else 'attack_skill'))
    if scaling == 'weapon':
        scaled = _operation('mul', scaled, _variable('attack_value'))
    powered = _operation('add', scaled, _operation('div', power, _constant(4)))
    spread = _operation('div', _operation('div', _constant(buckets), power), _constant(2))
    variation = _operation('add', _constant(1), _operation('mul', _constant(sign), spread))
    varied = _operation('mul', variation, powered)
    # Positive magnitude half-up. Rounding only the powered term keeps the
    # integer world level contribution outside spread and rounding.
    if rounding == 'round':
        varied = _operation('add', varied, _constant('0.5'))
    rounded = _operation('floor', varied)
    flat = _operation('floor', {'fn': 'level_base_damage_healing', 'args': [_variable('level')]})
    return _operation('ceil', _operation('mul', _operation('add', flat, rounded), _constant(1)))


def correct(name, spell_type, formula, base_power):
    """Return (replacement, evidence notes), or None outside the qualified scope.

    Preserve caller-owned identity and other formula metadata; no attack_factor,
    Wheel stage or runtime buff is invented. Missing/nonmatching BP fails closed.
    """
    if not isinstance(name, str) or spell_type != 'instant' or not isinstance(formula, dict):
        return None
    model = MODELS.get(name.strip().lower())
    if model is None or formula.get('kind') != 'player_expression':
        return None
    if isinstance(base_power, bool) or not isinstance(base_power, (int, float)):
        return None
    if not math.isfinite(base_power) or base_power != model[0] or formula.get('stage', 0) != 0:
        return None
    if model[3] == 'magic' and formula.get('inputs') != 'level_magic':
        return None
    if model[3] != 'magic' and formula.get('inputs') != 'skill':
        return None
    replacement = copy.deepcopy(formula)
    replacement.update(minimum=_bound(model, -1), maximum=_bound(model, 1))
    notes = [
        'Skill formula repair: world F is outside variation; powered term uses declared BP, '
        + ('magic_level' if model[3] == 'magic' else 'attack_skill * attack_value' if model[3] == 'weapon'
           else 'distance attack_skill without weapon attack'),
        'Evidence: captured Fandom/BR/tibiopedia BP facts (2026-09-27/28); '
        'Formulae revision1205374 level curve; Canary99902524 and Crystalff7ede5 callbacks '
        'establish actor inputs but retain conflicting legacy coefficient/spread models.',
        'Independent numeric compatibility reference: TibiaTools eeed345c86a76eb00f5ff7b2f0e0b49927799bf1 '
        f'ordinary card reference BP{model[0]}, '
        f'declared BP{model[0]}, buckets{model[1]}, divisor{model[2]}, rounding{model[4]}; '
        'this repair is not a live Tibia or Oteryn runtime cast qualification.',
    ]
    if model[3] == 'distance':
        notes.append('Distance-model conflict remains explicit: legacy Ethereal Barrage reads weapon attack; '
                     'the reference numeric model uses distance skill alone. No attack_factor input is supplied.')
    if name.strip().lower() in OWNER_DECISIONS:
        notes.append(OWNER_DECISIONS[name.strip().lower()])
    return replacement, notes
