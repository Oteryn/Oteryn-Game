"""Hash-qualified house, location, vertical movement and outfit spell adapters.

Only typed domain operations are emitted. Lua remains source evidence, never an
executable payload. S21 prefers Canary when donor registrations disagree; the
Fandom distance-band correction in accepted B.3 P7 supersedes both donor scripts.
This module makes no claim that a downstream executor supports these operations.
"""
import copy
import hashlib
from pathlib import Path

REVISIONS = {'canary': '99902524e052f37574194466c2949c576e4ab269',
             'crystal': 'ff7ede593c69d4c658b382c97443e8155926924a'}
# Full-file SHA256s, not pattern-based script acceptance.
DIGESTS = {
    'house door list': '4b7c51c85b8c834ee90a54035ca43fe6260cf886f3fad6f00b5b54ed0e4d2536',
    'house guest list': '0f30d379edd60550b18750cb62cb00e81135a99081ce99073b4a7f30dd62a2a7',
    'house kick': '4d0216484db1e5a8e142105c1d07b4b25eb5299a75edbc83e96d221968b68f05',
    'house subowner list': 'de049763deb15cba09530da93666073152e55b4b7ecd0cb2b265dca3d26c8298',
    'creature illusion': 'ae37364d6449702486fa049356456d624e63bea3fe302d80424124b08108bd5f',
    'find fiend': '6343e2a6c5ba57b632a591d809839a448add731b562735d049644e379fde281d',
    'find person': '5635f5bc286cd958755d03f8049915c292e0d3f1e725b5091df02cfd0890f1d6',
    'levitate': '31f390d1162f9dbd5ff8638a5a90afb401f8d3ebf405f6870a7999c1829360a7',
    'magic rope': '914f6064468401a5f16948b7c69f97d46d7e4c5e33505ddbcd3c05b7b32d6dc5',
}
CRYSTAL_FIEND = 'dbf94210b858ae01e2b54667126d521ebf6f272a7fb7d1520b810c77a5672a65'
FILES = {name: 'data/scripts/spells/' + ('house/' if name.startswith('house ') else 'support/')
         + name.replace(' ', '_') + '.lua' for name in DIGESTS}

SUPPORT_FILES = {'creature_appearance': [],
 'house_access': ['src/map/house/house.cpp',
                  'src/map/map_definitions.hpp',
                  'src/creatures/combat/spells.cpp',
                  'src/game/game.cpp'],
 'locate_message': ['src/creatures/combat/spells.cpp',
                    'data/libs/systems/exaltation_forge.lua',
                    'src/lua/functions/map/position_functions.cpp'],
 'vertical_move': ['data/global.lua',
                   'data/libs/functions/tile.lua',
                   'data/libs/functions/position.lua']}
