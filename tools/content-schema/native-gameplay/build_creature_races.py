"""Build content/creatures/definitions/creature-races.json: each Creature's race (SPELL-PRESENT-0 §3).

The file is a Creature family overlay keyed by definition identity, registered on the family index
beside the other native profile overlays: the shards themselves are regenerated from the World
declarations, so a hand-added field would not survive their round trip.

The race comes from the monster file each Creature's source binding names, at that binding's
revision: `monster.race = "<race>"`, or Canary's MonsterType default `RACE_BLOOD`
(src/creatures/monsters/monsters.hpp) when the file sets none. A Creature without a monster file
(a wiki-authored one) takes the same default. Players are blood (hit-table.json).
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

from build_presentation_bindings import CREATURE_RACES, RACES, dumps

ROOT = Path(__file__).resolve().parents[3]
DEFINITIONS = ROOT / 'content' / 'creatures' / 'definitions'
OUT = CREATURE_RACES
# namespace -> (repository, revision, path of an external id); a Canary id is the file below
# data-otservbr-global/monster/ without its extension, a Crystal id is the repository path. A
# primal variant's id is `<file>.lua#registered-type=<name>`: a helper registers it from that
# file, overriding experience, loot, name, description and health only, so the race is the file's.
SOURCES = {
    'canary/monster-file': ('opentibiabr/canary', '47dfd51f45280a59a1d3e50ba7edd573d7234446',
                            lambda i: 'data-otservbr-global/monster/' + (
                                i.split('#', 1)[0] if '#' in i else i + '.lua')),
    'crystalserver/monster-file': ('zimbadev/crystalserver', '00ce02a57ca5a12e48f32a3476e37471167e4c3f',
                                   lambda i: i),
}
DEFAULT = 'blood'
RACE = re.compile(r'^\s*monster\.race\s*=\s*"([a-z]+)"\s*$', re.M)


def without_lua_comments(text):
    return re.sub(r'--\[\[.*?\]\]|--[^\n]*', '', text, flags=re.S)


def source_race(text):
    found = RACE.findall(without_lua_comments(text))
    if len(found) > 1:
        raise ValueError('more than one monster.race')
    if found and found[0] not in RACES:
        raise ValueError('unknown race ' + found[0])
    return (found[0], 'explicit') if found else (DEFAULT, 'source_default')


def creature_records():
    index = json.loads((DEFINITIONS / 'index.json').read_bytes())
    for shard in index['shards']:
        yield from json.loads((ROOT / shard).read_bytes())['records']


def build(checkouts):
    records = []
    for record in creature_records():
        identity = record['definition']['identity']
        bindings = [b for b in record['source_bindings'] if b['identity_namespace'] in SOURCES]
        if len(bindings) > 1:
            raise ValueError('more than one monster file for ' + identity['key'])
        if not bindings:
            records.append({'creature': identity, 'race': DEFAULT,
                            'disposition': 'no_monster_file', 'source': None})
            continue
        binding = bindings[0]
        repository, revision, path_of = SOURCES[binding['identity_namespace']]
        path = path_of(binding['external_id'])
        if not binding['source_revision'].endswith(revision):
            raise ValueError('unpinned source revision for ' + identity['key'])
        blob = subprocess.check_output(['git', '-C', str(checkouts[repository]), 'show',
                                        revision + ':' + path])
        race, disposition = source_race(blob.decode())
        records.append({'creature': identity, 'race': race, 'disposition': disposition,
                        'source': {'repository': repository, 'path': path,
                                   'sha256': hashlib.sha256(blob).hexdigest()}})
    records.sort(key=lambda r: r['creature']['key'])
    return {
        'schema': 'OTERYN_CREATURE_RACES/v1',
        'decision': 'SPELL-PRESENT-0 §3',
        'default': {'race': DEFAULT, 'source': 'opentibiabr/canary src/creatures/monsters/monsters.hpp '
                    'MonsterInfo::race = RACE_BLOOD'},
        'sources': {repository: revision for repository, revision, _ in SOURCES.values()},
        'record_count': len(records),
        'records': records,
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--canary', type=Path, required=True, help='opentibiabr/canary clone')
    parser.add_argument('--crystal', type=Path, required=True, help='zimbadev/crystalserver clone')
    args = parser.parse_args()
    OUT.write_text(dumps(build({'opentibiabr/canary': args.canary,
                                'zimbadev/crystalserver': args.crystal})))
