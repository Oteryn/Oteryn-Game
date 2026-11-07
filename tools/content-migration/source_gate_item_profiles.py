"""Admit existing native expertise-gate pairs from pinned Item source facts.

Canary04b83 items.xml:14552-14559 declares5111/5112 as doors and the
closed state's levelDoor base1000. The existing native DoorBook owns pair,
placed-action level predicates, toggling and step-off closure. No rule is
inferred from missing metadata, and no coordinate or identity is allocated.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

IDS = {5111, 5112}
XML = 'imports/canary/items-xml/items.xml'
XML_SHA = '1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2'
CLIENT_SHA = '2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2'
CLIENT = f'content/assets/files/appearances-{CLIENT_SHA}.dat'
BINDINGS = 'imports/crystalserver/bindings/items.json'
BINDINGS_SHA = '1d19709c28a649d8a9f32bf53e5bf43ad847d7901d6c6254833f456154d2de5a'
PROFILE = 'content/items/definitions/spell-native-profiles.json'
MANIFEST = 'content/spells.manifest.json'
PROOF = 'tools/content-schema/native-gameplay/source-gate-item-profiles.json'


def capture(root=ROOT):
    raw = {}
    for path, sha in [(XML, XML_SHA), (CLIENT, CLIENT_SHA), (BINDINGS, BINDINGS_SHA)]:
        raw[path] = (root / path).read_bytes()
        if hashlib.sha256(raw[path]).hexdigest() != sha:
            raise ValueError('SOURCE_GATE_ITEM_PIN')
    xml = source.xml_index(raw[XML])
    objects = source.engine_items.load_appearance_objects(raw[CLIENT])
    physical = {r['server_item_id']: r for r in source.policies(raw[CLIENT], raw[XML], IDS)}
    bindings = source.binding_index(raw[BINDINGS])
    definitions = {r['definition']['identity']['key']: r['definition']
                   for p in (root / 'content/items/definitions').glob('items-*.json')
                   for r in json.loads(p.read_bytes())['records']
                   if r['definition']['identity']['key'] in {f'oteryn:item.tibia.i{i}' for i in IDS}}
    records = []
    for item in sorted(IDS):
        if source.attrs(xml[item]).get('type') != 'door':
            raise ValueError('SOURCE_GATE_ITEM_CLASS')
        if item == 5111 and source.attrs(xml[item]).get('leveldoor') != '1000':
            raise ValueError('SOURCE_GATE_ITEM_LEVEL_BASE')
        record = source.profile(item, xml[item], objects[item], physical[item], bindings, None)
        canonical = definitions[f'oteryn:item.tibia.i{item}']
        semantics = canonical.get('semantics', {})
        for group in ('physical', 'stack', 'container'):
            if accepted.known_conflict(semantics.get(group, {'state': 'UNKNOWN'}), record['semantics'][group]):
                raise ValueError('SOURCE_GATE_ITEM_CANONICAL_CONFLICT')
        record['semantics'] = accepted.accepted_first(semantics, record['semantics'])
        if (record['semantics']['classification']['value']['item_type'] != source.K('DOOR')
                or record['admission']['legal_destinations'] != ['Ground']):
            raise ValueError('SOURCE_GATE_ITEM_NOT_STATIC_DOOR')
        records.append(record)
    records = source.bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1',
                                    'records': records}, bindings, IDS)['records']
    return {'schema': 'OTERYN_SOURCE_GATE_ITEM_PROFILES/v1',
            'sources': [{'path': XML, 'sha256': XML_SHA}, {'path': CLIENT, 'sha256': CLIENT_SHA},
                        {'path': BINDINGS, 'sha256': BINDINGS_SHA}], 'records': records}


def admit(root, generated, canonical):
    packet = capture(root)
    generated[PROOF] = (json.dumps(packet, indent=2, ensure_ascii=False) + '\n').encode()
    document = json.loads(generated[PROFILE])
    present = {r['production_definition']['production_key']: r for r in document['records']}
    for record in packet['records']:
        key = record['production_definition']['production_key']
        if key in present:
            if present[key] != record:
                raise ValueError('SOURCE_GATE_ITEM_EXISTING_CONFLICT')
        else:
            document['records'].append(record)
    document['records'].sort(key=lambda r: (r['production_definition']['family'],
                                           r['production_definition']['production_key']))
    raw = canonical(document)
    generated[PROFILE] = raw
    manifest = json.loads(generated[MANIFEST])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[MANIFEST] = canonical(manifest)
