"""Restore the source startup UIDs of the qualified R–S quest chests.

The donor map omits IDs assigned by its startup loader. Reproduce that loader's
position/appearance precondition, preserving every other tile and item fact.
Inputs are the source-bound RewardClaim records; no Lua runs at authoring or boot.
"""
import argparse
import hashlib
import json
from pathlib import Path

import world_region_codec as codec

ROOT = Path(__file__).resolve().parents[3]
INDEX = 'content/world/placements/index.json'
SUMMARY = 'tools/content-schema/world-authoring/samples/world-base-capture-v1.json'
CLAIMS = {
    'kv.bananafree', 'kv.bananapremium', 'kv.honeyflower', 'kv.rapier', 'kv.ringquest1', 'kv.ringquest2',
    'kv.scalearmor', 'kv.shamantreasure', 'kv.shortswordbook', 'kv.silveramulet',
    'kv.silverbrooch', 'kv.sixrubies', 'kv.smallsapphire', 'kv.spikesword', 'kv.stealhring1',
    'kv.stealhring2', 'kv.steelhelmet1',
    'kv.steelhelmet2', 'kv.steelhelmet3', 'kv.steelhelmet4',
    'kv.witch_house_quest',
    'quest.u7_6.explorer_society.skull_of_ratha.bag1',
    'quest.u7_6.explorer_society.skull_of_ratha.bag2',
    'quest.u8_1.strong_potions.reward',
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()


def bindings():
    definitions = {}
    for path in sorted((ROOT / 'content/interactions/reward_claims').glob('reward-claim*.json')):
        for row in json.loads(path.read_bytes())['records']:
            definition = row['definition']
            key = definition['identity']['key'].removeprefix('oteryn:reward-claim.')
            if key not in CLAIMS:
                continue
            if key in definitions:
                raise ValueError(f'duplicate R–S claim: {key}')
            definitions[key] = definition
    if definitions.keys() != CLAIMS:
        raise ValueError('R–S source claim inventory changed')
    result = {}
    for key, definition in sorted(definitions.items()):
        for placement in definition['placements']:
            source = placement['source_binding']
            position = tuple(source['project_position'][axis] for axis in 'xyz')
            unique = [row['unique_id'] for row in source['legacy_unique_ids']
                      if row['server'] == 'crystalserver']
            if key == 'kv.smallsapphire':
                if unique or definition.get('definition_profile') != 'scripted_rs_v1':
                    raise ValueError('Small Sapphire native marker is not a donor UID')
                marker = definition['native_contents'].get('native_unique_id')
                if marker != 64993:
                    raise ValueError('Small Sapphire native import marker changed')
                unique = [marker]
            if len(unique) != 1 or position in result:
                raise ValueError(f'ambiguous R–S chest binding: {key}')
            result[position] = {'claim': key, 'appearance': placement['appearance_tibia_id'],
                                'unique': unique[0]}
    return result, hashlib.sha256(canonical(definitions)).hexdigest()


def overlay_region(raw, palette, bindings):
    z, rx, ry, _ = codec.parse_region(raw)
    selected = {position: binding for position, binding in bindings.items()
                if position[2] == z and position[0] // 256 == rx and position[1] // 256 == ry}
    if not selected:
        return raw, set(), 0
    _, _, _, sectors = codec.decode_region(raw)
    imported, encoded, added = set(), {}, 0
    changed = False
    for local, tiles in sectors:
        for x, y, _, house, _, items in tiles:
            position = (x, y, z)
            binding = selected.get(position)
            if binding is None:
                continue
            candidates = [slot for slot, (index, depth, _) in enumerate(items)
                          if not depth and palette[index]['source_item_id'] == binding['appearance']]
            if house or len(candidates) != 1:
                raise ValueError(f'R–S source chest missing or ambiguous at {position}')
            slot = candidates[0]
            index, depth, attrs = items[slot]
            previous = (attrs or {}).get('unique')
            if previous is not None and previous != binding['unique']:
                raise ValueError(f'R–S source chest UID conflict at {position}')
            if previous is None:
                items[slot] = (index, depth, {**(attrs or {}), 'unique': binding['unique']})
                added += 1
                changed = True
            imported.add(position)
        sx, sy = rx * 8 + local % 8, ry * 8 + local // 8
        encoded[local] = codec.encode_sector_tiles(sx, sy, tiles)
    return (codec.encode_region(z, rx, ry, encoded) if changed else raw), imported, added


def overlay_outputs(outputs):
    selected, digest = bindings()
    regions = {codec.region_name(z, x // 256, y // 256) for x, y, z in selected}
    index = json.loads(outputs[INDEX])
    imported, added, byte_delta = set(), 0, 0
    for region in index['regions']:
        path = region['path']
        if Path(path).name not in regions:
            continue
        before = outputs[path]
        if hashlib.sha256(before).hexdigest() != region['sha256']:
            raise ValueError(f'R–S chest input region digest mismatch: {path}')
        after, found, new_ids = overlay_region(before, index['palette'], selected)
        outputs[path] = after
        region['sha256'] = hashlib.sha256(after).hexdigest()
        imported.update(found)
        added += new_ids
        byte_delta += len(after) - len(before)
    if imported != selected.keys():
        raise ValueError(f'R–S source chest tiles missing: {sorted(selected.keys() - imported)}')
    index['source']['quest_chests_rs'] = {'claims_sha256': digest, 'imported': len(imported)}
    outputs[INDEX] = canonical(index)
    summary = json.loads(outputs[SUMMARY])
    summary['source'] = index['source']
    summary['item_attributes']['unique'] = summary['item_attributes'].get('unique', 0) + added
    summary['bytes_on_disk'] += byte_delta
    outputs[SUMMARY] = canonical(summary)


def apply(check):
    index = json.loads((ROOT / INDEX).read_bytes())
    selected, _ = bindings()
    regions = {codec.region_name(z, x // 256, y // 256) for x, y, z in selected}
    outputs = {path: (ROOT / path).read_bytes()
               for path in [INDEX, SUMMARY, *(r['path'] for r in index['regions']
                                             if Path(r['path']).name in regions)]}
    overlay_outputs(outputs)
    changed = [path for path, data in outputs.items() if (ROOT / path).read_bytes() != data]
    if check and changed:
        raise ValueError(f'R–S quest chest binding drift: {changed}')
    for path in changed:
        (ROOT / path).write_bytes(outputs[path])
    print(f'R–S quest chest bindings: {len(CLAIMS)} claims; check={check}; changed={len(changed)}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    apply(parser.parse_args().check)
