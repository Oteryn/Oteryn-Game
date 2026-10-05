#!/usr/bin/env python3
"""Evidence-only compiler for literal NPC markers in the pinned official client map.

This is a source observation schema, NOT a WorldProject record or placement.
No target identity, coordinate frame, appearance, or motion is allocated here.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import re
from pathlib import Path

def repository_root():
    for base in (*Path(__file__).resolve().parents, Path.cwd(), Path.cwd() / 'Oteryn-Game'):
        if (base / 'tools/content-schema/world-authoring/client_map_reader.py').is_file():
            return base
    raise RuntimeError('run inside the repository or copy helper to tools/content-schema/npc-authoring')


ROOT = repository_root()
HERE = Path(__file__).resolve().parent
SOURCE = ROOT / 'content/assets/files/map-c54dfeb8f3880e9534f1330fa547f06d99c5afa55f641b875ceee23cc04e3496.dat'
MANIFEST = ROOT / 'imports/official/client-assets/15.30/manifest.json'
PARSER = ROOT / 'tools/content-schema/world-authoring/client_map_reader.py'
SCHEMA = 'OTERYN_OFFICIAL_CLIENT_NPC_MAP_MARKER_SOURCE_FACTS/v1'
SOURCE_SHA256 = 'c54dfeb8f3880e9534f1330fa547f06d99c5afa55f641b875ceee23cc04e3496'
MANIFEST_SHA256 = 'febaff9f4bd7e0f8a029736e446a81f1626805e895e7c2268018e0a9a8493fe4'
spec = importlib.util.spec_from_file_location('official_map_reader', PARSER)
reader = importlib.util.module_from_spec(spec)
import sys
sys.modules[spec.name] = reader
spec.loader.exec_module(reader)


class MarkerError(ValueError):
    pass


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode()


def wire_fields(data, absolute_start=0):
    """Strict bounded wire reader preserving full field and payload byte spans."""
    at = 0
    while at < len(data):
        start = at
        key, at = reader.varint(data, at)
        number, wire = key >> 3, key & 7
        if number == 0:
            raise MarkerError('zero field number')
        if wire == 0:
            payload_start = at
            value, at = reader.varint(data, at)
        elif wire == 2:
            size, at = reader.varint(data, at)
            payload_start = at
            if size > len(data) - at:
                raise MarkerError('truncated length-delimited field')
            value = data[at:at + size]
            at += size
        elif wire in (1, 5):
            size = 8 if wire == 1 else 4
            payload_start = at
            if size > len(data) - at:
                raise MarkerError('truncated fixed-width field')
            value = data[at:at + size]
            at += size
        else:
            raise MarkerError(f'unsupported wire type {wire}')
        yield {'number': number, 'wire': wire, 'value': value,
               'byte_start': absolute_start + start, 'byte_end': absolute_start + at,
               'payload_start': absolute_start + payload_start}


def exact_message(data, expected, absolute_start=0):
    fields = list(wire_fields(data, absolute_start))
    if [(f['number'], f['wire']) for f in fields] != expected:
        raise MarkerError('source message shape changed')
    return fields


def proof(data, field):
    start, end = field['byte_start'], field['byte_end']
    return {'byte_start': start, 'byte_end': end, 'byte_range_convention': 'half-open',
            'fragment_sha256': digest(data[start:end])}


def qualified_source_names(qualified_data, targets):
    """Only explicit NPC detail image names; never guessed wiki-title suffix removal."""
    if qualified_data is None:
        return {}
    qualification = json.loads(qualified_data)
    if qualification['schema'] != 'OTERYN_NPC_LITERAL_SOURCE_NAME_QUALIFICATIONS/v1':
        raise MarkerError('unknown qualified-name schema')
    target_keys = {t['key'] for t in targets}
    result = {}
    for row in qualification['records']:
        if row['target_proposal_key'] not in target_keys:
            raise MarkerError('qualified name refers to an unlisted proposal')
        p = row['proof']
        source_bytes = Path(p['path']).read_bytes()
        if digest(source_bytes) != p['sha256']:
            raise MarkerError('qualified-name capture digest mismatch')
        start, end = p['byte_start'], p['byte_end']
        if not (0 <= start < end <= len(source_bytes)):
            raise MarkerError('qualified-name byte range invalid')
        fragment = source_bytes[start:end]
        if digest(fragment) != p['fragment_sha256']:
            raise MarkerError('qualified-name fragment digest mismatch')
        from html import unescape
        tag = fragment.decode('utf8')
        attrs = {k: unescape(v) for k, v in re.findall(r'([\w-]+)="([^"]*)"', tag)}
        if not re.fullmatch(r'<img\s[^>]+/?>', tag, re.S) or 'npc_name' not in attrs.get('class', '').split():
            raise MarkerError('qualification is not an explicit NPC image name')
        if attrs.get('alt') != row['source_name'] or attrs.get('title') != row['source_name']:
            raise MarkerError('explicit alt and title names do not agree')
        result.setdefault(row['target_proposal_key'], []).append(row)
    return result


def compile_packet(data, target_data, manifest_bytes, source_path=SOURCE, target_path=None,
                   manifest_path=MANIFEST, qualified_data=None, qualified_path=None):
    if digest(data) != SOURCE_SHA256 or digest(manifest_bytes) != MANIFEST_SHA256:
        raise MarkerError('source is outside the pinned official 15.30 map/manifest')
    manifest = json.loads(manifest_bytes)
    source_entries = [f for f in manifest['files'] if f['name'] == source_path.name]
    if len(source_entries) != 1 or source_entries[0]['sha256'] != digest(data) or source_entries[0]['bytes'] != len(data):
        raise MarkerError('official manifest does not pin source bytes')
    targets = json.loads(target_data)
    if len({t['key'] for t in targets}) != len(targets):
        raise MarkerError('duplicate target key')
    qualifications = qualified_source_names(qualified_data, targets)
    by_name = {}
    for target in targets:
        by_name.setdefault(target['name'].casefold(), {})[target['key']] = (target, None)
        for qualification in qualifications.get(target['key'], []):
            by_name.setdefault(qualification['source_name'].casefold(), {})[target['key']] = (target, qualification)
    areas, all_markers = {}, []
    marker_ordinal = 0
    for field in wire_fields(data):
        if field['number'] not in (1, 2, 3, 4, 5) or field['wire'] != 2:
            raise MarkerError('unrecognized top-level map field')
        if field['number'] == 1:
            area = reader.read_area(field['value'])
            if area.id in areas:
                raise MarkerError('duplicate source area id')
            areas[area.id] = {'source_area_id': area.id, 'source_area_name': area.name,
                              'proof': proof(data, field)}
        if field['number'] != 2:
            continue
        marker_ordinal += 1
        name_f, position_f, code_f = exact_message(field['value'], [(1, 2), (2, 2), (3, 0)], field['payload_start'])
        position_fields = exact_message(position_f['value'], [(1, 0), (2, 0), (3, 0)], position_f['payload_start'])
        x, y, z = [p['value'] for p in position_fields]
        if not (0 <= x <= 65535 and 0 <= y <= 65535 and 0 <= z <= 15):
            raise MarkerError('out-of-range source position')
        marker = {'source_marker_ordinal': marker_ordinal,
                  'source_name': reader.text(name_f['value'], 'marker name'),
                  'position': {'x': x, 'y': y, 'floor': z},
                  'source_field3_code': code_f['value'],
                  'proof': proof(data, field),
                  'field_proofs': {'name': proof(data, name_f), 'position': proof(data, position_f),
                                   'field3_code': proof(data, code_f)}}
        all_markers.append(marker)
    # Existing parser agrees on counts and extent; opaque field3 is never an outfit binding.
    parsed = reader.read_map(data)
    if len(all_markers) != parsed.markers:
        raise MarkerError('existing official map parser disagrees with count')
    observations, matched_names = [], set()
    for marker in all_markers:
        matches = list(by_name.get(marker['source_name'].casefold(), {}).values())
        if len(matches) > 1:
            raise MarkerError('multiple case-equal proposed identity names')
        if not matches:
            continue
        target, qualification = matches[0]
        matched_names.add(target['key'])
        observations.append({'kind': 'NpcMapMarkerSourceObservation',
            'target_proposal_key': target['key'], 'target_proposal_name': target['name'],
            'name_match': ('QUALIFIED_LITERAL_SOURCE_NAME' if qualification else
                          'LITERAL_EXACT' if marker['source_name'] == target['name'] else 'LITERAL_CASE_ONLY'),
            'name_qualification': qualification,
            'identity_disposition': 'EXISTING_NATIVE_VARIANTS_ONLY' if target['key'] == 'oteryn:npc.a_bloodshade' else 'SOURCE_PROPOSAL_ONLY',
            **marker,
            'field3_same_integer_area_observation': areas.get(marker['source_field3_code'])})
    observations.sort(key=lambda o: (o['target_proposal_key'], o['source_marker_ordinal']))
    return {'schema': SCHEMA,
        'source': {'path': str(source_path), 'sha256': digest(data), 'bytes': len(data),
                   'official_manifest_path': str(manifest_path), 'official_manifest_sha256': digest(manifest_bytes),
                   'official_archive_sha256': manifest['archive_sha256'], 'client_version': '15.30',
                   'target_source_path': str(target_path) if target_path else None, 'target_source_sha256': digest(target_data),
                   'qualified_names_source_path': str(qualified_path) if qualified_path else None,
                   'qualified_names_source_sha256': digest(qualified_data) if qualified_data else None,
                   'parser_path': str(PARSER), 'parser_sha256': digest(PARSER.read_bytes())},
        'authority': {'evidence_only': True, 'canonical_identity_allocation': False,
                      'world_project_admission': False, 'placement_admission': False, 'runtime_qualified': False},
        'source_schema': {'marker_field1': 'utf8_name', 'marker_field2': 'protobuf_xyz',
                          'marker_field3': 'opaque_varint_with_same_integer_area_join',
                          'appearance_or_movement_fields': []},
        'unknown_facts': ['presentation_asset_binding', 'movement_can_walk', 'movement_interval_ms',
                          'movement_radius_tiles', 'world_key', 'map_revision', 'coordinate_frame',
                          'spawn_interval', 'direction'],
        'summary': {'source_marker_count': len(all_markers), 'target_count': len(targets),
                    'matched_target_count': len(matched_names), 'matched_marker_count': len(observations),
                    'exact_literal_marker_count': sum(o['name_match'] == 'LITERAL_EXACT' for o in observations),
                    'case_only_marker_count': sum(o['name_match'] == 'LITERAL_CASE_ONLY' for o in observations),
                    'qualified_source_name_marker_count': sum(o['name_match'] == 'QUALIFIED_LITERAL_SOURCE_NAME' for o in observations),
                    'all_marker_field3_codes_have_same_integer_area': all(m['source_field3_code'] in areas for m in all_markers)},
        'observations': observations,
        'unmatched_targets': [{'proposal_key': t['key'], 'source_name': t['name']} for t in targets if t['key'] not in matched_names]}


def build(target_path, source_path=SOURCE, manifest_path=MANIFEST, qualified_path=None):
    return compile_packet(source_path.read_bytes(), target_path.read_bytes(), manifest_path.read_bytes(),
                          source_path, target_path, manifest_path,
                          qualified_path.read_bytes() if qualified_path else None, qualified_path)


def validate(packet, **paths):
    if packet != build(**paths):
        raise MarkerError('packet differs from byte-pinned deterministic recompile')
    return True


def main():
    cli = argparse.ArgumentParser()
    cli.add_argument('--targets', type=Path, required=True)
    cli.add_argument('--map', dest='source_path', type=Path, default=SOURCE)
    cli.add_argument('--manifest', type=Path, default=MANIFEST)
    cli.add_argument('--qualified-names', type=Path)
    cli.add_argument('--output', type=Path)
    cli.add_argument('--validate', type=Path)
    args = cli.parse_args()
    paths = {'target_path': args.targets, 'source_path': args.source_path, 'manifest_path': args.manifest,
             'qualified_path': args.qualified_names}
    if args.validate:
        validate(json.loads(args.validate.read_bytes()), **paths)
        print('VALID evidence-only marker packet')
    else:
        if not args.output:
            cli.error('--output or --validate is required')
        packet = build(**paths)
        args.output.write_bytes(canonical(packet))
        print(json.dumps({'output': str(args.output), 'sha256': digest(args.output.read_bytes()), **packet['summary']}))


if __name__ == '__main__':
    main()
