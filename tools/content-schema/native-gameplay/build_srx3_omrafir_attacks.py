#!/usr/bin/env python3
"""Bounded source overlay for the two omitted Omrafir callbacks; no stat edits."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
BODY = 'b04eb7dd064605680425fce89d5fdb45bd6c40196536e278a177a26db05a6fc5'
EVENT_CHILD_BODY = '399f30eebede69bd7b2720fa4e07a1370b89a16f25846b1d3b8b15e05ba2c992'
EVENT_CALLBACKS = {
 'data-otservbr-global/scripts/creaturescripts/monster/omrafir_explode.lua': 'bad7511af294bebc5a9d94345252d0ba03784172a73463d772be6350a467cbcf',
}
CALLBACKS = {
 'data-otservbr-global/monster/quests/roshamuul/omrafir.lua': 'd5d600fab74a3e4f56277552b1538f03a46bfcb067443257e4d7120e541fb798',
 'data-otservbr-global/scripts/spells/monster/omrafir_beam.lua': 'f87b630bdca5cf7ffc6167bc9301833e955f39586c7ff151322e0ae011e3d4ab',
 'data-otservbr-global/scripts/spells/monster/omrafir_healing_2.lua': 'ba2790a574ae9deba86d903e60cf8271e4ad86e51141256d90ceebb679927036',
}
BEAM_MATRICES = [['.....c.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '...........', '...........'], ['...........', '...........', '...........', '...........', '...........', '..xxxxxxxxc', '...........', '...........', '...........', '...........', '...........'], ['.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....x.....', '.....c.....', '...........'], ['...........', '...........', '...........', '...........', '...........', 'cxxxxxxxx..', '...........', '...........', '...........', '...........', '...........']]
OUTPUT = ROOT / 'tools/content-schema/native-gameplay/srx3-omrafir-attacks.json'


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def read_native():
    return json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())


def build(native=None):
    native = read_native() if native is None else native
    records = [r for r in native['records'] if r['profile']['target']['key'] == 'oteryn:creature.omrafir']
    if len(records) != 1 or hashlib.sha256(canonical(records[0])).hexdigest() != BODY:
        raise ValueError('Native Omrafir body changed')
    record = records[0]
    binding = {'disposition': 'EXACT', 'external_id': 'quests/roshamuul/omrafir',
               'identity_namespace': 'canary/monster-file', 'source_key': 'oteryn:source.canary',
               'source_revision': PIN, 'target': record['profile']['target']}
    creatures = [r for p in (ROOT / 'content/creatures/definitions').glob('creatures-*.json')
                 for r in json.loads(p.read_bytes())['records']
                 if r['definition']['identity']['key'] == record['profile']['target']['key']]
    if len(creatures) != 1 or binding not in creatures[0]['source_bindings']:
        raise ValueError('Native Omrafir source binding changed')
    abilities, schedules = [], []
    for name, family, chance in [('omrafir_beam','attacks',150000), ('omrafir_healing_2','defenses',200000)]:
        ability = {'family':'Ability','key':'oteryn:ability.spell.'+name,'revision':'definition-r1'}
        if any(p['target'] == ability for p in record['attack_definitions']['profiles']):
            raise ValueError('Omrafir callback already admitted')
        abilities.append({'target':ability,'data':{'kind':'Ability','profile':{'details':{
            'kind':'Spell','range_tiles':0,'needs_target':False,'needs_direction':False}}}})
        schedules.append({'list':family,'index':2,'schedule':{'ability':ability,'interval_ms':2000,'chance_ppm':chance}})
    children = [r for r in native['records'] if r['profile']['target']['key'] == 'oteryn:creature.hellfire_fighter']
    if len(children) != 1 or hashlib.sha256(canonical(children[0])).hexdigest() != EVENT_CHILD_BODY:
        raise ValueError('Omrafir event child body changed')
    child_binding = {'disposition':'EXACT', 'external_id':'demons/hellfire_fighter',
                     'identity_namespace':'canary/monster-file', 'source_key':'oteryn:source.canary',
                     'source_revision':PIN, 'target':children[0]['profile']['target']}
    child_sources = [r for p in (ROOT / 'content/creatures/definitions').glob('creatures-*.json')
                     for r in json.loads(p.read_bytes())['records']
                     if r['definition']['identity']['key'] == children[0]['profile']['target']['key']]
    if len(child_sources) != 1 or child_binding not in child_sources[0]['source_bindings']:
        raise ValueError('Omrafir event child source binding changed')
    event_path, event_hash = next(iter(EVENT_CALLBACKS.items()))
    creature_events = {
        'callback':{'path':event_path,'sha256':event_hash},
        'hp_at_most_ppm':500000,'terminal_hp_at_most_ppm':30000,'summons_below':4,
        'boom_delay_ms':10,'reform_delay_ms':26764,
        'storage':{'key':112416,'initial':-1,'normal_reform_below':2},
        'summon':{'name':'Hellfire Fighter','amount':9,'center':[33589,32379,12],
                  'offset_x':[-8,8],'offset_y':[-9,9],'extended':True,'forced':True},
        'reform':{'name':'Omrafir','position':[33586,32379,12],'extended':False,'forced':True},
        'terminal_reform':{'name':'Omrafir2','position':[33586,32379,12],'extended':False,'forced':True,
                           'result':'unknown-monster-nil-then-lua-error'},
    }
    return {'schema':'OTERYN_OMRAFIR_SOURCE_ATTACK_OVERLAY/v1','source_revision':PIN,
            'source_definitions_sha256':native['source_definitions_sha256'],
            'native':record,'source_binding':binding,'callbacks':CALLBACKS,
            'schedules':schedules,'abilities':abilities,
            'beam_matrices':BEAM_MATRICES,
            'beam_delay_ms':4000,
            'beam_damage':{'minimum':7000,'maximum':10000,'element':'fire'},
            'healing':{'minimum':7500,'maximum':9000,'health_below_ppm':999900,'source_name':'Omrafir','require_tile':True,'exact_ids':[1487,1492,1500]},
            'regeneration_marker':{'sub_id':88888,'duration_ms':5000,'health_gain':0},
            'creature_events':creature_events,'event_children':[{'native':children[0],'source_binding':child_binding}]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.source_root:
        for path, expected in {**CALLBACKS, **EVENT_CALLBACKS}.items():
            if hashlib.sha256((args.source_root/path).read_bytes()).hexdigest() != expected:
                raise ValueError('Pinned Omrafir source changed: '+path)
    raw = (json.dumps(build(),ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != raw:
            raise ValueError('Stale Omrafir source witness')
    else:
        OUTPUT.write_bytes(raw)
    print('W01_OMRAFIR_SOURCE_ATTACK_OVERLAY',len(raw),hashlib.sha256(raw).hexdigest())

if __name__ == '__main__':
    main()
