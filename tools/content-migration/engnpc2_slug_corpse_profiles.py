"""Exact native Slug corpse/decay custody from the already pinned item sources.

No USE action or carried loot permission is inferred. The existing corpse and
native temporal owners retain mutation authority.
"""
import hashlib
import json
import qengine_reward_item_profiles as shared

ROOT = shared.ROOT
PATH = 'tools/content-schema/native-gameplay/engnpc2-slug-corpse-profiles.json'
IDS = (12526, 12533, 12534, 12535)


def capture(root=ROOT):
    xml_raw = (root / shared.XML).read_bytes()
    client_raw = (root / shared.CLIENT).read_bytes()
    if (hashlib.sha256(xml_raw).hexdigest() != shared.XML_SHA
            or hashlib.sha256(client_raw).hexdigest() != shared.CLIENT_SHA):
        raise ValueError('ENGNPC2_CORPSE_SOURCE_PIN')
    source = shared.source
    xml = source.xml_index(xml_raw)
    objects = source.engine_items.load_appearance_objects(client_raw)
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    policy = {r['server_item_id']: r for r in source.policies(client_raw, xml_raw, set(IDS))}
    observed = []
    item = IDS[0]
    while item:
        if item in observed or item not in IDS:
            raise ValueError('ENGNPC2_CORPSE_DECAY_CLOSURE')
        observed.append(item)
        item = int(source.attrs(xml[item]).get('decayto', '0'))
    if tuple(observed) != IDS:
        raise ValueError('ENGNPC2_CORPSE_DECAY_ORDER')
    keys = {f'oteryn:item.tibia.i{i}' for i in IDS}
    definitions = {r['definition']['identity']['key']: r['definition']
                   for p in (root / 'content/items/definitions').glob('items-*.json')
                   for r in json.loads(p.read_bytes())['records']
                   if r['definition']['identity']['key'] in keys}
    if definitions.keys() != keys:
        raise ValueError('ENGNPC2_CORPSE_CANONICAL_SET')
    records = []
    for item in IDS:
        row = source.profile(item, xml[item], objects[item], policy[item], bindings, None)
        key = f'oteryn:item.tibia.i{item}'
        if row['admission']['legal_destinations'] != ['Ground']:
            raise ValueError('ENGNPC2_CORPSE_GROUND_ONLY')
        for group in ('physical', 'stack', 'container', 'temporal'):
            if shared.accepted.known_conflict(definitions[key].get('semantics', {}).get(group, source.U), row['semantics'][group]):
                raise ValueError('ENGNPC2_CORPSE_CANONICAL_CONFLICT:' + group)
        row['semantics'] = shared.accepted.accepted_first(definitions[key].get('semantics', {}), row['semantics'])
        records.append(row)
    records = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, set(IDS))['records']
    return {'schema': 'OTERYN_ENGNPC2_SLUG_CORPSE_PROFILES/v1',
            'producer': 'tools/content-migration/engnpc2_slug_corpse_profiles.py',
            'sources': [{'path': shared.XML, 'sha256': shared.XML_SHA},
                        {'path': shared.CLIENT, 'sha256': shared.CLIENT_SHA}],
            'scope': 'Exact Slug corpse and native decay chain; Ground custody only; no USE action.',
            'records': records}


def admit(root, generated, profile_path, manifest_path, order, canonical):
    packet = capture(root)
    generated[PATH] = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    shared.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])


def refresh_cohort(source, packet):
    """Declare the independently source-checked four-body cohort in the Rust pin."""
    import re
    if (packet['schema'] != 'OTERYN_ENGNPC2_SLUG_CORPSE_PROFILES/v1'
            or [r['production_definition']['production_key'] for r in packet['records']]
            != [f'oteryn:item.tibia.i{i}' for i in IDS]):
        raise ValueError('ENGNPC2_CORPSE_PIN_COHORT')
    line = f'    ("ENGNPC2-SLUG-CORPSES", {len(IDS)}),'
    pattern = r'^    \("ENGNPC2-SLUG-CORPSES", \d+\),$'
    matches = list(re.finditer(pattern, source, re.MULTILINE))
    if len(matches) > 1:
        raise ValueError('ENGNPC2_CORPSE_DUPLICATE_COHORT')
    if matches:
        return source[:matches[0].start()] + line + source[matches[0].end():]
    anchor = 'const PROFILE_ADDITIONS: &[(&str, usize)] = &[\n'
    if source.count(anchor) != 1:
        raise ValueError('ENGNPC2_CORPSE_PIN_ANCHOR')
    return source.replace(anchor, anchor + line + '\n', 1)
