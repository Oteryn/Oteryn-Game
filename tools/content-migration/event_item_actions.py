"""Capture W19 event-item outcome tables from the pinned Crystal revision.

This parser reads literal Lua tables only; it never executes donor code.
"""

import argparse
import hashlib
import json
import re
from pathlib import Path

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
DEST = 'tools/content-schema/native-gameplay/event-item-actions.json'
ITEMS = 'data/scripts/actions/items/'
TOKEN = re.compile(r'\s+|--[^\n]*|"(?:\\.|[^"\\])*"|\d+|[A-Za-z_][A-Za-z_0-9]*|[{}\[\]=,;]')


def table(raw, name):
    start = re.search(r'local\s+' + name + r'\s*=\s*{', raw)
    if start is None:
        raise ValueError(f'missing literal table {name}')
    tokens = [m.group() for m in TOKEN.finditer(raw[start.end() - 1:])
              if not m.group().isspace() and not m.group().startswith('--')]
    position = 0

    def value():
        nonlocal position
        token = tokens[position]
        position += 1
        if token == '{':
            rows = {}
            index = 1
            while tokens[position] != '}':
                if tokens[position] == '[':
                    position += 1
                    key = value()
                    if tokens[position:position + 2] != [']', '=']:
                        raise ValueError('invalid keyed Lua table')
                    position += 2
                elif tokens[position + 1] == '=':
                    key = tokens[position]
                    position += 2
                else:
                    key = index
                    index += 1
                if key in rows:
                    raise ValueError('duplicate literal Lua key')
                rows[key] = value()
                if tokens[position] in (',', ';'):
                    position += 1
            position += 1
            return rows
        if token.startswith('"'):
            return json.loads(token)
        if token.isdecimal():
            return int(token)
        return token

    return value(), raw[:start.start()].count('\n') + 1


