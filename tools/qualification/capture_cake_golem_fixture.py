#!/usr/bin/env python3
"""Pin the actual source Cake Golem production appearance/map snapshot bytes."""
import argparse
import hashlib
import json
from pathlib import Path

B3 = '2798be76221d6dc00695258d46d1eb375a64e804a7a84265014e2af0c679b05d'

def materialize(value):
    if value.get('schema') != 'OTERYN_SEASONAL_GOLEM_NATIVE_CAPTURE/v1':
        raise ValueError('actual Golem producer schema')
    source = value['source']
    if source.get('producer_test') != 'srv7_cake_golem_real_machine_runtime_extracts_and_removes_once' or source.get('b3_sha256') != B3 or source.get('production_source_world_preserved') is not True:
        raise ValueError('actual sourceB3/production activation evidence')
    for key in ('world_id_hex', 'character_id_hex', 'actor_identity_hex', 'client_content_generation', 'item_keys_sha256', 'bundle_sha256'):
        text=source[key]
        width=32 if key.endswith('_hex') else 64
        if not isinstance(text,str) or len(text)!=width or bytes.fromhex(text).hex()!=text:
            raise ValueError('actual source identity/digest')
    if type(source['item_6393_definition_ref']) is not int or not 0 < source['item_6393_definition_ref'] <= 0xffffffff or type(source['actor_generation']) is not int or source['actor_generation'] <= 0:
        raise ValueError('actual source typed body reference')
    for key in ('source_runtime_cell','observer_runtime_cell'):
        cell=source[key]
        if not isinstance(cell,list) or len(cell)!=3 or any(type(n) is not int for n in cell) or not 0<=cell[2]<=15:
            raise ValueError('independent source runtime positions')
    raw=bytes.fromhex(value['initial_frames_hex'])
    if not 0<len(raw)<=4*1024*1024 or hashlib.sha256(raw).hexdigest()!=value['initial_frames_sha256']:
        raise ValueError('actual production frame integrity')
    cursor=count=0
    while cursor<len(raw):
        if cursor+4>len(raw): raise ValueError('actual frame prefix')
        size=int.from_bytes(raw[cursor:cursor+4],'big');cursor+=4
        if not size or cursor+size>len(raw):raise ValueError('actual frame body')
        cursor+=size;count+=1
    if count<4:raise ValueError('actual accepted/fulljoin required')
    return (json.dumps(value,indent=2,sort_keys=True)+'\n').encode()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture',type=Path);parser.add_argument('output',type=Path);parser.add_argument('--check',action='store_true')
    args=parser.parse_args();raw=args.capture.read_bytes()
    if len(raw)>8*1024*1024:raise ValueError('capture bound')
    data=materialize(json.loads(raw))
    if args.check:
        if args.output.read_bytes()!=data:raise ValueError('actual Golem capture fixture drift')
    else:
        args.output.parent.mkdir(parents=True,exist_ok=True);args.output.write_bytes(data)
    print('actual source Golem fixture '+('verified' if args.check else 'generated'))
if __name__=='__main__':main()
