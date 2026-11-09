#!/usr/bin/env python3
"""Rebuild batch-four callbacks and source-only NPC packets offline.

The committed packets are authored inputs, not an executable Lua interpreter.
Existing callback keys outside those inputs retain their values. The combined
registry is sorted by key, matching the batch-three compiler.
No dialogue manifest, NPC qualification, item, map or quest recipe is changed.
The default requires every registered packet. Explicit --quests and --available modes
support regenerating independently cherry-picked quest inputs.
"""
import argparse
import copy
import json
from pathlib import Path
import re
import sys

SOURCE_REVISION = 'zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE_PREFIX = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
INPUT_DIRECTORY = Path('tools/content-schema/quest-authoring/batches/quests4')
CALLBACK_OUTPUT = Path('content/quests/world-callbacks.json')
NPC_OUTPUT_DIRECTORY = Path('content/dialogues/quest-sources')
QUEST_ORDER = (
    'cults_of_tibia', 'dreamers_challenge_quest', 'elemental_spheres',
    'formogar_mine_hoist', 'kilmaresh', 'koshei_the_deathless_quest',
    'krailos', 'lions_rock', 'newhaven', 'rotten_blood_quest', 'spirit_hunters',
    'svargrond_arena', 'thais_lighthouse', 'the_explorer_society',
    'the_first_dragon', 'the_hunt_for_the_sea_serpent', 'the_lost_brother',
    'the_order_of_lion_quest', 'the_spike_tasks', 'tibia_tales',
    'triangle_tower_quest', 'unnatural_selection', 'waterfall',
)
QUEST_OWNERS = {
    'cults_of_tibia': 'oteryn:quest.cults_of_tibia_quest',
    'dreamers_challenge_quest': 'oteryn:quest.dreamer_s_challenge_quest',
    'elemental_spheres': 'oteryn:quest.elemental_spheres_quest',
    'formogar_mine_hoist': 'oteryn:quest.formogar_mine_hoist',
    'kilmaresh': 'oteryn:quest.kilmaresh_quest',
    'koshei_the_deathless_quest': 'oteryn:quest.koshei_the_deathless_quest',
    'krailos': 'oteryn:quest.krailos_quest',
    # Existing Questline/InnerSanctum.Message tracks are owned by Tibia Tales.
    'lions_rock': 'oteryn:quest.tibia_tales',
    'newhaven': 'oteryn:quest.newhaven_quest',
    'rotten_blood_quest': 'oteryn:quest/rotten-blood',
    'spirit_hunters': None,
    'svargrond_arena': 'oteryn:quest.barbarian_arena_quest',
    'thais_lighthouse': 'oteryn:quest.thais_lighthouse_quest',
    'the_explorer_society': 'oteryn:quest.the_explorer_society_quest',
    'the_first_dragon': 'oteryn:quest.the_first_dragon_quest',
    'the_hunt_for_the_sea_serpent': 'oteryn:quest.the_hunt_for_the_sea_serpent',
    'the_lost_brother': 'oteryn:quest.adventurers_guild',
    'the_order_of_lion_quest': 'oteryn:quest.the_order_of_the_lion_quest',
    'the_spike_tasks': 'oteryn:quest.spike_task',
    'tibia_tales': 'oteryn:quest.tibia_tales',
    'triangle_tower_quest': 'oteryn:quest.triangle_tower_quest',
    'unnatural_selection': 'oteryn:quest.unnatural_selection_quest',
    'waterfall': 'oteryn:quest.waterfall_quest',
}

