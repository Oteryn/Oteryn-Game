"""Focused positive/negative checks of the Encounter schema and semantic validator (synthetic fixtures only)."""
import copy
import json
from pathlib import Path

from validate_encounter import validate

ROOT = Path(__file__).resolve().parent


def ref(family, name):
    return {'family': family, 'key': 'oteryn:' + name, 'revision': 'r1'}


def fixture():
    encounter = {
        'identity': {'key': 'oteryn:encounter/example', 'revision': 'r1'}, 'display_name': 'Example', 'scope': 'instance_per_party',
        'participants': [{'role': 'boss', 'creatures': [ref('Creature', 'boss')]}],
        'anchors': [{'key': 'arena', 'kind': 'area', 'description': 'arena'}, {'key': 'exit', 'kind': 'point', 'description': 'exit'}],
        'phases': ['first', 'second'],
        'state': {'counters': [{'name': 'adds_killed', 'initial': 0}], 'flags': [{'name': 'enraged', 'initial': False}],
                  'timers': [{'name': 'enrage', 'duration_ms': 60000, 'repeat': False}]},
        'rules': [{'key': 'lethal', 'trigger': {'kind': 'lethal_damage', 'role': 'boss'},
                   'conditions': [{'kind': 'chance_percent', 'value': 10}],
                   'actions': [{'kind': 'prevent_death', 'role': 'boss'}, {'kind': 'heal', 'subject': {'role': 'boss'}, 'amount': 'full'}]},
                  {'key': 'death', 'trigger': {'kind': 'creature_died', 'role': 'boss'}, 'conditions': [],
                   'actions': [{'kind': 'emit_outcome', 'outcome': 'victory', 'credited': 'damage_contributors'},
                               {'kind': 'teleport', 'who': {'players_in': 'arena'}, 'to': 'exit'}]}],
        'outcomes': ['victory'], 'reset_after_ms': 300000}
    catalog = {'definitions': [ref('Creature', 'boss'), ref('Creature', 'add'), ref('Item', 'vortex'), ref('Ability', 'summon')]}
    return encounter, catalog


results = []


def case(name, mutate=None, expected=False, error=None):
    """`error`, for a negative case, is a text that one of the errors must contain, so the case fails for its own reason."""
    encounter, catalog = fixture()
    if mutate:
        mutate(encounter, catalog)
    errors = validate(encounter, catalog)
    passed = (not errors) == expected and (error is None or any(error in e for e in errors))
    results.append({'name': name, 'expected_valid': expected, 'passed': passed, 'first_error': errors[0] if errors else None})


def rule(extra_actions=None, trigger=None, conditions=None):
    def mutate(e, c):
        e['rules'].append({'key': 'extra', 'trigger': trigger or {'kind': 'encounter_started'}, 'conditions': conditions or [],
                           'actions': extra_actions or [{'kind': 'flag', 'flag': 'enraged', 'value': True}]})
    return mutate


case('fixture accepted', expected=True)
case('unknown trigger kind', rule(trigger={'kind': 'moon_rises'}))
case('unknown action kind', rule([{'kind': 'explode_everything'}]))
case('scope is closed', lambda e, c: e.update(scope='global'))
case('rules are required', lambda e, c: e.update(rules=[]))
case('unknown role in trigger', rule(trigger={'kind': 'creature_died', 'role': 'ghost'}))
case('prevent_death outside lethal_damage', rule([{'kind': 'prevent_death', 'role': 'boss'}]))
case('prevent_death for another role', lambda e, c: e['rules'][0]['actions'].__setitem__(0, {'kind': 'prevent_death', 'role': 'adds'}))
case('undeclared outcome', rule([{'kind': 'emit_outcome', 'outcome': 'defeat', 'credited': 'damage_contributors'}]))
case('outcome credit is required', rule([{'kind': 'emit_outcome', 'outcome': 'victory'}]))
case('killer credit outside a death trigger', rule([{'kind': 'emit_outcome', 'outcome': 'victory', 'credited': 'killer'}]))
case('players_in_anchor credit needs an area', rule([{'kind': 'emit_outcome', 'outcome': 'victory', 'credited': 'players_in_anchor',
                                                      'anchor': 'exit'}]))
