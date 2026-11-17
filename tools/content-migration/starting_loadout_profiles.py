"""SRVGAP-C1: admit the eight missing login-kit definitions, using pinned source facts.

Canary 47dfd51f data/items/items.xml:11278-11300,20985-21007,21318-21340,
22257-22265,83482-83498; the appearance input captures only mechanical flags.
The shared source importer owns loader defaults, equipment, weight and weapon parsing.
"""
import hashlib
import json
import sys

from native_profile_composition import encode_native_profile, merge_fact

IDS = {3571, 3572, 7773, 7774, 7991, 7992, 8095, 50171}
INPUT = 'tools/content-schema/native-gameplay/starting-loadout-appearance-facts.json'
XML = 'imports/canary/items-xml/items.xml'
XML_SHA256 = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
FRAGMENT = 'content/items/definitions/starting-loadout-profiles.json'
APPEARANCES_SHA256 = 'aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    sys.path.insert(0, str(root / 'tools/content-schema/native-gameplay'))
    import build_source_item_profiles as native
    captured = json.loads((root / INPUT).read_bytes())
    xml = (root / XML).read_bytes()
    if (captured['schema'] != 'OTERYN_STARTING_LOADOUT_APPEARANCE_FACTS/v1'
            or captured['source_path'] != 'data/items/appearances.dat'
            or captured['source_revision'] != '47dfd51f45280a59a1d3e50ba7edd573d7234446'
            or captured['source_sha256'] != APPEARANCES_SHA256
            or {row['item_id'] for row in captured['records']} != IDS
            or len(captured['records']) != len(IDS)
            or hashlib.sha256(xml).hexdigest() != XML_SHA256):
        raise ValueError('STARTING_LOADOUT_SOURCE_FACTS')
    bindings = native.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    index = native.xml_index(xml)
    records = [native.profile(row['item_id'], index[row['item_id']], {'flags': row['flags']},
                              row['physical'], bindings, None) for row in captured['records']]
    # The common closure importer leaves protection unknown; the new garments have
    # explicit armor in pinned Canary items.xml:11278-11291,21318-21333,22257-22265.
    # Keep unparsed resistance facts unknown, as the existing native Item contract requires.
    for record, captured_row in zip(records, captured['records']):
        attributes = {node.get('key', '').lower(): node.get('value')
                      for node in index[captured_row['item_id']].findall('attribute')}
        if 'armor' in attributes:
            record['semantics']['protection'] = native.K({
                'armor': native.K(int(attributes['armor'])), 'resistances': {'state': 'UNKNOWN'},
            })
    bound = native.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records},
                                 bindings, set(index))['records']
    document = json.loads(generated[profile_path])
    existing = {row['production_definition']['production_key']: row
                for row in document['records']}
    if len(existing) != len(document['records']):
        raise ValueError('STARTING_LOADOUT_PROFILE_DUPLICATE')
    additions = []
    for record in bound:
        key = record['production_definition']['production_key']
        previous = existing.get(key)
        if previous is None:
            additions.append(record)
            continue
        # Another owner may already admit a starter reward. Keep its equipment
        # policy, but require the same identity, binding and portable physics.
        if any(previous[field] != record[field] for field in (
                'production_definition', 'production_binding', 'admission')):
            raise ValueError(f'STARTING_LOADOUT_PROFILE_CONFLICT:{key}')
        if previous['semantics']['physical'] != record['semantics']['physical']:
            raise ValueError(f'STARTING_LOADOUT_PROFILE_CONFLICT:{key}')
        protection = record['semantics']['protection']
        old_protection = previous['semantics'].get('protection', {'state': 'UNKNOWN'})
        if old_protection.get('state') == 'KNOWN' and old_protection != protection:
            raise ValueError(f'STARTING_LOADOUT_PROTECTION_CONFLICT:{key}')
        if protection.get('state') == 'KNOWN':
            previous['semantics']['protection'] = protection
        for group, fact in record['semantics'].items():
            old = previous['semantics'].get(group, {'state': 'UNKNOWN'})
            if group == 'presentation' and old.get('state') != 'UNKNOWN':
                continue
            previous['semantics'][group] = merge_fact(old, fact, f'{key}/{group}')
    # Separate providers must never repeat an identity. The legacy provider
    # retains existing owners; only missing source-derived records enter this one.
    records = sorted(additions, key=order)
    raw = ('{\n  "schema": "OTERYN_NATIVE_ITEM_PROFILES/v1",\n  "records": [\n'
           + ',\n'.join('    ' + json.dumps(row, sort_keys=True, separators=(',', ':')) for row in records)
           + '\n  ]\n}\n').encode()
    # Publish only after every overlap has passed identity and known-fact checks.
    generated[FRAGMENT] = raw
    profile_raw = encode_native_profile(document)
    if additions != bound:
        generated[profile_path] = profile_raw
    manifest = json.loads(generated[manifest_path])
    if additions != bound:
        manifest['item_profiles']['sha256'] = hashlib.sha256(profile_raw).hexdigest()
    manifest['starting_loadout_item_profiles'] = {
        'path': 'items/definitions/starting-loadout-profiles.json',
        'sha256': hashlib.sha256(raw).hexdigest(),
    }
    generated[manifest_path] = canonical(manifest)