PACKET_FIELDS = {
    'schema', 'quest_directory', 'source_repository_revision', 'callbacks',
    'runtime_exclusions', 'npc_source', 'step_inventory', 'preserved_claims',
}
CALLBACK_FIELDS = {
    'key', 'quest', 'trigger', 'selector', 'lever', 'reversible', 'passages',
    'teleport', 'storage', 'minimum_level', 'experience', 'monk_reward',
    'offerings', 'item_on_target', 'return_on_failure', 'failure_message',
    'success_message', 'native_rule', 'sources', 'facing', 'effects',
    'requires_world_admission',
}
CALLBACK_REQUIRED = {
    'key', 'quest', 'trigger', 'selector', 'reversible', 'passages',
    'storage', 'offerings', 'return_on_failure', 'sources',
}
NATIVE_TRACKS = {
    'cults_symbol': (
        'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/mission',
        'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/temp',
        'oteryn:quest-progress/quest/u11_40/cults_of_tibia/barkless/objects',
    ),
    'unnatural_dance': (
        'oteryn:quest-progress/quest/u8_54/unnatural_selection/dance_status',
        'oteryn:quest-progress/quest/u8_54/unnatural_selection/mission03',
        'oteryn:quest-progress/quest/u8_54/unnatural_selection/questline',
    ),
}
# Pinned literal bindings, mirrored by world_callbacks/native.rs. These are
# selector/guard qualification data, not implementations of either state rule.
NATIVE_BINDINGS = {
    'cults_symbol': {
        'trigger': 'use', 'action': 5535,
        'positions': (
            (32731, 31531, -9), (32739, 31489, -9), (32739, 31507, -9),
            (32761, 31518, -9), (32720, 31545, -8), (32745, 31423, -8),
            (32742, 31410, -8), (32685, 31430, -8), (32746, 31462, -8),
            (32683, 31537, -9), (32740, 31494, -9), (32741, 31494, -9),
            (32745, 31523, -9),
        ),
        'ranges': ((0, 7), (0, 2147483647), (0, 2147483647)),
        'source': 'data-global/scripts/quests/cults_of_tibia/actions_cult_symbol.lua:',
    },
    'unnatural_dance': {
        'trigger': 'step_in', 'action': None,
        'positions': (
            (32990, 31496, -1), (32991, 31496, -1), (32992, 31496, -1),
            (32990, 31497, -1), (32991, 31497, -1), (32992, 31497, -1),
            (32990, 31498, -1), (32991, 31498, -1), (32992, 31498, -1),
        ),
        'ranges': ((1, 16), (-1, 3), (-1, 15)),
        'source': 'data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:',
    },
}
KEY = re.compile(r'oteryn:[A-Za-z0-9._:/-]+\Z')
SOURCE = re.compile(re.escape(SOURCE_PREFIX) + r'([^:\r\n]+):([1-9][0-9]*)(?:-([1-9][0-9]*))?\Z')
MAX_CALLBACKS = 511  # The runtime adds its existing dynamic Rotten Blood altar.
MAX_EFFECTS = 8


def require(condition, message):
    if not condition:
        raise ValueError(message)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate JSON field: ' + key)
        result[key] = value
    return result


def read(path):
    try:
        return json.loads(path.read_text(encoding='utf-8'), object_pairs_hook=unique_object)
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f'{path}: {error}') from error


def encoded(value):
    # Keep mapping insertion order as well as callback order. The existing
    # world-callbacks document uses this exact JSON encoding.
    return json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n'


def integer(value, minimum, maximum):
    return type(value) is int and minimum <= value <= maximum


def valid_key(value):
    return isinstance(value, str) and len(value) <= 128 and KEY.fullmatch(value) is not None


def fields(value, required, allowed, context):
    require(isinstance(value, dict), context + ': expected object')
    require(required <= value.keys() <= allowed, context + ': missing or unknown fields')


def position(value, context):
    fields(value, {'x', 'y', 'floor'}, {'x', 'y', 'floor'}, context)
    require(integer(value['x'], 0, 65535) and integer(value['y'], 0, 65535)
            and integer(value['floor'], -15, 0), context + ': invalid position')
    return value['x'], value['y'], value['floor']


def nullable_id(value, context):
    require(value is None or integer(value, 0, 65535), context + ': invalid u16')


def text_message(value, context):
    require(value is None or isinstance(value, str) and 0 < len(value.encode('utf-8')) <= 256
            and not any(character in value for character in ('\n', '\r', '\0')),
            context + ': invalid message')


def provenance(value, context, *, outfit=False):
    require(isinstance(value, list) and value, context + ': empty provenance')
    for reference in value:
        if outfit and isinstance(reference,str):
            pinned=re.fullmatch(r'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:(data-global/[a-zA-Z0-9_./-]+\.lua):sha256:[a-f0-9]{64}',reference)
            if pinned:
                require('..' not in pinned.group(1).split('/'),context + ': invalid source path')
                continue
        match = SOURCE.fullmatch(reference) if isinstance(reference, str) else None
        require(match is not None, context + ': callback source must use the exact Crystal pin and line')
        source_path, first, last = match.groups()
        require(not source_path.startswith(('/', '\\')) and '\\' not in source_path
                and '..' not in source_path.split('/') and source_path.endswith('.lua'),
                context + ': invalid source path')
        require(last is None or int(first) <= int(last), context + ': reversed source lines')