case('has_master condition accepted', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                           conditions=[{'kind': 'has_master', 'role': 'boss', 'value': False}]), True)
case('summon_count condition accepted', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                             conditions=[{'kind': 'summon_count', 'role': 'boss', 'op': '<', 'value': 8}]), True)
case('summon_count for an unknown role', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                              conditions=[{'kind': 'summon_count', 'role': 'ghost', 'op': '<', 'value': 8}]))
case('teleport to unknown anchor', rule([{'kind': 'teleport', 'who': {'role': 'boss'}, 'to': 'nowhere'}]))
case('players_in needs an area', rule([{'kind': 'teleport', 'who': {'players_in': 'exit'}, 'to': 'exit'}]))
case('unknown counter', rule([{'kind': 'counter', 'counter': 'missing', 'operation': 'add', 'value': 1}]))
case('unknown timer trigger', rule(trigger={'kind': 'timer_elapsed', 'timer': 'missing'}))
case('timer trigger accepted', rule(trigger={'kind': 'timer_elapsed', 'timer': 'enrage'}), True)
case('set unknown phase', rule([{'kind': 'set_phase', 'phase': 'third'}]))
case('phase change accepted', rule([{'kind': 'set_phase', 'phase': 'second'}]), True)
case('killer outside a death trigger', rule([{'kind': 'heal', 'subject': {'killer': True}, 'amount': 'full'}]))
case('killer_progress outside a death trigger', rule(conditions=[{'kind': 'killer_progress', 'progress': 'oteryn:quest/x',
                                                                   'op': '==', 'value': True}]))
case('killer_progress on death accepted', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                               conditions=[{'kind': 'killer_progress', 'progress': 'oteryn:quest/x', 'op': '>=', 'value': 4}]),
     True)
case('map_item transform needs into', rule([{'kind': 'map_item', 'operation': 'transform', 'item': ref('Item', 'vortex'),
                                             'anchor': 'exit'}]))
case('map_item create forbids into', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'),
                                            'into': ref('Item', 'vortex'), 'anchor': 'exit'}]))
case('timed map item accepted', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'), 'anchor': 'exit',
                                       'revert_after_ms': 60000}]), True)
case('teleporter with destinations accepted', rule([{'kind': 'map_item', 'operation': 'transform', 'item': ref('Item', 'vortex'),
                                                    'into': ref('Item', 'vortex'), 'anchor': 'exit', 'destination': 'exit',
                                                    'revert_after_ms': 60000, 'revert_destination': 'exit'}]), True)
case('revert destination needs a revert time', rule([{'kind': 'map_item', 'operation': 'transform', 'item': ref('Item', 'vortex'),
                                                      'into': ref('Item', 'vortex'), 'anchor': 'exit', 'revert_destination': 'exit'}]))
case('map_item needs a place', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex')}]))
case('map_item takes one place', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'), 'anchor': 'exit',
                                        'at': 'death_position'}]))
case('map_item death position outside a death trigger', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'),
                                                                'at': 'death_position'}]))
case('map_item at the death position accepted', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'),
                                                       'at': 'death_position', 'destination': 'exit', 'revert_after_ms': 120000}],
                                                     trigger={'kind': 'creature_died', 'role': 'boss'}), True)
case('remove needs exactly one target', rule([{'kind': 'remove', 'role': 'boss', 'all_in': 'arena'}]))
case('spawn a new role accepted', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'role': 'adds', 'count': 2,
                                         'at': {'random_in': 'arena'}, 'owner': 'none', 'health': 'full'}]), True)
case('spawn random_in needs an area', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': 1, 'at': {'random_in': 'exit'},
                                             'owner': 'none', 'health': 'full'}]))
case('undeclared creature', rule([{'kind': 'spawn', 'creature': ref('Creature', 'ghost'), 'count': 1, 'at': 'death_position',
                                   'owner': 'none', 'health': 'full'}]))
case('damage modifier until timer needs its timer', rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': 0,
                                                           'sources': 'any', 'until': 'timer'}]))
case('delayed rule accepted', lambda e, c: e['rules'][1].update(delay_ms=6000), True)
case('zero delay rejected', lambda e, c: e['rules'][1].update(delay_ms=0))
case('random transform accepted', rule([{'kind': 'transform', 'role': 'boss', 'into': {'random_of': [ref('Creature', 'boss'), ref('Creature', 'add')]},
                                         'health': 'full'}]), True)
