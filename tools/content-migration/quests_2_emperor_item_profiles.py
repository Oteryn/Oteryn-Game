"""Qualify the finite portable Items used by batch 2 world callbacks.

The existing native importer reads Crystal's pinned object flags and loader
defaults. Canonical identities and known facts remain authoritative. This does
not grant an NPC exchange, container capacity or unimplemented item action.
"""
import argparse
import copy
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

PATH = 'tools/content-schema/native-gameplay/quests-2-emperor-item-profiles.json'
PRODUCER = 'tools/content-migration/quests_2_emperor_item_profiles.py'
SCHEMA = 'OTERYN_QUESTS_2_EMPEROR_ITEM_PROFILES/v1'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = (3206, 3217, 19084)
SOURCE_HASHES = {
    'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    'src/items/items.hpp': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
    'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
    'src/protobuf/appearances.proto': '3aaf2965b14d9c3c0252876f8d3e228f20188568a03f8840b122588dbb76b78f',
    'data-global/scripts/quests/draconia/movement-escape.lua': '53f3313e99fb541c3e016b5ac09a2e372c0ad8a382786ccd79db6bb40a700312',
}
RULES = {
    3206: 'data-global/scripts/quests/draconia/movement-escape.lua:10-25',
    3217: 'data-global/scripts/quests/the_postman_missions_quest/actions_package.lua:3-8',
    19084: 'data-global/scripts/quests/the_gravedigger_of_drefia/actions_tears.lua:3-10',
}


def encode(value):
    return (json.dumps(value, indent=2, ensure_ascii=False) + '\n').encode()


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def provenance():
    return [{'repository': 'zimbadev/crystalserver', 'revision': REVISION,
             'path': path, 'sha256': sha} for path, sha in SOURCE_HASHES.items()]


def definitions(root):
    wanted = {f'oteryn:item.tibia.i{item}' for item in IDS}
    found, inputs = {}, []
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        raw = path.read_bytes()
        selected = {r['definition']['identity']['key']: r['definition']
                    for r in json.loads(raw)['records']
                    if r['definition']['identity']['key'] in wanted}
        if selected:
            if found.keys() & selected.keys():
                raise ValueError('QUESTS_2_EMPEROR_CANONICAL_DUPLICATE')
            found.update(selected)
            inputs.append({'path': path.relative_to(root).as_posix(), 'sha256': digest(raw)})
    if set(found) != wanted:
        raise ValueError('QUESTS_2_EMPEROR_CANONICAL_MISSING')
    return found, inputs