def validate_native(callback, context):
    rule = callback.get('native_rule')
    if rule is None:
        return
    require(isinstance(rule, dict), context + ': native_rule must be an object')
    kind = rule.get('kind')
    require(isinstance(kind, str) and kind in NATIVE_TRACKS, context + ': unknown native rule')
    parameter, low, high, owner = (
        ('symbol', 0, 12, QUEST_OWNERS['cults_of_tibia']) if kind == 'cults_symbol'
        else ('tile', 1, 9, QUEST_OWNERS['unnatural_selection'])
    )
    fields(rule, {'kind', parameter}, {'kind', parameter}, context + ': native_rule')
    require(integer(rule[parameter], low, high) and callback['quest'] == owner,
            context + ': invalid native identifier or owner')
    storage = callback['storage']
    require(len(storage) == 3 and {entry['track'] for entry in storage} == set(NATIVE_TRACKS[kind])
            and all(entry.get('preserve') is True for entry in storage),
            context + ': native rule requires exactly three preserved tracks')
    binding = NATIVE_BINDINGS[kind]
    expected = binding['positions'][rule[parameter] - low]
    selector = callback['selector']
    require(callback['trigger'] == binding['trigger']
            and selector.get('position') == dict(zip(('x', 'y', 'floor'), expected))
            and selector.get('action') == binding['action']
            and selector.get('unique') is None and selector.get('appearance') is None,
            context + ': native selector differs from pinned source')
    by_track = {entry['track']: entry for entry in storage}
    require(all((by_track[track]['minimum'], by_track[track]['maximum']) == bounds
                for track, bounds in zip(NATIVE_TRACKS[kind], binding['ranges'])),
            context + ': native guard differs from pinned source')
    require(any(source.startswith(SOURCE_PREFIX + binding['source']) for source in callback['sources']),
            context + ': native rule source file differs')
    require(all(callback.get(field) is None for field in (
        'lever', 'teleport', 'minimum_level', 'experience', 'monk_reward',
        'item_on_target', 'failure_message', 'success_message', 'facing',
    )) and not callback['reversible'] and not callback['return_on_failure']
        and not callback['passages'] and not callback['offerings'] and not callback.get('effects'),
        context + ': native rule has unrelated effects or feedback')


