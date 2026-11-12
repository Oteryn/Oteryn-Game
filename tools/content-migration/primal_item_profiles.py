"""Closed native Primal pod and corpse custody from pinned Crystal Item loaders.

Crystal ff7ede593c69d4c658b382c97443e8155926924a items.xml:71977,
73314-73335 supplies the pod identity and four exact corpse decay stages.
appearances.dat supplies their physical flags; items.cpp:236-251 and
items.hpp:300,384 supply movement, pickup, zero-weight/non-pausing defaults.
The existing source.profile interpreter owns those defaults and temporal refs.
Only these five already-bound identities are admitted, all Ground-only.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import importlib.util
import json

from native_profile_composition import encode_native_profile
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/native-gameplay'))
import build_source_item_profiles as source
from grizzly_shop_profiles import accepted_first, known_conflict

IDS = (39176, 39527, 39528, 39529, 39530)
REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
PATH = 'tools/content-schema/native-gameplay/primal-item-profiles.json'
CAPTURE_SHA256 = 'bb4d9768dddc754796a189856e77aa3d0ca738bc4b85d6f1d8dc49c810ee10d9'
XML_ATTRIBUTES = {
    39176: {},
    39527: {'decayto': '0', 'duration': '60'},
    39528: {'containersize': '15', 'decayto': '39527', 'duration': '300'},
    39529: {'containersize': '30', 'decayto': '39528', 'duration': '300', 'fluidsource': 'blood'},
    39530: {'containersize': '30', 'decayto': '39529', 'duration': '10', 'fluidsource': 'blood'},
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def capture(crystal, root=ROOT):
    pins = source.engine_items.ENGINE_ARTIFACT_DIGESTS[source.engine_items.CRYSTAL_PROFILE]
    blobs = {}
    for path in ('data/items/items.xml', 'data/items/appearances.dat'):
        raw = (crystal / path).read_bytes()
        if hashlib.sha256(raw).hexdigest() != pins[path]:
            raise ValueError('PRIMAL_ITEM_SOURCE_HASH:' + path)
        blobs[path] = raw
    xml = source.xml_index(blobs['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    physical = {row['server_item_id']: row for row in source.policies(
        blobs['data/items/appearances.dat'], blobs['data/items/items.xml'], set(IDS))}
    bindings = source.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    definitions = {row['definition']['identity']['key']: row['definition']
                   for path in (root / 'content/items/definitions').glob('items-*.json')
                   for row in json.loads(path.read_bytes())['records']}
    records, evidence = [], []
    for item in IDS:
        if item not in xml or item not in objects or source.attrs(xml[item]) != XML_ATTRIBUTES[item]:
            raise ValueError('PRIMAL_ITEM_SOURCE_ATTRIBUTES:' + str(item))
        binding = bindings.get(str(item))
        key = f'oteryn:item.tibia.i{item}'
        expected = {'family': 'Item', 'key': key, 'revision': 'definition-r1'}
        if (binding is None or binding['target'] != expected or binding['source_revision'] != REVISION
                or binding['disposition'] != 'EXACT' or binding['source_key'] != 'oteryn:source.crystalserver'
                or binding['identity_namespace'] != 'ots/item_server_id'):
            raise ValueError('PRIMAL_ITEM_BINDING:' + str(item))
        definition = definitions.get(key)
        if definition is None or definition['identity'] != expected:
            raise ValueError('PRIMAL_ITEM_CANONICAL_IDENTITY:' + str(item))
        row = source.profile(item, xml[item], objects[item], physical[item], bindings, None)
        if row['admission'] != {'materializable': True, 'stack_class': 'NonStackable', 'legal_destinations': ['Ground']}:
            raise ValueError('PRIMAL_ITEM_GROUND_CUSTODY:' + str(item))
        accepted = definition.get('semantics', {})
        for group in ('physical', 'stack', 'container', 'temporal'):
            if group in accepted and known_conflict(accepted[group], row['semantics'][group]):
                raise ValueError('PRIMAL_ITEM_ACCEPTED_CONFLICT:' + str(item) + ':' + group)
        row['semantics'] = accepted_first(accepted, row['semantics'])
        row.update(production_binding=copy.deepcopy(binding), production_binding_qualification=None,
                   production_definition={'family': 'Item', 'production_key': key, 'revision_ref': 'definition-r1'})
        records.append(row)
        evidence.append({'item': item, 'xml_attributes': source.attrs(xml[item]),
                         'appearance_flags': objects[item]['flags'], 'physical': physical[item]})
    pod = records[0]
    if (pod['semantics']['physical'] != source.K({'weight': source.K(0), 'movable': source.K(False), 'pickupable': source.K(False)})
            or pod['attributes']['blocks_movement'] or pod['attributes']['blocks_projectile']
            or physical[39176]['block_pathfind'] is not True):
        raise ValueError('PRIMAL_POD_PHYSICAL_CLOSURE')
    return {'schema': 'OTERYN_NATIVE_PRIMAL_ITEM_PROFILES/v1', 'source_revision': REVISION,
            'sources': [{'path': path, 'sha256': pins[path]} for path in blobs],
            'item_ids': list(IDS), 'source_items': evidence, 'records': records}


def load_capture(root=ROOT):
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CAPTURE_SHA256:
        raise ValueError('PRIMAL_ITEM_CAPTURE_DRIFT')
    document = json.loads(raw)
    if (document['schema'] != 'OTERYN_NATIVE_PRIMAL_ITEM_PROFILES/v1'
            or document['source_revision'] != REVISION or document['item_ids'] != list(IDS)
            or [row['production_binding']['external_id'] for row in document['records']] != list(map(str, IDS))):
        raise ValueError('PRIMAL_ITEM_CAPTURE_SET')
    return document


def admit(root, generated, profile_path, manifest_path, order, encode):
    overlay = load_capture(root)
    document = json.loads(generated[profile_path])
    present = {row['production_definition']['production_key']: row for row in document['records']}
    for row in overlay['records']:
        key = row['production_definition']['production_key']
        if key in present:
            current = present[key]
            if current['production_binding'] != row['production_binding']:
                raise ValueError('PRIMAL_ITEM_EXISTING_IDENTITY_CONFLICT')
            if current['admission'] != row['admission']:
                raise ValueError('PRIMAL_ITEM_EXISTING_CUSTODY_CONFLICT')
            for group in ('physical', 'stack', 'container', 'temporal'):
                if known_conflict(current['semantics'][group], row['semantics'][group]):
                    raise ValueError('PRIMAL_ITEM_EXISTING_SEMANTIC_CONFLICT:' + group)
            current['semantics'] = accepted_first(current['semantics'], row['semantics'])
        else:
            document['records'].append(copy.deepcopy(row))
    document['records'].sort(key=order)
    data = encode_native_profile(document)
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = encode(manifest)


COHORT_LEDGER = 'crates/gameplay/dispatch/src/gameplay_transport/equipment_effects_owner_tests/ability_tests.rs'
COHORT_NAME = 'W10-PRIMAL'


def cohort_receipt(text, capture):
    """Generate only this producer's source-bound addition; every other byte stays owned."""
    records = capture['records']
    keys = {row['production_definition']['production_key'] for row in records}
    expected = {f'oteryn:item.tibia.i{item}' for item in IDS}
    if keys != expected or len(records) != len(keys):
        raise ValueError('PRIMAL_COHORT_IDENTITY_SET')
    match = re.search(r'(const PROFILE_ADDITIONS: &\[\(&str, usize\)\] = &\[\n)(.*?)(^\];)',
                      text, re.M | re.S)
    if match is None:
        raise ValueError('PRIMAL_COHORT_LEDGER_SHAPE')
    body = match.group(2)
    entries = list(re.finditer(r'^    \("([^"\n]+)", ([0-9]+)\),\n', body, re.M))
    names = [entry.group(1) for entry in entries]
    if not entries or len(names) != len(set(names)):
        raise ValueError('PRIMAL_COHORT_LEDGER_IDENTITIES')
    marker = '    // W10-PRIMAL: generated by primal_item_profiles.py --refresh-cohort.\n'
    evidence = ('    // Crystal ff7ede59 data/items/items.xml:71977,73314-73335; '
                'Items 39176, 39527..=39530, speed_bonus=0.\n')
    block = marker + evidence + f'    ("{COHORT_NAME}", {len(keys)}),\n'
    if COHORT_NAME in names:
        own = re.compile(re.escape(marker + evidence) + r'    \("W10-PRIMAL", [0-9]+\),\n')
        if len(own.findall(body)) != 1:
            raise ValueError('PRIMAL_COHORT_RECEIPT_OWNERSHIP')
        body = own.sub(lambda _: block, body, count=1)
    else:
        successor = next((index for index, entry in enumerate(entries)
                          if entry.group(1) > COHORT_NAME), len(entries))
        offset = entries[successor - 1].end() if successor else 0
        body = body[:offset] + block + body[offset:]
    return text[:match.start(2)] + body + text[match.end(2):]


