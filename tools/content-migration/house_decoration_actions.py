"""Capture source-backed House decorations; never execute donor Lua or invent identities."""
import argparse
import copy
import hashlib
import json
import re
import sys
from pathlib import Path

from event_item_actions import PIN, table
from compact_record_json import encode

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


def initialized_item_ids(raw):
    """Crystal items.cpp:156-169,226: only explicit id+flags initialize ItemType."""
    initialized = set()
    for number, object_raw in source.engine_items.protobuf_fields(raw):
        if number != 1:
            continue
        fields = dict(source.engine_items.protobuf_fields(object_raw))
        if fields.get(1, 0) and 3 in fields:
            initialized.add(fields[1])
    return initialized


def native_item_specs(xml, objects, initialized, carpet_type, known_xml_types):
    """Read-only C++ item facts, independent of Item materialization admission."""
    grouped = {}
    # Crystal items.cpp:203-208,216-217,232-244; item_parse.cpp:117-136,
    # 210-212,798-803 applies the source XML overrides after initialization.
    for item in sorted(initialized):
        flags = objects[item]['flags']
        attributes = source.attrs(xml[item]) if item in xml else {}
        order = next((value for flag, value in (('clip', 1), ('top', 3), ('bottom', 2))
                      if flags.get('flags.' + flag, False)), 0)
        carpet = flags.get('market.category', 0) == carpet_type
        if attributes.get('type', '').lower() in known_xml_types:
            carpet = attributes['type'].lower() == 'carpet'
        movable = not flags.get('flags.unmove', False)
        solid = bool(flags.get('flags.unpass', False))
        # pugixml::xml_attribute::as_bool reads the first character.
        if 'movable' in attributes:
            movable = attributes['movable'][:1] in ('1', 't', 'T', 'y', 'Y')
        if 'blocking' in attributes:
            solid = attributes['blocking'][:1] in ('1', 't', 'T', 'y', 'Y')
        facts = (carpet, movable, order != 0, order, solid,
                 bool(flags.get('flags.has_height', False)))
        grouped.setdefault(facts, []).append(item)
    fields = ('carpet', 'movable', 'always_on_top', 'always_on_top_order',
              'block_solid', 'has_height')
    return [{'item_ids': ids, **dict(zip(fields, facts))} for facts, ids in sorted(grouped.items())]


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
    wrap_parser = read('src/items/functions/item/item_parse.cpp')
    bed_engine = read('src/items/bed.cpp')
    if ('it.transformToOnUse[player->getSex()] != 0' not in bed_engine
            or 'if (newType.type == ITEM_TYPE_BED)' not in bed_engine):
        raise ValueError('HOUSE_BED_APPEARANCE_SOURCE_DRIFT')
    if 'itemType.wrapableTo = pugi::cast<int32_t>(valueAttribute.value());\n\t\titemType.wrapable = true;' not in wrap_parser:
        raise ValueError('HOUSE_WRAP_XML_SOURCE_DRIFT')
    item_definitions = read('src/items/items_definitions.hpp')
    item_parse_header = read('src/items/functions/item/item_parse.hpp')
    enum_body = re.search(r'enum ItemTypes_t\s*\{(.*?)\};', item_definitions, re.S)[1]
    enum_body = re.sub(r'//[^\n]*', '', enum_body)
    ordinal = -1
    carpet_type = None
    for entry in enum_body.split(','):
        match = re.fullmatch(r'\s*(ITEM_TYPE_\w+)(?:\s*=\s*(\d+))?\s*', entry)
        if not match:
            continue
        ordinal = int(match[2]) if match[2] else ordinal + 1
        if match[1] == 'ITEM_TYPE_CARPET':
            carpet_type = ordinal
            break
    if carpet_type is None:
        raise ValueError('HOUSE_CARPET_CPP_TYPE_MISSING')
    known_xml_types = set(re.findall(r'\{\s*"([a-z]+)"\s*,\s*ITEM_TYPE_', item_parse_header))
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
        # Crystal items.xml:67147-67173,67218-67244: aqueduct phases can
        # rotate after decay too. Harvest USE/ROTATE from the entire phase graph.
        decay_target = int(attrs.get('decayto', '0'))
        if decay_target > 0:
            pending.add(decay_target)
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
    crystal_appearance_raw = (crystal / 'data/items/appearances.dat').read_bytes()
    initialized = initialized_item_ids(appearance_raw) & initialized_item_ids(crystal_appearance_raw)
    qualified = {item for item in xml if item in initialized and str(item) in bindings}
    unavailable = [{**row, 'runtime_noop': True, 'engine_donor': 'src/game/game.cpp:3054-3057'}
                   for row in native_transforms + bed_rotations if row['target'] not in qualified]
    native_transforms = [row for row in native_transforms if row['target'] in qualified]
    native_transforms.extend({**row, 'requested_target': row['target'], 'target': row['item_id'], 'noop': True}
                             for row in unavailable)
    bed_item_ids = sorted(item for item in qualified if source.attrs(xml[item]).get('type') == 'bed')
    crystal_objects = source.engine_items.load_appearance_objects(crystal_appearance_raw)
    item_specs = native_item_specs(xml, crystal_objects,
                                   initialized_item_ids(crystal_appearance_raw),
                                   carpet_type, known_xml_types)
    wrap_eligible = []
    for item in sorted(qualified):
        attributes = source.attrs(xml[item])
        default = 23398 if any(crystal_objects[item]['flags'].get('flags.' + flag, False)
                              for flag in ('wrap', 'unwrap')) else 0
        wrapable_to = int(attributes.get('wrapableto', str(default)))
        if wrapable_to > 0:
            # Crystal item_parse.cpp:203-206 overrides the appearance default
            # from items.cpp:238-241. Game::wrapItem:4897-4899 emits23398.
            wrap_eligible.append({'item_id': item, 'source_wrapable_to': wrapable_to,
                                  'target': 23398,
                                  'donor': (f'data/items/items.xml:{line_ids[item]}' if 'wrapableto' in attributes
                                            else 'src/items/items.cpp:238-241'),
                                  'runtime_donor': 'src/game/game.cpp:4897-4899'})

    # Crystal Items::parseItemNode: wrapableTo is a native forward transform;
    # reverse unwrap still requires the owning prior-wrap/purchased-origin proof.
    wrap_pairs = []
    for item in sorted(qualified):
        target = int(source.attrs(xml[item]).get('wrapableto', '0'))
        if target > 0 and target in qualified:
            wrap_pairs.append({'item_id': item, 'target': target,
                               'donor': f'data/items/items.xml:{line_ids[item]}'})

    catalogue = {'schema': 'OTERYN_HOUSE_DECORATION_ACTIONS/v1', 'crystal_commit': PIN,
                 'sources': [{'path': path, 'sha256': digest} for path, digest in sorted(sources.items())],
                 'carpets': carpets, 'windows': windows, 'mirror': mirror, 'bathtub': bathtub,
                 'music': music, 'birdcage': birdcage, 'fish_tank': fish_tank, 'magic_trees': trees,
                 'bed_kits': bed_kits, 'bed_item_ids': bed_item_ids,
                 'bed_semantics_donors': ['src/items/functions/item/item_parse.cpp:397-398,733-766', 'src/items/bed.cpp:288-303'], 'wrap_pairs': wrap_pairs, 'wrap_eligible': [row['item_id'] for row in wrap_eligible],
                 'wrap_eligibility': wrap_eligible, 'native_item_specs': item_specs,
                 'native_transforms': sorted(native_transforms, key=lambda row: (row['item_id'], row['kind'])),
                 'unavailable_native_transforms': unavailable}
    ids = set(mirror['item_ids']) | {bathtub['item_id'], bathtub['target'], birdcage['item_id'], birdcage['target'], fish_tank['item_id'], 26100}
    ids.update(row['item_id'] for row in music)
    for row in carpets + windows + trees + native_transforms + wrap_pairs + wrap_eligible:
        ids.update((row['item_id'], row['target']))
    for row in bed_kits:
        ids.update((row['item_id'], *row['south'], *row['east']))
    ids.update((2491, 5501, 15506, 2489, 5499, 15508))
    ids.update(bed_item_ids)
    # Crystal XML:86273-86281 includes51222/51223, although carpets.lua has no
    # USE handlers for them. Auto-carpet is ItemType, not Lua action membership.
    ids.update(item for item in qualified if source.attrs(xml[item]).get('type') == 'carpet')
    # Crystal Items::parseItemNode's decayTo targets remain real Item references;
    # close the native policy graph rather than admitting only the active variants.
    pending = list(ids)
    while pending:
        item = pending.pop()
        attrs = source.attrs(xml[item])
        target = int(attrs.get('decayto', '0'))
        if target > 0 and target not in ids:
            if target not in qualified:
                raise ValueError(f'HOUSE_DECORATION_UNQUALIFIED_DECAY_TARGET:{item}:{target}')
            ids.add(target)
            pending.append(target)
    if ids - qualified:
        raise ValueError('HOUSE_RECIPE_UNAVAILABLE_INPUTS:' + str(sorted(ids - qualified)))
    physical = {row['server_item_id']: row for row in source.policies(appearance_raw, xml_raw, ids)}
    records = [source.profile(item, xml[item], objects[item], physical[item], bindings, None) for item in sorted(ids)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, ids,
                          independent_crystal_source=(PIN, xml_raw))
    apply_source_beds(bound['records'], xml, bindings)
    # Crystal items.cpp:203-208 and item.hpp:553-555 establish auto-carpet eligibility.
    top = {str(item): next((order for flag, order in (('clip', 1), ('top', 3), ('bottom', 2))
                           if obj['flags'].get('flags.' + flag, False)), 0) for item, obj in objects.items()}
    profiles = {'schema': 'OTERYN_HOUSE_DECORATION_PROFILES/v1', 'crystal_commit': PIN,
                'canary_commit': CANARY,
                'catalogue_sha256': hashlib.sha256(encode(catalogue)).hexdigest(),
                'appearance_sha256': hashlib.sha256(appearance_raw).hexdigest(),
                'sources': [{'reference': 'crystal', 'revision': PIN, 'path': 'data/items/items.xml',
                             'sha256': hashlib.sha256(xml_raw).hexdigest()},
                            {'reference': 'crystal', 'revision': PIN, 'path': 'data/items/appearances.dat',
                             'sha256': hashlib.sha256(crystal_appearance_raw).hexdigest()},
                            {'reference': 'canary', 'revision': CANARY, 'path': 'data/items/appearances.dat',
                             'sha256': hashlib.sha256(appearance_raw).hexdigest()}],
                'always_on_top': top, 'records': bound['records']}
    return catalogue, profiles