case('random transform needs two forms', rule([{'kind': 'transform', 'role': 'boss', 'into': {'random_of': [ref('Creature', 'add')]},
                                                'health': 'full'}]))
case('spawn owned by the dying master accepted', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': 2, 'at': 'death_position',
                                                        'owner': 'death_master', 'health': 'full'}],
                                                      trigger={'kind': 'creature_died', 'role': 'boss'}), True)
case('death_master outside a death trigger', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': 1, 'at': {'anchor': 'exit'},
                                                    'owner': 'death_master', 'health': 'full'}]))
case('spawn trigger accepted', rule(trigger={'kind': 'creature_spawned', 'role': 'boss'}), True)
case('spawn trigger for an unknown role', rule(trigger={'kind': 'creature_spawned', 'role': 'ghost'}))
case('ability cast trigger accepted', rule(trigger={'kind': 'ability_cast', 'role': 'boss', 'ability': ref('Ability', 'summon')}), True)
case('world state condition accepted', rule(conditions=[{'kind': 'world_state', 'state': 'oteryn:world/flask', 'op': '==', 'value': True}]),
     True)
case('fractional health threshold accepted', rule(trigger={'kind': 'health_crossed', 'role': 'boss', 'percent': 78.125}), True)
case('absolute health threshold accepted', rule(trigger={'kind': 'health_crossed', 'role': 'boss', 'health': 400000}), True)
case('health threshold needs one form', rule(trigger={'kind': 'health_crossed', 'role': 'boss', 'percent': 50, 'health': 1000}))
case('health threshold needs a form', rule(trigger={'kind': 'health_crossed', 'role': 'boss'}))
case('radius presence accepted', rule(conditions=[{'kind': 'creature_present', 'role': 'boss', 'near': {'role': 'boss', 'radius': 7},
                                                   'present': False}]), True)
case('presence needs one place', rule(conditions=[{'kind': 'creature_present', 'role': 'boss', 'present': False}]))
case('presence takes one place', rule(conditions=[{'kind': 'creature_present', 'role': 'boss', 'anchor': 'arena',
                                                   'near': {'role': 'boss', 'radius': 7}, 'present': False}]))
case('ranged spawn count accepted', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': {'min': 1, 'max': 3},
                                           'at': {'anchor': 'exit'}, 'owner': 'none', 'health': 'full'}]), True)
case('inverted range rejected', rule([{'kind': 'heal', 'subject': {'role': 'boss'}, 'amount': {'min': 500, 'max': 100}}]))
case('ranged delay accepted', lambda e, c: e['rules'][1].update(delay_ms={'min': 10000, 'max': 20000}), True)
case('damage action accepted', rule([{'kind': 'damage', 'subject': {'role': 'boss'}, 'amount': 4500, 'damage_type': 'ice'}]), True)
case('damage action needs a type', rule([{'kind': 'damage', 'subject': {'role': 'boss'}, 'amount': 4500}]))
case('primary-only modifier accepted', rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': 200, 'sources': 'any',
                                              'until': 'this_hit', 'component': 'primary'}]), True)
case('map item interaction accepted', rule([{'kind': 'map_item', 'operation': 'create', 'item': ref('Item', 'vortex'), 'anchor': 'exit',
                                             'interaction': 'canary:interaction/5580'}]), True)
died = {'kind': 'creature_died', 'role': 'boss'}
add = lambda: {'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': 1, 'at': 'death_position', 'owner': 'none', 'health': 'full'}
case('heal trigger accepted', rule(trigger={'kind': 'heal_received', 'role': 'boss', 'source': 'any'}), True)
case('circle presence accepted', rule(conditions=[{'kind': 'creature_present', 'role': 'boss',
                                                   'near': {'role': 'boss', 'radius': 3, 'shape': 'circle'}, 'present': True}]), True)
