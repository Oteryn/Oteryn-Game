#!/usr/bin/env python3
"""Seal actual Messenger of Santa caller-map/native appearance bytes."""
import argparse
import hashlib
import json
from pathlib import Path
from seasonal_capture_fields import metadata

def materialize(value):
    if value.get('schema') != 'OTERYN_SEASONAL_NPC_NATIVE_CAPTURE/v1':raise ValueError('actual Messenger capture schema')
    source=value['source']
    if source.get('npc_key')!='oteryn:npc.messenger_of_santa' or source.get('source_look_type')!=159 or source.get('source_colours')!=[39,99,60,117] or source.get('production_source_world_preserved') is not True:raise ValueError('source NPC production proof')
    if not source.get('map_fixture'):raise ValueError('truthful caller-map geometry label')
    raw=bytes.fromhex(value['initial_frames_hex'])
    if not raw or len(raw)>4*1024*1024 or hashlib.sha256(raw).hexdigest()!=value['initial_frames_sha256']:raise ValueError('actual frames integrity')
    value['initial_server_sequence'],value['palette_rgb']=metadata(raw)
    if not value['palette_rgb']:raise ValueError('actual source palette')
    return (json.dumps(value,sort_keys=True,indent=2)+'\n').encode()

def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('capture',type=Path);p.add_argument('output',type=Path);p.add_argument('--check',action='store_true');a=p.parse_args();data=materialize(json.loads(a.capture.read_bytes()))
    if a.check:
        if a.output.read_bytes()!=data:raise ValueError('actual Messenger fixture drift')
    else:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_bytes(data)
    print('actual Messenger capture '+('verified' if a.check else 'generated'))
if __name__=='__main__':main()
