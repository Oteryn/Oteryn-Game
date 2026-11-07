#!/usr/bin/env python3
"""Bind two real prepare-death events; never fabricate an Ability schedule slot."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[3]
PIN='47dfd51f45280a59a1d3e50ba7edd573d7234446'
OUTPUT=ROOT/'crates/gameplay/content/src/content/native_soul_war_callbacks/w01_prepare_death_witness.json'
ROWS=[('hirintror','HirintrorHeal',1000,'1434508564b041fa18599f3fd3ee23b216812d5aab872dba455b0e92a85698f2'),
      ('white_pale','WhitePaleHeal',400,'7010c437424343031ed9029f1e4a0605c4cecb5181551afccc32e89171e44a63')]
SOURCES={
 'data-otservbr-global/monster/raids/hirintror.lua':'9aa9aa3b82f03a0318d068e664ade1993a9e83940dd0cea25459d0179be5b1ea',
 'data-otservbr-global/monster/raids/white_pale.lua':'cc1ac834e8f85d9771640c22d2542dd02749e439f07d24b584c00c36d6dd68eb',
 'data-otservbr-global/scripts/creaturescripts/monster/hirintror_heal.lua':'15316485b3e6cfb4b6fefbaaaa9900a24ff58f22778bf81cdba9f2a6b5eaf5bd',
 'data-otservbr-global/scripts/creaturescripts/monster/white_pale_heal.lua':'c180115e58f68d117195b689d98a301931a382693589e105b178d035712d329c',
}
def canonical(value):return json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
def read_native():return json.loads((ROOT/'content/creatures/definitions/spell-native-profiles.json').read_bytes())
def build(native=None):
    native=read_native() if native is None else native
    canonical_rows=[row for path in (ROOT/'content/creatures/definitions').glob('creatures-*.json')
                    for row in json.loads(path.read_bytes())['records']]
    events=[]
    for slug,event,heal,body in ROWS:
        key='oteryn:creature.'+slug
        records=[row for row in native['records'] if row['profile']['target']['key']==key]
        if len(records)!=1 or hashlib.sha256(canonical(records[0])).hexdigest()!=body:
            raise ValueError('Native prepare-death body changed: '+key)
        record=records[0]
        binding={'disposition':'EXACT','external_id':'raids/'+slug,'identity_namespace':'canary/monster-file',
                 'source_key':'oteryn:source.canary','source_revision':PIN,'target':record['profile']['target']}
        declared=[row for row in canonical_rows if row['definition']['identity']['key']==key]
        if len(declared)!=1 or binding not in declared[0]['source_bindings']:
            raise ValueError('Native prepare-death source binding changed: '+key)
        events.append({'native':record,'source_binding':binding,'event':event,'heal':heal,
                       'cancel_incoming':True,'marker_sub_id':88888,'marker_duration_ms':900000,'marker_health_gain':0})
    return {'schema':'OTERYN_W01_PREPARE_DEATH/v1','source_revision':PIN,
            'source_definitions_sha256':native['source_definitions_sha256'],'sources':SOURCES,'events':events}
def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source-root',type=Path);parser.add_argument('--check',action='store_true');args=parser.parse_args()
    if args.source_root:
        for path,expected in SOURCES.items():
            if hashlib.sha256((args.source_root/path).read_bytes()).hexdigest()!=expected:
                raise ValueError('Pinned prepare-death source changed: '+path)
    raw=(json.dumps(build(),ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes()!=raw:raise ValueError('Stale W01 prepare-death witness')
    else:
        OUTPUT.parent.mkdir(parents=True,exist_ok=True);OUTPUT.write_bytes(raw)
    print('W01_PREPARE_DEATH_WITNESS',len(raw),hashlib.sha256(raw).hexdigest())
if __name__=='__main__':main()
