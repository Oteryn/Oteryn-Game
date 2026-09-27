"""Build the player Spell authoring schema candidate v1. This is not WorldProject/v2 serialization.

Ability, Effect and the import disposition ledger are the monster authoring definitions (monster D11:
a player spell or rune used by a monster is the one shared Ability), referenced by URN. Only the
player-casting layer and the player Formula kind are defined here.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIALECT = 'https://json-schema.org/draft/2020-12/schema'
ID = 'urn:oteryn:spell-authoring:candidate:1'
DEPS_ID = 'urn:oteryn:spell-dependencies:candidate:1'
MONSTER_ID = 'urn:oteryn:monster-authoring:candidate:1'


def obj(props, required=(), **extra):
    return {'type': 'object', 'additionalProperties': False, 'properties': props, 'required': list(required), **extra}


def array(item, minimum=0, unique=False, **extra):
    return {'type': 'array', 'items': item, 'minItems': minimum, **({'uniqueItems': True} if unique else {}), **extra}


def integer(minimum=0, maximum=None, **extra):
    return {'type': 'integer', 'minimum': minimum, **({'maximum': maximum} if maximum is not None else {}), **extra}


def text(minimum=1, **extra): return {'type': 'string', 'minLength': minimum, **extra}
def enum(*values): return {'enum': list(values)}
def use(name): return {'$ref': '#/$defs/' + name}
def monster(name): return {'$ref': MONSTER_ID + '#/$defs/' + name}
def forbid(*names): return {'not': {'anyOf': [{'required': [n]} for n in names]}}
def when(field, value): return {'properties': {field: {'const': value}}, 'required': [field]}


VOCATIONS = ('druid', 'elder_druid', 'sorcerer', 'master_sorcerer', 'knight', 'elite_knight',
             'paladin', 'royal_paladin', 'monk', 'exalted_monk')
FORMULA_INPUTS = ('level', 'magic_level', 'base_power', 'attack_skill', 'attack_value', 'attack_factor',
                  'shielding_skill')

d = {}
for name in ('key', 'revision', 'identity', 'bool', 'ms', 'AbilityRef', 'EffectRef', 'FormulaRef', 'ItemRef',
             'InteractionRef'):
    d[name] = monster(name)
d['SpellRef'] = obj({'family': {'const': 'Spell'}, 'key': use('key'), 'revision': use('revision')},
                    ('family', 'key', 'revision'))
d['vocation'] = {**enum(*VOCATIONS), 'description': 'S8: explicit base or promoted vocation key; a promoted '
                 'vocation is never implied by its base vocation.'}
d['groupKey'] = text(pattern=r'^[a-z][a-z0-9_]*$', description='Cooldown group key, lowercase without spaces '
                     '(attack, healing, support, focus, greatbeams, ...).')
d['cooldownGroup'] = obj({'group': use('groupKey'), 'cooldown_ms': use('ms')}, ('group', 'cooldown_ms'))
d['decimal'] = text(pattern=r'^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE]-?[0-9]+)?$', maxLength=64,
                    description='S5: decimal literal, read as the nearest IEEE-754 double (the source literal).')
d['expression'] = {
    'description': 'S5: formula expression over declared inputs, evaluated in IEEE-754 double in authored '
                   'operation order.',
    'oneOf': [
        obj({'const': use('decimal')}, ('const',)),
        obj({'var': enum(*FORMULA_INPUTS)}, ('var',)),
        obj({'op': enum('add', 'sub', 'mul', 'div'), 'args': array(use('expression'), 2, maxItems=2)}, ('op', 'args')),
        obj({'op': enum('neg', 'floor', 'ceil', 'sqrt', 'abs'), 'args': array(use('expression'), 1, maxItems=1)},
            ('op', 'args')),
        obj({'op': enum('min', 'max'), 'args': array(use('expression'), 2)}, ('op', 'args')),
        obj({'fn': enum('level_base_damage_healing'), 'args': array(use('expression'), 1, maxItems=1)},
            ('fn', 'args'),
            description='S5: the one world level curve (Canary Player::calculateFlatDamageHealing, Crystal '
                        'calculateBaseDamageHealing); its exact rounding is an owner decision (S5).'),
    ]}
d['formula'] = obj({
    'identity': use('identity'), 'kind': {'const': 'player_expression'},
    'inputs': {**enum('level_magic', 'skill'), 'description': 'level_magic: level, magic_level, base_power; '
               'skill: level, attack_skill, attack_value, attack_factor, base_power, shielding_skill. The engine '
               'fills attack_* from the wielded weapon (a world combat rule).'},
    'minimum': use('expression'), 'maximum': use('expression')},
    ('identity', 'kind', 'inputs', 'minimum', 'maximum'),
    description='S5: magnitude range of a damage or heal Effect cast by a player. Each bound is evaluated, '
                'truncated toward zero and then drawn by the world damage distribution.')
d['rune'] = obj({
    'item': {**use('ItemRef'), 'description': 'S2: the rune Item (Item authority stays with the Item catalogue).'},
    'charges': integer(1), 'magic_level': integer(), 'allow_far_use': use('bool'),
    'blocking': {**enum('solid', 'creature', 'none'), 'description': 'Canary/Crystal isBlocking(true, false) '
                 'as solid/creature; absent call = none.'}},
    ('item', 'charges', 'magic_level', 'allow_far_use', 'blocking'))
d['conjure'] = obj({'reagent': use('ItemRef'), 'result': use('ItemRef'), 'count': integer(1)}, ('result', 'count'),
                   description='S2: removes one reagent (when present) and creates count result items.')
d['nativeBehavior'] = obj({
    'key': text(pattern=r'^[a-z][a-z0-9_]*$'),
    'parameters': {'type': 'object', 'description': 'Data parameters of the shared behaviour (S7).'}},
    ('key', 'parameters'))
d['execution'] = {
    'description': 'Exactly one execution: an Ability (plain or random combat), a conjure, or a native behaviour.',
    'oneOf': [obj({'ability': use('AbilityRef')}, ('ability',)), obj({'conjure': use('conjure')}, ('conjure',)),
              obj({'native_behavior': use('nativeBehavior')}, ('native_behavior',))]}
d['spell'] = obj({
    'identity': use('identity'),
    'name': text(), 'carrier': enum('instant', 'rune'),
    'words': text(maxLength=64, description='Spoken words, lowercase; required for instant spells.'),
    'reference_spell_id': integer(1, description='Client spell id (Canary/Crystal spell:id, wiki spellid).'),
    'requirements': obj({
        'vocations': array(use('vocation'), 1, True), 'level': integer(1), 'premium': use('bool'),
        'learning_required': use('bool'),
        'acquisition_interactions': array(use('InteractionRef'), 1, True, description='S10: trainer NPC services.')},
        ('vocations', 'level', 'premium', 'learning_required')),
    'costs': obj({'mana': integer(), 'mana_percent': integer(0, 100), 'soul': integer()}, ('soul',),
                 oneOf=[{'required': ['mana'], **forbid('mana_percent')},
                        {'required': ['mana_percent'], **forbid('mana')}]),
    'cooldown_ms': use('ms'),
    'groups': array(use('cooldownGroup'), 1, maxItems=2,
                    description='S9: primary group first, then an optional secondary group; group keys differ.'),
    'targeting': obj({
        'aggressive': use('bool'), 'self_target': use('bool'), 'needs_target': use('bool'),
        'needs_direction': use('bool'), 'target_or_direction': use('bool'), 'range_tiles': integer(),
        'block_walls': use('bool'), 'allow_on_self': use('bool'), 'check_floor': use('bool'),
        'parameter': enum('none', 'player_name', 'text')},
        ('aggressive', 'self_target', 'needs_target', 'needs_direction', 'target_or_direction', 'block_walls',
         'parameter')),
    'pz_locks_caster': use('bool'), 'needs_weapon': use('bool'),
    'base_power': integer(1, description='Reference base power (wiki basepower, Crystal basePower); a formula '
                          'input, not a damage value.'),
    'rune': use('rune'), 'execution': use('execution'),
    'presentation': obj({'cast_cue': text(), 'impact_cue': text()})},
    ('identity', 'name', 'carrier', 'requirements', 'costs', 'cooldown_ms', 'groups', 'targeting',
     'pz_locks_caster', 'needs_weapon', 'execution'),
    allOf=[{'if': when('carrier', 'instant'), 'then': {'required': ['words'], **forbid('rune')},
            'else': {'required': ['rune']}}],
    description='CANDIDATE player Spell (docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md).')

main = {'$schema': DIALECT, '$id': ID, 'title': 'Player Spell authoring schema candidate v1 - structural validation',
        'description': 'CANDIDATE authoring schema (docs/architecture/OTERYN_SPELL_AUTHORING_SCHEMA_V1.md). Not a '
                       'WorldProject/v2 contract or runtime activation; semantic rules live in validate_spell.py.',
        **obj({'spell': use('spell')}, ('spell',)), '$defs': d}
deps = {'$schema': DIALECT, '$id': DEPS_ID, 'title': 'Direct Spell dependencies authoring schema candidate v1',
        **obj({'abilities': array(monster('ability')), 'effects': array(monster('effect')),
               'formulas': array({'oneOf': [monster('formula'), {'$ref': ID + '#/$defs/formula'}]})},
              ('abilities', 'effects', 'formulas'))}


def template(schema):
    """Empty field template: every property with a placeholder that is deliberately invalid data."""
    def fill(node, defs, depth=0):
        if depth > 20:
            raise ValueError('template recursion')
        if '$ref' in node:
            ref = node['$ref']
            if ref.startswith('#/$defs/'):
                return fill(defs[ref[8:]], defs, depth + 1)
            return '<' + ref.rsplit('/', 1)[-1] + '>'
        kind = node.get('type')
        if kind is None and 'oneOf' in node:
            return fill(node['oneOf'][0], defs, depth + 1)
        if kind == 'object':
            return {k: fill(v, defs, depth + 1) for k, v in node.get('properties', {}).items()}
        if kind == 'array':
            return [fill(node['items'], defs, depth + 1)]
        if 'enum' in node:
            return '<' + '|'.join(map(str, node['enum'])) + '>'
        if 'const' in node:
            return node['const']
        return '<' + str(kind) + '>'
    return fill(schema, schema['$defs'])


def dump(name, value):
    (ROOT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')


def build():
    dump('spell.schema.json', main)
    dump('spell-dependencies.schema.json', deps)
    dump('spell-template.json', template(main))


if __name__ == '__main__':
    build()
