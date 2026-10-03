#!/usr/bin/env python3
"""Reuse accepted Item authoring APIs for bounded noncorpse evidence; never admit Items."""
from __future__ import annotations

import argparse
import collections
import hashlib
import importlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile

SCHEMA = 'OTERYN_NONCORPSE_ITEM_AUTHORING_PREPARATION/v1'
SCOPE_SCHEMA = 'OTERYN_CANONICAL_CORPSE_ITEM_CANDIDATE_PREPARATION/v1'
REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
ENGINE_SHA256 = '69e4c228e25775fd17e8652cd6accfad1a7b4780c337354bec90a2a5e7c38626'
VALIDATOR_SHA256 = '9328d4d93e28a4715541b7928311e74537bb51a8f3225fb943feb2cec883e019'
ARTIFACTS = ('data/items/items.xml', 'data/items/appearances.dat',
             'data/scripts/lib/task_board_delivery_items.lua')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_verified(path, expected):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError('input hash mismatch: ' + str(path))
    return json.loads(data)


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def scope_packets(directory, expected, expected_count=46):
    summary = read_verified(directory / 'summary.json', expected)
    if (summary['schema'] != SCOPE_SCHEMA or summary['admission_authorized'] is not False
            or summary['runtime_qualified'] is not False):
        raise ValueError('scope is not canonical evidence-only preparation')
    packets, seen = [], set()
    for relative, digest in sorted(summary['output_sha256'].items()):
        if not re.fullmatch(r'items/[0-9]+\.json', relative):
            raise ValueError('scope packet path is not an Item packet')
        packet = read_verified(directory / relative, digest)
        item_id = packet['source_item_id']
        if (type(item_id) is not int or item_id in seen
                or relative != f'items/{item_id}.json'
                or type(packet['scope_is_corpse']) is not bool
                or packet.get('admission_authorized') is not False
                or packet.get('runtime_qualified') is not False):
            raise ValueError('ambiguous scope identity/classification')
        seen.add(item_id)
        if not packet['scope_is_corpse']:
            packets.append((packet, digest))
    if len(packets) != expected_count:
        raise ValueError('noncorpse scope count drifted')
    return packets


def native_items(document):
    result = {}
    for record in document['records']:
        identity = record['identity']
        if record['kind'] != 'Item' and identity['family'] != 'Item':
            continue
        if record['kind'] != 'Item' or identity['family'] != 'Item':
            raise ValueError('native Item kind/family mismatch')
        if identity['key'] in result or type(record['materializable']) is not bool:
            raise ValueError('ambiguous native Item record')
        result[identity['key']] = record
    return result


def candidate(packet, digest, item, dependencies, report, native, validate):
    binding = packet.get('canonical_binding')
    record = native.get(binding['key']) if binding else None
    if type(report.get('item_id')) is not int or report['item_id'] != packet['source_item_id']:
        raise ValueError('converter source numeric identity mismatch')
    source_binding_proven = report.get('key') is not None
    if source_binding_proven and (not binding or report['key'] != binding['key']):
        raise ValueError('converter source key/native binding mismatch')
    if binding and (binding.get('family') != 'Item' or not record
                    or record['identity']['revision'] != binding.get('revision')):
        raise ValueError('canonical source/native binding mismatch')
    if item and (not source_binding_proven or not binding or item['identity']['revision'] != binding['revision']
                 or item['identity']['key'] != binding['key']):
        raise ValueError('existing converter rebound the source Item')
    errors, warnings = validate(item, dependencies) if item else (None, None)
    status = ('AUTHORING_SCHEMA_INVALID' if errors else 'AUTHORING_SCHEMA_VALID') if item else (
        'ROUTED_TO_ACCEPTED_OTHER_OWNER' if 'routed_non_item' in report and source_binding_proven
        else 'SOURCE_AUTHORING_UNRESOLVED')
    return {'schema': SCHEMA, 'source_item_id': packet['source_item_id'],
            'scope_packet_sha256': digest, 'source_revision': REVISION,
            'canonical_binding': binding, 'current_native_materializable': record['materializable'] if record else None,
            'source_identity_binding_proven': source_binding_proven,
            'item': item, 'dependencies': dependencies, 'report': report,
            'validation_errors': errors, 'validation_warnings': warnings, 'status': status,
            'native_weight_unit': 'UNKNOWN', 'native_temporal_consumption_mode': 'UNKNOWN',
            'native_applied': False, 'admission_authorized': False, 'runtime_qualified': False}