case('heal range from zero accepted', rule([{'kind': 'heal', 'subject': {'role': 'boss'}, 'amount': {'min': 0, 'max': 2000}}]), True)
case('fixed zero heal rejected', rule([{'kind': 'heal', 'subject': {'role': 'boss'}, 'amount': 0}]))
case('remove triggering creature accepted', rule([{'kind': 'remove', 'triggering': True}], trigger=died), True)
case('remove triggering needs a creature trigger', rule([{'kind': 'remove', 'triggering': True}]))
case('remove takes one target of three', rule([{'kind': 'remove', 'role': 'boss', 'triggering': True}], trigger=died))
case('remove keeping summons accepted', rule([{'kind': 'remove', 'all_in': 'arena', 'keep_summons': True}]), True)
case('keep_summons needs all_in', rule([{'kind': 'remove', 'role': 'boss', 'keep_summons': True}]))
case('player message accepted', rule([{'kind': 'message', 'to': {'players_in': 'arena'}, 'text': 'You lost.'}]), True)
case('player message needs an area', rule([{'kind': 'message', 'to': {'players_in': 'exit'}, 'text': 'You lost.'}]))
case('weighted choice accepted', rule([{'kind': 'one_of', 'branches': [{'weight': 1, 'actions': [add()]},
                                                                       {'weight': 3, 'actions': [{'kind': 'flag', 'flag': 'enraged', 'value': True}]}]}],
                                      trigger=died), True)
case('weighted choice needs two branches', rule([{'kind': 'one_of', 'branches': [{'weight': 1, 'actions': [add()]}]}], trigger=died))
case('choice branches are validated', rule([{'kind': 'one_of', 'branches': [{'weight': 1, 'actions': [add()]},
                                                                            {'weight': 1, 'actions': [{'kind': 'flag', 'flag': 'ghost', 'value': True}]}]}],
                                           trigger=died))
case('role position accepted', rule([{**add(), 'at': {'role_position': 'boss'}}],
                                    conditions=[{'kind': 'creature_present', 'role': 'boss', 'anchor': 'arena', 'present': True}]), True)
case('role position needs a presence condition', rule([{**add(), 'at': {'role_position': 'boss'}}]))
case('party credit accepted', rule([{'kind': 'emit_outcome', 'outcome': 'victory', 'credited': 'party'}], trigger=died), True)
case('party credit needs a death', rule([{'kind': 'emit_outcome', 'outcome': 'victory', 'credited': 'party'}]))
case('spawned speaker accepted', rule([add(), {'kind': 'say', 'subject': {'spawned': True}, 'text': 'Free!', 'mode': 'say'}], trigger=died),
     True)
case('spawned speaker needs a spawn', rule([{'kind': 'say', 'subject': {'spawned': True}, 'text': 'Free!', 'mode': 'say'}], trigger=died))
case('role position fallback accepted', rule([{**add(), 'at': {'role_position': 'boss', 'otherwise': 'death_position'}}], trigger=died), True)
case('chance above 100 rejected', rule(conditions=[{'kind': 'chance_percent', 'value': 150}]))
case('duplicate rule key', lambda e, c: e['rules'].append(copy.deepcopy(e['rules'][0])))
case('heal scaling accepted', rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': 200, 'component': 'primary',
                                     'sources': 'any', 'until': 'this_hit'}],
                                   trigger={'kind': 'heal_received', 'role': 'boss', 'source': 'any'}), True)
case('non-player source accepted', rule(trigger={'kind': 'damage_taken', 'role': 'boss', 'source': 'non_player'}), True)
case('worn slot accepted', rule(conditions=[{'kind': 'attacker_wears', 'item': ref('Item', 'vortex'), 'wears': False, 'slot': 'armor'}],
                                trigger={'kind': 'heal_received', 'role': 'boss', 'source': 'player'}), True)
case('unknown worn slot', rule(conditions=[{'kind': 'attacker_wears', 'item': ref('Item', 'vortex'), 'wears': False, 'slot': 'backpack'}],
                               trigger={'kind': 'damage_taken', 'role': 'boss', 'source': 'player'}))
case('amount chance accepted', rule(conditions=[{'kind': 'chance_from_amount', 'per': 1000000}],
                                    trigger={'kind': 'damage_taken', 'role': 'boss', 'source': 'any'}), True)
case('amount chance needs a health change', rule(conditions=[{'kind': 'chance_from_amount', 'per': 1000000}]))
case('move lock accepted', rule([{'kind': 'move_lock', 'role': 'boss', 'locked': True}]), True)
case('move lock needs a known role', rule([{'kind': 'move_lock', 'role': 'nobody', 'locked': True}]))
case('time-scaled modifier accepted', rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': {'timer_remaining': 'enrage', 'floor': 1},
                                              'sources': 'any', 'until': 'reset'}]), True)
