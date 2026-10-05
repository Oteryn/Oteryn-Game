"""Source-qualified magic instant damage AST corrections.

This module constructs authoring expressions; it contains no calculator code.
World F stays outside the spell's variation, and the magic contribution is floored
before it is added to F. Parameters are short numerical reference facts, qualified
against captured wiki base powers and the pinned independent calculator metadata.
The original Formula identity and declared Spell base power are never rewritten.
"""
from copy import deepcopy

# name: (captured Spell base power, independent damage variation buckets)
# Wiki captures are samples/wiki-spell-facts-fandom-2026-09-27.json. The numerical
# variation facts are kik-tibia/tibiatools@eeed345c86a76eb00f5ff7b2f0e0b49927799bf1
# src/data/spells.json. Both OTS pins retain conflicting legacy/estimated formulas;
# those conflicts are evidence, not silently treated as agreement.
PARAMETERS = {
    'Energy Strike': (45, 20),
    'Death Strike': (45, 20),
    'Flame Strike': (45, 20),
    'Ice Strike': (45, 20),
    'Terra Strike': (45, 20),
    'Eternal Winter': (200, 130),
    'Forked Glacier': (90, 4),
    'Forked Thorns': (97, 4),
    'Great Death Beam': (155, 50),
    'Great Fire Wave': (100, 50),
    "Hell's Core": (250, 100),
    'Terra Wave': (120, 80),
    'Wrath of Nature': (175, 70),
}


# These independent API cards do not publish bounds. Their source interval width
# is retained, and only its numerical center is qualified against the oracle.
AVERAGE_ONLY = {
    'Fire Wave': 40, 'Ice Wave': 35, 'Rage of the Skies': 200,
    'Physical Strike': 50, 'Lightning': 110,
    'Strong Flame Strike': 125, 'Strong Energy Strike': 125,
    'Strong Ice Strike': 115, 'Strong Terra Strike': 115,
    'Ultimate Flame Strike': 210, 'Ultimate Energy Strike': 210,
    'Ultimate Ice Strike': 195, 'Ultimate Terra Strike': 195,
}


_TITLES = {' '.join(name.casefold().split()): name
           for name in {*PARAMETERS, *AVERAGE_ONLY}}


def _constant(value):
    return {'const': str(value)}


def _op(name, *arguments):
    return {'op': name, 'args': list(arguments)}


def _magic_center():
    power = {'var': 'base_power'}
    return _op('add',
                 _op('mul', _op('div', power, _constant(25)),
                     {'var': 'magic_level'}),
                 _op('div', power, _constant(4)))


def nominal_center():
    """Reference nominal center, never inferred from endpoint midpoint or RNG mean."""
    flat = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    return _op('add', flat, _op('floor', _magic_center()))


def _bound(buckets, direction):
    power = {'var': 'base_power'}
    center = _magic_center()
    # The expression preserves the reference's authored IEEE-754 operation
    # order. Algebraically merging P +/- buckets/2 can move floor at an integer.
    variation = _op('div', _op('div', _constant(buckets), power), _constant(2))
    multiplier = _op('sub' if direction < 0 else 'add', _constant(1), variation)
    spell_term = _op('floor', _op('mul', multiplier, center))
    flat = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    return _op('add', flat, spell_term)


def _magic_interval(expression):
    if expression.get('fn') == 'level_base_damage_healing':
        return _constant(0)
    result = deepcopy(expression)
    if 'args' in result:
        result['args'] = [_magic_interval(arg) for arg in result['args']]
    return result


def _variables(expression):
    result = {expression['var']} if 'var' in expression else set()
    for arg in expression.get('args', []):
        result.update(_variables(arg))
    return result


