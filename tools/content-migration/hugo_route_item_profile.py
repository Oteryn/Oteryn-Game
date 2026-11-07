"""Three source-qualified static objects needed by the real Hugo-to-Simon route."""
from pathlib import Path
import argparse
import copy
import hashlib
import importlib.util
import json
import sys
ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'tools/content-schema/world-authoring/samples/hugo-route-buttress-physics.json'
IDS = {1125, 1634, 1948}
FLAGS = ('blocks_movement', 'blocks_projectile', 'immovable_block_solid', 'has_height')

def capture(canary, crystal):
    spec = importlib.util.spec_from_file_location('_hugo_source_capture', ROOT/'tools/content-migration/seasonal_item_profiles.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.IDS = IDS
    packet = module.capture(canary, crystal)
    sys.path.insert(0, str(ROOT/'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    observed = []
    raw_flags = []
    for donor in (canary, crystal):
        observed.append(source.policies((donor/'data/items/appearances.dat').read_bytes(), (donor/'data/items/items.xml').read_bytes(), IDS))
        objects = source.engine_items.load_appearance_objects((donor/'data/items/appearances.dat').read_bytes())
        raw_flags.append({str(item): objects[item]['flags'] for item in sorted(IDS)})
    fields = ('block_solid', 'block_projectile', 'has_height', 'movable', 'pickupable', 'floor_change', 'floor_change_kind')
    first = {row['server_item_id']: row for row in observed[0]}
    second = {row['server_item_id']: row for row in observed[1]}
    if set(first) != IDS or set(second) != IDS or any(first[item][field] != second[item][field] for item in IDS for field in fields):
        raise ValueError('HUGO_BUTTRESS_CROSS_DONOR_PHYSICS')
    if any(first[item][field] for item in IDS for field in fields[:-1]) or any(first[item]['floor_change_kind'] is not None for item in IDS):
        raise ValueError('HUGO_BUTTRESS_SOURCE_CHANGED')
    packet['schema'] = 'OTERYN_HUGO_ROUTE_STATIC_PHYSICS/v2'
    packet['crosscheck_physics_fields'] = list(fields)
    if raw_flags[0] != raw_flags[1] or raw_flags[0]['1948'].get('flags.usable') is not True or raw_flags[0]['1948'].get('flags.forceuse') is not True:
        raise ValueError('HUGO_SOURCE_LADDER_USABILITY')
    packet['source_world_physics'] = observed[0]
    packet['source_appearance_flags'] = raw_flags[0]
    packet['crosscheck_appearance_flags'] = raw_flags[1]
    for row in packet['records']:
        row['admission']['materializable'] = False
    return packet

def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root/PROFILE).read_bytes())
    if packet['schema'] != 'OTERYN_HUGO_ROUTE_STATIC_PHYSICS/v2' or len(packet['records']) != len(IDS):
        raise ValueError('HUGO_BUTTRESS_EXACT_PACKET')
    if packet['source_revision'] != '47dfd51f45280a59a1d3e50ba7edd573d7234446' or packet['crosscheck_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8':
        raise ValueError('HUGO_BUTTRESS_SOURCE_REVISION')
    if packet['source_appearance_flags'] != packet['crosscheck_appearance_flags'] or packet['source_appearance_flags']['1948'].get('flags.usable') is not True or packet['source_appearance_flags']['1948'].get('flags.forceuse') is not True:
        raise ValueError('HUGO_SOURCE_LADDER_USABILITY')
    if {int(row['production_binding']['external_id']) for row in packet['records']} != IDS:
        raise ValueError('HUGO_EXACT_SOURCE_COVERAGE')
    native = json.loads(generated[profile_path])
    for record in packet['records']:
        item = int(record['production_binding']['external_id'])
        target = {'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1'}
        if record['production_binding']['target'] != target or record['production_binding']['disposition'] != 'EXACT':
            raise ValueError('HUGO_BUTTRESS_SOURCE_IDENTITY')
        if record['admission']['materializable'] or record['admission']['legal_destinations'] != ['Ground'] or any(record['attributes'].get(flag) is not False for flag in FLAGS):
            raise ValueError('HUGO_BUTTRESS_STATIC_ONLY')
        existing = [r for r in native['records'] if r['production_definition']['production_key'] == target['key']]
        if existing and existing != [record]:
            raise ValueError('HUGO_BUTTRESS_EXISTING_AUTHORITY_OR_PHYSICS_CONFLICT')
        if not existing:
            native['records'].append(copy.deepcopy(record))
    native['records'].sort(key=order)
    raw = (json.dumps(native, indent=2, ensure_ascii=False)+'\n').encode()
    generated[profile_path] = raw
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[manifest_path] = canonical(manifest)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.write_text(json.dumps(capture(args.canary, args.crystal), indent=2)+'\n')
