"""Capture W19 novelty item families and costume appearances from Crystal 201189f."""
import argparse
import hashlib
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

from event_item_actions import PIN, ITEMS, table

DEST = 'tools/content-schema/native-gameplay/novelty-item-actions.json'


def capture(crystal):
    sources = {}

    def read(path):
        raw = (crystal / path).read_bytes()
        sources[path] = hashlib.sha256(raw).hexdigest()
        return raw.decode()

    constants_path = 'src/utils/utils_definitions.hpp'
    constants = {name: int(value) for name, value in re.findall(r'(CONST_ME_\w+)\s*=\s*(\d+)', read(constants_path))}
    xml = ET.fromstring(read('data/items/items.xml'))
    xml_items = {int(item.attrib['id']): item for item in xml.findall('item') if 'id' in item.attrib}
    map_item_ids = set()
    for path in [ITEMS + 'ice_flower.lua', ITEMS + 'ice_flower_seeds.lua',
                 ITEMS + 'blueberry_bush.lua', 'data/scripts/movements/seeds.lua']:
        raw = read(path)
        map_item_ids.update(int(value) for value in re.findall(r'(?::(?:id|transform|addItem)|Game\.createItem)\((\d+)', raw))
        map_item_ids.update(int(value) for value in re.findall(r'\.itemid\s*==\s*(\d+)', raw))
    map_action_item_ids = set(map_item_ids)
    horse_station = read('data-global/scripts/globalevents/others/horse_station.lua')
    horse_return_ids = {int(value) for value in re.findall(r'horseStable:id\((\d+)\)', horse_station)}
    if horse_return_ids != {31159}:
        raise ValueError('HORSE_RETURN_FLOOR_SOURCE_DRIFT')
    map_item_ids.update(horse_return_ids)
    die = read(ITEMS + 'die.lua')
    dice_ranges = re.search(r'Game\.getSpectators\(position,\s*false,\s*true,\s*(\d+),\s*(\d+)\)', die)
    game_functions = read('src/lua/functions/core/game/game_functions.cpp')
    spectators = read('src/map/spectators.cpp')
    map_constants = read('src/map/map_const.hpp')
    client_y = re.search(r'MAP_MAX_CLIENT_VIEW_PORT_Y\s*=\s*(\d+)', map_constants)
    server_y = re.search(r'MAP_MAX_VIEW_PORT_Y\s*=\s*MAP_MAX_CLIENT_VIEW_PORT_Y\s*\+\s*(\d+)', map_constants)
    if (dice_ranges is None or client_y is None or server_y is None
            or any(value not in game_functions for value in
                   ('Lua::getNumber<int32_t>(L, 6, 0)', 'Lua::getNumber<int32_t>(L, 7, 0)'))
            or any(value not in spectators for value in
                   ('minRangeY = (minRangeY == 0 ? -MAP_MAX_VIEW_PORT_Y : -minRangeY);',
                    'maxRangeY = (maxRangeY == 0 ? MAP_MAX_VIEW_PORT_Y : maxRangeY);'))):
        raise ValueError('DICE_SPECTATOR_SOURCE_DRIFT')
    dice_y = int(client_y[1]) + int(server_y[1])
    dice_speech_range = [int(dice_ranges[1]), int(dice_ranges[2]), dice_y, dice_y]
    for item in list(map_item_ids):
        element = xml_items.get(item)
        if element is not None:
            attrs = {a.get('key'): a.get('value') for a in element.findall('attribute')}
            if int(attrs.get('decayTo', '0')) > 0:
                target = int(attrs['decayTo'])
                map_item_ids.add(target)
                if item in map_action_item_ids:
                    map_action_item_ids.add(target)
    balloon_path = ITEMS + 'anniversary_balloons.lua'
    balloons, balloon_line = table(read(balloon_path), 'settings')
    trumpet = xml.find("item[@id='6573']")
    if trumpet is None:
        raise ValueError('TRUMPET_DECAY_SOURCE_MISSING')
    trumpet_attrs = {a.get('key'): a.get('value') for a in trumpet.findall('attribute')}
    trumpet_duration = int(trumpet_attrs['duration']) * 1000
    trumpet_decay = int(trumpet_attrs['decayTo'])
    costume_path = ITEMS + 'costume_bags.lua'
    costumes, line = table(read(costume_path), 'config')
    wanted = {name for row in costumes.values() for name in row.values()}
    looks = {}
    for path in sorted((crystal / 'data-global/monster').rglob('*.lua')):
        raw = path.read_text()
        name = re.search(r'Game\.createMonsterType\("([^"]+)"\)', raw)
        if name is None or name[1].lower() not in wanted:
            continue
        outfit = re.search(r'monster\.outfit\s*=\s*{(.*?)}', raw, re.S)
        look = re.search(r'lookType\s*=\s*(\d+)', outfit[1]) if outfit else None
        if look is None:
            raise ValueError(f'COSTUME_LOOK_TYPE_MISSING:{path}')
        relative = path.relative_to(crystal).as_posix()
        read(relative)
        looks[name[1].lower()] = {'monster': name[1].lower(), 'look_type': int(look[1]),
                                 'donor': f'{relative}:{raw[:outfit.start()].count(chr(10)) + 1}'}
    if wanted != set(looks):
        raise ValueError('COSTUME_MONSTER_SOURCE_MISSING:' + ','.join(sorted(wanted - set(looks))))
    rules = [{'item_ids': [item], 'donor': f'{costume_path}:{line}', 'kind': 'costume',
              'costumes': [looks[name] for name in values.values()],
              'duration_ms': 5 * 60 * 10 * 1000,
              'cues': ['appearance:effect/magic_red'],
              'achievement_progress': {'key': 'oteryn:achievement/masquerader', 'target': 100}}
             for item, values in costumes.items()]
    families = [
        ('die.lua', {'item_ids': list(range(5792, 5798)), 'kind': 'dice',
                     'dice_speech_range': dice_speech_range,
                     'cues': ['appearance:effect/craps']}),
        ('fireworks_rocket.lua', {'item_ids': [6576], 'kind': 'firework',
                                  'cues': ['appearance:effect/firearea', 'appearance:effect/hitbyfire'],
                                  'ground_cues': ['appearance:effect/firework_yellow', 'appearance:effect/firework_red', 'appearance:effect/firework_blue'],
                                  'speech': 'Ouch! Rather place it on the ground next time.',
                                  'damage': 10,
                                  'secondary_achievement_progress': {'key': 'oteryn:achievement/rocket_in_pocket', 'target': 3},
                                  'achievement_progress': {'key': 'oteryn:achievement/fireworks_in_the_sky', 'target': 250}}),
        ('party_trumpet.lua', {'item_ids': [6572], 'kind': 'transform', 'cue_origin': 'wire_source', 'output': 6573,
                               'duration_ms': trumpet_duration, 'decay_to': trumpet_decay,
                               'cues': ['appearance:effect/sound_blue'], 'speech': 'TOOOOOOT!'}),
        ('party_hat.lua', {'item_ids': [6578], 'kind': 'equipped_effect',
                           'cues': ['appearance:effect/gift_wraps'],
                           'achievement_progress': {'key': 'oteryn:achievement/party_animal', 'target': 200}}),
        ('explosive_present.lua', {'item_ids': [906], 'kind': 'consume_effect',
                                   'cues': ['appearance:effect/firearea'], 'speech': 'KABOOOOOOOOOOM!',
                                   'achievement': 'oteryn:achievement/joke_s_on_you'}),
        ('voodoo_doll.lua', {'item_ids': [3002], 'kind': 'player_target_effect',
                            'success_percent': 5, 'cues': ['appearance:effect/drawblood'],
                            'success_text': 'You concentrate on your victim and hit the needle in the doll.',
                            'failure_text': 'You concentrate on your victim, hit the needle in the doll.......but nothing happens.',
                            'achievement': 'oteryn:achievement/dark_voodoo_priest'}),
    ]
    for name, row in families:
        path = ITEMS + name
        read(path)
        row['donor'] = path + ':3'
        rules.append(row)
    for row in rules:
        row['effect_ids'] = [constants['CONST_ME_' + cue.rsplit('/', 1)[1].upper()] for cue in row.get('cues', [])]
        if 'ground_cues' in row:
            row['ground_effect_ids'] = [constants['CONST_ME_' + cue.rsplit('/', 1)[1].upper()] for cue in row['ground_cues']]
    rules.sort(key=lambda row: row['item_ids'][0])
    return {'schema': 'OTERYN_NOVELTY_ITEM_ACTIONS/v1', 'crystal_commit': PIN,
            'sources': [{'path': path, 'sha256': digest} for path, digest in sorted(sources.items())],
            'map_item_ids': sorted(map_item_ids),
            'map_action_item_ids': sorted(map_action_item_ids),
            'house_transforms': [{'item_id': item, 'target': target, 'donor': f'{balloon_path}:{balloon_line}'}
                                 for item, target in sorted(balloons.items())],
            'rules': rules}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = args.root / DEST
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('NOVELTY_ITEM_ACTIONS_DRIFT')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
