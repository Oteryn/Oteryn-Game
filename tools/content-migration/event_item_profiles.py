"""W19: source-derived native profiles for the event-action catalogue."""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
from event_item_identity import DECISION as BRIDGE_IDENTITY_DECISION, evidence as bridge_source_evidence, source_evidence
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from build_world_change_layers import BLOBS as WORLD_LAYER_BLOBS
from build_world_change_layers import PREFIX as WORLD_LAYER_PREFIX
from build_world_change_layers import REVISION as WORLD_LAYER_REVISION

PATH = 'tools/content-schema/native-gameplay/event-item-profiles.json'
ACTIONS = 'tools/content-schema/native-gameplay/event-item-actions.json'
NOVELTY = 'tools/content-schema/native-gameplay/novelty-item-actions.json'
COSMETICS = 'tools/content-schema/native-gameplay/item-cosmetic-actions.json'
WORLD_LAYERS = 'content/encounters/world-changes/layers.json'
# Existing DoorBook states for the two key-door families placed by Yasir's ships.
# Capture verifies these exact Crystal KeyDoorTable rows before admitting targets.
WORLD_DOOR_FAMILIES = ((4912, 5007, 4911), (4913, 5006, 4914))
WORLD_DOOR_SOURCE = 'data/libs/tables/doors.lua'
# Crystal's Cake bridge explicitly creates this appearance-only Ground item.
# Its own Item loader supplies physical facts before optional XML overrides.
# This is evidence only: ANSWER6549960114 does not admit a canonical reference.
APPEARANCE_ONLY = {48227}
CANONICAL_IDENTITY_UNAVAILABLE = {48227}


def world_layers(root):
    document = json.loads((root / WORLD_LAYERS).read_bytes())
    layers = document.get('layers', [])
    if (document.get('schema') != 'OTERYN_WORLD_CHANGE_LAYERS/v1'
            or document.get('source_revision') != WORLD_LAYER_REVISION
            or len(layers) != len(WORLD_LAYER_BLOBS)
            or {layer['key'] for layer in layers} != WORLD_LAYER_BLOBS.keys()
            or any(layer['source'] != WORLD_LAYER_PREFIX + layer['key'] + '.otbm'
                   or layer['blob'] != WORLD_LAYER_BLOBS[layer['key']] for layer in layers)):
        raise ValueError('EVENT_WORLD_LAYER_SOURCE_DRIFT')
    for layer in layers:
        for tile in layer['tiles']:
            for entry in tile['entries']:
                if len(entry) != 3 or type(entry[0]) is not int or not 0 < entry[0] <= 65535:
                    raise ValueError('EVENT_WORLD_LAYER_ITEM_ID')
    return layers


def world_layer_item_ids(root):
    return {entry[0] for layer in world_layers(root)
            for tile in layer['tiles'] for entry in tile['entries']}


def world_door_families(root):
    placed = world_layer_item_ids(root)
    return [family for family in WORLD_DOOR_FAMILIES if placed.intersection(family)]


def seasonal_item_sets(catalogue):
    if catalogue['schema'] != 'OTERYN_EVENT_ITEM_REWARDS/v1':
        raise ValueError('EVENT_ITEM_ACTION_SCHEMA')
    inventory = set(catalogue['seasonal_inventory_item_ids'])
    world_map = set(catalogue['seasonal_map_item_ids'])
    if inventory & world_map or sorted(inventory | world_map) != catalogue['seasonal_item_ids']:
        raise ValueError('EVENT_ITEM_SEASONAL_SOURCE_PARTITION')
    return inventory, world_map


def world_map_item_ids(root):
    novelty = json.loads((root / NOVELTY).read_bytes())
    if novelty['schema'] != 'OTERYN_NOVELTY_ITEM_ACTIONS/v1':
        raise ValueError('NOVELTY_ITEM_ACTION_SCHEMA')
    _, seasonal_map = seasonal_item_sets(json.loads((root / ACTIONS).read_bytes()))
    return (world_layer_item_ids(root) | set(novelty['map_item_ids']) | seasonal_map
            | {item for family in world_door_families(root) for item in family})