def _recenter_source_interval(formula):
    bounds = [formula.get(field, {}) for field in ('minimum', 'maximum')]
    projected = []
    for bound, operation in zip(bounds, ('sub', 'add')):
        args = bound.get('args', [])
        if bound.get('op') != 'add' or len(args) != 2:
            break
        floor_args = args[1].get('args', [])
        if args[1].get('op') != 'floor' or len(floor_args) != 1:
            break
        term = floor_args[0]
        term_args = term.get('args', [])
        if term.get('op') != operation or len(term_args) != 2:
            break
        if term_args[0] != _magic_center():
            break
        projected.append(term_args[1])
    if len(projected) == 2 and projected[0] == projected[1]:
        return deepcopy(formula)
    lower = _magic_interval(formula['minimum'])
    upper = _magic_interval(formula['maximum'])
    half_width = _op('div', _op('sub', upper, lower), _constant(2))
    if _variables(half_width) - {'magic_level', 'base_power'}:
        return None
    flat = {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}
    replacement = deepcopy(formula)
    for bound, operation in [('minimum', 'sub'), ('maximum', 'add')]:
        contribution = _op(operation, _magic_center(), deepcopy(half_width))
        replacement[bound] = _op('add', deepcopy(flat), _op('floor', contribution))
    return replacement


def correct(name, spell_type, formula, base_power):
    """Return an identity-preserving corrected Formula plus evidence notes.

    Only an existing player expression with the qualified base power is eligible.
    Unknown spells, runes, mismatching power and component-specific formulas are
    left alone. This qualifies a numerical base component, not the complete cast.
    """
    # Converter census keys are lowercase; this is exact title normalization,
    # not a synonym, punctuation-stripping or stage/component alias join.
    if not isinstance(name, str):
        return None
    name = _TITLES.get(' '.join(name.casefold().split()))
    parameters = PARAMETERS.get(name)
    average_only = name in AVERAGE_ONLY
    if (parameters is None and not average_only) or spell_type != 'instant':
        return None
    expected_power, buckets = parameters if parameters else (AVERAGE_ONLY[name], None)
    if (type(base_power) is not int or base_power != expected_power
            or formula.get('kind') != 'player_expression'
            or formula.get('inputs') != 'level_magic'):
        return None
    # A central beam formula cannot replace separately authored flank components.
    key = formula.get('identity', {}).get('key', '')
    if name == 'Great Death Beam' and not key.endswith('/combat-1'):
        return None
    if average_only:
        replacement = _recenter_source_interval(formula)
        if replacement is None:
            return None
        notes = [
            f'MAGIC-F2: {name}: retain captured wiki base power {expected_power}; '
            'qualify the BP/25*magic_level+BP/4 nominal center against independent '
            'TibiaTools eeed345c86a76eb00f5ff7b2f0e0b49927799bf1 raw average.',
            'MAGIC-F2: retain the original source magic interval half-width '
            '(source bounds with world F removed), recenter and floor before '
            'adding world F. Formula identity retained.',
            'MAGIC-F2: independent API publishes no min/max for this card. '
            'Only nominal center and interval containment are checked; neither '
            'corrected endpoints, midpoint-as-mean, RNG expected value nor full '
            'cast behavior are independently qualified.',
        ]
        if name == 'Lightning':
            notes.append('MAGIC-F2: only the per-target base chain component '
                         'nominal center and source interval containment are '
                         'qualified; chain attenuation, target order and extra '
                         'targets require separate execution validation.')
        return replacement, notes
    replacement = deepcopy(formula)
    replacement['minimum'] = _bound(buckets, -1)
    replacement['maximum'] = _bound(buckets, 1)
    notes = [
        f'MAGIC-F1: {name}: retain captured wiki base power {expected_power}; '
        f'qualify variation buckets {buckets} against independent '
        'TibiaTools eeed345c86a76eb00f5ff7b2f0e0b49927799bf1 numerical metadata.',
        'MAGIC-F1: world level_base_damage_healing is outside variation; floor '
        'the magic contribution before adding the world F. Formula identity retained.',
        'MAGIC-F1: Canary 99902524 and Crystal ff7ede5 contain conflicting '
        'legacy/estimated coefficients or whole-total variation; this correction '
        'does not assert agreement with those OTS formulas or qualify RNG/cast behavior.',
    ]
    if name.startswith('Forked '):
        notes.append('MAGIC-F1: only the per-target base damage component is '
                     'qualified; chain attenuation, target order and wheel targets '
                     'require separate execution validation.')
    if name == 'Great Death Beam':
        notes.append('MAGIC-F1: only the stage-0 central beam component is '
                     'qualified; Beam Mastery stages and left/right side beams '
                     'are not inferred from this Formula.')
    return replacement, notes