SUPPORT_DIGESTS = {('canary', 'data/global.lua'): '84e980d77d4bcab1a0f4a27751f8aea1bab575909ec548475e9de24d7cf9a575',
 ('canary', 'data/libs/functions/position.lua'): 'd933c6df3c0d0e0e974048bf0a58063f14d7f872adca2d5bb0250d6cdc261920',
 ('canary', 'data/libs/functions/tile.lua'): '78921505d34485a2087bfa050fc1852e0c2a45f1b5e36350306d33ed624f3fe9',
 ('canary', 'data/libs/systems/exaltation_forge.lua'): 'f207512b540b8da687c2dc61ffbe830aaa0722cc72ee6cb60de41f9da205d38c',
 ('canary', 'src/creatures/combat/spells.cpp'): '933cb7e46f2c83dc9ca03221b2db713655e24cbde21c7bd66d903fd0babf4fe8',
 ('canary', 'src/game/game.cpp'): 'b337fb7d7ce61d9ccde4f315cb696df0add9a0bf933c9b5c634b9ed79d7f01dd',
 ('canary', 'src/lua/functions/map/position_functions.cpp'): 'e60adf2eb7e6a59ebd97edca605769b87c313352ac0da4287dab1ae134f196f4',
 ('canary', 'src/map/house/house.cpp'): 'bf43d2360fff2aec75e88ba926d5feb571a7697ef19f7647b5f6af4f1854cc5d',
 ('canary', 'src/map/map_definitions.hpp'): '9735e529bb07dcd7fd90542b22a766016d67eef52489558e7c0e045e29968633',
 ('crystal', 'data/global.lua'): '6949be337ff3e7607af1141780b089083058146e8b5b268748482fb526311d5b',
 ('crystal', 'data/libs/functions/position.lua'): 'c18d16465394a4064e82d35463105511c1f273e7c6de588f29b9da8b1eadaf9b',
 ('crystal', 'data/libs/functions/tile.lua'): '0c9a116b5d34feb7a2fd0083c89dec7f07c525d6553f65a23367b9dacf49abb7',
 ('crystal', 'data/libs/systems/exaltation_forge.lua'): '2a13a58f92e3c20585502a368daa2b9aa4d524a45857c3783758b732c0d0a435',
 ('crystal', 'src/creatures/combat/spells.cpp'): '06906bc067b56d89fc3f1d6c0d30fc58d405425d4d00e0127f49a14b6e01385d',
 ('crystal', 'src/game/game.cpp'): '4b26d0c5168e0907ae9044b2587a37ba647f6b6651a94e02f1c12a2e3af58d45',
 ('crystal', 'src/lua/functions/map/position_functions.cpp'): '2e137cc7bb3a1dc8d0cccac9405c5f87c2c7d7efce52cdcb9acdedaeb0e9f937',
 ('crystal', 'src/map/house/house.cpp'): '9f563752993a705861151f1ba4c7ea29de8d09633340ebb4a35d6db4a07f077f',
 ('crystal', 'src/map/map_definitions.hpp'): '9cf30f306b94d2c7eff66f5a1ccf0736de394bbb2ef59b2a2fbe579b04bc4242'}


def _edit(which):
    return {'action': 'edit_list', 'list': which,
            'house_lookup': 'caster_tile', 'door_lookup': ['front', 'own_tile'] if which == 'door' else [],
            'require_can_edit': True, 'record_editor_session': True,
            'save_requires_matching_window_and_session': True, 'save_rechecks_access': True,
            'evict_uninvited_on_save': which != 'door',
            'denied_cast_succeeds': which != 'door', 'no_house_succeeds': False,
            'no_house_failure_silent': which != 'door', 'failure_effect': 'poff'}


def _locate(source):
    return {'source': source, 'distance_metric': 'chebyshev_xy', 'delta': 'caster_minus_target',
            'bands_tiles': [5, 101, 251], 'direction_tangents': [0.4142, 2.4142],
            'vertical_direction': 'positive_delta_is_higher', 'message_class': 'look',
            'success_effect': 'magic_blue', 'failure_effect': 'poff',
            'beside_messages': ['is below you', 'is standing next to you', 'is above you'],
            'close_messages': ['is on a lower level to the', 'is to the', 'is on a higher level to the'],
            'far_message': 'is far to the', 'very_far_message': 'is very far to the'}


PERSON = {**_locate('online_player'), 'parameter': 'player_name', 'parameter_length': [1, 29],
          'prefix_suffix': '~', 'case_insensitive': True, 'prefix_requires_unique_match': True,
          'hide_staff_from_nonstaff': True, 'respect_no_pvp_exiva_restrictions': True,
          'name_resolution_failure_starts_cooldown': True,
          'name_resolution_failure_spends_mana': False, 'message_template': '{name} {location}.'}
