"""Hold the raw Doomforge teleport until a hard-victory gate owns passage."""
import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec
from quest_access_bindings import INDEX, SUMMARY, canonical

ROOT = Path(__file__).resolve().parents[3]
REGION = 'content/world/placements/region-z14-x132-y126.b3'
PORTAL = (34041, 32330)
DESTINATION = (34062, 32334, 14)
_COORDINATES = ('x', 'y', 'floor')
APPEARANCE = 37000
TRANSITION = 'oteryn:transition.teleport.x34041_y32330_z14'
REJECTION = 'doomforge_requires_hard_victory'


def holds_link(origin, appearance, destination):
    return (origin, appearance, destination) == ((*PORTAL, 14), APPEARANCE, DESTINATION)

SOURCE = ('crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
          'data-global/scripts/quests/no_rest_for_the_wicked/forge_teleport.lua:9-33')


def overlay_outputs(outputs):
    index = json.loads(outputs[INDEX])
    summary = json.loads(outputs[SUMMARY])
    raw = outputs[REGION]
    z, rx, ry, table = codec.parse_region(raw)
    if (z, rx, ry) != (14, 132, 126):
        raise ValueError('unexpected Doomforge region')
    local_index = (PORTAL[1] // 32 - ry * 8) * 8 + PORTAL[0] // 32 - rx * 8
    sectors = {
        local: codec.zstandard.ZstdDecompressor().decompress(
            raw[offset:offset + length], max_output_size=codec.MAX_SECTOR_BYTES)
        for local, offset, length in table
    }
    sx, sy = (coord // 32 for coord in PORTAL)
    tiles = codec.decode_sector(sectors[local_index], sx, sy)
    tile, = (tile for tile in tiles if tile[:2] == PORTAL)
    items = tile[5]
    slot, = (slot for slot, (palette, depth, _) in enumerate(items)
             if depth == 0 and index['palette'][palette]['source_item_id'] == APPEARANCE)
    palette, depth, attrs = items[slot]
    evidence = {
        'source': SOURCE,
        'position': [*PORTAL, z],
        'appearance': APPEARANCE,
        'held_destination': list(DESTINATION),
    }
    region, = (region for region in index['regions'] if region['path'] == REGION)
    previous = index['source'].get('doomforge_portal_hold')
    if previous is not None:
        if (previous != evidence or (attrs or {}).get('teleport') is not None
                or region['sha256'] != hashlib.sha256(raw).hexdigest()
                or summary['source'] != index['source']
                or summary['excluded_teleports'].get('no_transition') != 1):
            raise ValueError('Doomforge portal hold drift')
        return
    destination = (attrs or {}).get('teleport')
    already_filtered = summary['excluded_teleports'].get('no_transition', 0) == 1
    if destination is None and already_filtered:
        corrected = raw
    elif destination == DESTINATION and not already_filtered:
        kept = dict(attrs)
        del kept['teleport']
        items[slot] = (palette, depth, kept or None)
        sectors[local_index] = codec.encode_sector_tiles(sx, sy, tiles)
        corrected = codec.encode_region(z, rx, ry, sectors)
        summary['item_attributes']['teleport'] -= 1
        summary['excluded_teleports']['no_transition'] += 1
    else:
        raise ValueError('unexpected raw Doomforge destination')
    region['sha256'] = hashlib.sha256(corrected).hexdigest()
    index['source']['doomforge_portal_hold'] = evidence
    summary['source'] = index['source']
    summary['bytes_on_disk'] += len(corrected) - len(raw)
    outputs.update({REGION: corrected, INDEX: canonical(index), SUMMARY: canonical(summary)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    import convert_world_metadata as metadata

    transition_index = metadata.FAMILIES['Transition.Teleport']['dir'] + '/index.json'
    index = json.loads((ROOT / transition_index).read_bytes())
    records = [row for path in index['shards']
               for row in json.loads((ROOT / path).read_bytes())['records']]
    kept = [row for row in records if row['declaration']['identity']['key'] != TRANSITION]
    summary_path = str(metadata.SUMMARY.relative_to(ROOT))
    summary = json.loads((ROOT / summary_path).read_bytes())
    if len(records) != len(kept):
        removed, = (row['declaration'] for row in records
                    if row['declaration']['identity']['key'] == TRANSITION)
        origin = tuple(removed['from'][key] for key in _COORDINATES)
        destination = tuple(removed['to'][key] for key in _COORDINATES)
        if (not holds_link(origin, APPEARANCE, destination)
                or removed['object']['key'] != f'oteryn:item.tibia.i{APPEARANCE}'):
            raise ValueError('Doomforge Transition differs')
        summary['families']['Transition.Teleport'] -= 1
        summary['teleport_object_families']['Item'] -= 1
        summary['not_imported']['teleports'][REJECTION] = 1
    elif summary['not_imported']['teleports'].get(REJECTION) != 1:
        raise ValueError('missing Doomforge Transition hold')
    outputs = metadata.shard_files('Transition.Teleport', kept, index['source'], index['generator'])
    outputs[summary_path] = canonical(summary)
    stale = set(index['shards']) - outputs.keys()
    original = {path: (ROOT / path).read_bytes() for path in (REGION, INDEX, SUMMARY)}
    outputs.update(original)
    overlay_outputs(outputs)
    for path in stale:
        if args.check:
            raise ValueError('stale Doomforge Transition shard: ' + path)
        (ROOT / path).unlink()
    for path, data in outputs.items():
        if (ROOT / path).exists() and data == (ROOT / path).read_bytes():
            continue
        if args.check:
            raise ValueError('stale Doomforge portal placement: ' + path)
        (ROOT / path).write_bytes(data)


if __name__ == '__main__':
    main()
