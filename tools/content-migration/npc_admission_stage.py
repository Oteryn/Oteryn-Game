#!/usr/bin/env python3
"""Stage NPC promotion candidates as WorldProject/v2 records (OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1 §3-§4).

Pure: reads the committed promotion candidates and the protected v2 Item registry, writes one canonical JSON
packet that `materialize_content_world_project_v2` pins by SHA-256. No text, no placements, no gated rows.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CANDIDATES = ROOT / 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json'
REFERENCE = ROOT / 'content/world/definitions/reference.json'
CANARY_REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CRYSTAL_REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
ITEM_MAP_SHA256 = '83ba3c26d10af8834191bf5491280882b6453bca0911b86d180c07a15cec679a'
COORDINATE_FRAME = 'global-target-2026-09-27'
REVISION = 'definition-r1'
SLUG = re.compile(r'[a-z0-9]+(?:_[a-z0-9]+)*')
PILOT_SIZE = 20


class StageError(Exception):
    pass


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def ref(family: str, key: str) -> dict:
    return {'family': family, 'key': key, 'revision': REVISION}


def slug_of(key: str, prefix: str) -> str:
    if not key.startswith(prefix):
        raise StageError(f'{key}: expected prefix {prefix}')
    slug = key[len(prefix):]
    if not SLUG.fullmatch(slug) or len(slug) > 64:
        raise StageError(f'{key}: slug is not a production slug')
    return slug


def route_key(keyword: str) -> str:
    key = '_'.join(re.findall(r'[a-z0-9]+', keyword.lower()))
    if not SLUG.fullmatch(key) or len(key) > 64:
        raise StageError(f'route keyword {keyword!r} has no slug')
    return key


def text_field(path: str, value: Any) -> list[dict]:
    if not isinstance(value, str) or not value:
        return []
    return [{'field_path': f'oteryn:source.npc.{path}', 'value': {'type': 'Text', 'value': value.lower()}}]


def presentation_profile(outfit: dict) -> dict:
    if 'item_look' in outfit:
        return {'asset_binding': f'canary.appearance:object/{outfit["item_look"]}', 'light_level': 0}
    look = outfit['look_type']
    profile: dict[str, Any] = {'asset_binding': f'canary.appearance:outfit/{look}', 'light_level': 0}
    palette = [{'slot': slot, 'asset_binding': f'canary.appearance:palette/{outfit[field]}'}
               for slot, field in (('Head', 'head'), ('Body', 'body'), ('Legs', 'legs'), ('Feet', 'feet'))
               if outfit.get(field)]
    attachments = [{'slot': 'Addon', 'asset_binding': f'canary.appearance:outfit/{look}/addon-{bit}'}
                   for bit in (1, 2) if outfit.get('addons', 0) & bit]
    if outfit.get('mount'):
        attachments.append({'slot': 'Mount', 'asset_binding': f'canary.appearance:outfit/{outfit["mount"]}'})
    if palette:
        profile['palette_bindings'] = palette
    if attachments:
        profile['attachment_bindings'] = attachments
    return profile


def behavior_profile(movement: dict) -> dict:
    if movement['floor_change']:
        raise StageError('floor-changing NPCs are outside wave A')
    walks = movement['walk_interval_ms'] > 0
    motion: dict[str, Any] = {'can_walk': walks, 'pass_through': False, 'pushable': False, 'push_items': False,
                              'push_creatures': False, 'walks_on_energy': False, 'walks_on_fire': False,
                              'walks_on_poison': False}
    if walks:
        motion['wander'] = {'interval_ms': movement['walk_interval_ms'], 'radius_tiles': movement['walk_radius']}
    return {'movement': motion,
            'targeting': {'hostile': False, 'can_target': False, 'sense_invisible': False, 'target_distance_tiles': 0,
                          'static_attack_chance_ppm': 0, 'flee_health': 0}}


def trade_declaration(service: dict, registered: set[str]) -> dict:
    currency = service['currency']
    offers = []
    for offer in service['offers']:
        for item in [offer['item']] + ([currency] if currency else []):
            if item['family'] != 'Item' or item['key'] not in registered:
                raise StageError(f'{service["identity"]["key"]}: unregistered Item {item["key"]}')
        row = {'item': ref('Item', offer['item']['key']), 'direction': offer['direction'],
               'unit_price': offer['unit_price']}
        if currency:
            row['currency'] = ref('Item', currency['key'])
        for field in ('count', 'sub_type'):
            if offer[field] is not None:
                row[field] = offer[field]
        offers.append(row)
    offers.sort(key=lambda row: (row['item']['key'], row['direction'], row.get('currency', {}).get('key', ''),
                                 row.get('count', 0), row.get('sub_type', -1)))
    return {'kind': 'Service', 'identity': {'key': service['identity']['key'], 'revision': REVISION},
            'offers': offers, 'fields': []}


def travel_declaration(service: dict) -> dict:
    routes = []
    for route in service['routes']:
        row = {'key': route_key(route['destination_keyword']),
               'destination': {'coordinate_frame': COORDINATE_FRAME, 'x': route['destination']['x'],
                               'y': route['destination']['y'], 'floor': route['destination']['z']},
               'price': route['price'], 'premium': route['premium']}
        if route['min_level'] is not None:
            row['min_level'] = route['min_level']
        routes.append(row)
    routes.sort(key=lambda row: row['key'])
    if len({row['key'] for row in routes}) != len(routes):
        raise StageError(f'{service["identity"]["key"]}: duplicate route key')
    return {'kind': 'Service', 'identity': {'key': service['identity']['key'], 'revision': REVISION},
            'routes': routes, 'fields': []}


def pilot(candidates: list[dict]) -> list[dict]:
    """The slice-3 pilot: the first NPC (by key) for each shape the admission must prove, then the first by key."""
    def first(predicate):
        return next(c for c in candidates if predicate(c))

    def offers(c):
        return c['trade_service']['offers'] if c['trade_service'] else []

    shapes = [
        lambda c: c['travel_service'] is not None and any(r['premium'] or r['min_level'] for r in c['travel_service']['routes']),
        lambda c: c['trade_service'] is not None and c['trade_service']['currency'] is not None,
        lambda c: any(o['count'] is not None for o in offers(c)),
        lambda c: any(o['sub_type'] is not None for o in offers(c)),
        lambda c: set(c['provenance']) == {'crystal'},
        lambda c: any(r['fact'] == 'placements' for r in c['arbitration']),
        lambda c: any(r['fact'].startswith('travel.') for r in c['arbitration']),
        lambda c: any(r['fact'].startswith('trade.') for r in c['arbitration']),
        lambda c: 'item_look' in c['presentation']['outfit'],
        lambda c: bool(c['presentation']['outfit'].get('mount')),
        lambda c: c['movement']['walk_interval_ms'] == 0,
    ]
    chosen = {first(shape)['identity']['key'] for shape in shapes}
    for candidate in candidates:
        if len(chosen) >= PILOT_SIZE:
            break
        chosen.add(candidate['identity']['key'])
    return [c for c in candidates if c['identity']['key'] in chosen]


def count_dialogue_nodes(nodes: list) -> int:
    return sum(1 + count_dialogue_nodes(node.get('children', [])) for node in nodes)


def stage(report: dict, registered: set[str], pilot_only: bool, dialogue_index: dict | None = None,
          dialogue_staged_sha256: str | None = None) -> dict:
    if report['schema'] != 'OTERYN_NPC_PROMOTION_CANDIDATES/v1' or report['item_map_sha256'] != ITEM_MAP_SHA256:
        raise StageError('promotion candidate report drifted')
    candidates = sorted(report['candidates'], key=lambda c: c['identity']['key'])
    # The source declares no walk interval or radius; the engine default is not evidence, so the NPC waits.
    deferred = [{'npc': c['identity']['key'], 'reason': 'MOVEMENT_UNDECLARED'} for c in candidates
                if c['movement']['walk_interval_ms'] is None or c['movement']['walk_radius'] is None]
    candidates = [c for c in candidates if c['identity']['key'] not in {d['npc'] for d in deferred}]
    if pilot_only:
        candidates = pilot(candidates)
    records, declarations, profiles, bindings = [], [], [], []
    wiki_pages = 0
    dialogue_count = 0
    dialogue_node_count = 0
    # D8: Day/Night and stage variants share one wiki page; a page binds only the NPC it names alone.
    wiki_bound = [c['wiki']['pageid'] for c in candidates if wiki_decided(c)]
    shared_pages = {page for page in wiki_bound if wiki_bound.count(page) > 1}
    for candidate in candidates:
        key = candidate['identity']['key']
        slug = slug_of(key, 'oteryn:npc.')
        presentation = f'oteryn:presentation.npc.{slug}'
        behavior = f'oteryn:behavior.npc.{slug}'
        records.append({'kind': 'Generic', 'identity': ref('Presentation', presentation), 'client_projection': 'ClientSafe'})
        records.append({'kind': 'Generic', 'identity': ref('Behavior', behavior), 'client_projection': 'ServerOnly'})
        profiles.append({'target': ref('Presentation', presentation),
                         'data': {'kind': 'Presentation', 'profile': presentation_profile(candidate['presentation']['outfit'])}})
        profiles.append({'target': ref('Behavior', behavior),
                         'data': {'kind': 'Behavior', 'profile': behavior_profile(candidate['movement'])}})
        services = []
        if candidate['trade_service']:
            declarations.append(trade_declaration(candidate['trade_service'], registered))
            services.append(ref('Service', candidate['trade_service']['identity']['key']))
        if candidate['travel_service']:
            declarations.append(travel_declaration(candidate['travel_service']))
            services.append(ref('Service', candidate['travel_service']['identity']['key']))
        dialogue_declaration = dialogue_index.get(key) if dialogue_index else None
        dialogue_ref = None
        if dialogue_declaration is not None:
            dialogue_ref = ref('Dialogue', dialogue_declaration['identity']['key'])
            declarations.append(dialogue_declaration)
            dialogue_count += 1
            dialogue_node_count += count_dialogue_nodes(dialogue_declaration.get('keywords', []))
        declarations.append({'kind': 'NPC', 'identity': {'key': key, 'revision': REVISION},
                             'presentation': ref('Presentation', presentation), 'behavior': ref('Behavior', behavior),
                             'dialogue': dialogue_ref, 'services': sorted(services, key=lambda r: r['key']),
                             'fields': text_field('profession', candidate['profession'])
                             + text_field('speech_bubble', candidate['presentation'].get('speech_bubble'))})
        for source, source_key, revision in (('canary', 'oteryn:source.canary', CANARY_REVISION),
                                             ('crystal', 'oteryn:source.crystalserver', CRYSTAL_REVISION)):
            if source in candidate['provenance']:
                stem = candidate['provenance'][source]['key'].split(':npc/', 1)[1]
                namespace = 'canary/npc-file' if source == 'canary' else 'crystalserver/npc-file'
                bindings.append({'source_key': source_key, 'source_revision': revision, 'identity_namespace': namespace,
                                 'external_id': stem, 'target': ref('NPC', key), 'disposition': 'EXACT'})
        if wiki_decided(candidate) and candidate['wiki']['pageid'] not in shared_pages:
            wiki_pages += 1
            bindings.append({'source_key': 'oteryn:source.tibiawiki', 'source_revision': wiki_revision(report),
                             'identity_namespace': 'mediawiki/page_id', 'external_id': str(candidate['wiki']['pageid']),
                             'target': ref('NPC', key), 'disposition': 'EXACT'})
    records.sort(key=lambda r: (r['identity']['family'], r['identity']['key']))
    profiles.sort(key=lambda p: (p['target']['family'], p['target']['key']))
    declarations.sort(key=lambda d: (d['kind'], d['identity']['key']))
    bindings.sort(key=lambda b: (b['source_key'], b['source_revision'], b['identity_namespace'], b['external_id']))
    services = [d for d in declarations if d['kind'] == 'Service']
    source = {'canary_revision': CANARY_REVISION, 'crystal_revision': CRYSTAL_REVISION,
             'candidates_sha256': hashlib.sha256(CANDIDATES.read_bytes()).hexdigest(),
             'item_map_sha256': ITEM_MAP_SHA256, 'wiki_snapshot_sha256': report['snapshot_sha256'],
             'wiki_revision': wiki_revision(report)}
    if 'br_facts_sha256' in report:  # D12: prices both wikis agree on also come from the TibiaWiki BR facts
        source['br_facts_sha256'] = report['br_facts_sha256']
        source['br_revision'] = f'tibiawiki-br-npc-{report["br_facts_sha256"][:16]}'
    counts = {'npcs': len(candidates), 'records': len(records), 'profiles': len(profiles),
             'trade_services': sum(1 for d in services if 'offers' in d),
             'travel_services': sum(1 for d in services if 'routes' in d),
             'offers': sum(len(d.get('offers', [])) for d in services),
             'routes': sum(len(d.get('routes', [])) for d in services),
             'declarations': len(declarations), 'bindings': len(bindings), 'wiki_pages': wiki_pages}
    if dialogue_index is not None:
        source['dialogue_staged_sha256'] = dialogue_staged_sha256
        counts['dialogues'] = dialogue_count
        counts['dialogue_nodes'] = dialogue_node_count
    return {
        'schema': 'OTERYN_NPC_ADMISSION_STAGED/v1',
        'wave': 'pilot' if pilot_only else 'A',
        'source': source,
        'counts': counts,
        'records': records,
        'declarations': declarations,
        'authoring_profiles': profiles,
        'source_identity_bindings': bindings,
        'deferred': [] if pilot_only else deferred,
    }


def wiki_decided(candidate: dict) -> bool:
    """D6: the wiki decided a fact or confirmed a single-source NPC."""
    return bool(candidate['wiki']) and (bool(candidate['arbitration']) or len(candidate['provenance']) == 1)


def wiki_revision(report: dict) -> str:
    return f'tibiawiki-npc-{report["snapshot_sha256"][:16]}'


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--pilot', action='store_true', help='stage only the slice-3 pilot NPCs')
    parser.add_argument('--dialogues', type=Path, help='OTERYN_NPC_DIALOGUE_STAGED/v1 packet to bind into NPCs')
    args = parser.parse_args()
    report = json.loads(CANDIDATES.read_text())
    registered = {r['identity']['key'] for r in json.loads(REFERENCE.read_text())['records']
                  if r['identity'].get('family') == 'Item' or r.get('kind') == 'Item'}
    dialogue_index = dialogue_sha256 = None
    if args.dialogues:
        dialogue_bytes = args.dialogues.read_bytes()
        dialogue_sha256 = hashlib.sha256(dialogue_bytes).hexdigest()
        dialogue_packet = json.loads(dialogue_bytes)
        if dialogue_packet['schema'] != 'OTERYN_NPC_DIALOGUE_STAGED/v1':
            print('npc admission stage: dialogue packet drifted', file=sys.stderr)
            return 1
        dialogue_index = {entry['npc']: entry['declaration'] for entry in dialogue_packet['dialogues']}
    try:
        packet = stage(report, registered, args.pilot, dialogue_index, dialogue_sha256)
    except StageError as error:
        print(f'npc admission stage: {error}', file=sys.stderr)
        return 1
    args.out.write_bytes(canonical(packet))
    print(json.dumps(packet['counts'], sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
