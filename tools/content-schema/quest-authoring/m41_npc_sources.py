#!/usr/bin/env python3
"""M41: the pinned Alyxo achievement/reward branch, on its existing aggregate quest.

Only this finale is admitted; this is not a declaration that all Kilmaresh is ready.
Canonical Remains already starts at zero; no absent-value comparison is changed.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PATH = 'content/dialogues/quest-rules-supplemental/m41-kilmaresh.json'
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
SOURCE = 'data-global/npc/alyxo.lua'
SHA = '86f3cf5aa793d5ccffd14bdf35bda841bb00f8c3ce3352826b088f4f25c4b008'
QUEST = 'oteryn:quest.kilmaresh_quest'
FAFNAR = 'oteryn:quest-progress/crystalserver/kilmaresh/thirteen/fafnar'
REMAINS = 'oteryn:quest-progress/crystalserver/kilmaresh/fourteen/remains'
PROGRAM = 'oteryn:quest-npc-program.m41_kilmaresh_finale'


def build(root=ROOT):
    capture = json.loads(gzip.decompress((root / 'tools/content-schema/quest-authoring/samples/donor-source/npc.json.gz').read_bytes()))
    witnesses = [row for row in capture['files'] if row['repository'] == 'zimbadev/crystalserver' and row['path'] == SOURCE]
    if len(witnesses) != 1 or witnesses[0]['blob_sha256'] != SHA:
        raise ValueError('M41_ALYXO_SOURCE_WITNESS')
    # The captured 9f5a72c and checked 201189f file have identical bytes; the
    # source URL/revision and SHA are recorded in M41-NPC-ACHIEVEMENTS-AND-BEAKS.md.
    source = lambda lines: [f'crystal:{REVISION}:{SOURCE}:{lines}']
    guard = {'kind': 'storage', 'track': FAFNAR, 'op': 'eq', 'value': 301}
    common = {'quest': QUEST, 'npc': 'oteryn:npc.alyxo', 'trigger': 'say'}
    return {'schema': 1, 'program_key': PROGRAM, 'quest': QUEST,
            'source_files': [{'path': SOURCE, 'sha256': SHA, 'revision': REVISION}],
            'tracks': [dict(key=key, quest=QUEST, initial=0, min=0, max=maximum, sources=source('190-204'))
                       for key, maximum in [(FAFNAR, 301), (REMAINS, 3)]],
            'rules': [
                dict(common, key='oteryn:quest-npc/m41-kilmaresh/alyxo/mission-offer',
                     keywords=['mission'], guards=[guard], effects=[{'kind': 'set_topic', 'value': 13}],
                     reply=['Did you finish the 3 jobs I gave you?'], sources=source('190-194')),
                dict(common, key='oteryn:quest-npc/m41-kilmaresh/alyxo/sculptor-reward',
                     keywords=['yes'], topics=[13], guards=[guard],
                     effects=[{'kind': 'achievement', 'name': 'Sculptor Apprentice'},
                              {'kind': 'give_item', 'item': 'oteryn:item.tibia.i31574', 'count': 1},
                              {'kind': 'set', 'track': REMAINS, 'value': 1},
                              {'kind': 'set_topic', 'value': 14}],
                     reply=['Congratulations, you have completed the 3 jobs I gave you.'],
                     sources=source('195-204'))]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--source-checkout', type=Path)
    args = parser.parse_args()
    if args.source_checkout and hashlib.sha256((args.source_checkout / SOURCE).read_bytes()).hexdigest() != SHA:
        raise SystemExit('M41_ALYXO_SOURCE_BYTES')
    data = (json.dumps(build(), ensure_ascii=False, indent=2) + '\n').encode()
    if args.check:
        if (ROOT / PATH).read_bytes() != data:
            raise SystemExit('M41_ALYXO_PROGRAMME_DRIFT')
    else:
        (ROOT / PATH).write_bytes(data)
