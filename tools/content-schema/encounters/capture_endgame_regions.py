#!/usr/bin/env python3
"""M26: extract pinned Soul War/Rotten Blood map facts; no activation authority."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

import capture_primal_region as primal
import import_boss_levers as levers
import m26_endgame_capture as source


def source_levers(root, rows):
    positions = set()
    for path in sorted({row['path'] for row in rows}):
        raw = (root / path).read_bytes()
        source.verify_source_file(path, raw)
        if not all(hashlib.sha256(raw).hexdigest() == row['sha256'] for row in rows if row['path'] == path):
            raise ValueError('endgame source lever substitution')
        text = re.sub(r'--[^\n]*', '', raw.decode())
        if path == source.SOUL_PATH:
            source.soul_configurations(raw, levers.table)
            for name in source.SOUL_CONFIGURATIONS:
                table = levers.table(text, r'\b' + name + 'Position') or ''
                values = [re.findall(r'\b' + axis + r'\s*=\s*(\d+)', table) for axis in ('x','y','z')]
                if any(len(value) != 1 for value in values):
                    raise ValueError('nonliteral Soul War lever position')
                positions.add(tuple(int(value[0]) for value in values))
        else:
            variables = re.findall(r'\blocal\s+([A-Za-z_]\w*)\s*=\s*BossLever\(config\)', text)
            if len(variables) != 1:
                raise ValueError('nonliteral Rotten Blood constructor')
            matches = re.findall(r'\b' + re.escape(variables[0]) + r':position\(\s*' + levers.POSITION + r'\s*\)', text)
            if len(matches) != 1:
                raise ValueError('nonliteral Rotten Blood lever position')
            positions.add(tuple(int(value) for value in matches[0]))
    if len(positions) != 12:
        raise ValueError('endgame lever set incomplete')
    return positions


def capture(raw, source_root, candidates):
    blob = hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()
    if blob != primal.MAP_BLOB:
        raise ValueError('pinned Crystal map blob mismatch')
    if candidates['source_revision'] != primal.PIN:
        raise ValueError('endgame candidate revision mismatch')
    rooms = [row for row in candidates['records'] if row['path'] in source.ENDGAME_PATHS]
    if len(rooms) != 12 or len({row['creature_key'] for row in rooms}) != 11:
        raise ValueError('endgame candidate set incomplete')
    required = source_levers(source_root, rooms)
    bounds = []
    for room in rooms:
        if not room['complete_literal_layout']:
            raise ValueError('endgame source layout incomplete')
        bounds.append((room['zone_from'], room['zone_to']))
        required.update(tuple(p) for p in room['lever_positions'] + room['entry_positions'] +
                        [room['boss_position'], room['exit_position']])
    rows = {}

    def handle(node):
        if node[0] != primal.otbm.NODE_TILE_AREA:
            return
        bx, by, z = struct.unpack_from('<HHB', node[1])
        selected = [(low, high) for low, high in bounds if low[2] <= z <= high[2]
                    and bx <= high[0] and bx+255 >= low[0]
                    and by <= high[1] and by+255 >= low[1]]
        if not selected and not any(bx <= x <= bx+255 and by <= y <= by+255 and floor == z
                                    for x,y,floor in required):
            return
        for tile in node[2]:
            if tile[0] not in (primal.otbm.NODE_TILE, primal.otbm.NODE_HOUSE_TILE):
                raise ValueError('unsupported source tile kind')
            position = (bx + tile[1][0], by + tile[1][1], z)
            if position not in required and not any(
                    all(a <= p <= b for a,p,b in zip(low,position,high)) for low,high in selected):
                continue
            if position in rows:
                raise ValueError('duplicate endgame source tile')
            facts = primal.otbm.MapFacts()
            house, flags, zones, items = primal.otbm._tile_content(tile, facts)
            if facts.unknown_item_attrs or facts.unknown_tile_attrs:
                raise ValueError('unknown selected source attributes')
            rows[position] = {'source_position':list(position), 'house_id':house,
                'otbm_flags':flags or 0, 'zones':list(zones), 'items':[
                    {'server_item_id':identity,'depth':depth,'attributes':{str(k):v for k,v in attrs.items()}}
                    for identity,depth,attrs in items]}
            if len(rows) > 8192:
                raise ValueError('endgame region capacity exceeded')

    primal.otbm._scan(primal.otbm.load_bytes(raw),handle)
    if not required.issubset(rows):
        raise ValueError('required endgame source placement missing: '+str(sorted(required-rows.keys())))
    return {'schema':'OTERYN_CRYSTAL_ENDGAME_REGIONS_CAPTURE/v1',
        'source_repository':'zimbadev/crystalserver','source_revision':primal.PIN,
        'map_path':primal.MAP_PATH,'map_git_blob':blob,'map_sha256':hashlib.sha256(raw).hexdigest(),
        'levers':[{key:room[key] for key in ('path','sha256','line','creature_key','zone_from','zone_to')}
                  for room in rooms],
        'required_positions':[list(p) for p in sorted(required)],
        'tiles':[rows[p] for p in sorted(rows)]}


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--source-root',type=Path,required=True)
    parser.add_argument('--candidates',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    document=capture((args.source_root/primal.MAP_PATH).read_bytes(),args.source_root,
                     json.loads(args.candidates.read_bytes()))
    document=primal.attach_item_semantics(document,(args.source_root/'data/items/appearances.dat').read_bytes(),
                                         (args.source_root/'data/items/items.xml').read_bytes())
    args.output.write_text(json.dumps(document,sort_keys=True,separators=(',',':'))+'\n')
    print(json.dumps({'rooms':len(document['levers']),'tiles':len(document['tiles']),
                      'required_positions':len(document['required_positions']),
                      'sha256':hashlib.sha256(args.output.read_bytes()).hexdigest()}))


if __name__=='__main__':
    main()