def prepare(repo, crystal, packets, scope_sha256, native_reference, native_sha256, out):
    scope = scope_packets(packets, scope_sha256)
    native = native_items(read_verified(native_reference, native_sha256))
    authoring = repo / 'tools/content-schema/item-authoring'
    inputs = {p: sha(p) for p in authoring.iterdir() if p.suffix in ('.py', '.json')}
    inputs[repo / 'imports/crystalserver/bindings/items.json'] = sha(repo / 'imports/crystalserver/bindings/items.json')
    if (inputs[authoring / 'engine_items.py'] != ENGINE_SHA256
            or inputs[authoring / 'validate_item.py'] != VALIDATOR_SHA256):
        raise ValueError('existing authoring API changed; review the new code first')
    sys.path.insert(0, str(authoring))
    engine, validator = importlib.import_module('engine_items'), importlib.import_module('validate_item')
    if any(Path(module.__file__).resolve().parent != authoring.resolve() for module in (engine, validator)):
        raise ValueError('unexpected imported authoring API location')
    with tempfile.TemporaryDirectory(prefix='oteryn-item-reference-') as temporary:
        source = Path(temporary)
        for relative in ARTIFACTS:
            path = source / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(subprocess.check_output(['git', '-C', str(crystal), 'show', REVISION + ':' + relative]))
        # No digest overrides: the existing API verifies every pinned source artifact.
        sources = engine.load_engine_sources('crystal', source)
    results = []
    for packet, digest in scope:
        item, dependencies, report = engine.convert_item(sources, packet['source_item_id'])
        results.append(candidate(packet, digest, item, dependencies, report, native, validator.validate))
    if any(sha(path) != digest for path, digest in inputs.items()):
        raise ValueError('authoring inputs changed during preparation')
    scope_packets(packets, scope_sha256)
    read_verified(native_reference, native_sha256)
    hashes = {}
    for result in results:
        relative = f"items/{result['source_item_id']}.json"
        write(out / relative, result)
        hashes[relative] = sha(out / relative)
    counts = dict(collections.Counter(result['status'] for result in results))
    summary = {'schema': SCHEMA, 'counts': counts, 'scope_count': len(results),
               'source_revision': REVISION, 'source_artifact_digests': sources['artifact_digests'],
               'source_access': 'Local Git objects; temporary raw reference files are not redistributed.',
               'input_sha256': {str(packets / 'summary.json'): scope_sha256, str(native_reference): native_sha256,
                                **{str(path): digest for path, digest in inputs.items()}},
               'generator_sha256': sha(Path(__file__)), 'output_sha256': hashes,
               'native_weight_unit': 'UNKNOWN', 'native_temporal_consumption_mode': 'UNKNOWN',
               'native_applied': False, 'admission_authorized': False, 'runtime_qualified': False,
               'boundaries': ['Schema validity does not resolve source report blockers or native Item admission.',
                              'Existing family decisions are reused; no portable corpse family profile is invented.',
                              'D3 corpse policy and capacity16 are unchanged; source capacities remain source facts.']}
    write(out / 'summary.json', summary)
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('repo', 'crystal', 'packets', 'native-reference', 'out'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('scope-sha256', 'native-sha256'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    result = prepare(args.repo, args.crystal, args.packets, args.scope_sha256,
                     args.native_reference, args.native_sha256, args.out)
    print(json.dumps(result['counts']))