def validate_effects(callback, context):
    """The optional Channel effects share the Rust world_callbacks closed shape."""
    legacy_facing = callback.get('facing')
    require(legacy_facing is None or integer(legacy_facing, 0, 3)
            and callback.get('teleport') is not None, context + ': invalid facing')
    effects = callback.get('effects', {})
    if isinstance(effects, list):
        require(len(effects) <= MAX_EFFECTS, context + ': invalid effects')
        for effect in effects:
            fields(effect, {'position','effect'}, {'position','effect'}, context + ': effect')
            position(effect['position'], context + ': effect')
            require(integer(effect['effect'],1,65535), context + ': invalid effect')
        require(callback.get('native_rule') is None or not effects, context + ': native rule has unrelated effects')
        effects = {}
    allowed = {'transform', 'facing', 'magic', 'portals', 'item_cycle', 'encounter', 'outfit_reward'}
    fields(effects, set(), allowed, context + ': effects')
    require(legacy_facing is None or effects.get('facing') is None,
            context + ': conflicting destination facings')
    outfit = effects.get('outfit_reward')
    if outfit is not None:
        fields(outfit, {'outfit','addons'}, {'outfit','addons'}, context + ': outfit reward')
        require(isinstance(outfit['outfit'],str) and len(outfit['outfit'])<=160
                and re.fullmatch(r'oteryn:outfit\.[a-z0-9_]+',outfit['outfit']) is not None
                and integer(outfit['addons'],0,3), context + ': invalid outfit reward')
        require(callback['trigger']=='use' and callback['storage']
                and any(not s.get('preserve',False) and not s['minimum']<=s['set']<=s['maximum']
                        for s in callback['storage']), context + ': repeatable outfit reward')
    transform = effects.get('transform')
    require(transform is None or integer(transform, 1, 65535), context + ': invalid transform')
    facing = effects.get('facing')
    require(facing is None or facing in ('north', 'east', 'south', 'west')
            and callback.get('teleport') is not None, context + ': invalid destination facing')
    require(callback['trigger'] != 'step_out' or not callback['return_on_failure'],
            context + ': stepout cannot return to the departed tile')
    require(callback.get('native_rule') is None or all(value is None for value in effects.values()),
            context + ': native rule has unrelated effects')
    magic = effects.get('magic')
    if magic is not None:
        fields(magic, {'position', 'effect'}, {'position', 'effect'}, context + ': magic')
        position(magic['position'], context + ': magic')
        require(integer(magic['effect'], 1, 65535), context + ': invalid magic effect')
    operations = sum(effects.get(field) is not None for field in ('portals', 'item_cycle', 'encounter'))
    require(operations <= 1 and (operations == 0 or callback['trigger'] == 'use'
            and callback.get('lever') is not None and not callback['storage']
            and not callback['passages'] and callback.get('teleport') is None),
            context + ': incompatible map effects')
    pair = effects.get('portals')
    if pair is not None:
        fields(pair, {'appearance', 'portals'}, {'appearance', 'portals', 'lifetime_us'}, context + ': pair')
        require(integer(pair['appearance'], 1, 65535)
                and (pair.get('lifetime_us') is None or integer(pair['lifetime_us'], 1, 2**64-1)),
                context + ': invalid portal appearance/lifetime')
        require(isinstance(pair['portals'], list) and len(pair['portals']) == 2, context + ': paired endpoints required')
        positions = []
        for portal in pair['portals']:
            fields(portal, {'position', 'destination'}, {'position', 'destination'}, context + ': portal')
            positions.append(position(portal['position'], context + ': portal'))
            position(portal['destination'], context + ': destination')
        require(positions[0] != positions[1], context + ': duplicate portal endpoint')
    cycle = effects.get('item_cycle')
    if cycle is not None:
        fields(cycle, {'position', 'remove', 'create'}, {'position', 'remove', 'create'}, context + ': cycle')
        position(cycle['position'], context + ': cycle')
        require(integer(cycle['remove'], 1, 65535) and integer(cycle['create'], 1, 65535)
                and cycle['remove'] != cycle['create'], context + ': invalid item cycle')
    encounter = effects.get('encounter')
    if encounter is not None:
        names = {'minimum_level', 'players', 'monsters', 'room_min', 'room_max'}
        fields(encounter, names, names, context + ': encounter')
        require(integer(encounter['minimum_level'], 1, 2**32-1) and not callback['reversible'],
                context + ': invalid encounter level/reversal')
        lower = position(encounter['room_min'], context + ': room min')
        upper = position(encounter['room_max'], context + ': room max')
        require(lower[2] == upper[2] and lower[0] <= upper[0] and lower[1] <= upper[1],
                context + ': invalid encounter room')

        def in_room(at):
            return at[2] == lower[2] and lower[0] <= at[0] <= upper[0] and lower[1] <= at[1] <= upper[1]

        for name in ('players', 'monsters'):
            require(isinstance(encounter[name], list) and 1 <= len(encounter[name]) <= MAX_EFFECTS,
                    context + ': invalid encounter ' + name)
        origins, destinations, monsters = set(), set(), set()
        for player in encounter['players']:
            fields(player, {'from', 'to'}, {'from', 'to'}, context + ': member')
            origin = position(player['from'], context + ': staging tile')
            destination = position(player['to'], context + ': landing tile')
            require(origin not in origins and destination not in destinations
                    and not in_room(origin) and in_room(destination), context + ': invalid encounter member')
            origins.add(origin)
            destinations.add(destination)
        for monster in encounter['monsters']:
            fields(monster, {'position', 'creature'}, {'position', 'creature'}, context + ': monster')
            at = position(monster['position'], context + ': spawn tile')
            require(at not in monsters and at not in destinations and in_room(at)
                    and valid_key(monster['creature']), context + ': invalid encounter spawn')
            monsters.add(at)


