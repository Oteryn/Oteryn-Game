#!/usr/bin/env python3
"""Pin actual source-B3 Hare observer frames; never construct protocol bytes."""
import argparse
import hashlib
import json
from seasonal_capture_fields import metadata
from pathlib import Path

B3 = '6ca7aa92d56ae6175c091bc8476817e5d8cadfa9ca4bc10af5dc2a8905b6b047'
SCHEMA = 'OTERYN_SEASONAL_HARE_NATIVE_CAPTURE/v1'
MAX_BYTES = 4 * 1024 * 1024

def framed(raw, digest):
    if not isinstance(raw, str) or len(raw) % 2:
        raise ValueError('captured hex shape')
    data = bytes.fromhex(raw)
    if not data or len(data) > MAX_BYTES or hashlib.sha256(data).hexdigest() != digest:
        raise ValueError('captured frame digest/size')
    cursor = count = 0
    while cursor < len(data):
        if cursor + 4 > len(data):
            raise ValueError('truncated captured frame prefix')
        size = int.from_bytes(data[cursor:cursor + 4], 'big')
        cursor += 4
        if size == 0 or cursor + size > len(data):
            raise ValueError('truncated captured frame payload')
        cursor += size
        count += 1
    return count

def encode(capture, allow_partial=False):
    if capture.get('schema') != SCHEMA or capture['source']['b3_sha256'] != B3:
        raise ValueError('source schema/B3 pin')
    source = capture['source']
    if source['producer_test'] != 'srv7_halloween_hare_native_map_runtime_e2e':
        raise ValueError('production capture test')
    runtime = source['observer_runtime']
    native = source['observer_native']
    if runtime != {'x': 32625, 'y': 32018, 'floor': 7}:
        raise ValueError('independent source-B3 observer')
    if native != {'x': runtime['x'], 'y': runtime['y'], 'floor': -runtime['floor']}:
        raise ValueError('native/source floor binding')
    for field in ['bundle_sha256', 'item_keys_sha256', 'world_id_hex', 'actor_hex', 'character_id_hex']:
        value = source[field]
        expected = 64 if field.endswith('sha256') else 32
        if not isinstance(value, str) or len(value) != expected or bytes.fromhex(value).hex() != value:
            raise ValueError('source digest/identity')
    if not isinstance(source['item_2096_definition_ref'], int) or not 1 <= source['item_2096_definition_ref'] <= 0xffffffff:
        raise ValueError('actual compiled Item reference')
    if framed(capture['initial_frames_hex'], capture['initial_sha256']) < 4:
        raise ValueError('actual Accepted + full join snapshot required')
    capture['initial_server_sequence'], capture['palette_rgb'] = metadata(bytes.fromhex(capture['initial_frames_hex']))
    if capture['palette_rgb'] is None:raise ValueError('actual source appearance palette required')
    rows = capture['transformations']
    complete = capture.get('complete', True)
    if type(complete) is not bool or capture.get('expected_transformations', 15) != 15:
        raise ValueError('source completion declaration')
    if not complete and not allow_partial:
        raise ValueError('partial diagnostic cannot become a completed native fixture')
    expected = 15 if complete else len(rows)
    if not 0 <= expected <= 15 or len(rows) != expected or len({row['look'] for row in rows}) != expected:
        raise ValueError('distinct actual source transformation set')
    if complete and expected != 15:
        raise ValueError('all fifteen actual transformations required')
    last = -1
    for row in rows:
        if not isinstance(row['owner_time_us'], int) or row['owner_time_us'] <= last:
            raise ValueError('actual owner clock ordering')
        last = row['owner_time_us']
        framed(row['frames_hex'], row['sha256'])
        row['last_server_sequence'], _ = metadata(bytes.fromhex(row['frames_hex']))
    if capture['cleanup_frames_hex']:
        framed(capture['cleanup_frames_hex'], capture['cleanup_sha256'])
        capture['cleanup_server_sequence'], _ = metadata(bytes.fromhex(capture['cleanup_frames_hex']))
    elif complete:
        raise ValueError('completed capture requires actual expiry frames')
    if not complete:
        capture = dict(capture)
        capture['schema'] = 'OTERYN_SEASONAL_HARE_NATIVE_PARTIAL_PLAY/v1'
    return (json.dumps(capture, ensure_ascii=False, indent=2, sort_keys=True) + '\n').encode()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--allow-partial', action='store_true', help='diagnostic body proof only; never all15 completion')
    args = parser.parse_args()
    raw = args.capture.read_bytes()
    if len(raw) > 2 * MAX_BYTES:
        raise ValueError('capture size bound')
    result = encode(json.loads(raw), args.allow_partial)
    if args.check:
        if args.output.read_bytes() != result:
            raise ValueError('generated Hare fixture drift')
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(result)
    print('actual Hare capture fixture ' + ('verified' if args.check else 'generated'))
if __name__ == '__main__':
    main()