case('time-scaled modifier needs a known timer', rule([{'kind': 'damage_modifier', 'role': 'boss',
                                                        'multiplier_percent': {'timer_remaining': 'nothing', 'floor': 1},
                                                        'sources': 'any', 'until': 'reset'}]))
case('time-scaled modifier needs a timer that does not repeat',
     lambda e, c: (e['state']['timers'][0].update(repeat=True),
                   rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': {'timer_remaining': 'enrage', 'floor': 1},
                          'sources': 'any', 'until': 'reset'}])(e, c)))
case('shared life accepted', rule([{'kind': 'shared_life', 'role': 'boss'}]), True)
case('shared life needs a known role', rule([{'kind': 'shared_life', 'role': 'nobody'}]))
EXPLOSION = {'key': 'explosion', 'area': {'shape': 'circle', 'radius': 2}, 'damage': {'damage_type': 'life_drain', 'min': 2000, 'max': 2500},
             'affects': {'players': True, 'creatures': []}}
case('encounter ability cast accepted',
     lambda e, c: (e.update(abilities=[EXPLOSION]),
                   rule([{'kind': 'cast', 'encounter_ability': 'explosion', 'at': 'death_position'}], trigger=died)(e, c)), True)
case('encounter ability must be authored', rule([{'kind': 'cast', 'encounter_ability': 'explosion', 'at': 'death_position'}], trigger=died))
case('cast takes one ability',
     lambda e, c: (e.update(abilities=[EXPLOSION]),
                   rule([{'kind': 'cast', 'encounter_ability': 'explosion', 'ability': ref('Ability', 'summon'), 'at': 'death_position'}],
                        trigger=died)(e, c)))
case('encounter ability creatures are catalogued',
     lambda e, c: (e.update(abilities=[{**EXPLOSION, 'affects': {'players': True, 'creatures': [ref('Creature', 'stranger')]}}]),
                   rule([{'kind': 'cast', 'encounter_ability': 'explosion', 'at': 'death_position'}], trigger=died)(e, c)))
case('item use accepted', rule(trigger={'kind': 'item_used', 'role': 'boss', 'item': ref('Item', 'vortex')}), True)
case('item use needs a catalogued item', rule(trigger={'kind': 'item_used', 'role': 'boss', 'item': ref('Item', 'stranger')}))
case('item use by one base vocation accepted', rule(trigger={'kind': 'item_used', 'role': 'boss', 'item': ref('Item', 'vortex'),
                                                          'base_vocation': 'monk'}), True)
case('item use vocation must be a base vocation', rule(trigger={'kind': 'item_used', 'role': 'boss', 'item': ref('Item', 'vortex'),
                                                             'base_vocation': 'elite_knight'}))
HIT = {'kind': 'damage_taken', 'role': 'boss', 'source': 'any'}
PER_PLAYER = {'kind': 'spawn_per_player', 'players_in': 'arena', 'by_base_vocation': {'knight': ref('Creature', 'add')},
              'at': {'offset_tiles': 1}, 'owner': 'none', 'health': 'full', 'counter': 'adds_killed'}
case('per-player spawn accepted', rule([PER_PLAYER], trigger=HIT), True)
case('spawn at the closest free tile accepted', rule([{**PER_PLAYER, 'at': 'closest_free_tile'}], trigger=HIT), True)
case('stepped-on trigger accepted (D46)', rule(trigger={'kind': 'stepped_on', 'role': 'boss', 'item': ref('Item', 'vortex')}), True)
case('stepped-on needs a catalogued item', rule(trigger={'kind': 'stepped_on', 'role': 'boss', 'item': ref('Item', 'stranger')}))
case('stepped-on needs a known role', rule(trigger={'kind': 'stepped_on', 'role': 'ghost', 'item': ref('Item', 'vortex')}))
case('stepped-on remove triggering accepted', rule([{'kind': 'remove', 'triggering': True}],
                                                   trigger={'kind': 'stepped_on', 'role': 'boss', 'item': ref('Item', 'vortex')}), True)
case('has_condition accepted (D46)', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                          conditions=[{'kind': 'has_condition', 'role': 'boss', 'conditions': ['poison', 'bleeding'],
                                                       'present': True}]), True)
case('has_condition needs a known role', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                              conditions=[{'kind': 'has_condition', 'role': 'ghost', 'conditions': ['poison'],
                                                           'present': True}]))
