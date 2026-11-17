"""Bind the selected First Dragon s5 fight to the existing timed Boss Activity owner.

The chosen recipe explicitly omits the full multi-phase source encounter. Its
final native creature, entry/exit coordinates and timers remain source-backed.
"""
import argparse
import hashlib
import json
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TARGET = ROOT / 'content/encounters/bosses/selected-first-dragon.json'
REVISION = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
QUEST = 'oteryn:quest.the_first_dragon_quest'
CREATURE = 'oteryn:creature.the_first_dragon'
BASE = 'data-global/scripts/quests/the_first_dragon/'


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def sha(value):
    return hashlib.sha256(value).hexdigest()


def source_blobs():
    paths = [BASE + name for name in ('actions_lever.lua', 'creaturescripts_death_fallen.lua',
             'creaturescripts_death_dragon_essence.lua', 'creaturescripts_death_somewhat_beatable.lua')]
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as corpus:
        manifest = json.load(corpus.extractfile('corpus-manifest.json'))
        result = {}
        for path in paths:
            row = next(r for r in manifest['files'] if r['source'] == 'crystalserver'
                       and r['revision'] == REVISION and r['path'] == path)
            raw = corpus.extractfile(row.get('blob_path', row.get('cache_path'))).read()
            if sha(raw) != row['sha256']:
                raise ValueError('FIRST_DRAGON_DONOR_DRIFT:' + path)
            result[path] = (raw.decode(), row['sha256'])
    return result


def build():
    definitions = json.loads((ROOT / 'content/quests/definitions/quests-00200-00299.json').read_bytes())
    definition = definitions['records'][23]['definition']
    recipe = definition['oteryn_recipe']['payload']['recipe']
    stages = {s['key']: s for s in recipe['stages']}
    if (stages['s4']['count'] != 2 or stages['s5']['count'] != 1
            or stages['s5']['kind'] != 'kill' or stages['s5']['targets'] != ['The First Dragon']
            or not any('omit seasonal scheduling and the full boss mechanics' in a
                       for a in recipe['adaptations'])):
        raise ValueError('FIRST_DRAGON_SELECTED_RECIPE_CHANGED')
    blobs = source_blobs()
    literal_proofs = {
        'actions_lever.lua': ['timeToFightAgain = 20 * 60 * 60', 'exit = Position(33597, 30994, 14)'],
        'creaturescripts_death_fallen.lua': ['corpse:setDestination(Position(33621, 31023, 14))'],
        'creaturescripts_death_dragon_essence.lua': ['Game.createMonster("The First Dragon", Position(33617, 31023, 14)'],
        'creaturescripts_death_somewhat_beatable.lua': ['Position(33617, 31023, 14), false, false, 14, 14, 14, 14'],
    }
    for suffix, anchors in literal_proofs.items():
        if any(anchor not in blobs[BASE + suffix][0] for anchor in anchors):
            raise ValueError('FIRST_DRAGON_SOURCE_ANCHOR_CHANGED:' + suffix)
    original = next(r for r in json.loads((ROOT / 'content/encounters/bosses/crystal-lever-candidates.json').read_bytes())['records']
                    if r['path'] == BASE + 'actions_lever.lua')
    if original['cooldown_seconds'] != 72000 or original['time_limit_seconds'] != 1200:
        raise ValueError('FIRST_DRAGON_CAPTURED_TIMER_CHANGED')
    profiles = json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())
    profile = next(r for r in profiles['records'] if r['profile']['target']['key'] == CREATURE)
    candidate = {
        'path': str(TARGET.relative_to(ROOT)), 'line': 1, 'boss_name': 'The First Dragon',
        'creature_key': CREATURE, 'boss_position': [33617, 31023, 14],
        'lever_positions': original['lever_positions'],
        'entry_positions': [[33621, 31023, 14] for _ in original['lever_positions']],
        'zone_from': [33603, 31009, 14], 'zone_to': [33631, 31037, 14],
        'exit_position': original['exit_position'], 'blockers': [],
        'required_callbacks': ['BossLeverOnDeath', 'SelectedFirstDragonDeath'],
        'callbacks_known': True, 'native_profile_sha256': sha(canonical(profile)),
        'complete_literal_layout': True, 'minimum_players': 1,
        'minimum_level': recipe['requirements']['min_level'],
        'cooldown_seconds': original['cooldown_seconds'],
        'time_limit_seconds': original['time_limit_seconds'],
        'after_kill_seconds': original['after_kill_seconds'],
    }
    document = {
        'schema': 'OTERYN_SELECTED_QUEST_ACTIVITY/v1', 'quest': QUEST, 'stage': 's5',
        'basis': 'CHOSEN_OTERYN_APPROXIMATION', 'recipe_sha256': sha(canonical(recipe)),
        'candidate': candidate,
        'sources': [{'path': path, 'sha256': digest, 'revision': REVISION,
                     'url': f'https://github.com/zimbadev/crystalserver/blob/{REVISION}/{path}'}
                    for path, (_, digest) in sorted(blobs.items())],
    }
    candidate['sha256'] = sha(canonical(document))
    return document


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('content', nargs='?')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    payload = canonical(build()) + b'\n'
    if args.check:
        if TARGET.read_bytes() != payload:
            raise ValueError('STALE_SELECTED_FIRST_DRAGON_ACTIVITY')
    else:
        TARGET.write_bytes(payload)
    print('First Dragon selected s5: 15 slots; cooldown72000s; deadline1200s')


if __name__ == '__main__':
    main()
