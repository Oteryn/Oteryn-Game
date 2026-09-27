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
    catalog = {'definitions': [ref('Creature', 'boss'), ref('Creature', 'add'), ref('Item', 'vortex')]}
    return encounter, catalog


results = []


def case(name, mutate=None, expected=False):
    encounter, catalog = fixture()
    if mutate:
        mutate(encounter, catalog)
    errors = validate(encounter, catalog)
    results.append({'name': name, 'expected_valid': expected, 'passed': (not errors) == expected,
                    'first_error': errors[0] if errors else None})


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
case('remove needs exactly one target', rule([{'kind': 'remove', 'role': 'boss', 'all_in': 'arena'}]))
case('spawn a new role accepted', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'role': 'adds', 'count': 2,
                                         'at': {'random_in': 'arena'}, 'owner': 'none', 'health': 'full'}]), True)
case('spawn random_in needs an area', rule([{'kind': 'spawn', 'creature': ref('Creature', 'add'), 'count': 1, 'at': {'random_in': 'exit'},
                                             'owner': 'none', 'health': 'full'}]))
case('undeclared creature', rule([{'kind': 'spawn', 'creature': ref('Creature', 'ghost'), 'count': 1, 'at': 'death_position',
                                   'owner': 'none', 'health': 'full'}]))
case('damage modifier until timer needs its timer', rule([{'kind': 'damage_modifier', 'role': 'boss', 'multiplier_percent': 0,
                                                           'sources': 'any', 'until': 'timer'}]))
case('chance above 100 rejected', rule(conditions=[{'kind': 'chance_percent', 'value': 150}]))
case('duplicate rule key', lambda e, c: e['rules'].append(copy.deepcopy(e['rules'][0])))

if __name__ == '__main__':
    report = {'scope': 'Encounter schema and semantic validator, synthetic fixtures only; no Lua or Oteryn runtime executed',
              'checks': len(results), 'passed': sum(r['passed'] for r in results),
              'failed': [r for r in results if not r['passed']]}
    print(json.dumps({k: v for k, v in report.items() if k != 'failed'} | {'failed': len(report['failed'])}))
    for failure in report['failed']:
        print('FAILED', failure)
