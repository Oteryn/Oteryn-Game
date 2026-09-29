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


# D34: a fixed percent, or the percent of a timer's duration still remaining when the action runs (never below `floor`).
MULTIPLIER = {'oneOf': [integer(0), obj({'timer_remaining': NAME, 'floor': integer(0, 100)}, ('timer_remaining', 'floor'))]}


def amount(minimum, range_minimum=None):
    """A fixed integer or a range {min, max} drawn uniformly by the encounter instance (D29); a heal range may start at 0 (D31)."""
    low = minimum if range_minimum is None else range_minimum
    return {'oneOf': [integer(minimum), obj({'min': integer(low), 'max': integer(low)}, ('min', 'max'))]}


def span(maximum):
    return {'type': 'array', 'items': integer(0, maximum), 'minItems': 2, 'maxItems': 2}


# E2: an anchor's location in Canary map coordinates (the project frame on admission): a point, or boxes of whole
# tiles, each on one floor.
LOCATION = {'oneOf': [obj({'x': integer(0, 65535), 'y': integer(0, 65535), 'floor': integer(0, 15)}, ('x', 'y', 'floor')),
                      obj({'boxes': array(obj({'x': span(65535), 'y': span(65535), 'floor': integer(0, 15)}, ('x', 'y', 'floor')), 1)},
                          ('boxes',))]}


def kinded(kind, properties=None, required=()):
    return obj({'kind': const(kind), **(properties or {})}, ('kind', *required))


