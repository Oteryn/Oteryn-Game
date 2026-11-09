"""Lossless startup-key overlay for M01c's source-captured native doors.

The capture records Crystal 201189f's executable table entries, rather than
its conflicting comments (3012 is labelled 4193; 5010 is labelled 5009).
This module does not publish a World or change its pins.
"""
from pathlib import Path

import quest_door_bindings as base

CAPTURE = Path('tools/content-schema/world-authoring/samples/m01c-key-doors.json')
LOCKED = {5097, 6248}


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
