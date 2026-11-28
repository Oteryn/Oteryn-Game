"""Source-bound mount population and bindings for existing acquisition owners."""
import hashlib
import json
import re
import xml.etree.ElementTree as ET


MOUNTS_XML = 'data/XML/mounts.xml'
CATALOGUE_REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
CATALOGUE_INPUT = CATALOGUE_REVISION + ':' + MOUNTS_XML


def append_pegasus_feather(packet):
    # Crystal declares Pegasus in mounts.xml but has no feather action. The retained
    # TibiaWiki item revision 1194180 describes self-use and achievement 552 records it.
    donor = 'https://tibia.fandom.com/wiki/Special:PermanentLink/1194180'
    grant = 'oteryn:item-grant.mount.227'
    packet['grants'].append({'key': grant, 'kind': 'mount', 'mount_id': 227,
                             'look_type': 1730, 'donor': donor})
    packet['rules'].append({'item_id': 48424, 'donor': donor, 'mode': 'direct',
                            'premium': False, 'choices': [{
                                'grants': [grant], 'owned': grant, 'required': [], 'cues': [],
                                'achievements': ['oteryn:achievement/i_wanna_fly_away'],
                            }]})
    packet['grants'].sort(key=lambda row: row['key'])
    packet['rules'].sort(key=lambda row: row['item_id'])


def append_world_items(packet):
    # Global names the exact Key and Loco. Crystal (Global silent) mounts.lua:240-250
    # consumes both, while the source config at line 70 supplies the success message.
    donor = 'https://tibia.fandom.com/wiki/Gloothomotive'
    grant = 'oteryn:item-grant.mount.194'
    packet['grants'].append({'key': grant, 'kind': 'mount', 'mount_id': 194,
                             'look_type': 1459, 'donor': donor})
    packet['rules'].append({'item_id': 37397, 'donor': donor, 'mode': 'pieces',
                            'premium': False, 'target_item_id': 37398,
                            'required_items': [37397, 37398], 'choices': [{
                                'grants': [grant], 'owned': grant, 'required': [], 'cues': [],
                                'feedback': 'You wind the small locomotive up. It starts to move ... and grows!',
                                'achievements': ['oteryn:achievement/engine_driver',
                                                 'oteryn:achievement/natural_born_cowboy'],
                            }]})
    donor = 'https://tibia.fandom.com/wiki/Demon_in_a_Golden_Box'
    grant = 'oteryn:item-grant.mount.239'
    packet['grants'].append({'key': grant, 'kind': 'mount', 'mount_id': 239,
                             'look_type': 1827, 'donor': donor})
    packet['rules'].append({'item_id': 50063, 'donor': donor, 'mode': 'direct',
                            'premium': False, 'choices': [{
                                'grants': [grant], 'owned': grant, 'required': [], 'cues': [],
                                'achievements': ['oteryn:achievement/alpha_rider'],
                            }]})
    packet['grants'].sort(key=lambda row: row['key'])
    packet['rules'].sort(key=lambda row: row['item_id'])


def read_catalogue_xml(corpus, manifest):
    # The 201189f8 Git tree names the same blob as the retained Crystal Summer XML.
    row = next(row for row in manifest['files'] if row['path'] == MOUNTS_XML
               and row['git_blob_sha1'] == '1c93421c1141265988eb32afb9c56238eae30716')
    raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
    if hashlib.sha256(raw).hexdigest() != row['sha256']:
        raise ValueError('mount catalogue XML source drift')
    return raw


def name_key(value):
    return re.sub(r'[^a-z0-9]', '', value.lower().removesuffix('_mount').removesuffix(' (mount)'))


def populate(root, inputs, looks, source, item_packet):
    rows = [row.attrib for row in ET.fromstring(inputs[MOUNTS_XML])]
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
    def definition(row, key, citation):
        look = int(row['clientid'])
        # Global's Dreadhare and Wolpertinger pages name opposite looks to the
        # Crystal XML. Keep the production identities and correct the visuals.
        look = {'Dreadhare': 907, 'Wolpertinger': 906}.get(row['name'], look)
        if look not in looks:
            raise ValueError('mount appearance not admitted: ' + row['name'])
        if key is None:
            key = 'oteryn:content.mount.' + re.sub(r'[^a-z0-9]+', '_', row['name'].lower()).strip('_')
        return {'key': key, 'revision': 'crystal-' + citation['revision'][:7] + '-r1',
                'look': {'id': look, 'source_record_sha256': looks[look]},
                'speed_bonus': int(row['speed']), 'premium': row['premium'] == 'yes',
                'source': citation}

    mounts, mappings = [], []
    admitted = {}
    for row in rows:
        key = stable_keys.get(int(row['id'])) or identities.get(name_key(row['name']))
        mount = definition(row, key, source)
        admitted[int(row['id'])] = mount['key']
        mounts.append(mount)
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
    # The current ordinals overlap retained acquisition inputs (Reliable Ram was 250).
    raw = inputs[CATALOGUE_INPUT]
    catalogue_source = {'reference': 'crystal:' + MOUNTS_XML, 'revision': CATALOGUE_REVISION,
                        'sha256': hashlib.sha256(raw).hexdigest()}
    catalogue_rows = {row.get('name'): row.attrib for row in ET.fromstring(raw)}
    for name in ('Jaracal', 'Radiant Nimbus'):
        mounts.append(definition(catalogue_rows[name], identities.get(name_key(name)), catalogue_source))
    # Crystal 201189f data/scripts/lib/task_board_shop.lua:95-126. The existing
    # shop owner resolves the look in the admitted catalogue before it spends points.
    for offer, mount_id, look, slug in (
        (6, 247, 1908, 'tidal_seawater_predator'),
        (7, 248, 1909, 'ashen_coast_predator'),
        (8, 249, 1910, 'crimson_bay_predator'),
        (9, 163, 1281, 'antelope'),
    ):
        key = f'oteryn:content.mount.{slug}'
        if int(by_id[mount_id]['clientid']) != look or [
            row['key'] for row in mounts if row['look']['id'] == look
        ] != [key] or admitted[mount_id] != key:
            raise ValueError('Task Board mount look drift')
        mappings.append({'grant_key': f'oteryn:taskboard/shop/offer-{offer}',
                         'target': {'kind': 'Mount', 'mount_key': admitted[mount_id]},
                         'compatible_provenance': ['task-board-shop-v1']})
    return sorted(mounts, key=lambda r: r['key']), mappings
