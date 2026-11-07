"""Capture source-backed House decorations; never execute donor Lua or invent identities."""
import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

from event_item_actions import PIN, table

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, bind_profiles, binding_index

CATALOGUE = 'tools/content-schema/native-gameplay/house-decoration-actions.json'
PROFILES = 'tools/content-schema/native-gameplay/house-decoration-profiles.json'
SQL = 'apps/game-server/migrations/1161_house_decoration_source_pairs.sql'
OBJECTS = 'data/scripts/actions/objects/'
KINDS = ('ROTATE', 'WRAP', 'USE', 'EQUIP', 'DEEQUIP', 'MALE', 'FEMALE', 'DESTROY', 'DECAY', 'WRITE_ONCE')
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'


def encode(document):
    return (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()


def capture(crystal, canary):
    sources = {}

    def read(path):
        raw = (crystal / path).read_bytes()
        sources[path] = hashlib.sha256(raw).hexdigest()
        return raw.decode()

    def literal(path, name):
        raw = read(path)
        values, line = table(raw if 'local ' + name in raw else 'local ' + raw, name)
        return raw, values, f'{path}:{line}'

    _, rows, donor = literal(OBJECTS + 'carpets.lua', 'carpetItems')
    carpets = [{'item_id': item, 'target': target, 'donor': donor}
               for item, target in sorted(rows.items())]
    _, rows, donor = literal('data/libs/tables/windows.lua', 'windowTable')
    windows = {}
    # windows.lua:33-44 checks all closedWindow entries before any openWindow.
    for row in rows.values():
        windows.setdefault(row['closedWindow'], row['openWindow'])
    for row in rows.values():
        windows.setdefault(row['openWindow'], row['closedWindow'])
    windows = [{'item_id': item, 'target': target, 'donor': donor}
               for item, target in sorted(windows.items())]
    read(OBJECTS + 'windows.lua')
    raw, rows, donor = literal(OBJECTS + 'wall_mirror.lua', 'setting')
    mirror = {'item_ids': list(map(int, re.search(r'wallMirror:id\(([^)]+)\)', raw)[1].split(','))),
              'speech': list(rows.values()), 'cooldown_ms': 72_000_000,
              'exhausted_speech': "Don't be so vain about your appearance.", 'donor': donor}
    if '20 * 60 * 60' not in raw:
        raise ValueError('HOUSE_MIRROR_COOLDOWN_SOURCE_DRIFT')
    constants = read('data/libs/functions/constants.lua')
    bathtub_ids = {name: int(value) for name, value in re.findall(r'(BATHTUB_\w+)\s*=\s*(\d+)', constants)}
    read(OBJECTS + 'bath_tub_drain.lua')
    bathtub = {'item_id': bathtub_ids['BATHTUB_FILLED'], 'target': bathtub_ids['BATHTUB_EMPTY'],
               'donor': OBJECTS + 'bath_tub_drain.lua:3-13'}
    enums = read('src/utils/utils_definitions.hpp')
    effect_ids = {name: int(value) for name, value in re.findall(r'(CONST_ME_\w+)\s*=\s*(\d+)', enums)}

    def effect(symbol):
        return {'effect_id': effect_ids[symbol],
                'effect_binding': 'canary.appearance:effect/' + symbol.removeprefix('CONST_ME_').lower()}

    _, rows, donor = literal('data-global/scripts/actions/other/music.lua', 'MusicEffect')
    music = [{'item_id': item, **effect(symbol), 'donor': donor,
              'achievement_progress': {'key': 'oteryn:achievement/rockstar', 'target': 10000}}
             for item, symbol in sorted(rows.items())]
    read('data-crystal/scripts/actions/other/birdcage.lua')
    birdcage = {'item_id': 2976, 'target': 2975, 'success_out_of': 100,
                **effect('CONST_ME_SOUND_YELLOW'), 'donor': 'data-crystal/scripts/actions/other/birdcage.lua:3-12'}
    read('data-global/scripts/actions/other/fish_tank.lua')
    fish_tank = {'item_id': 23691, 'effect_id': 175,
                 'effect_binding': 'canary.appearance:effect/plunging_fish',
                 'donor': 'data-global/scripts/actions/other/fish_tank.lua:3-8'}
    raw, rows, donor = literal('data-global/scripts/actions/other/magic_tree.lua', 'itemInfos')
    if 'local exhaustDelaySeconds = 1' not in raw:
        raise ValueError('HOUSE_TREE_COOLDOWN_SOURCE_DRIFT')
    trees = [{'item_id': item, 'target': values[1], 'storage': values[2],
              'cooldown_seconds': 1, 'donor': donor} for item, values in sorted(rows.items())]
    _, rows, donor = literal('data/scripts/actions/items/bed_modification_kits.lua', 'setting')
    bed_kits = [{'item_id': item, 'south': list(values[1].values()),
                 'east': list(values[2].values()), **effect('CONST_ME_POFF'),
                 'donor': donor} for item, values in sorted(rows.items())]
    xml_raw = read('data/items/items.xml').encode()
    xml = source.xml_index(xml_raw)
    read('src/items/items.cpp')
    read('src/items/item.hpp')
    engine = read('src/game/game.cpp')
    if 'const ItemType &newType = Item::items[newId];\n\tif (newType.id == 0) {\n\t\treturn item;' not in engine:
        raise ValueError('HOUSE_TRANSFORM_INVALID_TARGET_SOURCE_DRIFT')
    furniture = (ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/furniture_catalog.rs').read_text()
    pending = {int(n) for row in re.findall(r'\((\d+), (\d+)(?:, (\d+))?\)', furniture) for n in row if n}
    line_ids = {}
    for line, text in enumerate(xml_raw.decode().splitlines(), 1):
        match = re.search(r'<item id="(\d+)"', text)
        if match:
            line_ids[int(match[1])] = line
        else:
            match = re.search(r'<item fromid="(\d+)" toid="(\d+)"', text)
            if match:
                line_ids.update((item, line) for item in range(int(match[1]), int(match[2]) + 1))
    native_transforms, bed_rotations, scanned = [], [], set()
    while pending:
        item = min(pending)
        pending.remove(item)
        if item in scanned or item not in xml:
            continue
        scanned.add(item)
        attrs = source.attrs(xml[item])
        if attrs.get('type') in ('door', 'magicfield', 'teleport'):
            continue
        for name, kind in [('rotateto', 'ROTATE'), ('transformonuse', 'USE')]:
            if name in attrs and int(attrs[name]) != item:
                target = int(attrs[name])
                row = {'item_id': item, 'kind': kind, 'target': target,
                       'donor': f'data/items/items.xml:{line_ids[item]}'}
                (bed_rotations if attrs.get('type') == 'bed' else native_transforms).append(row)
                pending.add(target)
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    bindings = binding_index(BINDINGS.read_bytes())
    qualified = {item for item in xml if item in objects and str(item) in bindings}
    unavailable = [{**row, 'runtime_noop': True, 'engine_donor': 'src/game/game.cpp:3054-3057'}
                   for row in native_transforms + bed_rotations if row['target'] not in qualified]
    native_transforms = [row for row in native_transforms if row['target'] in qualified]
    native_transforms.extend({**row, 'requested_target': row['target'], 'target': row['item_id'], 'noop': True}
                             for row in unavailable)
    bed_item_ids = sorted(item for item in qualified if source.attrs(xml[item]).get('type') == 'bed')
    catalogue = {'schema': 'OTERYN_HOUSE_DECORATION_ACTIONS/v1', 'crystal_commit': PIN,
                 'sources': [{'path': path, 'sha256': digest} for path, digest in sorted(sources.items())],
                 'carpets': carpets, 'windows': windows, 'mirror': mirror, 'bathtub': bathtub,
                 'music': music, 'birdcage': birdcage, 'fish_tank': fish_tank, 'magic_trees': trees,
                 'bed_kits': bed_kits, 'bed_item_ids': bed_item_ids,
                 'native_transforms': sorted(native_transforms, key=lambda row: (row['item_id'], row['kind'])),
                 'unavailable_native_transforms': unavailable}
    ids = set(mirror['item_ids']) | {bathtub['item_id'], bathtub['target'], birdcage['item_id'], birdcage['target'], fish_tank['item_id'], 26100}
    ids.update(row['item_id'] for row in music)
    for row in carpets + windows + trees + native_transforms:
        ids.update((row['item_id'], row['target']))
    for row in bed_kits:
        ids.update((row['item_id'], *row['south'], *row['east']))
    ids.update((2491, 5501, 15506, 2489, 5499, 15508))
    ids.update(bed_item_ids)
    if ids - qualified:
        raise ValueError('HOUSE_RECIPE_UNAVAILABLE_INPUTS:' + str(sorted(ids - qualified)))
    physical = {row['server_item_id']: row for row in source.policies(appearance_raw, xml_raw, ids)}
    records = [source.profile(item, xml[item], objects[item], physical[item], bindings, None) for item in sorted(ids)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, ids,
                          independent_crystal_source=(PIN, xml_raw))
    # Crystal items.cpp:203-208 and item.hpp:553-555 establish auto-carpet eligibility.
    top = {str(item): next((order for flag, order in (('clip', 1), ('top', 3), ('bottom', 2))
                           if obj['flags'].get('flags.' + flag, False)), 0) for item, obj in objects.items()}
    profiles = {'schema': 'OTERYN_HOUSE_DECORATION_PROFILES/v1', 'crystal_commit': PIN,
                'canary_commit': CANARY,
                'catalogue_sha256': hashlib.sha256(encode(catalogue)).hexdigest(),
                'appearance_sha256': hashlib.sha256(appearance_raw).hexdigest(),
                'sources': [{'reference': 'crystal', 'revision': PIN, 'path': 'data/items/items.xml',
                             'sha256': hashlib.sha256(xml_raw).hexdigest()},
                            {'reference': 'canary', 'revision': CANARY, 'path': 'data/items/appearances.dat',
                             'sha256': hashlib.sha256(appearance_raw).hexdigest()}],
                'always_on_top': top, 'records': bound['records']}
    return catalogue, profiles


def apply_top_order(records, overlay):
    """Crystal items.cpp:203-208 source facts survive richer specialized profiles."""
    for row in records:
        item = row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i')
        if item in overlay['always_on_top']:
            top = overlay['always_on_top'][item]
            row['attributes']['always_on_top'] = top != 0
            row['attributes']['always_on_top_order'] = top


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PROFILES).read_bytes())
    if (overlay['schema'] != 'OTERYN_HOUSE_DECORATION_PROFILES/v1'
            or overlay['catalogue_sha256'] != hashlib.sha256((root / CATALOGUE).read_bytes()).hexdigest()):
        raise ValueError('HOUSE_DECORATION_PROFILE_SOURCE_DRIFT')
    catalogue = json.loads((root / CATALOGUE).read_bytes())
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key'] for row in document['records']}
    document['records'].extend(copy.deepcopy(row) for row in overlay['records']
                               if row['production_definition']['production_key'] not in existing)
    apply_top_order(document['records'], overlay)
    for row in document['records']:
        key = row['production_definition']['production_key']
        item = key.removeprefix('oteryn:item.tibia.i')
        if not item.isdecimal():
            continue
        matches = [entry for entry in catalogue['native_transforms'] if entry['item_id'] == int(item)]
        if matches:
            current = row['semantics']['use_transform']
            targets = (copy.deepcopy(current['value']['targets']) if current['state'] == 'KNOWN'
                       else [{'kind': kind, 'target': copy.deepcopy(source.U)} for kind in KINDS])
            for entry in matches:
                destination = next(value for value in targets if value['kind'] == entry['kind'])
                qualified = source.K({'key': f"oteryn:item.tibia.i{entry['target']}", 'revision': 'definition-r1'})
                if destination['target']['state'] == 'KNOWN' and destination['target'] != qualified:
                    raise ValueError('HOUSE_DECORATION_KNOWN_TRANSFORM_CONFLICT:' + key)
                destination['target'] = qualified
            row['semantics']['use_transform'] = source.K({'targets': targets})
    document['records'].sort(key=order)
    raw = encode(document)
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