def item_action_ids(root):
    catalogue = json.loads((root / ACTIONS).read_bytes())
    seasonal_inventory, _ = seasonal_item_sets(catalogue)
    ids = {item for row in catalogue['rules']
           for item in [row['item_id'], *(r['item_id'] for r in row['rewards'])]}
    ids.update(seasonal_inventory)
    cosmetics = json.loads((root / COSMETICS).read_bytes())
    if cosmetics['schema'] != 'OTERYN_ITEM_COSMETICS/v1':
        raise ValueError('EVENT_ITEM_COSMETIC_SCHEMA')
    ids.update(row['item_id'] for row in cosmetics['rules'])
    ids.update(item for row in cosmetics['rules'] for item in row.get('required_items', []))
    novelty = json.loads((root / NOVELTY).read_bytes())
    if novelty['schema'] != 'OTERYN_NOVELTY_ITEM_ACTIONS/v1':
        raise ValueError('NOVELTY_ITEM_ACTION_SCHEMA')
    for row in novelty['rules']:
        ids.update(row['item_ids'])
        for field in ('output', 'decay_to'):
            if field in row:
                ids.add(row[field])
    if not set(novelty['map_action_item_ids']) <= set(novelty['map_item_ids']):
        raise ValueError('EVENT_ITEM_MAP_ACTION_SOURCE_PARTITION')
    # Seed/flower actions still require ordinary native policy for their existing
    # mutation and harvesting owners. The Horse return floor only supplies geometry.
    ids.update(novelty['map_action_item_ids'])
    for row in novelty['house_transforms']:
        ids.update((row['item_id'], row['target']))
    return ids


def item_ids(root):
    return item_action_ids(root) | world_map_item_ids(root)


def admission_sets(root, physical, unavailable_ids):
    world_map = world_map_item_ids(root) - unavailable_ids
    # Exact source pickupability requires ordinary item admission. Raw map facts
    # alone never grant MINT, inventory custody, weight or stack authority.
    pickupable = {item_id for item_id in world_map if physical[item_id]['pickupable']}
    native = (item_action_ids(root) | pickupable) - unavailable_ids
    return native, world_map - native