def validate_callback(callback, directory):
    context = directory + ' callback'
    fields(callback, CALLBACK_REQUIRED, CALLBACK_FIELDS, context)
    key = callback['key']
    require(valid_key(key) and key.startswith('oteryn:quest-callback/'), context + ': foreign or invalid key')
    require(QUEST_OWNERS[directory] is not None and callback['quest'] == QUEST_OWNERS[directory],
            key + ': foreign quest owner or owner not qualified')
    require(callback['trigger'] in ('use', 'step_in', 'step_out'), key + ': invalid trigger')
    selector = callback['selector']
    selector_fields = {'position', 'unique', 'action', 'appearance', 'native'}
    fields(selector, set(), selector_fields, key + ': selector')
    require(any(selector.get(field) is not None for field in ('position', 'unique', 'action')),
            key + ': selector requires position, unique or action')
    if selector.get('position') is not None:
        position(selector['position'], key + ': selector')
    for field in ('unique', 'action', 'appearance'):
        nullable_id(selector.get(field), key + ': selector ' + field)
    require(type(callback.get('requires_world_admission', False)) is bool,
            key + ': invalid world admission')
    native = selector.get('native')
    if native is not None:
        fields(native, {'position', 'ordinal', 'appearance'},
               {'position', 'ordinal', 'appearance'}, key + ': native selector')
        position(native['position'], key + ': native selector')
        require(integer(native['ordinal'], 0, 255)
                and integer(native['appearance'], 1, 65535), key + ': invalid native selector')
    for field in ('reversible', 'return_on_failure'):
        require(type(callback[field]) is bool, key + ': invalid ' + field)
    lever = callback.get('lever')
    if lever is not None:
        require(isinstance(lever, list) and len(lever) == 2 and lever[0] != lever[1]
                and all(integer(value, 0, 65535) for value in lever) and callback['trigger'] == 'use',
                key + ': invalid lever')
    if callback.get('teleport') is not None:
        position(callback['teleport'], key + ': teleport')
    if callback.get('facing') is not None:
        require(integer(callback['facing'], 0, 3) and callback.get('teleport') is not None,
                key + ': invalid facing')
    for field in ('passages', 'storage', 'offerings'):
        require(isinstance(callback[field], list) and len(callback[field]) <= MAX_EFFECTS,
                key + ': invalid or excessive ' + field)
    seen = set()
    for storage in callback['storage']:
        fields(storage, {'track', 'minimum', 'maximum', 'set'},
               {'track', 'minimum', 'maximum', 'set', 'preserve', 'increment'}, key + ': storage')
        require(valid_key(storage['track']) and storage['track'] not in seen, key + ': duplicate or invalid track')
        seen.add(storage['track'])
        require(all(integer(storage[field], -(2 ** 63), 2 ** 63 - 1) for field in ('minimum', 'maximum', 'set'))
                and storage['minimum'] <= storage['maximum'], key + ': invalid storage range')
        require(type(storage.get('preserve', False)) is bool, key + ': invalid preserve')
        require(type(storage.get('increment', False)) is bool, key + ': invalid increment')
        if storage.get('increment', False):
            require(storage['set'] > 0 and not storage.get('preserve', False)
                    and callback.get('native_rule') is None
                    and storage['maximum'] <= 2 ** 63 - 1 - storage['set'], key + ': invalid increment')
    seen = set()
    for passage in callback['passages']:
        fields(passage, {'position', 'appearance'}, {'position', 'appearance', 'replacement', 'create_on_close'}, key + ': passage')
        location = position(passage['position'], key + ': passage')
        require(location not in seen and integer(passage['appearance'], 0, 65535), key + ': invalid or duplicate passage')
        seen.add(location)
        nullable_id(passage.get('replacement'), key + ': passage replacement')
        create = passage.get('create_on_close')
        if create is not None:
            require(integer(create, 1, 65535) and create != passage['appearance']
                    and passage.get('replacement') is None and len(callback['passages']) == 1
                    and callback.get('lever') is not None and callback['reversible']
                    and not callback['storage'] and callback.get('teleport') is None
                    and callback.get('item_on_target') is None and not callback['offerings']
                    and not callback.get('effects') and callback.get('experience') is None
                    and callback.get('monk_reward') is None and callback.get('success_message') is None,
                    key + ': invalid asymmetric wall')
    seen = set()
    for offering in callback['offerings']:
        fields(offering, {'position', 'item', 'count'}, {'position', 'item', 'count'}, key + ': offering')
        location = position(offering['position'], key + ': offering')
        require(location not in seen and valid_key(offering['item']) and integer(offering['count'], 1, 100)
                and bool(callback['storage']), key + ': invalid or duplicate offering')
        seen.add(location)
    item = callback.get('item_on_target')
    if item is not None:
        fields(item, {'item', 'count'}, {'item', 'count', 'replacement', 'target_replacement', 'reusable', 'alternatives', 'on_step'}, key + ': item_on_target')
        automatic = item.get('on_step', False)
        require(type(automatic) is bool, key + ': invalid automatic STEP input')
        trigger_matches = (callback['trigger'] == 'use' and not automatic) or (
            callback['trigger'] == 'step_in' and automatic
            and callback.get('requires_world_admission') is True
            and callback['selector'].get('native') is not None
            and not item.get('reusable', False) and item.get('replacement') is None
            and item.get('target_replacement') is None)
        require(valid_key(item['item']) and integer(item['count'], 1, 100)
                and bool(callback['storage']) and trigger_matches, key + ': invalid item_on_target')
        if item.get('replacement') is not None:
            require(valid_key(item['replacement']) and item['count'] == 1, key + ': invalid item replacement')
        require(type(item.get('reusable', False)) is bool, key + ': invalid reusable')
        require(not item.get('reusable', False) or item['count'] == 1
                and item.get('replacement') is None, key + ': invalid reusable replacement')
        alternatives = item.get('alternatives', [])
        require(isinstance(alternatives, list) and len(alternatives) <= 3
                and all(valid_key(value) and value != item['item'] for value in alternatives)
                and len(set(alternatives)) == len(alternatives), key + ': invalid alternatives')
        nullable_id(item.get('target_replacement'), key + ': target replacement')
        require(item.get('target_replacement') != 0, key + ': zero target replacement')
    if callback.get('minimum_level') is not None:
        require(integer(callback['minimum_level'], 1, 2 ** 32 - 1) and bool(callback['storage']), key + ': invalid minimum_level')
    if callback.get('experience') is not None:
        require(integer(callback['experience'], 1, 100_000_000), key + ': invalid experience')
    require(callback.get('monk_reward') is None, key + ': Monk rewards are outside batch four')
    text_message(callback.get('failure_message'), key + ': failure_message')
    text_message(callback.get('success_message'), key + ': success_message')
    provenance(callback['sources'], key, outfit=isinstance(callback.get('effects'), dict) and callback['effects'].get('outfit_reward') is not None)
    validate_native(callback, key)
    validate_effects(callback, key)