def sql_pairs(catalogue):
    base = {(row['item_id'], row['target']) for field in ('windows', 'native_transforms') for row in catalogue[field]}
    pairs = base | {(row['item_id'], row['target']) for field in ('carpets', 'magic_trees') for row in catalogue[field]}
    pairs.update((row['item_id'], row['item_id']) for row in catalogue['carpets'])
    pairs.add((catalogue['bathtub']['item_id'], catalogue['bathtub']['target']))
    bird = catalogue['birdcage']
    pairs.update(((bird['item_id'], bird['target']), (bird['item_id'], bird['item_id'])))
    identity = set(catalogue['mirror']['item_ids']) | {catalogue['fish_tank']['item_id']}
    identity.update(row['item_id'] for row in catalogue['music'])
    identity.update(row['item_id'] for row in catalogue['bed_kits'])
    pairs.update((item, item) for item in identity)
    for orientation, originals in [('south', (2491, 5501, 15506)), ('east', (2489, 5499, 15508))]:
        heads = set(originals) | {row[orientation][0] for row in catalogue['bed_kits']}
        tails = {row[orientation][1] for row in catalogue['bed_kits']}
        for row in catalogue['bed_kits']:
            pairs.update((item, row[orientation][0]) for item in heads)
            pairs.update((item, row[orientation][1]) for item in tails)
    text = '-- Generated by tools/content-migration/house_decoration_actions.py. Do not edit.\n'
    text += '-- W18 exact donor file:line locators and input hashes: house-decoration-actions.json.\n'
    for function, values in [('game_house_decoration_pair', pairs), ('game_house_base_decoration_pair', base)]:
        rows = ',\n'.join(f"  ('oteryn:item.tibia.i{a}','oteryn:item.tibia.i{b}')" for a, b in sorted(values))
        text += (f'CREATE OR REPLACE FUNCTION {function}(p_source TEXT,p_target TEXT)\n'
                 'RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$\n'
                 ' SELECT EXISTS(SELECT 1 FROM (VALUES\n' + rows +
                 '\n ) AS pairs(source,target) WHERE source=p_source AND target=p_target)\n$$;\n'
                 f'REVOKE ALL ON FUNCTION {function}(TEXT,TEXT) FROM PUBLIC,oteryn_game_control;\n'
                 f'GRANT EXECUTE ON FUNCTION {function}(TEXT,TEXT) TO oteryn_game_runtime;\n')
    bed_pairs = set()
    for kit in catalogue['bed_kits']:
        for orientation, originals in [('south', (2491, 5501, 15506)), ('east', (2489, 5499, 15508))]:
            head, tail = kit[orientation]
            heads = set(originals) | {row[orientation][0] for row in catalogue['bed_kits']}
            bed_pairs.update((kit['item_id'], item, head) for item in heads)
            bed_pairs.update((kit['item_id'], item, tail) for item in catalogue['bed_item_ids'])
    rows = ',\n'.join(f"  ('oteryn:item.tibia.i{k}','oteryn:item.tibia.i{a}','oteryn:item.tibia.i{b}')"
                       for k, a, b in sorted(bed_pairs))
    function = 'game_house_bed_kit_pair'
    text += (f'CREATE OR REPLACE FUNCTION {function}(p_kit TEXT,p_source TEXT,p_target TEXT)\n'
             'RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT EXISTS(SELECT 1 FROM (VALUES\n' + rows +
             '\n ) AS pairs(kit,source,target) WHERE kit=p_kit AND source=p_source AND target=p_target)\n$$;\n'
             f'REVOKE ALL ON FUNCTION {function}(TEXT,TEXT,TEXT) FROM PUBLIC,oteryn_game_control;\n'
             f'GRANT EXECUTE ON FUNCTION {function}(TEXT,TEXT,TEXT) TO oteryn_game_runtime;\n')
    return text.encode()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    catalogue, profiles = capture(args.crystal, args.canary)
    for path, raw in ((CATALOGUE, encode(catalogue)), (PROFILES, encode(profiles)), (SQL, sql_pairs(catalogue))):
        target = ROOT / path
        if args.check:
            if target.read_bytes() != raw:
                raise SystemExit('HOUSE_DECORATION_SOURCE_DRIFT:' + path)
        else:
            target.write_bytes(raw)