FIEND = {**_locate('nearest_fiendish'), 'nearest_metric': 'chebyshev_xyz',
         'same_floor_only': False, 'tie_break': 'source_traversal_order', 'require_live_target': True,
         'difficulty_requires_completed_bestiary': True,
         'difficulty_kill_upper_bounds': [25, 250, 500, 1000, 2500, 5000],
         'harmless_minimum_kills': 5,
         'difficulty_labels': ['Harmless', 'Trivial', 'Easy', 'Medium', 'Hard', 'Challenging'],
         'unknown_difficulty': 'Unknown', 'warning_below_minutes': 15,
         'warning_minutes_rounding': 'floor',
         'message_template': 'The monster {location}. Be prepared to find a creature of difficulty level "{difficulty}".',
         'warning_template': 'This monster will stay fiendish for less than {minutes} minutes and {seconds} seconds.',
         'warning_under_minute_template': 'This monster will stay fiendish for less than {seconds} seconds.',
         'no_target_message': 'At the moment there is no fiend with special loot roaming this world.'}
LEVITATE = {'mode': 'levitate', 'parameter': 'choice', 'choices': ['up', 'down'],
            'case_insensitive': True, 'blocked_floor_pairs': [[7, 8]],
            'up_probe': 'above_caster', 'down_probe': 'front_same_floor',
            'probe_requires_no_ground': True, 'up_probe_rejects': ['immovable_block_solid'],
            'down_probe_rejects': ['block_solid'], 'destination': 'front_adjacent_floor',
            'destination_requires_ground': True,
            'destination_rejects': ['immovable_block_solid', 'floor_change'],
            'move_ignore_block_items': True, 'move_ignore_block_creatures': True,
            'respect_entry_permissions': True, 'success_effect': 'teleport', 'failure_effect': 'poff'}
ROPE = {'mode': 'rope_up', 'require_rope_spot': True, 'requires_ground': True,
        'rope_ground_items': [386, 421, 7762, 12202, 12936, 14238, 17238, 23363, 21965, 21966, 21967, 21968],
        'rope_top_items': [12935], 'floor_delta': -1,
        'destination_order': ['south', 'north', 'east', 'west', 'west', 'south_west', 'south_east', 'north_west', 'north_east'],
        'walkable_requires_ground': True,
        'walkable_rejects': ['block_solid', 'block_projectile', 'immovable_block_solid', 'immovable_block_item', 'immovable_nonfield_block_item'],
        'ignore_creatures': True, 'ignore_floor_change': True, 'ignore_protection_zone': True,
        'fallback': 'south_if_tile_exists', 'respect_entry_permissions': True,
        'failed_teleport_counts_as_success': True,
        'before_check_effect': 'poff', 'success_effect': 'teleport',
        'no_rope_error': 'not_possible', 'missing_destination_error': 'not_enough_room'}
BEHAVIOURS = {name: {'key': 'house_access', 'parameters': _edit(name.split()[1])}
              for name in ('house door list', 'house guest list', 'house subowner list')}
BEHAVIOURS.update({
    'house kick': {'key': 'house_access', 'parameters': {
        'action': 'kick', 'parameter': 'player_name', 'parameter_length': [1, 29],
        'self_kick': True, 'self_kick_requires_house': True, 'self_kick_requires_access': False,
        'caster_requires_guest_list_edit': True, 'require_same_house': False,
        'require_caster_access_at_least_target': True, 'reject_target_edit_houses_flag': True,
        'destination': 'target_house_entry', 'failed_teleport_counts_as_success': True,
        'source_effect': 'poff', 'destination_effect': 'teleport', 'failure_effect': 'poff'}},
    'find person': {'key': 'locate_message', 'parameters': PERSON},
    'find fiend': {'key': 'locate_message', 'parameters': FIEND},
    'levitate': {'key': 'vertical_move', 'parameters': LEVITATE},
    'magic rope': {'key': 'vertical_move', 'parameters': ROPE},
    'creature illusion': {'key': 'creature_appearance', 'parameters': {
        'source': 'creature_name_parameter', 'case_insensitive': True, 'match': 'exact',
        'duration_ms': 180000, 'require_flag': 'illusionable', 'bypass_flag': 'can_illusion_all',
        'appearance': 'monster_type_outfit', 'replace_existing_outfit_condition': True,
        'unknown_creature_error': 'creature_does_not_exist', 'not_illusionable_error': 'not_possible',
        'failure_effect': 'poff', 'success_effect': 'magic_red'}},
})


