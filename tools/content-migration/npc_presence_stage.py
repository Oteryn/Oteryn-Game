#!/usr/bin/env python3
"""Stage NPC movement facts and voices for the NPC definitions (NPC-CONTENT-2).

NPC-BEHAVIOUR0 §5 and §6: `floor_change`, base speed, voice cadence, chance and lines on NPC
definitions, from the admitted sources (D9). Walk interval and radius already live on each NPC's
Behavior profile (`movement.wander`) and are not restaged.

Inputs are the `convert.py --include-text` outputs of the pinned Canary, Crystal and Crystal
summer-update checkouts. An NPC takes the facts of the source files its definition binds to:

- voices are admitted when every bound source has the same voices (two sources agree, or only one
  source has the NPC, D9); otherwise they are held and the NPC says nothing;
- a source that does not set `floorchange` or `speed` uses the engine default (Canary
  `npcs.hpp`: `floorChange` false, `baseSpeed` 55); differing sources hold the movement facts.

Writes one canonical JSON packet that `materialize_content_world_project_v2` pins by SHA-256.

Usage:
  python npc_presence_stage.py --canary OUT/canary --crystal OUT/crystal \
      --crystal-summer OUT/crystal-summer [--check]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
TREE = ROOT / 'content/npcs/definitions'
PACKET = ROOT / 'docs/agents/evidence/OTV2-20261015-npc-content-2-presence.json'
SCHEMA = 'OTERYN_NPC_PRESENCE_STAGE/v1'
FROM_REVISION = 'item-sem-use-2-consumables-20261013-r1'
REVISION = 'npc-content-2-presence-20261015-r1'
SOURCES = {
    ('crystalserver/npc-file', 'ff7ede593c69d4c658b382c97443e8155926924a'): 'crystal',
    ('crystalserver/npc-file', '00ce02a57ca5a12e48f32a3476e37471167e4c3f'): 'crystal-summer',
    ('canary/npc-file', '47dfd51f45280a59a1d3e50ba7edd573d7234446'): 'canary',
}
REPOSITORIES = {
    'canary': ('opentibiabr/canary', '47dfd51f45280a59a1d3e50ba7edd573d7234446'),
    'crystal': ('zimbadev/crystalserver', 'ff7ede593c69d4c658b382c97443e8155926924a'),
    'crystal-summer': ('zimbadev/crystalserver', '00ce02a57ca5a12e48f32a3476e37471167e4c3f'),
}
DEFAULT_FLOOR_CHANGE = False
DEFAULT_BASE_SPEED = 55
FIELD = 'oteryn:source.npc.'


class StageError(Exception):
    pass


def canonical(value: Any) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False) + '\n').encode()


def load_source(name: str, out: Path) -> dict[str, dict]:
    index = json.loads((out / 'index.json').read_text(encoding='utf-8'))
    repository, revision = REPOSITORIES[name]
    if index['source']['repository'] != repository or index['source']['revision'] != revision:
        raise StageError(f'{name}: converter output is not {repository}@{revision}')
    if index.get('include_text') is not True:
        raise StageError(f'{name}: converter output was written without --include-text')
    bundles = {}
    for path in sorted((out / 'bundles').glob('*.json')):
        bundle = json.loads(path.read_text(encoding='utf-8'))
        if bundle.get('definition') is not None:
            bundles[path.stem] = bundle
    return bundles


def voices_of(bundle: dict) -> tuple | None:
    voices = bundle.get('voices')
    if not voices or not voices['lines']:
        return None
    lines = []
    for line in voices['lines']:
        text = line['text']
        if not isinstance(text, dict) or not isinstance(text.get('text'), str):
            raise StageError(f'{bundle["key"]}: voice line without static text')
        if text['text']:  # an empty line says nothing
            lines.append((text['text'], line['yell']))
    if not lines or voices['chance_percent'] == 0:
        return None
    return voices['interval_ms'], voices['chance_percent'], tuple(lines)


def movement_of(bundle: dict) -> tuple[bool, int]:
    movement = bundle['definition']['movement']
    floor_change = movement['floor_change']
    base_speed = movement.get('base_speed')
    return (DEFAULT_FLOOR_CHANGE if floor_change is None else floor_change,
            DEFAULT_BASE_SPEED if base_speed is None else base_speed)


def npc_bindings() -> list[tuple[str, list[tuple[str, str]]]]:
    rows = []
    for shard in sorted(TREE.glob('npcs-*.json')):
        for record in json.loads(shard.read_text(encoding='utf-8'))['records']:
            key = record['declaration']['identity']['key']
            bound = sorted({(SOURCES[(b['identity_namespace'], b['source_revision'])], b['external_id'])
                            for b in record['source_bindings']
                            if (b['identity_namespace'], b['source_revision']) in SOURCES})
            rows.append((key, bound))
    return sorted(rows)


def field(name: str, kind: str, value: Any) -> dict:
    return {'field_path': FIELD + name, 'value': {'type': kind, 'value': value}}


def stage(sources: dict[str, dict[str, dict]]) -> dict:
    npcs, held = [], []
    counts = {'npcs': 0, 'sourced': 0, 'unsourced': 0, 'voiced': 0, 'voice_lines': 0, 'held_voices': 0,
              'held_movement': 0}
    for key, bound in npc_bindings():
        counts['npcs'] += 1
        bundles = [sources[name][stem] for name, stem in bound if stem in sources[name]]
        if not bundles:
            counts['unsourced'] += 1
            continue
        counts['sourced'] += 1
        fields = []
        movement = {movement_of(bundle) for bundle in bundles}
        if len(movement) == 1:
            floor_change, base_speed = movement.pop()
            fields += [field('base_speed', 'Integer', base_speed), field('floor_change', 'Boolean', floor_change)]
        else:
            counts['held_movement'] += 1
            held.append({'key': key, 'facts': 'movement', 'reason': 'SOURCES_DIFFER'})
        voices = {voices_of(bundle) for bundle in bundles}
        if len(voices) > 1:
            counts['held_voices'] += 1
            held.append({'key': key, 'facts': 'voices', 'reason': 'SOURCES_DIFFER'})
        elif (voice := voices.pop()) is not None:
            interval, chance, lines = voice
            if not isinstance(interval, int) or not isinstance(chance, int) or interval <= 0 or not 0 <= chance <= 100:
                counts['held_voices'] += 1
                held.append({'key': key, 'facts': 'voices', 'reason': 'TIMING_UNDECLARED'})
            else:
                counts['voiced'] += 1
                counts['voice_lines'] += len(lines)
                text = json.dumps([{'text': text, 'yell': yell} for text, yell in lines], ensure_ascii=False,
                                  separators=(',', ':'))
                fields += [field('voice_chance_percent', 'Integer', chance),
                           field('voice_interval_ms', 'Integer', interval),
                           field('voice_lines', 'Text', text)]
        if fields:
            npcs.append({'key': key, 'fields': sorted(fields, key=lambda f: f['field_path'])})
    return {
        'schema': SCHEMA,
        'from_project_revision': FROM_REVISION,
        'project_revision': REVISION,
        'sources': [{'source': name, 'repository': repo, 'revision': rev}
                    for name, (repo, rev) in sorted(REPOSITORIES.items())],
        'defaults': {'floor_change': DEFAULT_FLOOR_CHANGE, 'base_speed': DEFAULT_BASE_SPEED},
        'counts': counts,
        'npcs': npcs,
        'held': held,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--crystal', required=True, type=Path)
    parser.add_argument('--crystal-summer', required=True, type=Path)
    parser.add_argument('--check', action='store_true', help='fail if the committed packet differs')
    args = parser.parse_args()
    try:
        packet = stage({'canary': load_source('canary', args.canary),
                        'crystal': load_source('crystal', args.crystal),
                        'crystal-summer': load_source('crystal-summer', args.crystal_summer)})
    except StageError as error:
        sys.exit(f'npc_presence_stage: {error}')
    raw = canonical(packet)
    if args.check:
        if PACKET.read_bytes() != raw:
            sys.exit('npc_presence_stage: committed packet differs from the staged packet')
    else:
        PACKET.write_bytes(raw)
    print(json.dumps(packet['counts'], sort_keys=True))


if __name__ == '__main__':
    main()
