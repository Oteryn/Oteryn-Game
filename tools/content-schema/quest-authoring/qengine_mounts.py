"""Source-bound mount population and bindings for existing acquisition owners."""
import json
import re
import xml.etree.ElementTree as ET


def name_key(value):
    return re.sub(r'[^a-z0-9]', '', value.lower().removesuffix('_mount').removesuffix(' (mount)'))


def populate(root, inputs, looks, source, item_packet):
    rows = [row.attrib for row in ET.fromstring(inputs['data/XML/mounts.xml'])]
    by_name = {name_key(row['name']): row for row in rows}
    by_id = {int(row['id']): row for row in rows}
    # Source action config: mounts.lua:70 explicitly maps wind-up loco to mount194.
    by_name['winduploco'] = by_id[194]
    canonical = json.loads((root / 'content/cosmetics/mounts/mounts-00000-00251.json').read_bytes())
    identities = {name_key(r['editor']['display_name']): r['declaration']['identity']['key']
                  for r in canonical['records']}
    rules = json.loads((root / 'content/cosmetics/taming.json').read_bytes())
    stable_keys = {}
    for rule in rules:
        row = by_name.get(name_key(rule['mount_key'].rsplit('.', 1)[1]))
        if row is None:
            raise ValueError('unbound taming mount: ' + rule['mount_key'])
        previous = stable_keys.setdefault(int(row['id']), rule['mount_key'])
        if previous != rule['mount_key']:
            raise ValueError('conflicting retained mount identities')
    mounts, mappings = [], []
    admitted = {}
    for row in rows:
        look = int(row['clientid'])
        if look not in looks:
            raise ValueError('mount appearance not admitted: ' + row['name'])
        key = stable_keys.get(int(row['id'])) or identities.get(name_key(row['name']))
        if key is None:
            key = 'oteryn:content.mount.' + re.sub(r'[^a-z0-9]+', '_', row['name'].lower()).strip('_')
        admitted[int(row['id'])] = key
        mounts.append({'key': key, 'revision': 'crystal-9f5a72c-r1',
                       'look': {'id': look, 'source_record_sha256': looks[look]},
                       'speed_bonus': int(row['speed']), 'premium': row['premium'] == 'yes',
                       'source': source})
    taming_mappings = {}
    for rule in rules:
        mapping = {'grant_key': rule['grant_key'],
                   'target': {'kind': 'Mount', 'mount_key': rule['mount_key']},
                   'compatible_provenance': ['mount-taming-v1']}
        if taming_mappings.setdefault(rule['grant_key'], mapping) != mapping:
            raise ValueError('conflicting mount acquisition grant')
    mappings.extend(taming_mappings.values())
    for grant in item_packet['grants']:
        if grant['kind'] != 'mount':
            continue
        row = by_id[grant['mount_id']]
        if int(row['clientid']) != grant['look_type']:
            raise ValueError('item acquisition mount identity drift')
        mappings.append({'grant_key': grant['key'],
                         'target': {'kind': 'Mount', 'mount_key': admitted[grant['mount_id']]},
                         'compatible_provenance': [item_packet['provenance']]})
    return sorted(mounts, key=lambda r: r['key']), mappings
