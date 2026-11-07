"""SRV7 exact Cake Item states; canonical bindings already exist in both donors."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CRYSTAL = '201189f871d23597b82caf88fd611454a2ddefb8'
IDS = {1625,1866,1949,2585,2875,2899,2992,2996,3001,3553,3570,3654,5791,6218,6384,6510,6511,6566,6567,6568,8886,9604,10338,10339,12085,12086,12113,12816,19387,34111}
PROFILE = 'tools/content-schema/native-gameplay/seasonal-item-profiles.json'
HASHES = {'data/items/items.xml': '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2',
          'data/items/appearances.dat': 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50'}
CRYSTAL_XML = '8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509'


def capture(canary, crystal):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as source
    from build_spell_item_identities import BINDINGS, binding_index, bind_profiles
    raw = {name: (canary / name).read_bytes() for name in HASHES}
    if any(hashlib.sha256(raw[name]).hexdigest() != expected for name, expected in HASHES.items()):
        raise ValueError('SEASONAL_ITEM_CANARY_SOURCE_DIGEST')
    independent = (crystal / 'data/items/items.xml').read_bytes()
    if hashlib.sha256(independent).hexdigest() != CRYSTAL_XML:
        raise ValueError('SEASONAL_ITEM_CRYSTAL_SOURCE_DIGEST')
    xml = source.xml_index(raw['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(raw['data/items/appearances.dat'])
    bindings = binding_index(BINDINGS.read_bytes())
    physical = {row['server_item_id']: row for row in source.policies(raw['data/items/appearances.dat'], raw['data/items/items.xml'], IDS)}
    records = [source.profile(i, xml[i], objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS,
                          independent_crystal_source=(CRYSTAL, independent))
    # Static reward object: project exact map physics without an Item MINT entitlement.
    # This is the closed native restriction for12816; source flags stay unchanged.
    for record in bound['records']:
        if record['production_binding']['external_id'] == '12816':
            if record['admission'] != {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']} or record['attributes']['blocks_movement'] is not True or record['attributes']['blocks_projectile'] is not False:
                raise ValueError('PUMPKIN_STATIC_BODY_SOURCE_PHYSICS')
            record['admission']['materializable'] = False
        elif record['production_binding']['external_id'] == '19387':
            if record['admission'] != {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']} or any(record['attributes'][field] is not False for field in ('blocks_movement', 'blocks_projectile', 'immovable_block_solid', 'has_height')):
                raise ValueError('SPIKE_PORTAL_STATIC_SOURCE_PHYSICS')
            record['admission']['materializable'] = False
    return {'schema': 'OTERYN_NATIVE_SEASONAL_ITEM_PROFILES/v1', 'source_revision': CANARY,
            'crosscheck_revision': CRYSTAL, 'sources': [{'path': p, 'sha256': h} for p,h in HASHES.items()],
            'records': bound['records']}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PROFILE).read_bytes())
    if overlay['schema'] != 'OTERYN_NATIVE_SEASONAL_ITEM_PROFILES/v1' or overlay['source_revision'] != CANARY or overlay['crosscheck_revision'] != CRYSTAL:
        raise ValueError('SEASONAL_ITEM_PROFILE_SOURCE')
    if {int(row['production_definition']['production_key'].rsplit('i',1)[1]) for row in overlay['records']} != IDS:
        raise ValueError('SEASONAL_ITEM_PROFILE_COVERAGE')
    body = [row for row in overlay['records'] if row['production_binding']['external_id'] == '12816']
    if len(body) != 1 or body[0]['admission'] != {'materializable': False, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']} or body[0]['attributes']['blocks_movement'] is not True or body[0]['attributes']['blocks_projectile'] is not False:
        raise ValueError('PUMPKIN_STATIC_BODY_PHYSICS_ONLY')
    portals = [row for row in overlay['records'] if row['production_binding']['external_id'] == '19387']
    if len(portals) != 1 or portals[0]['admission'] != {'materializable': False, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']} or any(portals[0]['attributes'][field] is not False for field in ('blocks_movement', 'blocks_projectile', 'immovable_block_solid', 'has_height')):
        raise ValueError('SPIKE_PORTAL_STATIC_PHYSICS_ONLY')
    document = json.loads(generated[profile_path])
    # A source-qualified existing identity may predate captured collision flags.
    # Fill only unknown flags, retaining every other native semantic and refusing
    # a known conflict rather than replacing a previously admitted authority.
    source_by_key = {row['production_definition']['production_key']: row for row in overlay['records']}
    collision_fields = ('blocks_movement', 'blocks_projectile', 'immovable_block_solid', 'has_height')
    for record in document['records']:
        source = source_by_key.get(record['production_definition']['production_key'])
        if source is None:
            continue
        if source['production_binding']['external_id'] in {'12816', '19387'} and record['admission'] != source['admission']:
            raise ValueError('STATIC_SOURCE_OBJECT_EXISTING_MINT_AUTHORITY')
        attributes = record['attributes']
        captured = source['attributes']
        changes = [name for name in collision_fields if isinstance(captured.get(name), bool)
                   and (attributes.get(name) is None or attributes.get(name) != captured[name])]
        if not changes:
            continue
        definition = source['production_definition']
        target = {'family': definition['family'], 'key': definition['production_key'], 'revision': definition['revision_ref']}
        if (record['production_definition'] != definition
                or record['production_binding'] != source['production_binding']
                or source['production_binding']['disposition'] != 'EXACT'
                or source['production_binding']['target'] != target
                or record['semantics']['physical'] != source['semantics']['physical']):
            raise ValueError('SEASONAL_EXISTING_PHYSICS_AUTHORITY')
        for name in changes:
            if attributes.get(name) is not None:
                raise ValueError('SEASONAL_EXISTING_PHYSICS_KNOWN_CONFLICT:' + name)
            attributes[name] = captured[name]
    existing = {row['production_definition']['production_key'] for row in document['records']}
    document['records'] = sorted(document['records'] + [row for row in overlay['records'] if row['production_definition']['production_key'] not in existing], key=order)
    # Candy Cane is a reachable Santa gift; reuse the SHA-pinned existing food fact.
    food_raw = (root / 'docs/agents/evidence/OTV2-20261004-item-consumption-facts-v1.json').read_bytes()
    if hashlib.sha256(food_raw).hexdigest() != 'd13625eadd669ca491964e4a0cd1a76ec0e92999c613a55c9b8964c01687e5e6':
        raise ValueError('SEASONAL_CANDY_FOOD_SOURCE')
    food = json.loads(food_raw)
    facts = [row for row in food['rows'] if row['target']['key'] == 'oteryn:item.tibia.i3599']
    records = [row for row in document['records'] if row['production_definition']['production_key'] == 'oteryn:item.tibia.i3599']
    if len(facts) != 1 or len(records) != 1:
        raise ValueError('SEASONAL_CANDY_FOOD_IDENTITY')
    row, record = facts[0], records[0]
    definition = record['production_definition']
    if row['target'] != {'family': definition['family'], 'key': definition['production_key'], 'revision': definition['revision_ref']}:
        raise ValueError('SEASONAL_CANDY_FOOD_REVISION')
    value = {'state': 'KNOWN', 'value': row['consumption']}
    if record['semantics'].get('consumption', value) != value:
        raise ValueError('SEASONAL_CANDY_FOOD_CONFLICT')
    record['semantics']['consumption'] = value
    raw = (json.dumps(document, indent=2, ensure_ascii=False)+'\n').encode()
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
    raw = (json.dumps(capture(args.canary, args.crystal), indent=2, ensure_ascii=False)+'\n').encode()
    path = ROOT / PROFILE
    if args.check:
        if path.read_bytes() != raw: raise ValueError('SEASONAL_ITEM_PROFILE_DRIFT')
    else: path.write_bytes(raw)