def capture(canary, crystal):
    # Crystal owns gameplay attributes; Canary supplies the matching client appearance flags.
    xml_raw = (crystal / 'data/items/items.xml').read_bytes()
    secondary_raw = (canary / 'data/items/items.xml').read_bytes()
    appearance_raw = (canary / 'data/items/appearances.dat').read_bytes()
    xml = source.xml_index(xml_raw)
    secondary = source.xml_index(secondary_raw)
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    layers = world_layers(ROOT)
    layer_sources = []
    layer_donors = {}
    for layer in layers:
        raw = (crystal / layer['source']).read_bytes()
        if hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest() != layer['blob']:
            raise ValueError('EVENT_WORLD_LAYER_BLOB_DRIFT:' + layer['source'])
        layer_sources.append({'path': 'crystal-201189f/' + layer['source'],
                              'sha256': hashlib.sha256(raw).hexdigest()})
        for item in {entry[0] for tile in layer['tiles'] for entry in tile['entries']}:
            layer_donors.setdefault(item, []).append(layer['source'] + '@' + layer['blob'])
    door_raw = (crystal / WORLD_DOOR_SOURCE).read_bytes()
    door_text = door_raw.decode()
    door_rows = {tuple(map(int, match.groups())): match.start() for match in re.finditer(
        r'\{\s*lockedDoor\s*=\s*(\d+),\s*closedDoor\s*=\s*(\d+),\s*openDoor\s*=\s*(\d+)\s*\}', door_text)}
    door_families = world_door_families(ROOT)
    if any(family not in door_rows for family in door_families):
        raise ValueError('EVENT_WORLD_DOOR_SOURCE_DRIFT')
    door_records = [{'locked': family[0], 'closed': family[1], 'open': family[2],
                     'donor': WORLD_DOOR_SOURCE + ':' + str(door_text[:door_rows[family]].count('\n') + 1)}
                    for family in door_families]
    ids = item_ids(ROOT)
    primary_ids = ids & xml.keys()
    secondary_ids = (ids - primary_ids) & secondary.keys()
    appearance_only_ids = (ids - primary_ids - secondary_ids) & APPEARANCE_ONLY
    identity_unavailable_ids = ids & CANONICAL_IDENTITY_UNAVAILABLE
    unavailable_ids = (ids - primary_ids - secondary_ids - appearance_only_ids) | identity_unavailable_ids
    qualified_ids = (primary_ids | secondary_ids | appearance_only_ids) - unavailable_ids
    identity_evidence = [source_evidence(canary, crystal)] if identity_unavailable_ids else []
    loader_sources = []
    if appearance_only_ids:
        loader = {path: (crystal / path).read_bytes() for path in
                  ('src/items/items.hpp', 'src/items/items.cpp')}
        if (any(value not in loader['src/items/items.hpp'].decode() for value in
                ('int32_t weight = 0;', 'uint16_t maxItems = 8;', 'uint8_t stackSize = 100;'))
                or any(value not in loader['src/items/items.cpp'].decode() for value in
                       ('ItemType &iType = items[object.id()];',
                        'else if (object.flags().has_bank())',
                        'iType.group = ITEM_GROUP_GROUND;',
                        'iType.speed = object.flags().has_bank() ? static_cast<uint16_t>(object.flags().bank().waypoints()) : 0;',
                        'iType.pickupable = object.flags().take();',
                        'iType.movable = object.flags().unmove() == false;',
                        'iType.stackable = object.flags().cumulative();'))):
            raise ValueError('EVENT_ITEM_LOADER_SOURCE_DRIFT')
        loader_sources = [{'path': 'crystal-201189f/' + path,
                           'sha256': hashlib.sha256(raw).hexdigest()}
                          for path, raw in loader.items()]
    physical = {row['server_item_id']: row
                for row in source.policies(appearance_raw, xml_raw, primary_ids | appearance_only_ids)}
    if secondary_ids:
        physical.update({row['server_item_id']: row
                         for row in source.policies(appearance_raw, secondary_raw, secondary_ids)})
    for item_id in sorted(qualified_ids):
        if item_id not in objects or item_id not in physical:
            raise ValueError(f'EVENT_ITEM_APPEARANCE_MISSING:{item_id}')
    native_ids, projection_only_ids = admission_sets(ROOT, physical, unavailable_ids)
    bindings = binding_index(BINDINGS.read_bytes())
    records = []
    for item_id in sorted(native_ids):
        item = xml.get(item_id) if item_id in primary_ids | appearance_only_ids else secondary[item_id]
        records.append(source.profile(item_id, item, objects[item_id],
                                      physical[item_id], bindings, None))
    # Keep the exact observed source facts, without turning them into an Item policy,
    # binding, key-section entry, map qualifier or wire reference.
    for item_id in sorted(appearance_only_ids):
        if (item_id not in objects or not physical[item_id]['ground']
                or physical[item_id]['ground_speed'] != 150 or physical[item_id]['movable']
                or physical[item_id]['pickupable'] or physical[item_id]['block_solid']
                or physical[item_id]['block_projectile']
                or objects[item_id]['flags'].get('flags.cumulative', False)):
            raise ValueError(f'EVENT_CAKE_BRIDGE_PHYSICAL_SOURCE_DRIFT:{item_id}')
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          bindings, native_ids)
    cosmetic_rules = json.loads((ROOT / COSMETICS).read_bytes())['rules']
    unavailable = [{'item_id': item_id, 'production_key': f'oteryn:item.tibia.i{item_id}',
                    'reason': ('EVENT_ITEM_CANONICAL_IDENTITY_UNAVAILABLE' if item_id in identity_unavailable_ids
                               else 'EVENT_ITEM_PHYSICAL_DEFINITION_UNAVAILABLE'),
                    'donor': [row['donor'] for row in cosmetic_rules if row['item_id'] == item_id]
                             + layer_donors.get(item_id, [])
                             + [row['donor'] for row in door_records
                                if item_id in (row['locked'], row['closed'], row['open'])]
                             + (['data-global/scripts/lib/a_piece_of_cake_config.lua:249-292']
                                if item_id in identity_unavailable_ids else []),
                    'appearance_present': item_id in objects,
                    **({'decision': BRIDGE_IDENTITY_DECISION} if item_id in identity_unavailable_ids else {})}
                   for item_id in sorted(unavailable_ids)]
    return {'schema': 'OTERYN_NATIVE_EVENT_ITEM_PROFILES/v1',
            'sources': [{'path': path, 'sha256': hashlib.sha256(raw).hexdigest()}
                        for path, raw in [('canary-47dfd51f/data/items/appearances.dat', appearance_raw),
                                          ('canary-47dfd51f/data/items/items.xml', secondary_raw),
                                          ('crystal-201189f/data/items/items.xml', xml_raw),
                                          ('crystal-201189f/' + WORLD_DOOR_SOURCE, door_raw)]] + loader_sources + layer_sources,
            'world_layers_sha256': hashlib.sha256((ROOT / WORLD_LAYERS).read_bytes()).hexdigest(),
            'event_actions_sha256': hashlib.sha256((ROOT / ACTIONS).read_bytes()).hexdigest(),
            'novelty_actions_sha256': hashlib.sha256((ROOT / NOVELTY).read_bytes()).hexdigest(),
            'world_layer_item_ids': sorted(layer_donors),
            'world_door_families': door_records,
            'canonical_identity_unavailable_evidence': identity_evidence,
            'native_item_profile_ids': sorted(native_ids),
            'projection_only_item_ids': sorted(projection_only_ids),
            # These complete source dictionaries retain XML door/floor classification.
            # A scoped world owner may qualify geometry against the canonical index,
            # but this metadata does not change any global native Item policy.
            'world_map_physical': [physical[item_id] for item_id in sorted(world_map_item_ids(ROOT))
                                   if item_id in physical and item_id not in unavailable_ids],
            'secondary_item_ids': sorted(secondary_ids),
            'appearance_only_items': [{'item_id': item_id,
                'donor': 'data-global/scripts/lib/a_piece_of_cake_config.lua:249',
                'loader': ['src/items/items.cpp:148-180', 'src/items/items.cpp:227-251',
                           'src/items/items.hpp:300-350'],
                'physical': physical[item_id]}
                for item_id in sorted(appearance_only_ids)],
            'unavailable': unavailable,
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    unavailable = overlay['unavailable']
    unavailable_keys = {row['production_key'] for row in unavailable}
    record_keys = {r['production_definition']['production_key'] for r in records}
    keys = {f'oteryn:item.tibia.i{i}' for i in item_ids(root)}
    identity_unavailable_keys = {f'oteryn:item.tibia.i{i}' for i in CANONICAL_IDENTITY_UNAVAILABLE} & keys
    unavailable_ids = {row['item_id'] for row in unavailable}
    map_ids = world_map_item_ids(root) - unavailable_ids
    physical = {row['server_item_id']: row for row in overlay['world_map_physical']}
    if physical.keys() != map_ids:
        raise ValueError('EVENT_ITEM_MAP_PHYSICAL_SET')
    native_ids, projection_only_ids = admission_sets(root, physical, unavailable_ids)
    native_keys = {f'oteryn:item.tibia.i{i}' for i in native_ids}
    projection_only_keys = {f'oteryn:item.tibia.i{i}' for i in projection_only_ids}
    if (overlay['schema'] != 'OTERYN_NATIVE_EVENT_ITEM_PROFILES/v1'
            or overlay['world_layer_item_ids'] != sorted(world_layer_item_ids(root))
            or [(row['locked'], row['closed'], row['open']) for row in overlay['world_door_families']]
                != world_door_families(root)
            or overlay['world_layers_sha256'] != hashlib.sha256((root / WORLD_LAYERS).read_bytes()).hexdigest()
            or overlay['event_actions_sha256'] != hashlib.sha256((root / ACTIONS).read_bytes()).hexdigest()
            or overlay['novelty_actions_sha256'] != hashlib.sha256((root / NOVELTY).read_bytes()).hexdigest()
            or overlay['canonical_identity_unavailable_evidence'] != ([bridge_source_evidence()] if identity_unavailable_keys else [])
            or [row['server_item_id'] for row in overlay['world_map_physical']] != sorted(map_ids)
            or overlay['native_item_profile_ids'] != sorted(native_ids)
            or overlay['projection_only_item_ids'] != sorted(projection_only_ids)
            or not identity_unavailable_keys <= unavailable_keys
            or identity_unavailable_keys & record_keys
            or len(records) != len(record_keys) or record_keys != native_keys
            or len(unavailable) != len(unavailable_keys)
            or record_keys & unavailable_keys or projection_only_keys & unavailable_keys
            or record_keys | projection_only_keys | unavailable_keys != keys
            or any(row['reason'] != ('EVENT_ITEM_CANONICAL_IDENTITY_UNAVAILABLE'
                                    if row['item_id'] in CANONICAL_IDENTITY_UNAVAILABLE
                                    else 'EVENT_ITEM_PHYSICAL_DEFINITION_UNAVAILABLE')
                   or (row['item_id'] in CANONICAL_IDENTITY_UNAVAILABLE
                       and row.get('decision') != BRIDGE_IDENTITY_DECISION)
                   or row['production_key'] != f"oteryn:item.tibia.i{row['item_id']}"
                   for row in unavailable)):
        raise ValueError('EVENT_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key']: r for r in document['records']}
    for record in records:
        key = record['production_definition']['production_key']
        if key in existing:
            # Admitted item semantics have their own owner; an action table cannot rewrite them.
            if (existing[key]['production_binding'] != record['production_binding']
                    or existing[key]['production_definition'] != record['production_definition']):
                raise ValueError('EVENT_ITEM_PROFILE_IDENTITY_CONFLICT')
        else:
            document['records'].append(record)
    document['records'].sort(key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('EVENT_ITEM_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