d = {
    'identity': obj({'key': KEY, 'revision': TEXT}, ('key', 'revision')),
    'CreatureRef': obj({'family': const('Creature'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'ItemRef': obj({'family': const('Item'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'AbilityRef': obj({'family': const('Ability'), 'key': KEY, 'revision': TEXT}, ('family', 'key', 'revision')),
    'subject': {'oneOf': [obj({'role': NAME}, ('role',)), obj({'killer': const(True)}, ('killer',)),
                          obj({'spawned': const(True)}, ('spawned',))]},
    'health': {'oneOf': [enum('full', 'keep_percent', 'keep_absolute', 'remembered'), obj({'percent': integer(1, 100)}, ('percent',))]},
    # D34: the base vocation of a player (a promoted vocation counts as its base).
    'baseVocation': enum('knight', 'paladin', 'sorcerer', 'druid', 'monk'),
    # D34: closest_free_tile is the free tile nearest the subject (Canary getClosestFreePosition), searched outward ring by ring.
    'position': {'oneOf': [enum('death_position', 'subject_position', 'closest_free_tile'), obj({'anchor': NAME}, ('anchor',)),
                           # CW2-4: `free` draws only among the area's tiles a creature can be placed on now; without it,
                           # or with false, `random_in` keeps its original semantics.
                           obj({'random_in': NAME, 'free': BOOL}, ('random_in',)),
                           obj({'role_position': NAME, 'otherwise': enum('death_position')}, ('role_position',)),
                           obj({'offset_tiles': integer(0)}, ('offset_tiles',)),
                           # D46: the tile at a fixed offset from the subject on its floor, used even when occupied.
                           obj({'relative': obj({'x': {'type': 'integer'}, 'y': {'type': 'integer'}}, ('x', 'y'))}, ('relative',))]},
    # CW2-1: the creature or player that fired the rule. Only `teleport.who` and the `in_anchor` subject take it; it is
    # not part of the shared `subject` of say, heal, damage and the other actions.
    'triggering': obj({'triggering': const(True)}, ('triggering',)),
}

d['trigger'] = {'oneOf': [
    kinded('creature_died', {'role': NAME}, ('role',)),
    kinded('lethal_damage', {'role': NAME}, ('role',)),
    kinded('health_crossed', {'role': NAME, 'percent': PERCENT, 'health': integer(1)}, ('role',)),
    kinded('creature_spawned', {'role': NAME}, ('role',)),
    kinded('ability_cast', {'role': NAME, 'ability': use('AbilityRef')}, ('role', 'ability')),
    kinded('damage_taken', {'role': NAME, 'source': enum('player', 'non_player', 'any')}, ('role', 'source')),
    kinded('heal_received', {'role': NAME, 'source': enum('player', 'non_player', 'any')}, ('role', 'source')),
    # D34: a fixed amount, or a percent of the creature's maximum health (the resolved definition's, after wiki adoption).
    {**kinded('damage_accumulated', {'role': NAME, 'amount': integer(1), 'percent': PERCENT}, ('role',)),
     'oneOf': [{'required': ['amount']}, {'required': ['percent']}]},
    kinded('timer_elapsed', {'timer': NAME}, ('timer',)),
    kinded('counter_reached', {'counter': NAME, 'value': {'type': 'integer'}}, ('counter', 'value')),
    kinded('area_entered', {'anchor': NAME, 'who': enum('player', 'role'), 'role': NAME}, ('anchor', 'who')),
    kinded('area_left', {'anchor': NAME, 'who': enum('player', 'role'), 'role': NAME}, ('anchor', 'who')),
    kinded('phase_entered', {'phase': NAME}, ('phase',)),
    kinded('item_used', {'role': NAME, 'item': use('ItemRef'), 'base_vocation': use('baseVocation')}, ('role', 'item')),
    # D46: a creature of the role steps onto a tile that holds the item (a MoveEvent stepin registered on the item id).
    # CW2-2: or onto a tile holding the corpse a creature of the `corpse_of` role left in this instance, at any decay stage.
    {**kinded('stepped_on', {'role': NAME, 'item': use('ItemRef'), 'corpse_of': NAME}, ('role',)),
     'oneOf': [{'required': ['item']}, {'required': ['corpse_of']}]},
    kinded('encounter_started'), kinded('encounter_reset')]}

d['condition'] = {'oneOf': [
    kinded('chance_percent', {'value': {'type': 'number', 'exclusiveMinimum': 0, 'maximum': 100}}, ('value',)),
    kinded('chance_from_amount', {'per': integer(1)}, ('per',)),
    kinded('counter_compare', {'counter': NAME, 'op': OP, 'value': {'type': 'integer'}}, ('counter', 'op', 'value')),
    # D46: whether the role's creature has any of the listed conditions on itself now.
    kinded('has_condition', {'role': NAME, 'conditions': array(enum('poison', 'fire', 'energy', 'bleeding', 'drown', 'freezing',
                                                                     'dazzled', 'cursed'), 1, True), 'present': BOOL},
           ('role', 'conditions', 'present')),
    kinded('flag', {'flag': NAME, 'value': BOOL}, ('flag', 'value')),
    kinded('creature_present', {'role': NAME, 'anchor': NAME, 'near': obj({'role': NAME, 'radius': integer(0), 'shape': enum('square', 'circle')}, ('role', 'radius')),
                                 'present': BOOL}, ('role', 'present')),
    kinded('world_state', {'state': KEY, 'op': OP, 'value': {'type': ['integer', 'boolean']}}, ('state', 'op', 'value')),
    kinded('in_anchor', {'subject': {'oneOf': [use('subject'), use('triggering')]}, 'anchor': NAME}, ('subject', 'anchor')),
    kinded('killer_is_player'),
    kinded('has_master', {'role': NAME, 'value': BOOL}, ('role', 'value')),
    kinded('summon_count', {'role': NAME, 'op': OP, 'value': integer(0)}, ('role', 'op', 'value')),
    kinded('health_percent', {'role': NAME, 'op': OP, 'value': {'type': 'number', 'minimum': 0, 'maximum': 100}},
           ('role', 'op', 'value')),
    kinded('attacker_wears', {'item': use('ItemRef'), 'wears': BOOL,
                               'slot': enum('head', 'necklace', 'armor', 'right_hand', 'left_hand', 'legs', 'feet', 'ring', 'ammo')},
           ('item', 'wears')),
    kinded('killer_progress', {'progress': KEY, 'op': OP, 'value': {'type': ['integer', 'boolean']}},
           ('progress', 'op', 'value'))]}

d['action'] = {'oneOf': [
    kinded('spawn', {'creature': use('CreatureRef'), 'role': NAME, 'count': amount(1), 'at': use('position'),
                     'owner': enum('none', 'subject', 'death_master'), 'health': use('health')},
           ('creature', 'count', 'at', 'owner', 'health')),
    # D34: one creature for each player in an area, chosen by the player's base vocation; players of a vocation without
    # an entry get none. `counter` adds the number spawned.
    kinded('spawn_per_player', {'players_in': NAME,
                                'by_base_vocation': obj({v: use('CreatureRef') for v in ('knight', 'paladin', 'sorcerer', 'druid', 'monk')},
                                                        minProperties=1),
                                'at': use('position'), 'owner': enum('none', 'subject'), 'health': use('health'), 'counter': NAME},
           ('players_in', 'by_base_vocation', 'at', 'owner', 'health')),
    kinded('remove', {'role': NAME, 'all_in': NAME, 'triggering': const(True), 'keep_summons': BOOL}, ()),
    kinded('transform', {'role': NAME, 'into': {'oneOf': [use('CreatureRef'), obj({'next_stage': const(True)}, ('next_stage',)),
                                                           obj({'random_of': array(use('CreatureRef'), 2, True)}, ('random_of',))]},
                         'health': use('health')}, ('role', 'into', 'health')),
    kinded('heal', {'subject': use('subject'), 'amount': {'oneOf': [const('full'), amount(1, 0)]}}, ('subject', 'amount')),
    kinded('damage', {'subject': use('subject'), 'amount': amount(1), 'damage_type': NAME}, ('subject', 'amount', 'damage_type')),
    kinded('prevent_death', {'role': NAME}, ('role',)),
    kinded('damage_modifier', {'role': NAME, 'multiplier_percent': MULTIPLIER, 'damage_types': array(NAME, 0, True), 'component': COMPONENT,
                               'sources': enum('player', 'any'), 'until': enum('this_hit', 'reset', 'timer'), 'timer': NAME},
           ('role', 'multiplier_percent', 'sources', 'until')),
    kinded('reflect_damage', {'role': NAME, 'percent': integer(1, 100), 'damage_types': array(NAME, 0, True)}, ('role', 'percent')),
    kinded('convert_damage_to_heal', {'role': NAME, 'damage_types': array(NAME, 0, True), 'component': COMPONENT}, ('role',)),
    kinded('teleport', {'who': {'oneOf': [obj({'role': NAME}, ('role',)), obj({'players_in': NAME}, ('players_in',)), use('triggering')]},
                        'to': NAME}, ('who', 'to')),
    # CW2-3: `triggering` removes the item that fired a `stepped_on` rule, in place of `item` and a place.
    {**kinded('map_item', {'operation': enum('create', 'transform', 'remove'), 'item': use('ItemRef'), 'triggering': const(True),
                           'into': use('ItemRef'), 'anchor': NAME, 'at': const('death_position'), 'revert_after_ms': integer(1),
                           'destination': NAME, 'revert_destination': NAME, 'effect': TEXT, 'interaction': KEY}, ('operation',)),
     'oneOf': [{'required': ['item']}, {'required': ['triggering']}]},
    kinded('counter', {'counter': NAME, 'operation': enum('set', 'add'), 'value': {'type': 'integer'}}, ('counter', 'operation', 'value')),
    kinded('flag', {'flag': NAME, 'value': BOOL}, ('flag', 'value')),
    kinded('timer', {'timer': NAME, 'operation': enum('start', 'stop', 'add'), 'ms': integer(1)}, ('timer', 'operation')),
    kinded('set_phase', {'phase': NAME}, ('phase',)),
    kinded('move_lock', {'role': NAME, 'locked': BOOL}, ('role', 'locked')),
    kinded('attribute', {'role': NAME, 'attribute': enum('outgoing_damage_percent', 'defense'), 'operation': enum('add', 'reset'),
                         'value': {'oneOf': [integer(1), obj({'counter': NAME}, ('counter',))]}}, ('role', 'attribute', 'operation')),
    kinded('shared_life', {'role': NAME}, ('role',)),
    kinded('cast', {'ability': use('AbilityRef'), 'encounter_ability': NAME, 'at': use('position')}, ('at',)),
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
    'anchors': array(obj({'key': NAME, 'kind': enum('point', 'area'), 'description': TEXT, 'location': LOCATION},
                         ('key', 'kind', 'description'))),
    'phases': array(NAME, 0, True),
    'state': obj({'counters': array(obj({'name': NAME, 'initial': {'type': 'integer'}}, ('name', 'initial'))),
                  'flags': array(obj({'name': NAME, 'initial': BOOL}, ('name', 'initial'))),
                  'timers': array(obj({'name': NAME, 'duration_ms': amount(1), 'repeat': BOOL}, ('name', 'duration_ms', 'repeat')))},
                 ('counters', 'flags', 'timers')),
    'rules': array(use('rule'), 1),
    'outcomes': array(NAME, 0, True),
    # D34: an area effect authored by the encounter itself, such as a death explosion scripted in Lua rather than a monster spell.
    'abilities': array(obj({'key': NAME, 'area': obj({'shape': enum('square', 'circle'), 'radius': integer(0)}, ('shape', 'radius')),
                            'damage': obj({'damage_type': NAME, 'min': integer(1), 'max': integer(1)}, ('damage_type', 'min', 'max')),
                            'affects': obj({'players': BOOL, 'creatures': array(use('CreatureRef'), 0, True)}, ('players', 'creatures')),
                            'effect': TEXT}, ('key', 'area', 'damage', 'affects'))),
    'reset_after_ms': integer(1)},
    ('identity', 'display_name', 'scope', 'participants', 'anchors', 'phases', 'state', 'rules', 'outcomes'),
    **{'$schema': 'https://json-schema.org/draft/2020-12/schema', '$id': 'oteryn:encounter-authoring/v1', '$defs': d,
       'description': 'Oteryn Encounter authoring format v1 (CANDIDATE, D20/D26-D31, D34; CW2-1..4 accepted in section 12).'})


if __name__ == '__main__':
    (ROOT / 'encounter.schema.json').write_text(json.dumps(schema, indent=2) + '\n', encoding='utf-8', newline='\n')
    print('Generated encounter.schema.json')
