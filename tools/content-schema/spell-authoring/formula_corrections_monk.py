"""Repair qualified monk builder centers while retaining source callback dispersion.

The pinned calculator supplies an average model, not a monk RNG distribution: its
zero buckets deliberately omit min/max. Canary and Crystal both provide the
0.9/1.1 callback spread. Source-qualified spender baselines can be recovered with
the current whole-bound contract; a calculator-compatible spender model requires
a separate runtime/contract repair. Never bake charges or a full-Harmony
multiplier into an authored baseline.
"""
from copy import deepcopy


BACKEND_PIN = 'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1'
BUILDER_BASE_POWERS = {
    'chained penance': 70,
    'double jab': 40,
    'flurry of blows': 55,
    'forceful uppercut': 130,
    'greater flurry of blows': 86,
    'mystic repulse': 85,
    'thousand fist blows': 62,
}

CANARY_PIN = '99902524e052f37574194466c2949c576e4ab269'
SPENDER_SOURCES = {
    'tiger clash': {
        'power': 15, 'blob': 'd529695ac0e5b793ec8f6d584f5f2c63db5735dd',
        'sha256': '8ebb9d836429692db542a36f0567b6b1781991073580d829677ad75f6640bd9c',
        'error': 'branches on a runtime value (attempt to compare table with number)',
    },
    'greater tiger clash': {
        'power': 44, 'blob': 'f43c8b519319587faa8c5f2cc771e11485e6edf1',
        'sha256': '5c4689be0e44eca8eb4386cf924f0e46f13e12f3132d9f313bb24815ac467136',
        'error': '[string "<python>"]:18: attempt to perform arithmetic on local \'harmonyMax\' (a nil value)',
    },
    'devastating knockout': {
        'power': 62, 'blob': 'ccd497d4744023b3268ac14c147e5ea354abd95d',
        'sha256': 'e4c203514b629841a36e58d4fcd0c69f4edaece658e12732ea88b9f93536c02d',
        'error': '[string "<python>"]:18: attempt to perform arithmetic on local \'harmonyMax\' (a nil value)',
    },
}


def _const(value):
    return {'const': str(value)}


def _var(name):
    return {'var': name}


def _op(name, *args):
    return {'op': name, 'args': list(args)}


def center_expression():
    """Qualified unmodified-power center; all runtime skill/weapon inputs stay live.

    Math.round is represented by floor(x+0.5) only on the nonnegative term:
    P>0, skill>=0 and weapon attack>=0 are guaranteed by the owning inputs.
    F stays outside rounding, as in the reference model. The world F is integer.
    """
    power = _var('base_power')
    term = _op('add',
               _op('mul', _op('mul', _op('div', power, _const(1000)),
                              _var('attack_skill')), _var('attack_value')),
               _op('div', power, _const(4)))
    return _op('add',
               {'fn': 'level_base_damage_healing', 'args': [_var('level')]},
               _op('floor', _op('add', term, _const('0.5'))))


def correct(name, spell_type, formula, base_power):
    """Return a replacement and evidence notes, or None outside the proven slice.

    Keep identity, input class and arbitrary caller inputs. Exact names avoid
    conflating Double Jab40 with Swift Jab12 or manufacturing spender ownership.
    """
    if not isinstance(name, str) or spell_type != 'instant':
        return None
    expected = BUILDER_BASE_POWERS.get(name.casefold())
    if expected is None or type(base_power) is not int or base_power != expected:
        return None
    if (not isinstance(formula, dict) or formula.get('kind') != 'player_expression'
            or formula.get('inputs') != 'skill' or 'identity' not in formula):
        return None
    replacement = deepcopy(formula)
    center = center_expression()
    # Preserve the outer operation order of return -total*0.9, -total*1.1.
    replacement['minimum'] = _op('mul', deepcopy(center), _const('0.9'))
    replacement['maximum'] = _op('mul', deepcopy(center), _const('1.1'))
    notes = [
        'Monk builder center corrected to F + round((P/1000)*skill*weapon_attack + P/4); '
        'F is the S5 world level curve and stays outside rounding.',
        f'Average model independently checked against genuine TibiaTools backend {BACKEND_PIN}; '
        'power agrees with the current wiki/official resolution used by the candidate.',
        'The callback 0.9/1.1 spread is retained from pinned Canary/Crystal; calculator '
        'buckets=0 omits bounds and does not prove deterministic damage or its RNG mean.',
        'No Harmony is applied here: builder accumulation remains owned by the runtime '
        'actor after successful cast commit; failed casts do not gain charges.',
    ]
    return replacement, notes


def baseline_spender_formula(name, spell_type, base_power, source, callback_error):
    """Recover exactly the three pinned pre-Harmony Canary callback expressions.

    The caller must qualify CANARY_PIN/blob and retain harmony_role=spender.
    Harmony is removed from the callback once because the actor owns it. This is
    the accepted source baseline, not the separate calculator-compatible proposal.
    Return a body without identity so the owning converter can attach its own ref.
    """
    if not isinstance(name, str) or source != 'canary' or spell_type != 'instant':
        return None
    qualification = SPENDER_SOURCES.get(name.casefold())
    if (qualification is None or type(base_power) is not int
            or base_power != qualification['power'] or callback_error != qualification['error']):
        return None
    # Original Lua: P*(skill/100)*(attack/10) + calculateFlatDamageHealing().
    # S5 replaces only the defective flat helper with the accepted world curve.
    damage = _op('add',
                 _op('mul', _op('mul', _var('base_power'),
                                _op('div', _var('attack_skill'), _const(100))),
                     _op('div', _var('attack_value'), _const(10))),
                 {'fn': 'level_base_damage_healing', 'args': [_var('level')]})
    minimum = _op('sub', deepcopy(damage), _op('div', deepcopy(damage), _const(10)))
    maximum = _op('add', deepcopy(damage), _op('div', deepcopy(damage), _const(10)))
    if name.casefold() == 'tiger clash':
        minimum = _op('max', minimum, _const(5))
        maximum = _op('max', maximum, _const(10))
    return {'kind': 'player_expression', 'inputs': 'skill',
            'minimum': minimum, 'maximum': maximum}
