"""Admit fixed native traversal Items from pinned Item source facts.

Canary04b83 items.xml:14552-14559 declares5111/5112 as doors and the
closed state's levelDoor base1000. The existing native DoorBook owns pair,
placed-action level predicates, toggling and step-off closure. No rule is
inferred from missing metadata, and no coordinate or identity is allocated.
The same pinned XML:3918-3920 identifies ladder1948; exact client flags
prove walkable, immovable, non-pickup Ground custody. LADDER is an existing
closed native Item class, not a new name-based interaction or floor flag.
XML:3517-3519 declares open archways1626/1627, XML:43330 grass21935.
Pinned client flags, not those names, prove their static walkable collision.
The same capture admits passable buttresses1634/1635 (XML:3544), the final
original entrance frontier; no walls or collision flags are overridden.
XML:5314-5316 declares red carpet2572-2580. Their exact pinned client flags
prove immovable, non-pickup, nonblocking floor decorations; the importer
provides their missing native facts rather than changing source geometry.
"""
import hashlib
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
import grizzly_shop_profiles as accepted

CLASSES = {1626: None, 1627: None, 1634: None, 1635: None, 1948: "ladder",
           2572: None, 2573: None, 2574: None, 2575: None, 2576: None,
           2577: None, 2578: None, 2579: None, 2580: None,
           5111: "door", 5112: "door", 21935: None}
IDS = set(CLASSES)
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
        expected_class = CLASSES[item]
        if source.attrs(xml[item]).get('type') != expected_class:
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
        if (record['semantics']['classification']['value']['item_type'] != (source.K(expected_class.upper()) if expected_class else source.NA)
                or record['admission']['legal_destinations'] != ['Ground']):
            raise ValueError('SOURCE_GATE_ITEM_NOT_STATIC_CLASS')
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
                from questpc2_ladder_composition import compose
                combined = compose(present[key], record)
                if combined is None:
                    raise ValueError('SOURCE_GATE_ITEM_EXISTING_CONFLICT')
                present[key].clear()
                present[key].update(combined)
        else:
            document['records'].append(record)
    document['records'].sort(key=lambda r: (r['production_definition']['family'],
                                           r['production_definition']['production_key']))
    raw = canonical(document)
    generated[PROFILE] = raw
    manifest = json.loads(generated[MANIFEST])
    manifest['item_profiles']['sha256'] = hashlib.sha256(raw).hexdigest()
    generated[MANIFEST] = canonical(manifest)