def refresh_cohort(root=ROOT, check=False):
    document = load_capture(root)
    path = root / COHORT_LEDGER
    current = path.read_text()
    generated = cohort_receipt(current, document)
    if check:
        if generated != current:
            raise ValueError('PRIMAL_COHORT_RECEIPT_DRIFT')
    elif generated != current:
        path.write_text(generated)
    return {'cohort': COHORT_NAME, 'profiles': len(document['records'])}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path)
    parser.add_argument('--refresh-cohort', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.refresh_cohort:
        if args.crystal is not None:
            parser.error('--refresh-cohort consumes the pinned capture, not donor inputs')
        print(json.dumps(refresh_cohort(check=args.check)))
        raise SystemExit(0)
    if args.crystal is None:
        parser.error('--crystal is required for capture/check')
    raw = canonical(capture(args.crystal))
    digest = hashlib.sha256(raw).hexdigest()
    if args.check:
        if (ROOT / PATH).read_bytes() != raw or digest != CAPTURE_SHA256:
            raise SystemExit('PRIMAL_ITEM_CAPTURE_DRIFT')
    else:
        (ROOT / PATH).write_bytes(raw)
        this = Path(__file__)
        this.write_text(re.sub(r"(?m)^CAPTURE_SHA256 = '[0-9a-z]+'$",
                               "CAPTURE_SHA256 = '" + digest + "'", this.read_text(), count=1))
        Path(importlib.util.cache_from_source(str(this))).unlink(missing_ok=True)
    print(json.dumps({'records': len(json.loads(raw)['records']), 'sha256': digest}))