case('has_condition takes known conditions', rule(trigger={'kind': 'creature_died', 'role': 'boss'},
                                                  conditions=[{'kind': 'has_condition', 'role': 'boss', 'conditions': ['sleepy'],
                                                               'present': True}]))
case('relative position accepted (D46)', rule([{**PER_PLAYER, 'at': {'relative': {'x': 0, 'y': -1}}}], trigger=HIT), True)
case('relative position needs a creature trigger', rule([{**PER_PLAYER, 'at': {'relative': {'x': 0, 'y': -1}}}]))
case('closest free tile needs a creature trigger', rule([{**PER_PLAYER, 'at': 'closest_free_tile'}]))
case('relative position needs both offsets', rule([{**PER_PLAYER, 'at': {'relative': {'y': -1}}}], trigger=HIT))
case('damage accumulated as a percent of maximum health accepted',
     rule(trigger={'kind': 'damage_accumulated', 'role': 'boss', 'percent': 15}), True)
case('damage accumulated takes an amount or a percent, not both',
     rule(trigger={'kind': 'damage_accumulated', 'role': 'boss', 'amount': 100, 'percent': 15}))
case('damage accumulated needs an amount or a percent', rule(trigger={'kind': 'damage_accumulated', 'role': 'boss'}))
case('per-player spawn needs an area', rule([{**PER_PLAYER, 'players_in': 'exit'}], trigger=HIT))
case('per-player spawn needs a declared counter', rule([{**PER_PLAYER, 'counter': 'nothing'}], trigger=HIT))
case('per-player spawn needs a vocation entry', rule([{**PER_PLAYER, 'by_base_vocation': {}}], trigger=HIT))
case('per-player spawn creatures are catalogued', rule([{**PER_PLAYER, 'by_base_vocation': {'monk': ref('Creature', 'stranger')}}], trigger=HIT))
case('per-player spawn takes base vocations', rule([{**PER_PLAYER, 'by_base_vocation': {'royal_paladin': ref('Creature', 'add')}}], trigger=HIT))
case('timer add accepted', rule([{'kind': 'timer', 'timer': 'enrage', 'operation': 'add', 'ms': 10000}]), True)
case('timer add needs its ms', rule([{'kind': 'timer', 'timer': 'enrage', 'operation': 'add'}]))
case('timer start takes no ms', rule([{'kind': 'timer', 'timer': 'enrage', 'operation': 'start', 'ms': 10000}]))
case('attribute add accepted', rule([{'kind': 'attribute', 'role': 'boss', 'attribute': 'outgoing_damage_percent', 'operation': 'add',
                                      'value': 10}]), True)
case('attribute add from a counter accepted', rule([{'kind': 'attribute', 'role': 'boss', 'attribute': 'defense', 'operation': 'add',
                                                     'value': {'counter': 'adds_killed'}}]), True)
case('attribute add needs a known counter', rule([{'kind': 'attribute', 'role': 'boss', 'attribute': 'defense', 'operation': 'add',
                                                   'value': {'counter': 'nothing'}}]))
case('attribute reset takes no value', rule([{'kind': 'attribute', 'role': 'boss', 'attribute': 'defense', 'operation': 'reset', 'value': 1}]))
case('attribute is closed', rule([{'kind': 'attribute', 'role': 'boss', 'attribute': 'speed', 'operation': 'reset'}]))
case('remembered health accepted', rule([{**add(), 'role': 'boss', 'health': 'remembered'}], trigger=died), True)
case('remembered health needs a role', rule([{**add(), 'health': 'remembered'}], trigger=died))
case('transform cannot remember health', rule([{'kind': 'transform', 'role': 'boss', 'into': ref('Creature', 'add'), 'health': 'remembered'}], trigger=died))

# Section 12 (CW2-1..4), accepted 2026-09-29.
PORTAL = {'kind': 'area_entered', 'anchor': 'arena', 'who': 'player'}
TRIGGERING = {'triggering': True}
case('teleport triggering accepted (CW2-1)', rule([{'kind': 'teleport', 'who': TRIGGERING, 'to': 'exit'}], trigger=PORTAL), True)
case('teleport triggering in a creature trigger accepted', rule([{'kind': 'teleport', 'who': TRIGGERING, 'to': 'exit'}], trigger=HIT), True)
case('teleport triggering needs a creature or area trigger',
     rule([{'kind': 'teleport', 'who': TRIGGERING, 'to': 'exit'}], trigger={'kind': 'timer_elapsed', 'timer': 'enrage'}),
     error='teleport triggering needs')
