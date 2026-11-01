"""W06/W07: source-qualified native custody profiles, through the existing Item importer.

The reference-only Item shards stay unchanged. This overlay adds the exact pinned
Crystal soul-core catalogue, prism, exalted core and basic imbuement scrolls to
the native Item provider used by existing craft and corpse custody writers.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, binding_index

PATH = 'tools/content-schema/native-gameplay/soulpit-echo-item-profiles.json'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
SCHEMA = 'OTERYN_NATIVE_SOULPIT_ECHO_ITEM_PROFILES/v1'
# A source replay with --check reproduces this entire capture. Admission also binds the
# catalogue and every custody field, so correlated edits cannot silently shrink a draw.
CAPTURE_SHA256 = '94644f01fb0c574a33eb84f45abec2deb5e1f0c0316b57ac3f2684f0e9cc7192'
# Captured from the exact raw GitHub objects at PIN, never from a moving branch.
SOURCE_HASHES = {
    'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'src/items/items.hpp': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
    'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
}
EXTRA_IDS = {37110, 49164, *range(53751, 53775)}


def capture(crystal: Path) -> dict:
    captured = {path: (crystal / path).read_bytes() for path in SOURCE_HASHES}
    for path, raw in captured.items():
        if hashlib.sha256(raw).hexdigest() != SOURCE_HASHES[path]:
            raise ValueError(f'SOULPIT_SOURCE_DIGEST:{path}')
    if b'uint8_t stackSize = 100;' not in captured['src/items/items.hpp']:
        raise ValueError('SOULPIT_STACK_LOADER_DEFAULT')
    xml = source.xml_index(captured['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
    # items.cpp:215-216 imports the market category before XML. game_functions.cpp:327
    # admits either that Item type OR XML primarytype, including non-suffixed names.
    core_ids = {
        item_id for item_id, item in objects.items()
        if item['flags'].get('market.category') == 26
    } | {
        item_id for item_id, node in xml.items()
        if source.attrs(node).get('primarytype', '').lower() == 'soul cores'
    }
    wanted = core_ids | EXTRA_IDS
    if not core_ids or not wanted <= objects.keys():
        raise ValueError('SOULPIT_SOURCE_ITEM_SET')
    bindings = binding_index(BINDINGS.read_bytes())
    unadmitted = sorted(item_id for item_id in wanted if str(item_id) not in bindings)
    admitted = wanted - set(unadmitted)
    physical = {row['server_item_id']: row for row in source.policies(
        captured['data/items/appearances.dat'], captured['data/items/items.xml'], wanted,
    )}
    profiles = []
    for item_id in sorted(admitted):
        # Fifteen source cores have no XML override. The actual server loads their name,
        # stack and physical flags from appearances and keeps its ItemType weight default.
        # They are still in Game.getSoulCoreItems and its fusion output distribution.
        record = source.profile(item_id, xml.get(item_id), objects[item_id], physical[item_id], bindings, None)
        record['authoring']['item']['revision'] = 'soulpit-echo-crystal-201189f'
        if (not record['admission']['materializable']
                or record['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']):
            raise ValueError(f'SOULPIT_CUSTODY_POLICY:{item_id}')
        # Reuse the exact protected identity, including the already-admitted Summer
        # successor bindings. The older spell helper assumes only its own baseline pin;
        # this batch independently qualifies policy from the four exact source hashes.
        # Like mining_item_profiles, it allocates no new identity and changes no binding.
        binding = bindings[str(item_id)]
        record['production_binding'] = copy.deepcopy(binding)
        record['production_binding_qualification'] = None
        record['production_definition'] = {
            'family': 'Item', 'production_key': binding['target']['key'],
            'revision_ref': binding['target']['revision'],
        }
        profiles.append(record)
    return {
        'schema': SCHEMA,
        'producer': 'tools/content-migration/soulpit_echo_item_profiles.py',
        'sources': [{'repository': 'zimbadev/crystalserver', 'revision': PIN,
                     'path': path, 'sha256': digest} for path, digest in SOURCE_HASHES.items()],
        'core_catalogue': [{'id': item_id, 'name': xml[item_id].get('name', objects[item_id].get('name', '')) if item_id in xml else objects[item_id].get('name', '')}
                           for item_id in sorted(core_ids)],
        'unadmitted_source_ids': unadmitted,
        'records': profiles,
    }


def admit(root, generated, profile_path, manifest_path, order, canonical):
    raw_capture = (root / PATH).read_bytes()
    if hashlib.sha256(raw_capture).hexdigest() != CAPTURE_SHA256:
        raise ValueError('SOULPIT_ITEM_CAPTURE_DRIFT')
    overlay = json.loads(raw_capture)
    records = overlay['records']
    core_ids = [row['id'] for row in overlay['core_catalogue']]
    source_ids = [int(row['production_binding']['external_id']) for row in records]
    bindings = binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    unadmitted = sorted(item_id for item_id in set(core_ids) | EXTRA_IDS if str(item_id) not in bindings)
    expected_sources = [{'repository': 'zimbadev/crystalserver', 'revision': PIN,
                         'path': path, 'sha256': digest} for path, digest in SOURCE_HASHES.items()]
    if (overlay['schema'] != SCHEMA or overlay['sources'] != expected_sources
            or core_ids != sorted(set(core_ids))
            or overlay['unadmitted_source_ids'] != unadmitted
            or source_ids != sorted((set(core_ids) | EXTRA_IDS) - set(unadmitted))):
        raise ValueError('SOULPIT_ITEM_PROFILE_SET')
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    for record in records:
        identity = record['production_definition']
        key = identity['production_key']
        binding = record['production_binding']
        if (identity['family'] != 'Item' or binding['disposition'] != 'EXACT'
                or binding != bindings.get(binding['external_id'])
                or key != binding['target']['key'] or identity['revision_ref'] != binding['target']['revision']
                or record['admission']['legal_destinations'] != ['CharacterInventory', 'Ground']
                or record['admission']['materializable'] is not True):
            raise ValueError(f'SOULPIT_ITEM_PROFILE_AUTHORITY:{key}')
        if key in existing:
            previous = existing[key]
            if (previous['production_definition'] != identity or previous['production_binding'] != binding
                    or previous['admission'] != record['admission']
                    or any(previous['semantics'][part] != record['semantics'][part] for part in ('physical', 'stack'))):
                raise ValueError(f'SOULPIT_ITEM_PROFILE_CONFLICT:{key}')
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
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / PATH)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    if args.check:
        if args.output.read_bytes() != output:
            raise SystemExit('SOULPIT_ITEM_PROFILE_DRIFT')
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(output)