def source_only(value, context):
    if isinstance(value, dict):
        for key, nested in value.items():
            if key == 'runtime_admission':
                require(nested is False, context + ': runtime_admission must be false')
            source_only(nested, context + '/' + key)
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            source_only(nested, context + '/' + str(index))


def select_quests(root, selected=None, available=False):
    """Choose an explicit scope without weakening the default whole-batch gate."""
    require(type(available) is bool, 'available must be a boolean')
    require(not (available and selected is not None), 'selected and available are mutually exclusive')
    directory = Path(root) / INPUT_DIRECTORY
    actual = {path.name for path in directory.glob('*.json')}
    expected = {name + '.json' for name in QUEST_ORDER}
    if selected is None and not available:
        require(actual == expected, f'batch four requires exactly {len(QUEST_ORDER)} named inputs; missing=' +
                ','.join(sorted(expected - actual)) + '; foreign=' + ','.join(sorted(actual - expected)))
        return QUEST_ORDER
    require(actual <= expected, 'foreign batch-four input names: ' + ','.join(sorted(actual - expected)))
    if available:
        require(actual, 'available mode requires at least one named input')
        requested = {Path(name).stem for name in actual}
    else:
        require(isinstance(selected, (list, tuple)) and selected
                and all(isinstance(name, str) for name in selected),
                'selected must be a nonempty list or tuple of quest names')
        require(all(name in QUEST_ORDER for name in selected), 'selected contains a foreign quest name')
        require(len(selected) == len(set(selected)), 'selected contains duplicate quest names')
        requested = set(selected)
        missing = {name + '.json' for name in requested} - actual
        require(not missing, 'missing selected inputs: ' + ','.join(sorted(missing)))
    # Input argument order and filesystem enumeration must not change emission.
    return tuple(name for name in QUEST_ORDER if name in requested)