def apply_source_beds(records, xml, bindings):
    """Crystal item_parse.cpp:733-766 and bed.cpp:288-303, initialized BEDs only."""
    by_id = {int(row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i')): row
             for row in records}
    count = 0
    free = {}
    for source_item, node in xml.items():
        for attribute in node.findall('attribute'):
            key = attribute.get('key', '').lower()
            if key in ('maletransformto', 'femaletransformto'):
                target = int(attribute.get('value'))
                if free.get(target, 0) == 0:
                    free[target] = source_item
            elif key == 'transformto':
                free[source_item] = int(attribute.get('value'))
    for item, row in by_id.items():
        attributes = source.attrs(xml[item])
        if attributes.get('type') != 'bed':
            continue
        part = {'pillow': 'HEAD', 'blanket': 'FOOT'}.get(attributes.get('bedpart'))
        direction = {'north': 'NORTH', 'east': 'EAST', 'south': 'SOUTH', 'west': 'WEST'}.get(attributes.get('partnerdirection'))
        if part is None or direction is None:
            raise ValueError(f'HOUSE_BED_UNKNOWN_PART_OR_DIRECTION:{item}')
        # The parser populates the other sex only while that slot is zero.
        male = female = 0
        for attribute in xml[item].findall('attribute'):
            key = attribute.get('key', '').lower()
            if key == 'maletransformto':
                male = int(attribute.get('value'))
                if female == 0:
                    female = male
            elif key == 'femaletransformto':
                female = int(attribute.get('value'))
                if male == 0:
                    male = female
        def target(value):
            if not value:
                value = free.get(item, 0)
            if not value or value not in xml or source.attrs(xml[value]).get('type') != 'bed':
                value = item
            if value not in by_id:
                raise ValueError(f'HOUSE_BED_TARGET_NOT_CAPTURED:{item}:{value}')
            identity = bindings[str(value)]['target']
            return {'key': identity['key'], 'revision': identity['revision']}
        row['semantics']['bed'] = source.K({'part': part, 'partner_direction': direction,
                                          'occupied_male': target(male), 'occupied_female': target(female)})
        count += 1
    return count


def apply_bed_semantics(records, overlay):
    """Enrich an admitted identity in place; conflicting known semantics remain an error."""
    beds = {row['production_definition']['production_key']: row['semantics']['bed']
            for row in overlay['records'] if row['semantics'].get('bed', {}).get('state') == 'KNOWN'}
    for row in records:
        key = row['production_definition']['production_key']
        if key not in beds:
            continue
        current = row['semantics'].get('bed', source.U)
        if current['state'] == 'KNOWN' and current != beds[key]:
            raise ValueError('HOUSE_BED_SEMANTICS_CONFLICT:' + key)
        row['semantics']['bed'] = copy.deepcopy(beds[key])

def apply_top_order(records, overlay):
    """Crystal items.cpp:203-208 source facts survive richer specialized profiles."""
    from qready1_reward_profiles import IDS as reward_ids
    from authored_static_item_constructors import SOURCE as authored_source
    reward_ids = {str(value) for value in reward_ids}
    for row in records:
        item = row['production_definition']['production_key'].removeprefix('oteryn:item.tibia.i')
        if item in overlay['always_on_top']:
            top = overlay['always_on_top'][item]
            # QREADY1 owns these exact reward profiles. Crystal items.cpp:203-208's
            # zero default adds no House behavior in either base or SRX enrichment.
            if item in reward_ids:
                if top != 0:
                    raise ValueError('HOUSE_QREADY1_TOP_ORDER_CONFLICT')
                continue
            # First-party constructors bind their complete row independently.
            # A donor's zero rendering default adds no behavior to that owner.
            if row.get('production_binding', {}).get('source_key') == authored_source:
                if top != 0:
                    raise ValueError('HOUSE_AUTHORED_TOP_ORDER_CONFLICT')
                continue
            row['attributes']['always_on_top'] = top != 0
            row['attributes']['always_on_top_order'] = top


def apply_carpet_types(records, overlay):
    """Initialized Crystal XML carpet classification preserves richer existing facts."""
    carpets = {row['production_definition']['production_key']: row['semantics']['classification']
               for row in overlay['records'] if row['semantics']['classification'].get('value', {}).get('item_type') == source.K('CARPET')}
    for row in records:
        key = row['production_definition']['production_key']
        if key not in carpets:
            continue
        current = row['semantics']['classification']
        if current['state'] != 'KNOWN':
            row['semantics']['classification'] = copy.deepcopy(carpets[key])
            continue
        item_type = current['value']['item_type']
        if item_type['state'] == 'KNOWN' and item_type != source.K('CARPET'):
            raise ValueError('HOUSE_CARPET_TYPE_CONFLICT:' + key)
        current['value']['item_type'] = source.K('CARPET')


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
    apply_carpet_types(document['records'], overlay)
    apply_bed_semantics(document['records'], overlay)
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


def compact_values(rows):
    rows = [row.strip() for row in rows]
    return ',\n'.join('  ' + ','.join(rows[start:start + 8]) for start in range(0, len(rows), 8))


def sql_pairs(catalogue, profiles=None):
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
    for function, values in [('game_house_decoration_pair', pairs), ('game_house_base_decoration_pair', base),
                             ('game_house_native_wrap_pair', {(row['item_id'], row['target']) for row in catalogue['wrap_pairs']})]:
        rows = compact_values(f"('oteryn:item.tibia.i{a}','oteryn:item.tibia.i{b}')" for a, b in sorted(values))
        text += (f'CREATE OR REPLACE FUNCTION {function}(p_source TEXT,p_target TEXT)\n'
                 'RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$\n'
                 ' SELECT EXISTS(SELECT 1 FROM (VALUES\n' + rows +
                 '\n ) AS pairs(source,target) WHERE source=p_source AND target=p_target)\n$$;\n'
                 f'REVOKE ALL ON FUNCTION {function}(TEXT,TEXT) FROM PUBLIC,oteryn_game_control;\n'
                 f'GRANT EXECUTE ON FUNCTION {function}(TEXT,TEXT) TO oteryn_game_runtime;\n')
    eligible = compact_values(f"('oteryn:item.tibia.i{row['item_id']}')" for row in catalogue['wrap_eligibility'])
    text += ('CREATE OR REPLACE FUNCTION game_house_native_wrap_eligible(p_source TEXT)\n'
             'RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT EXISTS(SELECT 1 FROM (VALUES\n' + eligible +
             '\n ) AS eligible(source) WHERE source=p_source)\n$$;\n'
             'REVOKE ALL ON FUNCTION game_house_native_wrap_eligible(TEXT) FROM PUBLIC,oteryn_game_control;\n'
             'GRANT EXECUTE ON FUNCTION game_house_native_wrap_eligible(TEXT) TO oteryn_game_runtime;\n')
    spec_rows = []
    for spec in catalogue['native_item_specs']:
        item_ids = ','.join(map(str, spec['item_ids']))
        facts = {key: value for key, value in spec.items() if key != 'item_ids'}
        spec_rows.append("  (ARRAY[" + item_ids + "]::bigint[],'" + json.dumps(facts, separators=(',', ':')) + "'::jsonb)")
    text += ('CREATE OR REPLACE FUNCTION game_house_native_item_spec(p_item_id BIGINT)\n'
             'RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT spec FROM (VALUES\n' + compact_values(spec_rows) +
             '\n ) AS specs(item_ids,spec) WHERE p_item_id=ANY(item_ids)\n$$;\n'
             'REVOKE ALL ON FUNCTION game_house_native_item_spec(BIGINT) FROM PUBLIC,oteryn_game_control;\n'
             'GRANT EXECUTE ON FUNCTION game_house_native_item_spec(BIGINT) TO oteryn_game_runtime;\n')
    bed_pairs = set()
    for kit in catalogue['bed_kits']:
        for orientation, originals in [('south', (2491, 5501, 15506)), ('east', (2489, 5499, 15508))]:
            head, tail = kit[orientation]
            heads = set(originals) | {row[orientation][0] for row in catalogue['bed_kits']}
            bed_pairs.update((kit['item_id'], item, head) for item in heads)
            bed_pairs.update((kit['item_id'], item, tail) for item in catalogue['bed_item_ids'])
    rows = compact_values(f"('oteryn:item.tibia.i{k}','oteryn:item.tibia.i{a}','oteryn:item.tibia.i{b}')"
                       for k, a, b in sorted(bed_pairs))
    function = 'game_house_bed_kit_pair'
    text += (f'CREATE OR REPLACE FUNCTION {function}(p_kit TEXT,p_source TEXT,p_target TEXT)\n'
             'RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT EXISTS(SELECT 1 FROM (VALUES\n' + rows +
             '\n ) AS pairs(kit,source,target) WHERE kit=p_kit AND source=p_source AND target=p_target)\n$$;\n'
             f'REVOKE ALL ON FUNCTION {function}(TEXT,TEXT,TEXT) FROM PUBLIC,oteryn_game_control;\n'
             f'GRANT EXECUTE ON FUNCTION {function}(TEXT,TEXT,TEXT) TO oteryn_game_runtime;\n')
    rows = compact_values("('oteryn:item.tibia.i" + str(row['item_id']) + "','"
                       + json.dumps({'south': row['south'], 'east': row['east']}, separators=(',', ':'))
                       + "'::jsonb)" for row in catalogue['bed_kits'])
    text += ('CREATE OR REPLACE FUNCTION game_house_bed_kit_layout(p_kit TEXT)\n'
             'RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT layout FROM (VALUES\n' + rows + '\n ) AS kits(kit,layout) WHERE kit=p_kit\n$$;\n'
             'REVOKE ALL ON FUNCTION game_house_bed_kit_layout(TEXT) FROM PUBLIC,oteryn_game_control;\n'
             'GRANT EXECUTE ON FUNCTION game_house_bed_kit_layout(TEXT) TO oteryn_game_runtime;\n')
    if profiles is None:
        profiles = json.loads((ROOT / PROFILES).read_bytes())
    timing_by_key = {row['production_definition']['production_key']: row['semantics']['temporal']
                     for row in profiles['records']}
    static_rotation = set()
    for row in catalogue['native_transforms']:
        keys = {f"oteryn:item.tibia.i{row['item_id']}", f"oteryn:item.tibia.i{row['target']}"}
        if any(timing_by_key[key]['state'] == 'KNOWN' for key in keys):
            static_rotation.update(keys)
    pending = list(static_rotation)
    while pending:
        timing = timing_by_key[pending.pop()]
        if timing['state'] == 'KNOWN' and timing['value']['decay_target']['state'] == 'KNOWN':
            target = timing['value']['decay_target']['value']['key']
            if target not in static_rotation:
                static_rotation.add(target)
                pending.append(target)
    temporal = []
    for row in profiles['records']:
        timing = row['semantics']['temporal']
        if timing['state'] != 'KNOWN':
            continue
        value = timing['value']
        duration, target = value['duration'], value['decay_target']
        if duration['state'] == 'KNOWN' and target['state'] == 'KNOWN':
            temporal.append((row['production_definition']['production_key'],
                             {'duration_ms': duration['value'], 'target': target['value']['key'],
                              'static_rotation': row['production_definition']['production_key'] in static_rotation}))
    rows = compact_values("('" + key + "','" + json.dumps(spec, separators=(',', ':')) + "'::jsonb)"
                       for key, spec in sorted(temporal))
    text += ('CREATE OR REPLACE FUNCTION game_house_base_decay_spec(p_key TEXT)\n'
             'RETURNS JSONB LANGUAGE sql IMMUTABLE AS $$\n'
             ' SELECT spec FROM (VALUES\n' + rows + '\n ) AS timings(key,spec) WHERE key=p_key\n$$;\n'
             'REVOKE ALL ON FUNCTION game_house_base_decay_spec(TEXT) FROM PUBLIC,oteryn_game_control;\n'
             'GRANT EXECUTE ON FUNCTION game_house_base_decay_spec(TEXT) TO oteryn_game_runtime;\n')
    return text.encode()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    catalogue, profiles = capture(args.crystal, args.canary)
    for path, raw in ((CATALOGUE, encode(catalogue)), (PROFILES, encode(profiles)), (SQL, sql_pairs(catalogue, profiles))):
        target = ROOT / path
        if args.check:
            if target.read_bytes() != raw:
                raise SystemExit('HOUSE_DECORATION_SOURCE_DRIFT:' + path)
        else:
            target.write_bytes(raw)
