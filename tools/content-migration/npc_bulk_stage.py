#!/usr/bin/env python3
"""Stage flagged basic NPCs through the existing WorldProject/v2 authoring path.

Donor profiles are documentary reference data, never verified Tibia facts. Missing
appearance uses an explicit placeholder; economic/quest actions remain disabled.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

from npc_admission_stage import behavior_profile, presentation_profile, ref

ROOT = Path(__file__).resolve().parents[2]
LEDGER = ROOT / 'docs/agents/evidence/OTV2-20261001-npc-source-audit-r19/original160-progress.json'
BR = ROOT / 'imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json'
TP = ROOT / 'imports/tibiawiki/npc-tibiopedia/2026-09-28/tibiopedia-npc-facts.json'
FROM = 'g4-npc-qualified-bounded-definitions-r18'
TO = 'g4-npc-provisional-first45-r19'


def canonical(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')) + '\n').encode()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def field(name, value):
    text = value if isinstance(value, str) else canonical(value).decode().strip()
    return {'field_path': 'oteryn:source.npc.bulk.' + name,
            'value': {'type': 'Text', 'value': text}}


def build(profiles, remaining=False):
    admitted = {'DEFINITION_ADMITTED_PREDECESSOR', 'QUALIFIED_EIGHT_CANDIDATE'}
    pending = [r for r in json.loads(LEDGER.read_text())['actors'] if r['status'] not in admitted]
    targets = pending[45:] if remaining else pending[:45]
    keys = {r['key'] for r in targets}
    if len(keys) != (88 if remaining else 45) or set(profiles) - keys:
        raise ValueError('unknown/duplicate target or incomplete bulk ledger')
    report = json.loads((ROOT / 'tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json').read_text())
    if digest(BR) != report['br_facts_sha256'] or digest(TP) != report['tibiopedia_facts_sha256']:
        raise ValueError('wiki snapshot digest drifted')
    br = {name.casefold(): r for r in json.loads(BR.read_text())['pages']
          for name in [r['name'], r['title']]}
    tp = {r['name'].casefold(): r for r in json.loads(TP.read_text())['pages']}
    sources = json.loads((ROOT / 'content/world/provenance/sources.json').read_text())['sources']
    revisions = ['tibiawiki-br-npc-' + digest(BR)[:16], 'tibiopedia-npc-' + digest(TP)[:16]]
    reused = [s for s in sources if s['key'] == 'oteryn:source.tibiawiki' and s['revision'] in revisions]
    if len(reused) != 2:
        raise ValueError('wiki source registration drifted')
    additions = {name: [] for name in ['records', 'declarations', 'authoring_profiles', 'source_identity_bindings']}
    progress = []
    for target in targets:
        key, name = target['key'], target['name']
        stem = key.removeprefix('oteryn:npc.')
        page = br.get(name.casefold()) or br.get(name.removesuffix(' (NPC)').casefold())
        other = tp.get(name.casefold()) or tp.get(name.removesuffix(' (NPC)').casefold())
        if page is None or page.get('removed'):
            raise ValueError('missing/removed BR identity: ' + name)
        donor = profiles.get(key)
        outfit = donor['outfit'] if donor else {'look_type': 128, 'head': 0, 'body': 0,
                                                'legs': 0, 'feet': 0, 'addons': 0, 'mount': None}
        movement = donor.get('movement', {}) if donor else {}
        interval = movement.get('walk_interval_ms')
        radius = movement.get('walk_radius')
        quality = {'name': 'verified', 'presentation': 'donor' if donor else 'placeholder',
                   'movement.walk_interval_ms': 'donor' if interval is not None else 'defaulted',
                   'movement.walk_radius': 'donor' if radius is not None else 'defaulted',
                   'movement.floor_change': 'defaulted', 'dialogue': 'placeholder',
                   'placements': 'todo', 'services': 'todo', 'quests': 'todo',
                   'profession': 'todo', 'voices': 'todo',
                   'wiki.tibiopedia': 'verified' if other else 'todo'}
        for part in outfit:
            quality['presentation.' + part] = ('defaulted' if part in donor.get('missing_fields', [])
                                               else 'donor') if donor else 'placeholder'
        if donor:
            source = donor['source']
            if source.get('literal_name', '').casefold() != name.casefold():
                raise ValueError('donor actor identity mismatch: ' + name)
            if not re.fullmatch(r'[0-9a-f]{64}', source.get('sha256', '')) or not re.fullmatch(r'[0-9a-f]{40}', source.get('revision', '')):
                raise ValueError('donor source custody missing: ' + name)
        motion = {'walk_interval_ms': 0 if interval is None else interval,
                  'walk_radius': 2 if radius is None else radius, 'floor_change': False}
        if type(motion['walk_interval_ms']) is not int or motion['walk_interval_ms'] < 0:
            raise ValueError('invalid walk interval: ' + name)
        if type(motion['walk_radius']) is not int or not 0 <= motion['walk_radius'] <= 255:
            raise ValueError('invalid walk radius: ' + name)
        presentation = ref('Presentation', 'oteryn:presentation.npc.' + stem)
        behavior = ref('Behavior', 'oteryn:behavior.npc.' + stem)
        dialogue = ref('Dialogue', 'oteryn:dialogue.npc.' + stem)
        for identity, projection, data in [
            (presentation, 'ClientSafe', {'kind': 'Presentation', 'profile': presentation_profile(outfit)}),
            (behavior, 'ServerOnly', {'kind': 'Behavior', 'profile': behavior_profile(motion)}),
        ]:
            additions['records'].append({'kind': 'Generic', 'identity': identity, 'client_projection': projection})
            additions['authoring_profiles'].append({'target': identity, 'data': data})
        disabled = ['trade', 'travel', 'quest', 'spells', 'blessings', 'promotion']
        metadata = {'status': 'provisional', 'name': page['name'], 'quality': quality,
                    'name_scope': 'Identity in pinned BR snapshot' + (' and Tibiopedia' if other else '; Tibiopedia absent') + '; no gameplay equivalence claim.',
                    'disabled_features': disabled, 'wiki_positions': page.get('positions', []),
                    'wiki_trade_reference': page.get('trades', {}),
                    'qualification_map': 'apps/game-server/src/content/project/native_entry_room.json',
                    'source_reference': donor['source'] if donor else {'kind': 'OTERYN_PLACEHOLDER'},
                    'previous_hold': target['status']}
        additions['declarations'].append({'kind': 'NPC', 'identity': {'key': key, 'revision': 'definition-r1'},
            'presentation': presentation, 'behavior': behavior, 'dialogue': dialogue, 'services': [],
            'fields': [field(k, v) for k, v in sorted(metadata.items())]})
        additions['declarations'].append({'kind': 'Dialogue',
            'identity': {'key': dialogue['key'], 'revision': 'definition-r1'},
            'greet': ['Hello.'], 'farewell': ['Goodbye.'],
            'keywords': [{'key': 'name', 'triggers': ['name'], 'reply': ['I am ' + page['name'] + '.']},
                         {'key': 'job', 'triggers': ['job'], 'reply': ['I can answer simple questions.']}],
            'fields': [field('quality', {'dialogue': 'placeholder'}), field('status', 'provisional')]})
        bindings = [(revisions[0], 'mediawiki/page_id', str(page['pageid']))]
        if other:
            bindings.append((revisions[1], 'tibiopedia/npc-page-url', other['url']))
        for revision, namespace, external_id in bindings:
            additions['source_identity_bindings'].append({'source_key': 'oteryn:source.tibiawiki',
                'source_revision': revision, 'identity_namespace': namespace, 'external_id': external_id,
                'target': ref('NPC', key), 'disposition': 'EXACT'})
        progress.append({'key': key, 'name': name, 'state': 'DATA_READY_PARTIAL', 'field_quality': quality,
                         'disabled_features': disabled, 'native_runtime_loaded': False})
    for rows in additions.values():
        rows.sort(key=canonical)
    packet = {'schema': 'OTERYN_NPC_BULK_PROVISIONAL/v1', 'from_project_revision': TO if remaining else FROM,
              'project_revision': 'g4-npc-provisional-remaining88-r20' if remaining else TO,
              'reused_sources': reused, 'native_additions': additions}
    return packet, progress


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profiles', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--remaining', action='store_true', help='Stage the remaining45+43 together.')
    args = parser.parse_args()
    packet, progress = build(json.loads(args.profiles.read_text()), args.remaining)
    args.output.write_bytes(canonical(packet))
    print(json.dumps({'npc': len(progress), 'dialogue': len(progress),
                      'donor_appearance': sum(r['field_quality']['presentation'] == 'donor' for r in progress),
                      'placeholder_appearance': sum(r['field_quality']['presentation'] == 'placeholder' for r in progress),
                      'runtime_loaded': 0, 'sha256': digest(args.output)}))