def evidence(name, spell_type, records, texts):
    """Return the exact validated source identities or raise on donor drift."""
    name = name.lower()
    if spell_type != 'instant' or name not in BEHAVIOURS:
        return []
    result = []
    for source, record in sorted(records.items()):
        if (source not in REVISIONS or record.get('file') != FILES[name]
                or record.get('source') != source or str(record.get('name', '')).lower() != name):
            raise ValueError('unqualified house/movement source identity')
        if 'revision' in record and record['revision'] != REVISIONS[source]:
            raise ValueError('house/movement pinned revision mismatch')
        text = texts.get((source, record['file']))
        if not isinstance(text, str):
            raise ValueError('missing house/movement source text')
        raw = text.encode('utf-8')
        digest = hashlib.sha256(raw).hexdigest()
        expected = CRYSTAL_FIEND if source == 'crystal' and name == 'find fiend' else DIGESTS[name]
        blob = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if digest != expected or record.get('blob') != blob or record.get('spell_type') != 'instant':
            raise ValueError('changed hash-qualified house/movement implementation')
        root = record.get('source_root')
        closure = []
        for path in SUPPORT_FILES[BEHAVIOURS[name]['key']]:
            if root is None:
                raise ValueError('missing source_root for house/movement helper closure')
            raw_helper = (Path(root) / path).read_bytes()
            helper_digest = hashlib.sha256(raw_helper).hexdigest()
            if helper_digest != SUPPORT_DIGESTS[(source, path)]:
                raise ValueError('changed house/movement helper closure: ' + path)
            closure.append({'file': path, 'sha256': helper_digest})
        result.append({'source': source, 'revision': REVISIONS[source], 'file': record['file'],
                       'blob': blob, 'sha256': digest, 'authority': 'OtsHypothesisOnly', 'helper_closure': closure,
                       'selection': 'S21 Canary preferred on a donor conflict',
                       'conflicts': (['Canary spell:id 248; Crystal spell:id 20'] if name == 'find fiend' else []),
                       'overrides': (['B.3 P7 Fandom distance bands 5/101/251 supersede donor 5/101/275']
                                     if name.startswith('find ') else [])
                                    + (['B.1 Fandom completed Bestiary and no-fiend message supersede donor unlocked/No creatures around']
                                       if name == 'find fiend' else []),
                       'uncertainties': (['Equal-distance Fiend registry traversal order; no donor tie guarantee']
                                         if name == 'find fiend' else [])
                                        + (['B.2 QH1 access-level provider decides account/character ownership']
                                           if name.startswith('house ') else [])})
    if not result:
        raise ValueError('house/movement adapter requires source evidence')
    return result


def build(name, spell_type, records, texts):
    name = name.lower()
    if spell_type != 'instant' or name not in BEHAVIOURS:
        return None
    evidence(name, spell_type, records, texts)
    return copy.deepcopy(BEHAVIOURS[name])


def schemas():
    """Strict discriminated schemas; no opaque operation or extra parameter."""
    result = {}
    for behaviour in BEHAVIOURS.values():
        parameters = behaviour['parameters']
        shape = {'type': 'object', 'properties': {key: {'const': copy.deepcopy(value)}
                 for key, value in parameters.items()}, 'required': list(parameters), 'additionalProperties': False}
        result.setdefault(behaviour['key'], []).append(shape)
    return {key: {'oneOf': alternatives} for key, alternatives in result.items()}


def location_phrase(caster, target, parameters):
    """The extracted P7 operator, independently testable at band/direction edges."""
    dx, dy, dz = (a - b for a, b in zip(caster, target))
    distance = max(abs(dx), abs(dy))
    vertical = 2 if dz > 0 else 0 if dz < 0 else 1
    beside, close, far = parameters['bands_tiles']
    if distance < beside:
        return parameters['beside_messages'][vertical]
    tangent = dy / dx if dx else 10
    low, high = parameters['direction_tangents']
    if abs(tangent) < low:
        direction = 'west' if dx > 0 else 'east'
    elif abs(tangent) < high:
        direction = ('north-west' if dy > 0 else 'south-east') if tangent > 0 else ('south-west' if dx > 0 else 'north-east')
    else:
        direction = 'north' if dy > 0 else 'south'
    phrase = parameters['close_messages'][vertical] if distance < close else parameters['far_message'] if distance < far else parameters['very_far_message']
    return phrase + ' ' + direction