case('in_anchor of triggering accepted (CW2-1)',
     lambda e, c: (rule([{'kind': 'teleport', 'who': TRIGGERING, 'to': 'exit'}], trigger=PORTAL,
                        conditions=[{'kind': 'in_anchor', 'subject': TRIGGERING, 'anchor': 'arena'}])(e, c),
                   e['rules'][-1].update(delay_ms=10000)), True)
case('in_anchor of triggering needs a creature or area trigger',
     rule(trigger={'kind': 'timer_elapsed', 'timer': 'enrage'}, conditions=[{'kind': 'in_anchor', 'subject': TRIGGERING, 'anchor': 'arena'}]),
     error='in_anchor of triggering needs')
case('triggering is not a shared action subject', rule([{'kind': 'heal', 'subject': TRIGGERING, 'amount': 'full'}], trigger=HIT),
     error='schema')
case('remove triggering never removes a player', rule([{'kind': 'remove', 'triggering': True}], trigger=PORTAL),
     error='never removes a player')


def gorzindel_portal(e, c):
    """The section 12.2 portal, room 1 of 5 and the return."""
    e['anchors'] += [{'key': 'portal_tile', 'kind': 'area', 'description': 'portal tile'},
                     {'key': 'knowledge_room_1', 'kind': 'point', 'description': 'room 1'},
                     {'key': 'knowledge_range', 'kind': 'area', 'description': 'main room and knowledge rooms'},
                     {'key': 'library_middle', 'kind': 'point', 'description': 'middle'}]
    e['state']['flags'] += [{'name': 'portal_assigned', 'initial': False}, {'name': 'room_1_busy', 'initial': False}]
    e['state']['timers'].append({'name': 'room_1_hold', 'duration_ms': 10000, 'repeat': False})
    step = {'kind': 'area_entered', 'anchor': 'portal_tile', 'who': 'player'}
    e['rules'] += [
        {'key': 'portal_step_starts', 'trigger': step, 'conditions': [], 'actions': [{'kind': 'flag', 'flag': 'portal_assigned', 'value': False}]},
        {'key': 'portal_to_room_1', 'trigger': step,
         'conditions': [{'kind': 'flag', 'flag': 'portal_assigned', 'value': False}, {'kind': 'flag', 'flag': 'room_1_busy', 'value': False}],
         'actions': [{'kind': 'teleport', 'who': TRIGGERING, 'to': 'knowledge_room_1'},
                     {'kind': 'flag', 'flag': 'room_1_busy', 'value': True}, {'kind': 'flag', 'flag': 'portal_assigned', 'value': True},
                     {'kind': 'timer', 'timer': 'room_1_hold', 'operation': 'start'}]},
        {'key': 'portal_return', 'trigger': step, 'delay_ms': 10000,
         'conditions': [{'kind': 'in_anchor', 'subject': TRIGGERING, 'anchor': 'knowledge_range'}],
         'actions': [{'kind': 'teleport', 'who': TRIGGERING, 'to': 'library_middle'}]},
        {'key': 'room_1_reopens', 'trigger': {'kind': 'timer_elapsed', 'timer': 'room_1_hold'}, 'conditions': [],
         'actions': [{'kind': 'flag', 'flag': 'room_1_busy', 'value': False}]}]


case('Gorzindel portal accepted (section 12.2)', gorzindel_portal, True)


def with_brood(mutate):
    def wrapped(e, c):
        e['participants'].append({'role': 'brood', 'creatures': [ref('Creature', 'add')]})
        mutate(e, c)
    return wrapped


CORPSE = {'kind': 'stepped_on', 'role': 'boss', 'corpse_of': 'brood'}
EAT = [{'kind': 'heal', 'subject': {'role': 'boss'}, 'amount': {'min': 100, 'max': 1000}},
       {'kind': 'map_item', 'operation': 'remove', 'triggering': True}]
case('stepped-on corpse accepted (CW2-2, CW2-3)', with_brood(rule(EAT, trigger=CORPSE)), True)
case('stepped-on takes an item or a corpse, not both',
     with_brood(rule(trigger={**CORPSE, 'item': ref('Item', 'vortex')})), error='schema')
