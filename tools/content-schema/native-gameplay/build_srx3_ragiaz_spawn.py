#!/usr/bin/env python3
"""Capture only Ragiaz's real hidden Bone Capsule; explicit output avoids live writes."""
import argparse
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

SOURCE='data-global/world/world-monster.xml'
REVISION='201189f871d23597b82caf88fd611454a2ddefb8'
SHA256='f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e'
GIT_BLOB='7949251d25d8db8f0b2ca65b67df76ac577a5caf'
OUTPUT=Path(__file__).with_name('srx3-ragiaz-spawn.json')


def extract(raw):
    """Pure parsing; only build() qualifies provenance for publication."""
    rows=[]
    lines=[i for i,line in enumerate(raw.decode().splitlines(),1) if '<monster name="Bone Capsule"' in line]
    for source in ET.fromstring(raw):
        for monster in source:
            if monster.get('name')!='Bone Capsule':continue
            at=[int(source.get('centerx'))+int(monster.get('x')),
                int(source.get('centery'))+int(monster.get('y')),int(source.get('centerz'))]
            if at!=[33485,32333,14] or int(source.get('radius'))!=1 or int(monster.get('spawntime'))!=60 or monster.get('direction') is not None:
                raise ValueError('unreviewed Ragiaz map spawn')
            rows.append(dict(position=at,radius=1,respawn_seconds=60,
                creature_key='oteryn:creature.bone_capsule'))
    if len(rows)!=1 or len(lines)!=1:raise ValueError('Ragiaz source map spawn must occur exactly once')
    rows[0]['line']=lines[0]
    return rows[0]


def build(path):
    raw=Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest()!=SHA256 or hashlib.sha1(b'blob '+str(len(raw)).encode()+b'\0'+raw).hexdigest()!=GIT_BLOB:
        raise ValueError('pinned Crystal monster XML changed')
    row=extract(raw)
    if row['line']!=182951:raise ValueError('pinned Ragiaz XML provenance changed')
    row.update(source_path=SOURCE,source_sha256=SHA256)
    return [row]


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source',type=Path)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    raw=(json.dumps(build(args.source),indent=2)+'\n').encode()
    if args.check:
        if args.output.read_bytes()!=raw:raise SystemExit('stale Ragiaz source spawn capture')
    else:args.output.write_bytes(raw)
