"""Promote taming identities only from retained mount XML and admitted appearance records."""
import hashlib
import json
import tarfile
import xml.etree.ElementTree as ET

# Crystal mounts.lua maps item 37397 ('wind-up loco') to mount ID 194,
# whose XML display name is Gloothomotive. Preserve the existing taming key.
ALIASES = {'giant_beaver_mount': 'giant_beaver', 'foxmouse_mount': 'foxmouse',
           'wind_up_loco': 'gloothomotive'}
REVISION = '00ce02a57ca5a12e48f32a3476e37471167e4c3f'


def append(root, document, looks):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        source = next(row for row in manifest['files'] if row['source'] == 'crystal-summer'
                      and row['revision'] == REVISION and row['path'] == 'data/XML/mounts.xml')
        raw = corpus.extractfile(source.get('blob_path', source.get('cache_path'))).read()
    if hashlib.sha256(raw).hexdigest() != source['sha256']:
        raise ValueError('taming mount XML source drift')
    citation = {'reference': 'crystal:data/XML/mounts.xml', 'revision': REVISION, 'sha256': source['sha256']}
    rows = {row.get('name').lower().replace(' ', '_').replace('-', '_'): row for row in ET.fromstring(raw)}
    existing = {row['key'] for row in document['mounts']}
    rules = json.loads((root / 'content/cosmetics/taming.json').read_bytes())
    for rule in sorted(rules, key=lambda row: row['mount_key']):
        key = rule['mount_key']
        if key in existing:
            continue
        name = key.removeprefix('oteryn:content.mount.')
        row = rows[ALIASES.get(name, name)]
        look = int(row.get('clientid'))
        if look not in looks:
            raise ValueError('taming mount appearance unavailable: ' + key)
        document['mounts'].append({'key': key, 'revision': 'taming-cosmetics-r1',
                                  'look': {'id': look, 'source_record_sha256': looks[look]},
                                  'speed_bonus': int(row.get('speed')), 'premium': row.get('premium') == 'yes',
                                  'source': citation})
        document['grant_mappings'].append({'grant_key': rule['grant_key'],
                                          'target': {'kind': 'Mount', 'mount_key': key},
                                          'compatible_provenance': ['mount-taming-v1']})
        existing.add(key)
    return citation
