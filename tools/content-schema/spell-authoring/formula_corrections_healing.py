"""Narrow source-fidelity fixes for authored healing formulas (S5 world level curve).

Raw OTS parity is not the world contract: level/5 and Canary's defective progressive
helper are deliberately replaced by level_base_damage_healing. Keep that decision.
"""
import copy
import re

CRYSTAL_PIN = 'ff7ede593c69d4c658b382c97443e8155926924a'
UH_SOURCE = ('https://github.com/zimbadev/crystalserver/blob/' + CRYSTAL_PIN
             + '/data/scripts/lib/register_spells.lua#L646-L651')


def _restoration_level_term(expression):
    legacy = {'op': 'div', 'args': [
        {'op': 'mul', 'args': [{'var': 'level'}, {'const': '1.4'}]}, {'const': '5'}]}
    if expression == legacy:
        return {'op': 'mul', 'args': [
            {'fn': 'level_base_damage_healing', 'args': [{'var': 'level'}]}, {'const': '1.4'}]}
    result = copy.deepcopy(expression)
    if 'args' in result:
        result['args'] = [_restoration_level_term(arg) for arg in result['args']]
    return result


def correct(name, spell_type, formula, base_power):
    """Return an immutable replacement and evidence notes, or None when unchanged.

    The Crystal UH helper evaluates floor(min) and ceil(max) before returning its
    bounds. S5 truncation is a later operation; it cannot replace the source ceil.
    Conjuring an UH rune has no healing callback and must never enter this fix.
    """
    normalized = re.sub(r'\s+', ' ', name.strip().lower())
    if formula.get('kind') != 'player_expression':
        return None
    if normalized == 'restoration' and spell_type == 'instant':
        replacement = copy.deepcopy(formula)
        for bound in ('minimum', 'maximum'):
            replacement[bound] = _restoration_level_term(replacement[bound])
        if replacement == formula:
            return None
        return replacement, [
            'S5 healing correction: Restoration retains its source 1.4 healing multiplier '
            'but replaces the nested (level * 1.4) / 5 with '
            'level_base_damage_healing(level) * 1.4. The basic level/5 matcher '
            'previously missed this scaled legacy level contribution.',
            'Source shape: Crystal ' + CRYSTAL_PIN +
            ' data/scripts/spells/healing/restoration.lua:onGetFormulaValues; '
            'world normalization: OTERYN_SPELL_AUTHORING_SCHEMA_V1 section5 S5.',
            'Magic-level coefficients, offsets, identity and authored order of the '
            'other terms are retained; this is an S5 world-curve correction, '
            'not a change made to match unnormalized OTS output.',
        ]
    if spell_type != 'rune' or normalized != 'ultimate healing rune':
        return None
    if formula.get('minimum', {}).get('op') == 'floor' and formula.get('maximum', {}).get('op') == 'ceil':
        return None
    replacement = copy.deepcopy(formula)
    for bound, operation in [('minimum', 'floor'), ('maximum', 'ceil')]:
        if replacement[bound].get('op') != operation:
            replacement[bound] = {'op': operation, 'args': [replacement[bound]]}
    return replacement, [
        'Healing source correction: Ultimate Healing Rune preserves Crystal '
        'calculateHealingSpellDamage floor(minimum) and ceil(maximum) before S5 '
        'bound truncation; fractional maximum otherwise loses one possible HP.',
        'Source: ' + UH_SOURCE + '; rune callback '
        'data/scripts/runes/ultimate_healing_rune.lua:onGetHealingValues.',
        'The existing coefficients, wiki base_power, identity, inputs and S5 '
        'level_base_damage_healing curve are retained; target guards and final '
        'healing buffs require their separate runtime contracts.',
    ]
