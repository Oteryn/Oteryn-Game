"""Generate the Encounter authoring JSON Schema (docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md).

The vocabulary is closed (D28): every trigger, condition and action kind is listed here with its own
parameters, so an unknown kind or a missing parameter fails schema validation.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def obj(properties, required=(), **extra):
    return {'type': 'object', 'additionalProperties': False, 'properties': properties, 'required': list(required), **extra}


def array(item, minimum=0, unique=False):
    return {'type': 'array', 'items': item, 'minItems': minimum, **({'uniqueItems': True} if unique else {})}


def integer(minimum=0, maximum=None):
    return {'type': 'integer', 'minimum': minimum, **({'maximum': maximum} if maximum is not None else {})}


def const(value):
    return {'const': value}


def enum(*values):
    return {'enum': list(values)}


def use(name):
    return {'$ref': '#/$defs/' + name}


KEY = {'type': 'string', 'pattern': r'^[a-z][a-z0-9_.-]*:[a-z0-9_./-]+$'}
NAME = {'type': 'string', 'pattern': r'^[a-z][a-z0-9_]*$'}
TEXT = {'type': 'string', 'minLength': 1}
BOOL = {'type': 'boolean'}
OP = enum('==', '!=', '<', '<=', '>', '>=')
PERCENT = {'type': 'number', 'exclusiveMinimum': 0, 'exclusiveMaximum': 100}
COMPONENT = enum('all', 'primary')  # D29: both parts of a hit (default) or only its primary damage


def amount(minimum, range_minimum=None):
    """A fixed integer or a range {min, max} drawn uniformly by the encounter instance (D29); a heal range may start at 0 (D31)."""
    low = minimum if range_minimum is None else range_minimum
    return {'oneOf': [integer(minimum), obj({'min': integer(low), 'max': integer(low)}, ('min', 'max'))]}


def kinded(kind, properties=None, required=()):
    return obj({'kind': const(kind), **(properties or {})}, ('kind', *required))


d = {
    'identity': obj({'key': KEY, 'revision': TEXT}, ('key', 'revision')),
    'CreatureRef': obj({'family': const('Creature'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'ItemRef': obj({'family': const('Item'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'AbilityRef': obj({'family': const('Ability'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'subject': {'oneOf': [obj({'role': NAME}, ('role',)), obj({'killer': const(True)}, ('killer',)),
                          obj({'spawned': const(True)}, ('spawned',))]},
    'health': {'oneOf': [enum('full', 'keep_percent', 'keep_absolute'), obj({'percent': integer(1, 100)}, ('percent',))]},
    'position': {'oneOf': [enum('death_position', 'subject_position'), obj({'anchor': NAME}, ('anchor',)),
                           obj({'random_in': NAME}, ('random_in',)),
                           obj({'role_position': NAME, 'otherwise': enum('death_position')}, ('role_position',)),
                           obj({'offset_tiles': integer(0)}, ('offset_tiles',))]},
}

d['trigger'] = {'oneOf': [
    kinded('creature_died', {'role': NAME}, ('role',)),
    kinded('lethal_damage', {'role': NAME}, ('role',)),
    kinded('health_crossed', {'role': NAME, 'percent': PERCENT, 'health': integer(1)}, ('role',)),
    kinded('creature_spawned', {'role': NAME}, ('role',)),
    kinded('ability_cast', {'role': NAME, 'ability': use('AbilityRef')}, ('role', 'ability')),
    kinded('damage_taken', {'role': NAME, 'source': enum('player', 'any')}, ('role', 'source')),
    kinded('heal_received', {'role': NAME, 'source': enum('player', 'any')}, ('role', 'source')),
    kinded('damage_accumulated', {'role': NAME, 'amount': integer(1)}, ('role', 'amount')),
    kinded('timer_elapsed', {'timer': NAME}, ('timer',)),
    kinded('counter_reached', {'counter': NAME, 'value': {'type': 'integer'}}, ('counter', 'value')),
    kinded('area_entered', {'anchor': NAME, 'who': enum('player', 'role'), 'role': NAME}, ('anchor', 'who')),
    kinded('area_left', {'anchor': NAME, 'who': enum('player', 'role'), 'role': NAME}, ('anchor', 'who')),
    kinded('phase_entered', {'phase': NAME}, ('phase',)),
    kinded('encounter_started'), kinded('encounter_reset')]}

d['condition'] = {'oneOf': [
    kinded('chance_percent', {'value': {'type': 'number', 'exclusiveMinimum': 0, 'maximum': 100}}, ('value',)),
    kinded('counter_compare', {'counter': NAME, 'op': OP, 'value': {'type': 'integer'}}, ('counter', 'op', 'value')),
    kinded('flag', {'flag': NAME, 'value': BOOL}, ('flag', 'value')),
    kinded('creature_present', {'role': NAME, 'anchor': NAME, 'near': obj({'role': NAME, 'radius': integer(0), 'shape': enum('square', 'circle')}, ('role', 'radius')),
                                 'present': BOOL}, ('role', 'present')),
    kinded('world_state', {'state': KEY, 'op': OP, 'value': {'type': ['integer', 'boolean']}}, ('state', 'op', 'value')),
    kinded('in_anchor', {'subject': use('subject'), 'anchor': NAME}, ('subject', 'anchor')),
    kinded('killer_is_player'),
    kinded('has_master', {'role': NAME, 'value': BOOL}, ('role', 'value')),
    kinded('health_percent', {'role': NAME, 'op': OP, 'value': {'type': 'number', 'minimum': 0, 'maximum': 100}},
           ('role', 'op', 'value')),
    kinded('attacker_wears', {'item': use('ItemRef'), 'wears': BOOL}, ('item', 'wears')),
    kinded('killer_progress', {'progress': KEY, 'op': OP, 'value': {'type': ['integer', 'boolean']}},
           ('progress', 'op', 'value'))]}

d['action'] = {'oneOf': [
    kinded('spawn', {'creature': use('CreatureRef'), 'role': NAME, 'count': amount(1), 'at': use('position'),
                     'owner': enum('none', 'subject', 'death_master'), 'health': use('health')},
           ('creature', 'count', 'at', 'owner', 'health')),
    kinded('remove', {'role': NAME, 'all_in': NAME, 'triggering': const(True), 'keep_summons': BOOL}, ()),
    kinded('transform', {'role': NAME, 'into': {'oneOf': [use('CreatureRef'), obj({'next_stage': const(True)}, ('next_stage',)),
                                                           obj({'random_of': array(use('CreatureRef'), 2, True)}, ('random_of',))]},
                         'health': use('health')}, ('role', 'into', 'health')),
    kinded('heal', {'subject': use('subject'), 'amount': {'oneOf': [const('full'), amount(1, 0)]}}, ('subject', 'amount')),
    kinded('damage', {'subject': use('subject'), 'amount': amount(1), 'damage_type': NAME}, ('subject', 'amount', 'damage_type')),
    kinded('prevent_death', {'role': NAME}, ('role',)),
    kinded('damage_modifier', {'role': NAME, 'multiplier_percent': integer(0), 'damage_types': array(NAME, 0, True), 'component': COMPONENT,
                               'sources': enum('player', 'any'), 'until': enum('this_hit', 'reset', 'timer'), 'timer': NAME},
           ('role', 'multiplier_percent', 'sources', 'until')),
    kinded('reflect_damage', {'role': NAME, 'percent': integer(1, 100), 'damage_types': array(NAME, 0, True)}, ('role', 'percent')),
    kinded('convert_damage_to_heal', {'role': NAME, 'damage_types': array(NAME, 0, True), 'component': COMPONENT}, ('role',)),
    kinded('teleport', {'who': {'oneOf': [obj({'role': NAME}, ('role',)), obj({'players_in': NAME}, ('players_in',))]},
                        'to': NAME}, ('who', 'to')),
    kinded('map_item', {'operation': enum('create', 'transform', 'remove'), 'item': use('ItemRef'), 'into': use('ItemRef'),
                        'anchor': NAME, 'at': const('death_position'), 'revert_after_ms': integer(1), 'destination': NAME,
                        'revert_destination': NAME, 'effect': TEXT, 'interaction': KEY}, ('operation', 'item')),
    kinded('counter', {'counter': NAME, 'operation': enum('set', 'add'), 'value': {'type': 'integer'}}, ('counter', 'operation', 'value')),
    kinded('flag', {'flag': NAME, 'value': BOOL}, ('flag', 'value')),
    kinded('timer', {'timer': NAME, 'operation': enum('start', 'stop')}, ('timer', 'operation')),
    kinded('set_phase', {'phase': NAME}, ('phase',)),
    kinded('cast', {'ability': use('AbilityRef'), 'at': use('position')}, ('ability', 'at')),
    kinded('say', {'subject': use('subject'), 'text': TEXT, 'mode': enum('say', 'yell')}, ('subject', 'text', 'mode')),
    kinded('drop_item', {'item': use('ItemRef'), 'at': use('position')}, ('item', 'at')),
    kinded('message', {'to': obj({'players_in': NAME}, ('players_in',)), 'text': TEXT}, ('to', 'text')),
    kinded('one_of', {'branches': array(obj({'weight': integer(1), 'actions': array(use('action'), 1)}, ('weight', 'actions')), 2)},
           ('branches',)),
    kinded('emit_outcome', {'outcome': NAME, 'credited': enum('damage_contributors', 'killer', 'players_in_anchor', 'party'),
                            'anchor': NAME}, ('outcome', 'credited'))]}

d['rule'] = obj({'key': NAME, 'trigger': use('trigger'), 'delay_ms': amount(1), 'conditions': array(use('condition')),
                 'actions': array(use('action'), 1)}, ('key', 'trigger', 'conditions', 'actions'))

schema = obj({
    'identity': use('identity'),
    'display_name': TEXT,
    'scope': enum('instance_per_party', 'channel_shared'),
    'participants': array(obj({'role': NAME, 'creatures': array(use('CreatureRef'), 1, True)}, ('role', 'creatures')), 1),
    'anchors': array(obj({'key': NAME, 'kind': enum('point', 'area'), 'description': TEXT}, ('key', 'kind', 'description'))),
    'phases': array(NAME, 0, True),
    'state': obj({'counters': array(obj({'name': NAME, 'initial': {'type': 'integer'}}, ('name', 'initial'))),
                  'flags': array(obj({'name': NAME, 'initial': BOOL}, ('name', 'initial'))),
                  'timers': array(obj({'name': NAME, 'duration_ms': amount(1), 'repeat': BOOL}, ('name', 'duration_ms', 'repeat')))},
                 ('counters', 'flags', 'timers')),
    'rules': array(use('rule'), 1),
    'outcomes': array(NAME, 0, True),
    'reset_after_ms': integer(1)},
    ('identity', 'display_name', 'scope', 'participants', 'anchors', 'phases', 'state', 'rules', 'outcomes'),
    **{'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'oteryn:encounter-authoring/v1', '$defs': d,
       'description': 'Oteryn Encounter authoring format v1 (CANDIDATE, D20/D26-D31).'})


if __name__ == '__main__':
    (ROOT / 'encounter.schema.json').write_text(json.dumps(schema, indent=2) + '\n', encoding='utf-8', newline='\n')
    print('Generated encounter.schema.json')