def fiend_difficulty(kills_required, completed, parameters=FIEND):
    if not completed:
        return parameters['unknown_difficulty']
    if parameters['harmless_minimum_kills'] <= kills_required <= parameters['difficulty_kill_upper_bounds'][0]:
        return parameters['difficulty_labels'][0]
    for upper, label in zip(parameters['difficulty_kill_upper_bounds'][1:], parameters['difficulty_labels'][1:]):
        if kills_required <= upper:
            return label
    return parameters['unknown_difficulty']


def nearest_fiend(caster, fiends):
    """Deterministic first-in-input ties; donor Lua sorting gives no tie guarantee."""
    live = [entry for entry in fiends if entry['alive'] and entry['fiendish']]
    return min(live, key=lambda entry: max(abs(a - b) for a, b in zip(caster, entry['position'])), default=None)


def can_edit_house_list(access_level, which):
    return access_level == 'owner' or access_level == 'subowner' and which == 'guest'


def house_list_cast(access_level, which, has_house=True, has_door=True):
    """Source success/cost boundary: denied guest/subowner editors still succeed."""
    if not has_house:
        return {'succeeds': False, 'open_editor': False}
    permitted = can_edit_house_list(access_level, which) and (which != 'door' or has_door)
    return {'succeeds': permitted or which != 'door', 'open_editor': permitted}


def house_kick_allowed(caster_house, target_house, is_self, caster_can_edit_guest,
                       caster_access, target_access, target_edit_houses=False):
    if is_self:
        return caster_house is not None
    rank = {'not_invited': 0, 'guest': 1, 'subowner': 2, 'owner': 3}
    return (caster_house is not None and target_house is not None and caster_can_edit_guest
            and rank[caster_access] >= rank[target_access] and not target_edit_houses)


def levitate_destination(caster, front_offset, choice, tiles):
    """Probe/destination logic; returned movement still requires entry admission."""
    choice = choice.lower()
    x, y, z = caster
    if choice not in ('up', 'down') or choice == 'up' and z == 8 or choice == 'down' and z == 7:
        return None
    fx, fy = x + front_offset[0], y + front_offset[1]
    probe = tiles.get((x, y, z - 1) if choice == 'up' else (fx, fy, z))
    rejected = 'immovable_block_solid' if choice == 'up' else 'block_solid'
    if probe is not None and (probe.get('ground', False) or probe.get(rejected, False)):
        return None
    destination = (fx, fy, z + (-1 if choice == 'up' else 1))
    tile = tiles.get(destination)
    if tile is None or not tile.get('ground', False) or tile.get('immovable_block_solid', False) or tile.get('floor_change', False):
        return None
    return destination


def rope_destination(caster, tiles):
    offsets = {'south': (0, 1), 'north': (0, -1), 'east': (1, 0), 'west': (-1, 0),
               'south_west': (-1, 1), 'south_east': (1, 1), 'north_west': (-1, -1), 'north_east': (1, -1)}
    x, y, z = caster
    origin = tiles.get(caster)
    if (origin is None or not origin.get('ground', False)
            or not (origin.get('ground_item') in ROPE['rope_ground_items']
                    or set(origin.get('top_items', [])) & set(ROPE['rope_top_items']))):
        return None
    for direction in ROPE['destination_order']:
        dx, dy = offsets[direction]
        position = (x + dx, y + dy, z - 1)
        tile = tiles.get(position)
        if tile and tile.get('ground', False) and not any(tile.get(flag, False) for flag in ROPE['walkable_rejects']):
            return position
    fallback = (x, y + 1, z - 1)
    return fallback if fallback in tiles else None
