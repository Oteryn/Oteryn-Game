"""P06/P07: qualify the seventeen physical Atelier inputs from pinned Crystal.

This adds a bounded native Item provider overlay. The legacy definition shards
remain reference-only; crusher use and promotion-scroll grants are not admitted.
Crystal's XML weights are hundredths of an ounce (Item::getWeightDescription,
src/items/item.cpp:3332-3352), matching the native carried-weight owner.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PATH = 'tools/content-schema/native-gameplay/wheel-gem-item-profiles.json'
PRODUCER = 'tools/content-migration/wheel_gem_item_profiles.py'
SCHEMA = 'OTERYN_NATIVE_WHEEL_GEM_ITEM_PROFILES/v1'
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
# Actual Git blob identities at PIN, checked before parsing any supplied source.
# SHA256 digests of those same bytes are retained in the generated source packet.
SOURCE_BLOBS = {
    'data/items/items.xml': 'ea4196beae8f77be898f05de89db81de134886b6',
    'data/items/appearances.dat': '37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0',
    'src/items/items.hpp': 'b502ec621701f1cebfe7e509207649081ee3e470',
    'src/items/items.cpp': 'bdb7b4b48aadfeb9e031311ca107aadb4ee6e30b',
    'src/items/functions/item/item_parse.cpp': 'd95e2c83f44a81e7da7dc0d066c3b6f3a3302426',
}
SOURCE_SHA256 = {
    'data/items/items.xml': '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509',
    'data/items/appearances.dat': '17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1',
    'src/items/items.hpp': '06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca',
    'src/items/items.cpp': 'f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d',
    'src/items/functions/item/item_parse.cpp': '08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7',
}
# Literal primary XML weights. Regular guardian/marksman/spiritualist gems use 20
# (legacy reference shards use 50); greater fragment uses 500 (legacy 10).
# Keep every entry explicit: profession-specific source values are not inferred.
WEIGHTS = {
    44602: 20, 44603: 20, 44604: 120,
    44605: 20, 44606: 20, 44607: 120,
    44608: 20, 44609: 50, 44610: 120,
    44611: 20, 44612: 50, 44613: 120,
    46625: 5, 46626: 500,
    49371: 20, 49372: 20, 49373: 120,
}
IDS = tuple(sorted(WEIGHTS))
KEYS = frozenset(f'oteryn:item.tibia.i{item}' for item in IDS)
# Exact primary appearance metadata: category 15 for the fifteen gems, 9 for
# both fragments. Crystal items.cpp:215-216,228 reads category/wareId from
# has_market. The existing Item schema qualifies marketability independently
# of general tradeability; no vocation or binding policy is inferred here.
MARKET_CATEGORIES = {
    44602: 15, 44603: 15, 44604: 15,
    44605: 15, 44606: 15, 44607: 15,
    44608: 15, 44609: 15, 44610: 15,
    44611: 15, 44612: 15, 44613: 15,
    46625: 9, 46626: 9,
    49371: 15, 49372: 15, 49373: 15,
}
KNOWN = lambda value: {'state': 'KNOWN', 'value': value}
STACK = KNOWN({'stackable': KNOWN(True), 'stack_max': KNOWN(100)})
ADMISSION = {'materializable': True, 'stack_class': 'StackCapable',
             'legal_destinations': ['CharacterInventory', 'Ground']}
MARKETABILITY = KNOWN({
    'tradeable': {'state': 'UNKNOWN'},
    'marketable': KNOWN(True),
    'vocations': {'state': 'UNKNOWN'},
    'account_binding_policy': {'state': 'UNKNOWN'},
    'character_binding_policy': {'state': 'UNKNOWN'},
})


def source_marketability(item: int, flags: dict) -> dict:
    category = MARKET_CATEGORIES.get(item)
    if (category is None or flags.get('flags.market') is not True
            or flags.get('market.category') != category
            or flags.get('market.trade_as_object_id') != item
            or flags.get('market.show_as_object_id') != item):
        raise ValueError(f'WHEEL_GEM_MARKET_SOURCE:{item}')
    return copy.deepcopy(MARKETABILITY)


def read_sources(crystal: Path) -> dict[str, bytes]:
    captured = {}
    for path, expected in SOURCE_BLOBS.items():
        raw = (crystal / path).read_bytes()
        identity = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        if identity != expected or hashlib.sha256(raw).hexdigest() != SOURCE_SHA256[path]:
            raise ValueError(f'WHEEL_GEM_SOURCE_BLOB:{path}')
        captured[path] = raw
    return captured


def validate(overlay: dict) -> list[dict]:
    if (overlay.get('schema') != SCHEMA or overlay.get('producer') != PRODUCER
            or overlay.get('weight_unit') != 'CENTI_OUNCE'):
        raise ValueError('WHEEL_GEM_PROFILE_SCHEMA')
    sources = overlay.get('sources', [])
    if (len(sources) != len(SOURCE_BLOBS)
            or {row.get('path') for row in sources} != set(SOURCE_BLOBS)):
        raise ValueError('WHEEL_GEM_SOURCE_SET')
    for row in sources:
        if (row.get('reference') != 'crystal' or row.get('revision') != PIN
                or row.get('git_blob_sha1') != SOURCE_BLOBS[row['path']]
                or row.get('sha256') != SOURCE_SHA256[row['path']]):
            raise ValueError('WHEEL_GEM_SOURCE_PIN')
    records = overlay.get('records', [])
    keys = [record.get('production_definition', {}).get('production_key') for record in records]
    if len(records) != len(IDS) or set(keys) != KEYS:
        raise ValueError('WHEEL_GEM_PROFILE_SET')
    for record, key in zip(records, keys, strict=True):
        item = int(key.rsplit('i', 1)[1])
        definition = {'family': 'Item', 'production_key': key, 'revision_ref': 'definition-r1'}
        physical = KNOWN({'weight': KNOWN(WEIGHTS[item]),
                          'movable': KNOWN(True), 'pickupable': KNOWN(True)})
        semantics = record.get('semantics', {})
        if (record['production_definition'] != definition
                or record.get('admission') != ADMISSION
                or semantics.get('stack') != STACK
                or semantics.get('physical') != physical
                or semantics.get('trade_restrictions') != MARKETABILITY
                or semantics.get('equipment') != {'state': 'NOT_APPLICABLE'}
                or record.get('attributes', {}).get('speed_bonus') != 0):
            raise ValueError(f'WHEEL_GEM_PROFILE_POLICY:{key}')
    return records


def capture(crystal: Path) -> dict:
    captured = read_sources(crystal)
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles

    xml_raw = captured['data/items/items.xml']
    appearance_raw = captured['data/items/appearances.dat']
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(appearance_raw)
    bindings = binding_index(BINDINGS.read_bytes())
    # Crystal items.hpp:348; XML parse:895-901; items.cpp:236,244,251.
    if b'uint8_t stackSize = 100;' not in captured['src/items/items.hpp']:
        raise ValueError('WHEEL_GEM_STACK_DEFAULT')
    physical = {row['server_item_id']: row
                for row in source.policies(appearance_raw, xml_raw, set(IDS))}
    records = []
    for item in IDS:
        node, appearance = xml[item], objects[item]
        attrs = source.attrs(node)
        flags = appearance['flags']
        if ('stacksize' in attrs or not flags.get('flags.cumulative')
                or not flags.get('flags.take') or flags.get('flags.unmove', False)
                or int(attrs['weight']) != WEIGHTS[item]):
            raise ValueError(f'WHEEL_GEM_PHYSICAL_SOURCE:{item}')
        record = source.profile(item, node, appearance, physical[item], bindings, None)
        # The generic physical profile leaves this independent field UNKNOWN.
        # Promote only affirmative, exactly bound primary Market evidence; never
        # overwrite a future conflicting or independently qualified trade group.
        if record['semantics']['trade_restrictions'] != {'state': 'UNKNOWN'}:
            raise ValueError(f'WHEEL_GEM_MARKET_BASE:{item}')
        record['semantics']['trade_restrictions'] = source_marketability(item, flags)
        records.append(record)
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                          bindings, set(IDS))
    overlay = {
        'schema': SCHEMA,
        'producer': PRODUCER,
        'weight_unit': 'CENTI_OUNCE',
        'sources': [{'reference': 'crystal', 'revision': PIN, 'path': path,
                     'git_blob_sha1': SOURCE_BLOBS[path],
                     'sha256': hashlib.sha256(raw).hexdigest()}
                    for path, raw in captured.items()],
        'records': bound['records'],
    }
    validate(overlay)
    return overlay


def admit(root, generated, profile_path, manifest_path, order, canonical):
    records = validate(json.loads((root / PATH).read_bytes()))
    document = json.loads(generated[profile_path])
    present = {record['production_definition']['production_key']
               for record in document['records']}
    if present & KEYS:
        raise ValueError('WHEEL_GEM_PROFILE_DUPLICATE')
    document['records'] = sorted(document['records'] + records, key=order)
    raw = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    manifest_raw = canonical(manifest)
    generated[profile_path] = raw
    generated[manifest_path] = manifest_raw


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = (json.dumps(capture(args.crystal), indent=2, ensure_ascii=False) + '\n').encode()
    path = ROOT / PATH
    if args.check:
        if path.read_bytes() != raw:
            raise SystemExit('WHEEL_GEM_PROFILE_DRIFT')
    else:
        path.write_bytes(raw)