def capture(crystal, root=ROOT, *, source_files=False):
    raw = {path: (crystal / path).read_bytes() if source_files else subprocess.check_output(['git', '-C', str(crystal), 'show',
                                         f'{REVISION}:{path}']) for path in SOURCE_HASHES}
    if any(digest(raw[path]) != expected for path, expected in SOURCE_HASHES.items()):
        raise ValueError('QUESTS_2_EMPEROR_SOURCE_PIN')
    defaults = raw['src/items/items.hpp'].decode()
    for declaration in ('int32_t weight = 0', 'uint8_t stackSize = 100',
                        'uint16_t maxItems = 8', 'uint32_t decayTime = 0'):
        if declaration not in defaults:
            raise ValueError('QUESTS_2_EMPEROR_LOADER_DEFAULTS')
    if 'iType.stackable = object.flags().cumulative();' not in raw['src/items/items.cpp'].decode():
        raise ValueError('QUESTS_2_EMPEROR_STACK_LOADER')
    xml = source.xml_index(raw['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    physical = {r['server_item_id']: r for r in source.policies(
        raw['data/items/appearances.dat'], raw['data/items/items.xml'], set(IDS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    canonical, inputs = definitions(root)
    records = [accepted.qualify(item, xml[item], objects[item], physical[item], bindings,
                                canonical[f'oteryn:item.tibia.i{item}']) for item in IDS]
    packet = {
        'schema': SCHEMA, 'producer': PRODUCER, 'source_revision': REVISION,
        'item_ids': list(IDS), 'sources': provenance(), 'canonical_inputs': inputs,
        'donor_rules': [{'item': item, 'source': f'zimbadev/crystalserver@{REVISION}:{RULES[item]}'}
                        for item in IDS],
        'records': records,
    }
    validate_packet(packet, root)
    return packet


def validate_packet(packet, root=ROOT):
    if (packet.get('schema') != SCHEMA or packet.get('producer') != PRODUCER
            or packet.get('source_revision') != REVISION or packet.get('item_ids') != list(IDS)
            or packet.get('sources') != provenance()
            or packet.get('donor_rules') != [{'item': item,
                'source': f'zimbadev/crystalserver@{REVISION}:{RULES[item]}'} for item in IDS]):
        raise ValueError('QUESTS_2_EMPEROR_PROFILE_PROVENANCE')
    canonical, inputs = definitions(root)
    if packet.get('canonical_inputs') != inputs:
        raise ValueError('QUESTS_2_EMPEROR_CANONICAL_DRIFT')
    records = packet['records']
    keys = [f'oteryn:item.tibia.i{item}' for item in IDS]
    if [row['production_definition']['production_key'] for row in records] != keys:
        raise ValueError('QUESTS_2_EMPEROR_PROFILE_ITEM_SET')
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    for item, record in zip(IDS, records):
        key = f'oteryn:item.tibia.i{item}'
        identity = canonical[key]['identity']
        if (record['production_binding'] != bindings[str(item)]
                or bindings[str(item)]['target'] != identity
                or record['production_definition'] != {'family': identity['family'],
                    'production_key': key, 'revision_ref': identity['revision']}):
            raise ValueError('QUESTS_2_EMPEROR_PROFILE_IDENTITY')
        if any(accepted.known_conflict(canonical[key]['semantics'][field], record['semantics'][field])
               for field in ('physical', 'stack', 'container')):
            raise ValueError('QUESTS_2_EMPEROR_ACCEPTED_FACT_CONFLICT')
        expected_stack = source.K({'stackable': source.K(item == 19084),
                                  'stack_max': source.K(100) if item == 19084 else source.NA})
        if (record['semantics']['stack'] != expected_stack
                or record['semantics']['container'] != source.NA
                or record['semantics']['temporal'] != source.NA
                or record['semantics']['charges'] != source.NA
                or record['admission'] != {'materializable': True,
                    'stack_class': 'StackCapable' if item == 19084 else 'NonStackable',
                    'legal_destinations': ['CharacterInventory', 'Ground']}):
            raise ValueError('QUESTS_2_EMPEROR_SOURCE_CUSTODY')


def admit_packet(packet, root, generated, profile_path, manifest_path, order, canonical):
    validate_packet(packet, root)
    document = json.loads(generated[profile_path])
    present = {r['production_definition']['production_key']: r for r in document['records']}
    additions = []
    for record in packet['records']:
        key = record['production_definition']['production_key']
        existing = present.get(key)
        if existing is None:
            additions.append(record)
        elif (any(existing[field] != record[field] for field in
                  ('production_definition', 'production_binding', 'admission'))
              or any(accepted.known_conflict(existing['semantics'][field], record['semantics'][field])
                     for field in ('physical', 'stack', 'container'))):
            raise ValueError('QUESTS_2_EMPEROR_EXISTING_PROFILE_CONFLICT:' + key)
    document['records'] = sorted(document['records'] + additions, key=order)
    data = encode(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = digest(data)
    generated[manifest_path] = canonical(manifest)


def callback_definition(packet, native, definition, root=ROOT):
    """Qualify only this source cohort at the existing callback import boundary.

    This derived view never rewrites canonical definitions. It requires the
    complete source-qualified native record, not merely presence of an Item key.
    Only unknown physical, stack and container facts receive source values;
    every other canonical semantic group retains its exact accepted value.
    """
    key = definition['identity']['key']
    if key not in {f'oteryn:item.tibia.i{item}' for item in IDS}:
        return definition
    validate_packet(packet, root)
    current, _ = definitions(root)
    if definition != current[key]:
        raise ValueError('QUESTS_2_EMPEROR_CALLBACK_CANONICAL')
    record = next(row for row in packet['records']
                  if row['production_definition']['production_key'] == key)
    if native != record:
        raise ValueError('QUESTS_2_EMPEROR_CALLBACK_NATIVE_PROFILE')
    stack_class = record['admission']['stack_class']
    if definition['stack_class'] not in ('Unknown', stack_class):
        raise ValueError('QUESTS_2_EMPEROR_CALLBACK_CANONICAL_STACK_CLASS')
    effective = copy.deepcopy(definition)
    effective['materializable'] = True
    effective['stack_class'] = stack_class
    for group in ('physical', 'stack', 'container'):
        effective['semantics'][group] = accepted.accepted_first(
            definition['semantics'][group], record['semantics'][group])
    return effective


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = json.loads((root / PATH).read_bytes())
    admit_packet(packet, root, generated, profile_path, manifest_path, order, canonical)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--source-files', action='store_true',
                        help='Read digest-checked files from --crystal instead of a Git checkout.')
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = args.out or args.root / PATH
    raw = encode(capture(args.crystal, args.root, source_files=args.source_files))
    if args.check:
        if output.read_bytes() != raw:
            raise SystemExit('QUESTS_2_EMPEROR_PROFILE_DRIFT')
    else:
        output.write_bytes(raw)
    print(json.dumps({'items': list(IDS), 'check': args.check}))
