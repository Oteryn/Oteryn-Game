"""Capture the literal activity geometry and policies in Crystal 201189f8."""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = '201189f871d23597b82caf88fd611454a2ddefb8'
PREFIX = 'data-global/scripts/quests/dangerous_depth/'
NAMES = ('The Baron from Below', 'The Count of the Core', 'The Duke of the Depths')
ROMANS = ('IV', 'V', 'VI')
SOURCE_DIGESTS = {
    'actions_using_crystals': 'ce4bf507f6e84b108edc8196887158ebc61e1a554604f8bde2d64b87290874d3',
    'creaturescripts_bosses_mission_depths': '6f4b1af04775b46536143729eeafc870e960e3cc9f3b048bd30155aaf7e18d49',
    'creaturescripts_fiery_heart': '0e216bfe168cefdfa1b41ce3241d0734e2eb73a88467c2b70bbb6b84da9fbcc6',
    'creaturescripts_snail_slime_kill': '2c5a474dcf269fc845d78dd117c6d9a221d5b4b64bda53562b62188a179e168e',
    'movements_boss_entrance': '25b0efc67d1544d737fb98cb4090f5d41e9ecba7b35a05d3cee92356adbaa5b9',
}
NATIVE_HOOKS = {
    'IV': ['DepthWarzoneBossDeath'],
    'V': ['DepthWarzoneBossDeath', 'SnailSlimeDeath', 'SnailSlimeThink'],
    'VI': ['DepthWarzoneBossDeath', 'FieryHeartThink'],
}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def canonical(data):
    return json.dumps(data, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def position(literal):
    return list(map(int, re.fullmatch(r'([0-9]+), ([0-9]+), ([0-9]+)', literal).groups()))


def capture(source):
    scripts = {name: (source / PREFIX / (name + '.lua')).read_bytes()
               for name in SOURCE_DIGESTS}
    if {name: sha(raw) for name, raw in scripts.items()} != SOURCE_DIGESTS:
        raise ValueError(f'Warzone source bytes do not match Crystal {REVISION}')
    opens = scripts['actions_using_crystals'].decode()
    death = scripts['creaturescripts_bosses_mission_depths'].decode()
    steps = scripts['movements_boss_entrance'].decode()
    sources = [{'path': PREFIX + name + '.lua', 'sha256': sha(bytes_)}
               for name, bytes_ in scripts.items()]
    assert 'end, 8 * 1000)' in opens
    cooldown_hours = int(re.search(r'timeToFightAgain = (\d+) -- hours', steps)[1])
    reward_minutes = int(re.search(r'addEvent\(revert, (\d+) \* 60 \* 1000', death)[1])
    profiles = {row['profile']['target']['key']: sha(canonical(row)) for row in
        json.loads((ROOT / 'content/creatures/definitions/spell-native-profiles.json').read_bytes())['records']}
    rooms = []
    for roman, name in zip(ROMANS, NAMES):
        suffix = name.lower().replace(' ', '_')
        capture_path = f'content/encounters/bosses/warzone-{roman.lower()}.json'
        creature = f'oteryn:creature.{suffix}'
        portal = position(re.search(rf'Warzone{roman} = Position\(([^)]+)\)', steps)[1])
        entry = position(re.search(rf'Warzone{roman}_b = Position\(([^)]+)\)', steps)[1])
        section = re.search(rf'local function startWarzone{roman}\(\)(.*?)\nend', opens, re.S)[1]
        spawn = position(re.search(r'Game.createMonster\("[^"]+", Position\(([^)]+)\)', section)[1])
        cleanup = re.search(rf'addEvent\(clearForgotten, (\d+) \* 60 \* 1000, Position\(([^)]+)\), Position\(([^)]+)\), Position\(([^)]+)\), .*\.Warzone{roman}\)', opens)
        timeout_minutes = int(cleanup[1])
        low, high, timeout_exit = [position(cleanup[i]) for i in (2,3,4)]
        teleports = re.search(r'\["' + re.escape(name.lower()) + r'"\] = \{ teleportPosition = Position\(([^)]+)\), toPosition = Position\(([^)]+)\), toPositionBack = Position\(([^)]+)\)', death)
        trigger, reward, exit_ = [position(teleports[i]) for i in (1,2,3)]
        line = opens[:cleanup.start()].count('\n') + 1
        candidate = dict(path=capture_path, line=line, boss_name=name, creature_key=creature,
                         boss_position=spawn, lever_positions=[portal], entry_positions=[entry],
                         zone_from=low, zone_to=high, exit_position=exit_,
                         source_exit_teleporter=trigger, blockers=[], required_callbacks=NATIVE_HOOKS[roman],
                         callbacks_known=True, native_profile_sha256=profiles[creature],
                         complete_literal_layout=True, minimum_players=1, minimum_level=1,
                         cooldown_seconds=cooldown_hours*3600, time_limit_seconds=timeout_minutes*60,
                         after_kill_seconds=reward_minutes*60)
        doc = dict(schema='OTERYN_CRYSTAL_WARZONE_ACTIVITY/v1', source_revision=REVISION,
                   sources=sources, timeout_exit=timeout_exit, reward_destination=reward,
                   candidate=candidate)
        candidate['sha256'] = sha(canonical(doc))
        rooms.append(doc)
    return rooms


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal-root', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for room in capture(args.crystal_root):
        path = ROOT / room['candidate']['path']
        raw = (json.dumps(room, indent=2) + '\n').encode()
        if args.check:
            if path.read_bytes() != raw:
                raise ValueError(f'stale Warzone activity capture: {path.relative_to(ROOT)}')
        else:
            path.write_bytes(raw)


if __name__ == '__main__':
    main()
