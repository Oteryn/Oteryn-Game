"""Admit the four Crystal condition potions without inventing HP/MP restores.

Crystal 201189f potions.lua:1-31,52-54,63,125-129 owns their runtime
conditions. The existing exact identity binding and physical Item importer
qualify their carried-item profiles; dispatch applies the source conditions.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, CANARY_PIN, XML_SHA

PATH = 'tools/content-schema/native-gameplay/condition-potion-profiles.json'
CRYSTAL_PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
CRYSTAL_INPUTS = {
    'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    'data/scripts/actions/items/potions.lua': '6eb180d90f8bfa8d14480b5703cf0b59dccc142caf1cbf5789219b61698d0527',
}
IDS = (7439, 7440, 7443, 49271)
DONOR_RULE = 'crystal/data/scripts/actions/items/potions.lua:1-31,52-54,63,125-129'
RUNTIME_OWNER = 'crates/gameplay/dispatch/src/gameplay_transport/condition_potions.rs'
# Crystal potions.lua:52-54,63. No level or magic-level restriction.
VOCATIONS = {7439: ['KNIGHT'], 7440: ['DRUID', 'SORCERER'], 7443: ['PALADIN'], 49271: None}


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def requirements(item):
    return source.K({
        'min_level': dict(source.NA),
        'min_magic_level': dict(source.NA),
        'vocations': source.K(VOCATIONS[item]) if VOCATIONS[item] else dict(source.NA),
        'enforcement_mode': 'ON_USE',
    })


def canonical_identities(root):
    """Discover every selected canonical row; never trust packet-provided paths alone."""
    keys = {f'oteryn:item.tibia.i{i}' for i in IDS}
    canonical, inputs = {}, []
    for path in sorted((root / 'content/items/definitions').glob('items-*.json')):
        raw = path.read_bytes()
        selected = []
        for record in json.loads(raw)['records']:
            definition = record['definition']
            key = definition['identity']['key']
            if key not in keys:
                continue
            if key in canonical:
                raise ValueError('CONDITION_POTION_CANONICAL_IDENTITIES')
            canonical[key] = definition
            selected.append(key)
        if selected:
            inputs.append({'path': path.relative_to(root).as_posix(), 'sha256': digest(raw)})
    if set(canonical) != keys:
        raise ValueError('CONDITION_POTION_CANONICAL_IDENTITIES')
    return canonical, inputs


def provenance():
    return [{'path': f'crystal/{path}', 'sha256': sha} for path, sha in CRYSTAL_INPUTS.items()] + [
        {'path': 'canary/data/items/items.xml', 'sha256': XML_SHA['canary']},
        {'path': 'canary/data/items/appearances.dat', 'sha256': CRYSTAL_INPUTS['data/items/appearances.dat']},
    ]


def capture(canary, crystal):
    captured = {path: (crystal / path).read_bytes() for path in CRYSTAL_INPUTS}
    if any(digest(raw) != CRYSTAL_INPUTS[path] for path, raw in captured.items()):
        raise ValueError('CONDITION_POTION_CRYSTAL_SOURCE_PIN')
    canary_xml = (canary / 'data/items/items.xml').read_bytes()
    canary_appearance = (canary / 'data/items/appearances.dat').read_bytes()
    if (digest(canary_xml) != XML_SHA['canary']
            or canary_appearance != captured['data/items/appearances.dat']):
        raise ValueError('CONDITION_POTION_CANARY_SOURCE_PIN')
    xml = source.xml_index(captured['data/items/items.xml'])
    other = source.xml_index(canary_xml)
    objects = source.engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
    physical = {r['server_item_id']: r for r in source.policies(
        captured['data/items/appearances.dat'], captured['data/items/items.xml'], set(IDS))}
    bindings = source.binding_index(BINDINGS.read_bytes())
    canonical, canonical_inputs = canonical_identities(ROOT)
    records = []
    for item in IDS:
        if (xml[item].get('name') != other[item].get('name')
                or source.attrs(xml[item]).get('weight') != source.attrs(other[item]).get('weight')):
            raise ValueError('CONDITION_POTION_DONOR_DRIFT')
        row = source.profile(item, xml[item], objects[item], physical[item], bindings, None)
        key = f'oteryn:item.tibia.i{item}'
        if canonical[key]['identity'] != bindings[str(item)]['target']:
            raise ValueError('CONDITION_POTION_CANONICAL_IDENTITIES')
        if row['semantics']['physical'] != canonical[key]['semantics']['physical']:
            raise ValueError('CONDITION_POTION_CANONICAL_PHYSICAL_DRIFT')
        row['semantics']['use_requirements'] = requirements(item)
        records.append(row)
    bound = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, set(IDS))
    return {
        'schema': 'OTERYN_NATIVE_CONDITION_POTION_PROFILES/v1',
        'producer': 'tools/content-migration/condition_potion_profiles.py',
        'source_revisions': {'crystal': CRYSTAL_PIN, 'canary': CANARY_PIN},
        'runtime_owner': RUNTIME_OWNER,
        'donor_rule': DONOR_RULE,
        'canonical_inputs': canonical_inputs,
        'sources': provenance(),
        'records': bound['records'],
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [f'oteryn:item.tibia.i{i}' for i in IDS]
    if (overlay['schema'] != 'OTERYN_NATIVE_CONDITION_POTION_PROFILES/v1'
            or [r['production_definition']['production_key'] for r in records] != keys):
        raise ValueError('CONDITION_POTION_PROFILE_SET')
    if (overlay['source_revisions'] != {'crystal': CRYSTAL_PIN, 'canary': CANARY_PIN}
            or overlay['sources'] != provenance()
            or overlay['runtime_owner'] != RUNTIME_OWNER
            or overlay['donor_rule'] != DONOR_RULE):
        raise ValueError('CONDITION_POTION_SOURCE_PROVENANCE')
    definitions, descriptors = canonical_identities(root)
    if overlay['canonical_inputs'] != descriptors:
        raise ValueError('CONDITION_POTION_CANONICAL_DRIFT')
    # Existing canonical identities retain their original source revision. New gameplay
    # facts come from CRYSTAL_PIN; neither capture nor admission allocates new identities.
    bindings = source.binding_index((root / BINDINGS.relative_to(ROOT)).read_bytes())
    for item, record in zip(IDS, records):
        key = f'oteryn:item.tibia.i{item}'
        binding = bindings[str(item)]
        if (record['production_binding'] != binding
                or record['production_binding_qualification'] is not None
                or definitions[key]['identity'] != binding['target']
                or record['authoring']['item'] != {
                    'family': 'Item', 'key': f'candidate:item/{item}', 'revision': 'spell-p2-r21'}
                or record['production_definition'] != {
                    'family': 'Item', 'production_key': key, 'revision_ref': binding['target']['revision']}
                or record['semantics']['physical'] != definitions[key]['semantics']['physical']
                or record['semantics']['stack'] != source.K({
                    'stackable': source.K(True), 'stack_max': source.K(100)})
                or record['semantics']['container'] != source.NA
                or record['admission'] != {'materializable': True, 'stack_class': 'StackCapable',
                                           'legal_destinations': ['CharacterInventory', 'Ground']}
                or record['semantics']['use_requirements'] != requirements(item)
                or 'consumption' in record['semantics']):
            raise ValueError('CONDITION_POTION_PROFILE_QUALIFICATION')
    document = json.loads(generated[profile_path])
    existing = {r['production_definition']['production_key'] for r in document['records']}
    if existing.intersection(keys):
        raise ValueError('CONDITION_POTION_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = digest(raw)
    manifest_raw = canonical(manifest)
    generated[profile_path] = raw
    generated[manifest_path] = manifest_raw


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary', type=Path, required=True)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    destination = ROOT / PATH
    if args.check:
        if destination.read_bytes() != raw:
            raise SystemExit('CONDITION_POTION_PROFILE_DRIFT')
    else:
        destination.write_bytes(raw)
