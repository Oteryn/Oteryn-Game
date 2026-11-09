"""Lossless startup-key overlay for M01c's source-captured native doors.

The capture records Crystal 201189f's executable table entries, rather than
its conflicting comments (3012 is labelled 4193; 5010 is labelled 5009).
This module does not publish a World or change its pins.
"""
import argparse
import hashlib
import json
from pathlib import Path

import quest_door_bindings as base

CAPTURE = Path('tools/content-schema/world-authoring/samples/m01c-key-doors.json')
LOCKED = {5097, 6248}
SUMMARY = 'tools/content-schema/world-authoring/samples/world-base-capture-v1.json'


def region_name(position):
    x, y, z = position
    return f'region-z{z:02d}-x{x // 256:03d}-y{y // 256:03d}.b3'


def overlay_region(raw, palette, capture):
    """Require exactly one compatible top-level locked door per source cell."""
    z, rx, ry, _ = base.codec.parse_region(raw)
    bindings = [row for row in capture['bindings']
                if row['position'][2] == z and row['position'][0] // 256 == rx
                and row['position'][1] // 256 == ry]
    if not bindings:
        return raw, []
    sectors = base.codec.decode_region(raw)[3]
    if len({tuple(row['position']) for row in bindings}) != len(bindings):
        raise ValueError('M01C_KEY_DOOR_TARGET: duplicate source position')
    for row in bindings:
        number = row['key_number']
        if type(number) is not int or number not in (3012, 3520, 5010):
            raise ValueError('M01C_KEY_DOOR_NUMBER')
        matches = [item for _, tiles in sectors
                   for x, y, _, house, _, items in tiles
                   if [x, y, z] == row['position'] and not house
                   for item in items if not item[1]
                   and palette[item[0]]['source_item_id'] in LOCKED
                   and row['item'] in (None, palette[item[0]]['source_item_id'])]
        if len(matches) != 1:
            raise ValueError(f'M01C_KEY_DOOR_TARGET: {row["position"]}')
    document = {'bindings': [{**row, 'action': row['key_number']} for row in bindings]}
    return base.overlay_region(raw, palette, document, item_ids=LOCKED)


def overlay_outputs(outputs, capture_bytes=None):
    """Restore the finite capture and regenerate its region/source accounting."""
    capture_bytes = capture_bytes if capture_bytes is not None else (base.ROOT / CAPTURE).read_bytes()
    capture = json.loads(capture_bytes)
    index = json.loads(outputs[base.INDEX.as_posix()])
    names = {region_name(row['position']) for row in capture['bindings']}
    imported, delta = [], 0
    for region in index['regions']:
        path = region['path']
        if Path(path).name not in names:
            continue
        previous = outputs[path]
        result, rows = overlay_region(previous, index['palette'], capture)
        outputs[path] = result
        region['sha256'] = hashlib.sha256(result).hexdigest()
        delta += len(result) - len(previous)
        imported.extend(rows)
    if not imported:
        return []
    previous_count = index['source'].get('m01c_key_doors', {}).get('imported', 0)
    index['source']['m01c_key_doors'] = {
        'path': CAPTURE.as_posix(), 'sha256': hashlib.sha256(capture_bytes).hexdigest(),
        'imported': len(imported),
    }
    summary = json.loads(outputs[SUMMARY])
    summary['source'] = index['source']
    summary['item_attributes']['action'] += len(imported) - previous_count
    summary['bytes_on_disk'] += delta
    outputs[base.INDEX.as_posix()] = base.canonical(index)
    outputs[SUMMARY] = base.canonical(summary)
    return imported


def apply(check=False):
    index_bytes = (base.ROOT / base.INDEX).read_bytes()
    capture = json.loads((base.ROOT / CAPTURE).read_bytes())
    names = {region_name(row['position']) for row in capture['bindings']}
    outputs = {base.INDEX.as_posix(): index_bytes, SUMMARY: (base.ROOT / SUMMARY).read_bytes()}
    for region in json.loads(index_bytes)['regions']:
        if Path(region['path']).name in names:
            outputs[region['path']] = (base.ROOT / region['path']).read_bytes()
    rows = overlay_outputs(outputs)
    if len(rows) != len(capture['bindings']):
        raise ValueError('M01C_KEY_DOOR_CAPTURE_INCOMPLETE')
    for path, data in outputs.items():
        target = base.ROOT / path
        if check:
            if target.read_bytes() != data:
                raise ValueError(f'M01C_KEY_DOOR_DRIFT: {path}')
        else:
            target.write_bytes(data)
    print(f'PASS M01c native key-door placements={len(rows)}; check={check}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    apply(parser.parse_args().check)
