"""Focused positive/negative cases for the Spell authoring schema candidate v1.

Regenerates the synthetic fixtures (Light Healing, Sudden Death rune and its conjuring spell, transcribed
from Crystal Server ff7ede5) and checks that each mutation is rejected for the intended reason.
"""
import copy
import json
import sys
from pathlib import Path

from validate_spell import level_base_damage_healing, validate

ROOT = Path(__file__).resolve().parent


def ref(family, key):
    return {'family': family, 'key': key, 'revision': 'definition-r1'}


def identity(key):
    return {'key': key, 'revision': 'definition-r1'}


def var(name):
    return {'var': name}


def const(value):
    return {'const': value}


def op(name, *args):
    return {'op': name, 'args': list(args)}


def base(level):
    return {'fn': 'level_base_damage_healing', 'args': [level]}


def targeting(**overrides):
    value = {'aggressive': False, 'self_target': False, 'needs_target': False, 'needs_direction': False,
             'target_or_direction': False, 'block_walls': False, 'parameter': 'none'}
    value.update(overrides)
    return value


LIGHT_HEALING = {'spell': {
    'identity': identity('oteryn:spell.light_healing'), 'name': 'Light Healing', 'carrier': 'instant',
    'words': 'exura', 'reference_spell_id': 1,
    'requirements': {'vocations': ['druid', 'elder_druid', 'exalted_monk', 'master_sorcerer', 'monk', 'paladin',
                                   'royal_paladin', 'sorcerer'], 'level': 8, 'premium': False,
                     'learning_required': False},
    'costs': {'mana': 20, 'soul': 0}, 'cooldown_ms': 1000, 'groups': [{'group': 'healing', 'cooldown_ms': 1000}],
    'targeting': targeting(self_target=True), 'pz_locks_caster': False, 'needs_weapon': False, 'base_power': 40,
    'execution': {'ability': ref('Ability', 'oteryn:ability.spell.light_healing')},
    'presentation': {'cast_cue': 'oteryn:audio.spell.light_healing'}}}
LIGHT_HEALING_DEPS = {
    'abilities': [{'identity': identity('oteryn:ability.spell.light_healing'), 'kind': 'spell', 'range_tiles': 0,
                   'needs_target': False, 'needs_direction': False,
                   'effects': [ref('Effect', 'oteryn:effect.spell.light_healing.heal'),
                               ref('Effect', 'oteryn:effect.spell.light_healing.cure_paralysis')]}],
    'effects': [{'identity': identity('oteryn:effect.spell.light_healing.heal'), 'operation': 'heal',
                 'damage_type': 'healing', 'formula': ref('Formula', 'oteryn:formula.spell.light_healing')},
                {'identity': identity('oteryn:effect.spell.light_healing.cure_paralysis'),
                 'operation': 'remove_condition', 'removed_condition': 'paralyze'}],
    'formulas': [{'identity': identity('oteryn:formula.spell.light_healing'), 'kind': 'player_expression',
                  'inputs': 'level_magic',
                  'minimum': op('add', op('add', base(var('level')), op('mul', var('magic_level'), const('1.4'))),
                                const('8')),
                  'maximum': op('add', op('add', base(var('level')), op('mul', var('magic_level'), const('1.795'))),
                                const('11'))}]}


def rune_average(factor):
    total = op('add', op('add', base(var('level')),
                         op('mul', var('magic_level'), op('sqrt', op('mul', var('base_power'), const('0.4'))))),
               op('div', var('base_power'), const('6')))
    return op('floor', op('mul', total, const(factor)))


SUDDEN_DEATH_RUNE = {'spell': {
    'identity': identity('oteryn:spell.sudden_death_rune'), 'name': 'sudden death rune', 'carrier': 'rune',
    'reference_spell_id': 21,
    'requirements': {'vocations': ['druid', 'elder_druid', 'elite_knight', 'exalted_monk', 'knight',
                                   'master_sorcerer', 'monk', 'paladin', 'royal_paladin', 'sorcerer'],
                     'level': 45, 'premium': False, 'learning_required': False},
    'costs': {'mana': 0, 'soul': 0}, 'cooldown_ms': 2000, 'groups': [{'group': 'attack', 'cooldown_ms': 2000}],
    'targeting': targeting(aggressive=True, needs_target=True), 'pz_locks_caster': False, 'needs_weapon': False,
    'base_power': 150,
    'rune': {'item': ref('Item', 'oteryn:item.sudden_death_rune'), 'charges': 3, 'magic_level': 15,
             'allow_far_use': True, 'blocking': 'solid'},
    'execution': {'ability': ref('Ability', 'oteryn:ability.spell.sudden_death_rune')}}}
