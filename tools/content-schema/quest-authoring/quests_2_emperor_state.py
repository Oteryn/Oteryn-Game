"""Closed native flag curation for the batch 2 world callback source guards.

This does not rewrite the historical SOURCE capture. The seven declared Crystal
symbols use the already established canonical quest progress paths. Each new
source binding explicitly identifies Crystal; numeric-ID coincidence is never
used to join state. Player::getStorageValue returns -1 when absent:
zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
src/creatures/players/player.cpp:1341-1349.
"""
import argparse
import copy
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
STORAGES = 'data-global/lib/core/storages.lua'
# Declaration line, numeric witness, and source writer. All references have
# repository zimbadev/crystalserver and revision PIN; numeric witnesses are
# validated evidence, never native state keys.
FLAGS = (
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission14', 2013, 44215, 'data-global/npc/omrabas.lua', 430),
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission32a', 2032, 44234, 'data-global/scripts/quests/the_gravedigger_of_drefia/actions_monks.lua', 2),
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission32b', 2033, 44235, 'data-global/scripts/quests/the_gravedigger_of_drefia/actions_monks.lua', 3),
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission33', 2034, 44236, 'data-global/scripts/quests/the_gravedigger_of_drefia/actions_monks.lua', 4),
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission39', 2044, 44246, 'data-global/scripts/quests/the_gravedigger_of_drefia/actions_pyramids1.lua', 5),
    ('the_gravedigger_of_drefia', 'u10_10', 'Mission74', 2079, 44281, 'data-global/npc/omrabas.lua', 407),
    ('the_new_frontier', 'u8_54', 'SnakeHeadTeleport', 1668, 42731, 'data-global/npc/cael.lua', 120),
)


def suffix(symbol):
    return 'snake_head_teleport' if symbol == 'SnakeHeadTeleport' else symbol.lower()


def key(flag):
    quest, version, symbol, *_ = flag
    return f'oteryn:quest-progress/quest/{version}/{quest}/{suffix(symbol)}'


KEYS = frozenset(key(flag) for flag in FLAGS)


def tracks():
    return [{'key': key(flag), 'quest': f'oteryn:quest.{flag[0]}_quest',
             'initial': -1, 'min': -1, 'max': 1, 'bounds_basis': 'observed_values',
             'source_key': 'crystalserver:' + key(flag).removeprefix('oteryn:')}
            for flag in FLAGS]


def verify_source(crystal):
    paths = {STORAGES, 'src/creatures/players/player.cpp'} | {flag[5] for flag in FLAGS}
    sources = {path: subprocess.check_output(['git', '-C', str(crystal), 'show',
                                             f'{PIN}:{path}'], text=True).splitlines()
               for path in paths}
    for _, _, symbol, line, storage_id, script, writer in FLAGS:
        if not sources[STORAGES][line - 1].strip().startswith(f'{symbol} = {storage_id},'):
            raise ValueError('QUESTS_2_EMPEROR_STORAGE_DECLARATION')
        if f'.{symbol}' not in sources[script][writer - 1]:
            raise ValueError('QUESTS_2_EMPEROR_FLAG_WRITER')
    if sources['src/creatures/players/player.cpp'][1341].strip() != 'int32_t value = -1;':
        raise ValueError('QUESTS_2_EMPEROR_ABSENT_STORAGE')
    # The dynamic configuration fields above are set by the same pinned scalar
    # assignment, not by a guessed counter or timestamp.
    for script, line, expression in (
        ('actions_monks.lua', 16, 'player:setStorageValue(cStorages[2], 1)'),
        ('actions_pyramids1.lua', 16, 'player:setStorageValue(cStorages[2], 1)'),
    ):
        path = 'data-global/scripts/quests/the_gravedigger_of_drefia/' + script
        if sources[path][line - 1].strip() != expression:
            raise ValueError('QUESTS_2_EMPEROR_DYNAMIC_FLAG_VALUE')


def append_tracks(quests):
    additions = tracks()
    owners = {q['quest']: q for q in quests}
    if len(owners) != len(quests) or any(t['quest'] not in owners for t in additions):
        raise ValueError('QUESTS_2_EMPEROR_QUEST_OWNER')
    current = {t['key']: t for q in quests for t in q['tracks']}
    for track in additions:
        if track['key'] in current and current[track['key']] != track:
            raise ValueError('QUESTS_2_EMPEROR_TRACK_CONFLICT:' + track['key'])
    for track in additions:
        if track['key'] not in current:
            owners[track['quest']]['tracks'].append(copy.deepcopy(track))
    for quest_key in {track['quest'] for track in additions}:
        owners[quest_key]['tracks'].sort(key=lambda track: track['key'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    args = parser.parse_args()
    verify_source(args.crystal)
    print(json.dumps({'source_revision': PIN, 'flags': len(FLAGS)}))
