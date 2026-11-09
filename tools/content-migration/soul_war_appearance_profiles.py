"""Retain exactly two source-qualified W11 callback outfits in native generation."""
import hashlib
import json

SOURCE = 'tools/content-schema/native-gameplay/srx3-soul-war-appearances.json'
SOURCE_SHA256 = '96c600071b4d4dcbbd44a853cfd1138efb40674a7742554376098fab5f6e8e0c'


def admit(root, generated, destination, manifest_path):
    raw = (root / SOURCE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError('SOUL_WAR_CALLBACK_APPEARANCE_SOURCE_CHANGED')
    fragment = json.loads(raw)
    if (fragment['schema'] != 'OTERYN_SOUL_WAR_CALLBACK_APPEARANCES/v1'
            or fragment['source_revision'] != '201189f871d23597b82caf88fd611454a2ddefb8'
            or len(fragment['records']) != 2
            or {r['look_type'] for r in fragment['records']} != {242, 286}):
        raise ValueError('SOUL_WAR_CALLBACK_APPEARANCE_SOURCE')
    document = json.loads(generated[destination])
    keys = {r['outfit_key'] for r in fragment['records']}
    existing = [r for r in document['records'] if r['outfit_key'] in keys]
    if existing and existing != fragment['records']:
        raise ValueError('SOUL_WAR_CALLBACK_APPEARANCE_CONFLICT')
    document['records'] = [r for r in document['records'] if r['outfit_key'] not in keys] + fragment['records']
    generated[destination] = (json.dumps(document, ensure_ascii=False, indent=2) + '\n').encode()
    manifest = json.loads(generated[manifest_path])
    manifest.setdefault('spell_appearances', {})['sha256'] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = (json.dumps(manifest, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()
