"""Offline, neutral damage Formula corrections for twelve runes.

This independently authored AST encodes observed formula facts, not third-party program code.
The genuine pinned calculator supplies the rune power/bucket data and rounding observations;
S5 supplies the integer world level curve. Canary/Crystal alternative models remain evidence
conflicts: a match here qualifies neutral calculator bounds, not RNG or runtime activation.
"""
import copy


CALCULATOR_COMMIT = 'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1'
# (retained base power, variation bucket count, current Fandom item revision)
MODELS = {
    'avalanche rune': (50, 40, 1189757),
    'explosion rune': (60, 40, 1190632),
    'fireball rune': (60, 30, 1189773),
    'great fireball rune': (50, 40, 1189801),
    'heavy magic missile rune': (30, 20, 1189775),
    'holy missile rune': (70, 50, 1186602),
    'icicle rune': (60, 30, 1189777),
    'light magic missile rune': (15, 10, 1189779),
    'stalagmite rune': (30, 20, 1197225),
    'stone shower rune': (50, 40, 1197224),
    'sudden death rune': (150, 70, 1189790),
    'thunderstorm rune': (50, 40, 1190612),
}


def _const(value):
    return {'const': str(value)}


def _op(name, *args):
    return {'op': name, 'args': list(args)}


def _expression(name, direction=None):
    """Neutral endpoint: floor(F(L)) + floor((1 ± B/P/2) * (P/25*ML + P/4)).

    Retain division/operation order and round the magic term before adding the level bonus.
    F is outside the variation. The calculator's final ceil at multiplier one is redundant
    because both summands are already integers, so it adds no extra AST node.
    """
    _, buckets, _ = MODELS[name]
    power = {'var': 'base_power'}
    level = _op('floor', {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]})
    magic = _op('add', _op('mul', _op('div', power, _const(25)), {'var': 'magic_level'}),
                _op('div', power, _const(4)))
    if direction is not None:
        variation = _op('div', _op('div', _const(buckets), power), _const(2))
        factor = _op('add', _const(1), _op('mul', _const(direction), variation))
        magic = _op('mul', factor, magic)
    return _op('add', level, _op('floor', magic))


def average_expression(name):
    """Diagnostic calculator average; Formula min/max do not prescribe a draw distribution."""
    name = name.strip().lower()
    return _expression(name) if name in MODELS else None


def correct(name, spell_type, formula, base_power):
    """Return (corrected Formula, evidence notes) for a known rune, or None if out of scope.

    A power/shape mismatch fails closed instead of overwriting another model. This function
    never changes base power, item identity, spell identity or the Formula identity.
    """
    name = name.strip().lower()
    if spell_type != 'rune' or name not in MODELS:
        return None
    expected_power, buckets, revision = MODELS[name]
    if isinstance(base_power, bool) or base_power != expected_power:
        raise ValueError(f'{name}: retained base power {base_power!r} differs from qualified {expected_power}')
    if formula.get('kind') != 'player_expression' or formula.get('inputs') != 'level_magic' or not formula.get('identity'):
        raise ValueError(f'{name}: correction requires an identified level_magic player_expression Formula')
    replacement = copy.deepcopy(formula)
    replacement['minimum'] = _expression(name, -1)
    replacement['maximum'] = _expression(name, 1)
    evidence = [
        f'Offline neutral rune model: base power {expected_power}, skill divisor 25, buckets {buckets}, '
        'floor the magic term, then add integer world F outside variation; no perks or additional multiplier.',
        f'Genuine calculator evidence: kik-tibia/tibiatools@{CALCULATOR_COMMIT}, src/data/spells.json '
        'and retained parseBuildRequest/computeDamage outputs. Formula facts faithfully reimplemented; '
        'EUPL implementation is not copied into Oteryn.',
        f'Power evidence: wiki-spell-facts-fandom-2026-09-27.json, current item revision {revision}; '
        f'{name}, basepower={expected_power}. Base power is preserved, not fitted to outputs.',
        'S5 world curve remains level_base_damage_healing (Crystal calculateBaseDamageHealing; '
        'Fandom Formulae). It is not replaced by Canary level/5 or a calculator-specific curve.',
        'Source conflict retained: Crystal calculateMagicSpellDamage uses sqrt(0.4*P), P/6 and '
        'floor(0.88/1.12 * (F + magic term)); Canary rune callbacks have separate fixed coefficients. '
        'Those models differ from this neutral calculator model and are retained in the repair evidence.',
        'Qualification is neutral min/max and a separate reference average only. Min/max agreement does '
        'not qualify normal_random versus uniform RNG, runic mastery, mitigated damage, cast legality or activation.',
    ]
    if name in ('stone shower rune', 'thunderstorm rune'):
        evidence.append('Power conflict retained: Fandom and TibiaWiki BR state 50, Tibiopedia states 45. '
                        'The existing primary-source power 50 is preserved; no silent power-policy override.')
    return replacement, evidence