def capture(crystal):
    sources = []
    rules = []
    enum_path = 'src/utils/utils_definitions.hpp'
    enum_raw = (crystal / enum_path).read_bytes()
    effect_ids = {name: int(value) for name, value in re.findall(
        rb'(CONST_ME_[A-Z_]+)\s*=\s*(\d+)', enum_raw)}
    sources.append({'path': enum_path, 'sha256': hashlib.sha256(enum_raw).hexdigest()})

    def read(path, name):
        raw = (crystal / path).read_bytes()
        sources.append({'path': path, 'sha256': hashlib.sha256(raw).hexdigest()})
        values, line = table(raw.decode(), name)
        return values, f'{path}:{line}'

    def reward(item, count=1, random_count=False, weight=1):
        return {'item_id': item, 'minimum': 1 if random_count else count,
                'maximum': count, 'weight': weight}

    def presentation(path, constant, origin, per_reward=False):
        raw = (crystal / path).read_text()
        calls = [f'{path}:{line}' for line, text in enumerate(raw.splitlines(), 1)
                 if ':sendMagicEffect(' + constant + ')' in text]
        if not calls or constant.encode() not in effect_ids:
            raise ValueError('EVENT_ITEM_EFFECT_SOURCE_MISSING')
        return {'cue': {'binding': 'appearance:effect/' + constant.removeprefix('CONST_ME_').lower(),
                        'effect_id': effect_ids[constant.encode()]},
                'origin': origin, 'per_reward': per_reward, 'donor': calls}

    def add(item, provenance, selection, rewards, cue, picks=1, roll_max=None,
            ground_output='character'):
        row = {'item_id': item, 'donor': provenance, 'selection': selection,
               'picks': picks, 'rewards': rewards, 'presentation': cue,
               'ground_output': ground_output}
        if roll_max is not None:
            row['roll_max'] = roll_max
        rules.append(row)

    for name, selection, picks, random_count in (
            ('surprise_bags', 'one', 1, True),
            ('christmas_bundle', 'without_replacement', 7, False)):
        entries, provenance = read(ITEMS + name + '.lua',
                                   'settings' if name == 'surprise_bags' else 'setting')
        for item, values in entries.items():
            rewards = [reward(v[1], v[2], random_count) if isinstance(v, dict)
                       else reward(v) for v in values.values()]
            cue = presentation(ITEMS + name + '.lua', 'CONST_ME_GIFT_WRAPS',
                               'wire_source' if name == 'surprise_bags' else 'player')
            add(item, provenance, selection, rewards, cue, picks)

    entries, provenance = read('data-global/scripts/actions/other/special_boxes.lua', 'config')
    for item, values in entries.items():
        cue = presentation('data-global/scripts/actions/other/special_boxes.lua',
                           'CONST_ME_PRISMATIC_SPARK', 'source_or_player')
        add(item, provenance, 'one', [reward(v) for v in values.values()], cue,
            ground_output='source')

    entries, provenance = read(ITEMS + 'anniversary_reward_boxes.lua', 'config')
    for item, values in entries.items():
        cue = presentation(ITEMS + 'anniversary_reward_boxes.lua',
                           'CONST_ME_PRISMATIC_SPARK', 'player', per_reward=True)
        add(item, provenance, 'all', [reward(v['id'], v['count'])
                                    for v in values.values()], cue)

    entries, provenance = read(ITEMS + 'surprise_music_box.lua', 'prize')
    constants = {'ITEM_GOLD_COIN': 3031, 'ITEM_PLATINUM_COIN': 3035,
                 'ITEM_CRYSTAL_COIN': 3043}
    add(12045, provenance, 'weighted',
        [reward(constants.get(v['id'], v['id']), v['amount'], weight=v['chance'])
         for v in entries.values()], presentation(ITEMS + 'surprise_music_box.lua',
            'CONST_ME_POFF', 'player'), roll_max=100)

    entries, provenance = read(ITEMS + 'glooth_bag.lua', 'config')
    add(21203, provenance, 'weighted',
        [reward(v['itemId'], v.get('count', 1),
                weight=v['chanceTo'] - v['chanceFrom'] + 1)
         for v in entries.values()], presentation(ITEMS + 'glooth_bag.lua',
            'CONST_ME_HITBYPOISON', 'item'), roll_max=10001)

    cake_path = 'data-global/scripts/lib/a_piece_of_cake_config.lua'
    cake_raw = (crystal / cake_path).read_bytes()
    sources.append({'path': cake_path, 'sha256': hashlib.sha256(cake_raw).hexdigest()})
    seasonal_inventory_items = []
    seasonal_map_items = []
    for target, names in (
            (seasonal_inventory_items, ('CakeCabinetKit', 'CakeBackpack', 'CakeTapestry',
             'CupcakeBlueberry', 'CupcakeStrawberry', 'CupcakeLemon', 'SweetAndSugarySubstance')),
            (seasonal_map_items, ('FloorId', 'RailId'))):
        for name in names:
            matches = re.findall(rb'^\s*' + name.encode() + rb'\s*=\s*(\d+),', cake_raw, re.M)
            if len(matches) != 1:
                raise ValueError('EVENT_ITEM_CAKE_SOURCE_MISSING:' + name)
            target.append(int(matches[0]))
    for name in ('CakeEdibleIds', 'UpperFloorCakeIds', 'CakeBorderIds', 'CakeBaseIds'):
        matches = re.findall(rb'^\s*' + name.encode() + rb'\s*=\s*{([^}]+)}', cake_raw, re.M)
        if len(matches) != 1 or not re.fullmatch(rb'[\s\d,]+', matches[0]):
            raise ValueError('EVENT_ITEM_CAKE_MAP_SOURCE_MISSING:' + name)
        seasonal_map_items.extend(int(value) for value in re.findall(rb'\d+', matches[0]))

    rules.sort(key=lambda row: row['item_id'])
    if len({row['item_id'] for row in rules}) != len(rules):
        raise ValueError('duplicate event action')
    return {'schema': 'OTERYN_EVENT_ITEM_REWARDS/v1', 'crystal_commit': PIN,
            'sources': sources,
            'seasonal_item_ids': sorted(set(seasonal_inventory_items + seasonal_map_items)),
            'seasonal_inventory_item_ids': sorted(set(seasonal_inventory_items)),
            'seasonal_map_item_ids': sorted(set(seasonal_map_items)), 'rules': rules}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = args.root / DEST
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('EVENT_ITEM_ACTIONS_DRIFT')
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)


if __name__ == '__main__':
    main()
