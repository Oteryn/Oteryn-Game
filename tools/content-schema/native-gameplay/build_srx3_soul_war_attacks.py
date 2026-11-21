#!/usr/bin/env python3
"""Bounded W11 source schedules omitted from the current native attack arrays.

The output is sealed implementation evidence, never a fabricated WorldProject.
Original native slots retain their exact order and shared attack-clock owner.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
CANARY = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
SPECS = {
 'goshnar_s_cruelty': ('d4ee2198ce417df9be2101b73047cf64d23dfe568a7eaca5eee88d437bd38da6', 'goshnars_cruelty', 5, 7000, 500000, 'CrueltyElemental'),
 'goshnar_s_megalomania_green': ('a1add91364a178e37c5e81b503e09604095ecfd783bc990cad0092bbc03db392', "goshnar's_megalomania_green", 7, 7000, 500000, 'MegalomaniaElemental'),
 'goshnar_s_megalomania_purple': ('b5dbc9b93939952331c86c5dd2bfde2cd0ffcf702838659daa3a4b337d297cc6', "goshnar's_megalomania_purple", 7, 7000, 500000, 'MegalomaniaElemental'),
 'goshnar_s_megalomania_blue': ('b47d9cc8b289deec0d63344695ff20533b053615cca57194f56ca243be993503', "goshnar's_megalomania_blue", 1, 6000, 1000000, 'MegalomaniaBlue'),
}
CALLBACKS = {
 'data-global/monster/quests/soul_war/goshnars_cruelty.lua': '5f71c7170921b537ba5bd397b70805c00cb3457da97f08a915383b3e34e58c0b',
 "data-global/monster/quests/soul_war/goshnar's_megalomania_green.lua": '073de955c173424a484e50885c2538749725fd437e3499382d3dc6c5199761f0',
 "data-global/monster/quests/soul_war/goshnar's_megalomania_purple.lua": 'cd2cae8c72c72a46064ec084c5db1684a4223e4150d3670f154a88bcb2589b2b',
 "data-global/monster/quests/soul_war/goshnar's_megalomania_blue.lua": 'd1ba4b9e8303d804ceaa75b0c4d24f9bf3181ed7ad6736861ba5ff67b7a64848',
 'data-global/lib/quests/soul_war.lua': '89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75',
 'data-global/scripts/quests/soul_war/spell-fire_beam_cruelty.lua': 'd65709890a2bfaefdd27ea83cf447cfe3f199a116b780acbea5c53bc81457476',
 'data-global/scripts/quests/soul_war/spell-fire_beam_megalomania.lua': '63622ea5ffab8761d2bcc042b8c7a1a01eb45bc1525666c186271013404be3b5',
 'data-global/scripts/quests/soul_war/spell-megalomania_blue.lua': 'fb972b25a4deb0070827f395199e8b4194ae57d5f219b244ef645a3d07444910',
}
OUTPUT = ROOT / 'tools/content-schema/native-gameplay/srx3-soul-war-attacks.json'


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()


def build():
    native = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
    canonical_creatures = [r for p in (ROOT / 'content/creatures/definitions').glob('creatures-*.json')
                           for r in json.loads(p.read_bytes())['records']]
    rows = []
    for slug, (body, file, index, interval, chance, kind) in SPECS.items():
        key = 'oteryn:creature.' + slug
        records = [r for r in native['records'] if r['profile']['target']['key'] == key]
        if len(records) != 1 or hashlib.sha256(canonical(records[0])).hexdigest() != body:
            raise ValueError('Native Soul War body changed: ' + key)
        record = records[0]
        creatures = [r for r in canonical_creatures if r['definition']['identity']['key'] == key]
        binding = {'disposition': 'EXACT', 'external_id': 'quests/soul_war/' + file,
                   'identity_namespace': 'canary/monster-file', 'source_key': 'oteryn:source.canary',
                   'source_revision': CANARY, 'target': record['profile']['target']}
        if len(creatures) != 1 or binding not in creatures[0]['source_bindings']:
            raise ValueError('Native source binding changed: ' + key)
        ability = {'family': 'Ability', 'key': f'oteryn:ability.srx3.soul-war.{slug}', 'revision': 'crystal-201189f'}
        if any(r['target'] == ability for r in record['attack_definitions']['profiles']):
            raise ValueError('Source callback would duplicate an admitted native ability')
        rows.append({'native': record, 'source_binding': binding, 'index': index,
                     'kind': kind, 'schedule': {'ability': ability, 'interval_ms': interval, 'chance_ppm': chance},
                     'ability': {'target': ability, 'data': {'kind': 'Ability', 'profile': {'details': {
                         'kind': 'Spell', 'range_tiles': 0, 'needs_target': False,
                         'needs_direction': kind != 'MegalomaniaBlue',
                     }}}}})
    return {'schema': 'OTERYN_SOUL_WAR_SOURCE_ATTACK_OVERLAY/v1', 'source_revision': PIN,
            'source_definitions_sha256': native['source_definitions_sha256'],
            'callbacks': CALLBACKS, 'records': rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-root', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.source_root:
        for path, expected in CALLBACKS.items():
            if hashlib.sha256((args.source_root / path).read_bytes()).hexdigest() != expected:
                raise ValueError('Pinned Soul War callback changed: ' + path)
    raw = (json.dumps(build(), ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()
    if args.check:
        if not OUTPUT.exists() or OUTPUT.read_bytes() != raw:
            raise ValueError('Stale Soul War source attack witness')
    else:
        OUTPUT.write_bytes(raw)
    print('W11_SOURCE_ATTACK_OVERLAY', len(raw), hashlib.sha256(raw).hexdigest())


if __name__ == '__main__':
    main()