SUDDEN_DEATH_RUNE_DEPS = {
    'abilities': [{'identity': identity('oteryn:ability.spell.sudden_death_rune'), 'kind': 'spell',
                   'range_tiles': 7, 'needs_target': True, 'needs_direction': False,
                   'effects': [ref('Effect', 'oteryn:effect.spell.sudden_death_rune.damage')]}],
    'effects': [{'identity': identity('oteryn:effect.spell.sudden_death_rune.damage'), 'operation': 'damage',
                 'damage_type': 'death', 'formula': ref('Formula', 'oteryn:formula.spell.sudden_death_rune')}],
    'formulas': [{'identity': identity('oteryn:formula.spell.sudden_death_rune'), 'kind': 'player_expression',
                  'inputs': 'level_magic', 'minimum': rune_average('0.88'), 'maximum': rune_average('1.12')}]}
CONJURE = {'spell': {
    'identity': identity('oteryn:spell.sudden_death'), 'name': 'Sudden Death Rune', 'carrier': 'instant',
    'words': 'adori gran mort', 'reference_spell_id': 21,
    'requirements': {'vocations': ['master_sorcerer', 'sorcerer'], 'level': 45, 'premium': False,
                     'learning_required': False},
    'costs': {'mana': 985, 'soul': 5}, 'cooldown_ms': 2000, 'groups': [{'group': 'support', 'cooldown_ms': 2000}],
    'targeting': targeting(), 'pz_locks_caster': False, 'needs_weapon': False,
    'execution': {'conjure': {'reagent': ref('Item', 'oteryn:item.blank_rune'),
                              'result': ref('Item', 'oteryn:item.sudden_death_rune'), 'count': 3}}}}
EMPTY_DEPS = {'abilities': [], 'effects': [], 'formulas': []}
CATALOG = {'definitions': [ref('Item', 'oteryn:item.sudden_death_rune'), ref('Item', 'oteryn:item.blank_rune')]}
POSITIVE = {'light_healing': (LIGHT_HEALING, LIGHT_HEALING_DEPS), 'sudden_death_rune': (SUDDEN_DEATH_RUNE, SUDDEN_DEATH_RUNE_DEPS),
            'sudden_death_conjure': (CONJURE, EMPTY_DEPS)}


def mutate(base_name, change, expect):
    spell, deps = copy.deepcopy(POSITIVE[base_name][0]), copy.deepcopy(POSITIVE[base_name][1])
    change(spell['spell'], deps)
    return base_name, spell, deps, expect


def set_path(target, path, value):
    for key in path[:-1]:
        target = target[key]
    if value is DELETE:
        del target[path[-1]]
    elif isinstance(target, list) and path[-1] == len(target):
        target.append(value)
    else:
        target[path[-1]] = value


DELETE = object()


def case(base_name, where, path, value, expect):
    def change(spell, deps):
        set_path(spell if where == 'spell' else deps, path, value)
    return mutate(base_name, change, expect)