case('stepped-on needs an item or a corpse', rule(trigger={'kind': 'stepped_on', 'role': 'boss'}), error='schema')
case('stepped-on corpse of an unknown role', rule(trigger={**CORPSE, 'corpse_of': 'ghost'}), error="unknown role 'ghost'")
case('map_item triggering on a stepped-on item accepted (CW2-3)',
     rule([{'kind': 'map_item', 'operation': 'remove', 'triggering': True}],
          trigger={'kind': 'stepped_on', 'role': 'boss', 'item': ref('Item', 'vortex')}), True)
case('map_item triggering cannot create', with_brood(rule([{'kind': 'map_item', 'operation': 'create', 'triggering': True}], trigger=CORPSE)),
     error='only with operation remove')
case('map_item triggering cannot transform',
     with_brood(rule([{'kind': 'map_item', 'operation': 'transform', 'triggering': True, 'into': ref('Item', 'vortex')}], trigger=CORPSE)),
     error='only with operation remove')
case('map_item triggering needs a stepped-on trigger', rule([{'kind': 'map_item', 'operation': 'remove', 'triggering': True}], trigger=died),
     error='needs a stepped_on trigger')
case('map_item triggering takes no item', with_brood(rule([{'kind': 'map_item', 'operation': 'remove', 'triggering': True,
                                                            'item': ref('Item', 'vortex')}], trigger=CORPSE)), error='schema')
case('map_item triggering takes no place', with_brood(rule([{'kind': 'map_item', 'operation': 'remove', 'triggering': True,
                                                             'anchor': 'exit'}], trigger=CORPSE)), error='forbids')
case('map_item needs an item or triggering', rule([{'kind': 'map_item', 'operation': 'remove', 'anchor': 'exit'}]), error='schema')
FREE_SPAWN = {'kind': 'spawn', 'creature': ref('Creature', 'add'), 'role': 'adds', 'count': 1, 'owner': 'none', 'health': 'full'}
case('random free tile accepted (CW2-4)', rule([{**FREE_SPAWN, 'at': {'random_in': 'arena', 'free': True}}]), True)
case('random tile with free false accepted', rule([{**FREE_SPAWN, 'at': {'random_in': 'arena', 'free': False}}]), True)
case('random free tile needs an area', rule([{**FREE_SPAWN, 'at': {'random_in': 'exit', 'free': True}}]), error='an area is required')
case('free is a boolean', rule([{**FREE_SPAWN, 'at': {'random_in': 'arena', 'free': 'yes'}}]), error='schema')
case('free applies only to random_in', rule([{**FREE_SPAWN, 'at': {'anchor': 'exit', 'free': True}}]), error='schema')


def locate(key, location):
    def mutate(e, c):
        next(a for a in e['anchors'] if a['key'] == key)['location'] = location
    return mutate


BOX = {'x': [100, 110], 'y': [200, 210], 'floor': 7}
case('point location accepted (E2)', locate('exit', {'x': 100, 'y': 200, 'floor': 7}), True)
case('area boxes accepted (E2)', locate('arena', {'boxes': [BOX, {**BOX, 'floor': 8}]}), True)
case('a box is on one floor', locate('arena', {'boxes': [{**BOX, 'floor': [7, 8]}]}))
case('a point takes a point location', locate('exit', {'boxes': [BOX]}))
case('an area takes boxes', locate('arena', {'x': 100, 'y': 200, 'floor': 7}))
case('a box does not start after it ends', locate('arena', {'boxes': [{**BOX, 'x': [110, 100]}]}))
case('a box floor stays on the map', locate('arena', {'boxes': [{**BOX, 'floor': 16}]}))
case('an area needs a box', locate('arena', {'boxes': []}))
case('a point needs its floor', locate('exit', {'x': 100, 'y': 200}))

if __name__ == '__main__':
    report = {'scope': 'Encounter schema and semantic validator, synthetic fixtures only; no Lua or Oteryn runtime executed',
              'checks': len(results), 'passed': sum(r['passed'] for r in results),
              'failed': [r for r in results if not r['passed']]}
    print(json.dumps({k: v for k, v in report.items() if k != 'failed'} | {'failed': len(report['failed'])}))
    for failure in report['failed']:
        print('FAILED', failure)