def load_packets(root, selected=None, available=False):
    directory = Path(root) / INPUT_DIRECTORY
    names = select_quests(root, selected=selected, available=available)
    packets = []
    all_keys = set()
    for name in names:
        packet = read(directory / (name + '.json'))
        fields(packet, PACKET_FIELDS, PACKET_FIELDS, name)
        require(type(packet['schema']) is int and packet['schema'] == 1 and packet['quest_directory'] == name,
                name + ': schema or quest_directory mismatch')
        require(packet['source_repository_revision'] == SOURCE_REVISION, name + ': incorrect source repository revision')
        for field in ('callbacks', 'runtime_exclusions', 'step_inventory', 'preserved_claims'):
            require(isinstance(packet[field], list) and all(isinstance(row, dict) for row in packet[field]),
                    name + ': ' + field + ' must be an object list')
        own_keys = set()
        for callback in packet['callbacks']:
            validate_callback(callback, name)
            key = callback['key']
            require(key not in all_keys, 'duplicate batch callback key: ' + key)
            own_keys.add(key)
            all_keys.add(key)
        excluded = set()
        for exclusion in packet['runtime_exclusions']:
            fields(exclusion, {'key', 'reason', 'question'}, {'key', 'reason', 'question'}, name + ': exclusion')
            key = exclusion['key']
            require(isinstance(key, str) and key in own_keys and key not in excluded,
                    name + ': duplicate or unmatched exclusion key')
            require(all(isinstance(exclusion[field], str) and exclusion[field].strip() for field in ('reason', 'question')),
                    name + ': exclusion needs reason and question')
            excluded.add(key)
        npc = packet['npc_source']
        require(isinstance(npc, dict) and npc.get('runtime_admission') is False,
                name + ': NPC source requires runtime_admission false')
        source_only(npc, name + ': npc_source')
        packets.append(packet)
    return packets


def expected_files(root, selected=None, available=False):
    root = Path(root)
    packets = load_packets(root, selected=selected, available=available)
    seed = read(root / CALLBACK_OUTPUT)
    fields(seed, {'schema', 'callbacks'}, {'schema', 'callbacks'}, 'existing world callbacks')
    require(type(seed['schema']) is int and seed['schema'] == 1 and isinstance(seed['callbacks'], list),
            'existing world callbacks: invalid schema')
    authored = {callback['key']: callback for packet in packets for callback in packet['callbacks']}
    existing_keys = set()
    retained = []
    for callback in seed['callbacks']:
        require(isinstance(callback, dict) and isinstance(callback.get('key'), str), 'invalid existing callback')
        key = callback['key']
        require(key not in existing_keys, 'duplicate existing callback key: ' + key)
        existing_keys.add(key)
        if key in authored:
            require(callback.get('quest') == authored[key]['quest'], 'foreign existing callback key: ' + key)
        else:
            retained.append(copy.deepcopy(callback))
    merged = copy.deepcopy(seed)
    merged['callbacks'] = retained
    outputs = {}
    for packet in packets:
        excluded = {row['key'] for row in packet['runtime_exclusions']}
        merged['callbacks'].extend(copy.deepcopy(callback) for callback in packet['callbacks'] if callback['key'] not in excluded)
        outputs[NPC_OUTPUT_DIRECTORY / (packet['quest_directory'] + '.json')] = encoded(packet['npc_source'])
    require(len(merged['callbacks']) <= MAX_CALLBACKS, f'callback cap is {MAX_CALLBACKS}, reserving one dynamic altar')
    merged['callbacks'].sort(key=lambda callback: callback['key'])
    outputs[CALLBACK_OUTPUT] = encoded(merged)
    return outputs


def generate(root, check=False, selected=None, available=False):
    """Validate the entire chosen scope before touching generated destinations."""
    root = Path(root)
    outputs = expected_files(root, selected=selected, available=available)
    changed = [path for path, text in outputs.items()
               if not (root / path).exists() or (root / path).read_bytes() != text.encode('utf-8')]
    if check:
        require(not changed, 'stale batch-four outputs: ' + ', '.join(path.as_posix() for path in changed))
        return []
    for relative in changed:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(outputs[relative], encoding='utf-8', newline='\n')
    return [path.as_posix() for path in changed]


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument('--check', action='store_true')
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument('--quests', nargs='+', metavar='QUEST', help='regenerate only these allowlisted quest inputs')
    scope.add_argument('--available', action='store_true', help='regenerate all present allowlisted inputs (at least one)')
    args = parser.parse_args(argv)
    try:
        names = select_quests(args.root, selected=args.quests, available=args.available)
        changed = generate(args.root, check=args.check, selected=args.quests, available=args.available)
    except (OSError, ValueError) as error:
        print('quest_batch4: ' + str(error), file=sys.stderr)
        return 1
    print(f'quest_batch4: {len(names)} source packets validated; {len(changed)} generated files changed')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
