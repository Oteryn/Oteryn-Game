#!/usr/bin/env python3
"""Verify Grizzly room bounds against the repository's map, never guessed coordinates.

Crystal 201189f movements_boss.lua supplies portal/player/spawn positions. Q2 in
PR2770 permits map-confirmed room bounds and refuses unverified rooms. Read the
existing region codec, Terrain/WorldObject collision facts and native Item fallback.
Eight-neighbour connected room components must be closed and contain both source
positions. Source rectangles are retained when they contain those positions; a
verified component extends them where the donor rectangle clips the actual room.
The search limits below are tooling limits, never gameplay values.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from world_region_codec import decode_region, region_name

OUTPUT = ROOT / 'content/quests/grizzly-boss-rooms.json'


def generate(root: Path = ROOT) -> dict:
    sources: dict[str, str] = {}
    def read(path: str) -> bytes:
        data = (root / path).read_bytes()
        sources[path] = hashlib.sha256(data).hexdigest()
        return data
    catalogue = json.loads(read('content/quests/killing-in-the-name-of.json'))
    palette = json.loads(read('content/world/placements/index.json'))['palette']
    terrain, objects = {}, {}
    for family, records in [('terrain', terrain), ('objects', objects)]:
        for path in sorted((root / 'content/world' / family).glob(family + '-*.json')):
            for row in json.loads(read(str(path.relative_to(root))))['records']:
                records[int(row['identity']['key'].rsplit('.i', 1)[1])] = row
    profiles = {}
    for row in json.loads(read('content/items/definitions/spell-native-profiles.json'))['records']:
        profiles.setdefault(row['production_definition']['production_key'], row['attributes'].get('blocks_movement'))
    cache: dict[tuple, dict] = {}
    def tile(z, x, y):
        key = z, x // 256, y // 256
        if key not in cache:
            path = 'content/world/placements/' + region_name(*key)
            if not (root / path).is_file():
                cache[key] = {}
            else:
                _, _, _, sectors = decode_region(read(path))
                cache[key] = {(t[0], t[1]): t for _, rows in sectors for t in rows}
        return cache[key].get((x, y))
    def walk(z, x, y):
        current = tile(z, x, y)
        if current is None:
            return False
        ground = False
        for pi, depth, _ in current[5]:
            if depth:
                continue
            item = palette[pi]['source_item_id']
            if item in terrain:
                row = terrain[item]
                if row['kind'].get('value') == 'ground':
                    if row['walkable'].get('value') is not True:
                        return False
                    ground = True
                elif row['kind'].get('value') == 'border':
                    if row['walkable'].get('value') is not True:
                        return False
                elif row['kind'].get('value') == 'wall':
                    return False
                elif profiles.get(palette[pi]['key']) is not False:
                    return False
            elif item in objects:
                if objects[item]['collision']['blocks_movement'].get('value') is not False:
                    return False
            elif profiles.get(palette[pi]['key']) is not False:
                return False
        return ground
    rooms = []
    for boss in catalogue['bosses']:
        player, spawn = boss['player_position'], boss['boss_position']
        z = player['z']
        start = player['x'], player['y']
        pending, visited, clipped = [start], set(), False
        while pending and len(visited) < 10_000:
            x, y = pending.pop()
            if (x, y) in visited or not walk(z, x, y):
                continue
            visited.add((x, y))
            for dx, dy in ((1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)):
                if abs(x + dx - start[0]) < 256 and abs(y + dy - start[1]) < 256:
                    pending.append((x + dx, y + dy))
                else:
                    clipped = True
        qualified = not pending and not clipped and (spawn['x'], spawn['y']) in visited and spawn['z'] == z
        center = boss['center']
        donor = [center['x'] - boss['range_x'], center['y'] - boss['range_y'], center['x'] + boss['range_x'], center['y'] + boss['range_y'], center['z']]
        bounds = None
        if qualified:
            bounds = [min(x for x,y in visited), min(y for x,y in visited), max(x for x,y in visited), max(y for x,y in visited), z]
            if donor[4] == z and all(donor[0] <= p['x'] <= donor[2] and donor[1] <= p['y'] <= donor[3] for p in [player, spawn]):
                bounds = [min(bounds[0], donor[0]), min(bounds[1], donor[1]), max(bounds[2], donor[2]), max(bounds[3], donor[3]), z]
        rooms.append({'boss_id': boss['id'], 'name': boss['name'], 'qualified': qualified,
            'bounds': bounds, 'donor_bounds': donor, 'connected_tiles': len(visited),
            'reason': None if qualified else 'Source player and spawn positions are not in one bounded map component with known collision facts.',
            'source': f"data-global/scripts/quests/killing_in_the_name_of/movements_boss.lua:{boss['donor_line']}"})
    return {'schema': 'OTERYN_GRIZZLY_BOSS_ROOMS/v1', 'generator': 'tools/content-schema/quest-authoring/grizzly_rooms.py',
        'source_revision': catalogue['source_revision'], 'decision': 'Oteryn/Oteryn-Game#2770 Q2=B',
        'rooms': rooms, 'inputs_sha256': dict(sorted(sources.items()))}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    data = generate()
    encoded = json.dumps(data, indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text() != encoded:
            print('Grizzly map room qualification is stale', file=sys.stderr)
            return 1
    else:
        OUTPUT.write_text(encoded)
    print(f"Grizzly rooms: {sum(r['qualified'] for r in data['rooms'])}/{len(data['rooms'])} map-qualified")
    return 0

if __name__ == '__main__':
    raise SystemExit(main())