NEGATIVE = {
    'instant without words': case('light_healing', 'spell', ('words',), DELETE, "'words' is a required property"),
    'instant with rune': case('light_healing', 'spell', ('rune',), SUDDEN_DEATH_RUNE['spell']['rune'], 'should not be valid'),
    'rune without rune block': case('sudden_death_rune', 'spell', ('rune',), DELETE, "'rune' is a required property"),
    'uppercase words': case('light_healing', 'spell', ('words',), 'Exura', 'lowercase'),
    'unsorted vocations': case('light_healing', 'spell', ('requirements', 'vocations'), ['sorcerer', 'druid'], 'sorted'),
    'unknown vocation': case('light_healing', 'spell', ('requirements', 'vocations'), ['necromancer'], 'is not one of'),
    'duplicate vocation': case('light_healing', 'spell', ('requirements', 'vocations'), ['druid', 'druid'], 'non-unique'),
    'mana and mana_percent': case('light_healing', 'spell', ('costs',), {'mana': 20, 'mana_percent': 10, 'soul': 0}, 'is not valid'),
    'no mana cost': case('light_healing', 'spell', ('costs',), {'soul': 0}, 'is not valid'),
    'three groups': case('light_healing', 'spell', ('groups',), [{'group': g, 'cooldown_ms': 1000} for g in ('a', 'b', 'c')], 'is too long'),
    'duplicate groups': case('light_healing', 'spell', ('groups',), [{'group': 'healing', 'cooldown_ms': 1000}] * 2, 'group keys must differ'),
    'group key with space': case('light_healing', 'spell', ('groups', 0, 'group'), 'great beams', 'does not match'),
    'zero cooldown': case('light_healing', 'spell', ('cooldown_ms',), 0, 'less than the minimum'),
    'two executions': case('light_healing', 'spell', ('execution', 'conjure'), CONJURE['spell']['execution']['conjure'], 'is not valid'),
    'rune conjures': case('sudden_death_rune', 'spell', ('execution',), CONJURE['spell']['execution'], 'only an instant spell conjures'),
    'conjure count zero': case('sudden_death_conjure', 'spell', ('execution', 'conjure', 'count'), 0, 'is not valid under any'),
    'unresolved rune item': case('sudden_death_rune', 'spell', ('rune', 'item', 'key'), 'oteryn:item.missing', 'unresolved exact definition'),
    'missing ability payload': case('light_healing', 'deps', ('abilities',), [], 'unresolved exact definition'),
    'melee ability': case('light_healing', 'deps', ('abilities', 0, 'kind'), 'melee', 'Ability of kind spell'),
    'unknown formula variable': case('light_healing', 'deps', ('formulas', 0, 'minimum'), var('mana'), 'is not valid'),
    'skill input in level formula': case('light_healing', 'deps', ('formulas', 0, 'minimum'), var('attack_skill'), 'do not provide attack_skill'),
    'base_power without spell value': case('sudden_death_rune', 'spell', ('base_power',), DELETE, 'has no base_power'),
    'minimum above maximum': case('light_healing', 'deps', ('formulas', 0, 'minimum'), op('add', LIGHT_HEALING_DEPS['formulas'][0]['maximum'], const('1')), 'exceeds maximum'),
    'negative magnitude': case('light_healing', 'deps', ('formulas', 0, 'minimum'), op('neg', var('level')), 'negative magnitude'),
    'division by zero': case('light_healing', 'deps', ('formulas', 0, 'maximum'), op('div', var('level'), const('0')), 'division by zero'),
    'binary op with one arg': case('light_healing', 'deps', ('formulas', 0, 'maximum'), {'op': 'add', 'args': [var('level')]}, 'is not valid'),
    'decimal with comma': case('light_healing', 'deps', ('formulas', 0, 'maximum'), const('1,5'), 'is not valid'),
    'unknown function': case('light_healing', 'deps', ('formulas', 0, 'maximum'), {'fn': 'calculateFlatDamageHealing', 'args': [var('level')]}, 'is not valid'),
    'unreached player formula': case('light_healing', 'deps', ('effects', 0, 'formula', 'key'), 'oteryn:formula.other', 'unresolved exact definition'),
    'caster_magnitude on player cast': case('light_healing', 'deps', ('formulas', 0), {'identity': identity('oteryn:formula.spell.light_healing'), 'kind': 'caster_magnitude'}, 'needs a monster schedule'),
    'mana percent above 100': case('light_healing', 'spell', ('costs',), {'mana_percent': 101, 'soul': 0}, 'greater than the maximum'),
    'unreached extra formula': case('light_healing', 'deps', ('formulas', 1), {**LIGHT_HEALING_DEPS['formulas'][0], 'identity': identity('oteryn:formula.other')}, 'is not reached'),
    'unknown targeting parameter': case('light_healing', 'spell', ('targeting', 'parameter'), 'number', 'is not one of'),
    'extra spell field': case('light_healing', 'spell', ('price',), 170, 'Additional properties'),
}


def official_level_bonus(level):
    """Tibia news 2022-10-17 (patch 13.05.12657): +1 every 5 levels up to 500, then +1 every 6 levels for
    501-1100, 7 for 1101-1800, 8 for 1801-2600, 9 for 2601-3500, each span 100 levels longer."""
    if level <= 500:
        return level // 5
    total, threshold, step, span = 100, 500, 6, 600
    while level > threshold + span:
        total, threshold, step, span = total + span // step, threshold + span, step + 1, span + 100
    return total + (level - threshold) // step


def write_fixtures():
    for name, (spell, deps) in POSITIVE.items():
        (ROOT / f'synthetic-valid-{name.replace("_", "-")}.json').write_text(
            json.dumps(spell, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
        (ROOT / f'synthetic-valid-{name.replace("_", "-")}-dependencies.json').write_text(
            json.dumps(deps, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    (ROOT / 'synthetic-catalog.json').write_text(json.dumps(CATALOG, indent=2) + '\n', encoding='utf-8', newline='\n')


def main():
    write_fixtures()
    failures, report = [], []
    for name, (spell, deps) in POSITIVE.items():
        errors = validate(spell, deps, CATALOG)
        report.append({'case': 'valid ' + name, 'passed': not errors, 'errors': errors})
        if errors:
            failures.append(f'valid {name}: {errors}')
    for name, (base_name, spell, deps, expect) in NEGATIVE.items():
        errors = validate(spell, deps, CATALOG)
        passed = any(expect in e for e in errors)
        report.append({'case': name, 'passed': passed, 'expected': expect, 'errors': errors})
        if not passed:
            failures.append(f'{name}: expected "{expect}", got {errors}')
    curve = [level for level in range(20001) if level_base_damage_healing(level) != official_level_bonus(level)]
    report.append({'case': 'level curve equals the official table for levels 0-20000', 'passed': not curve,
                   'errors': curve[:10]})
    if curve or level_base_damage_healing(800) != 150:
        failures.append(f'level curve differs from the official table at {curve[:10]}')
    (ROOT / 'formal-schema-validation-report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(f'{len(report) - len(failures)} of {len(report)} cases passed')
    for failure in failures:
        print('FAIL', failure)
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main())
