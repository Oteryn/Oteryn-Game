"""Transcribe Canary creature events into Encounter authoring documents (D20, D26-D28).

Evidence tooling only: every output is OTS_HYPOTHESIS_ONLY source evidence, never Game truth. Each
encounter is transcribed by hand from pinned Canary source lines (cited in its manifest); the
participants are read from the monster files that register the event. Keys use the provisional
`canary:` namespace, like the monster bundles.

Usage: python canary_encounters.py --canary <checkout of opentibiabr/canary at REVISION>
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'opentibiabr/canary'
REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'
MONSTER_DIR = 'data-otservbr-global/monster'
MECHANICS = 'data-otservbr-global/scripts/quests/soul_war/soul_war_mechanics.lua'
SOUL_WAR_LIB = 'data-otservbr-global/lib/quests/soul_war.lua'


def slug(name):
    return re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')


def ref(family, key):
    return {'family': family, 'key': key, 'revision': REV}


def blob_id(data):
    return hashlib.sha1(b'blob %d\0' % len(data) + data).hexdigest()


def registrants(canary, event):
    """(monster name, relative file) of every monster file that registers `event`."""
    found = []
    for path in sorted((canary / MONSTER_DIR).rglob('*.lua')):
        text = path.read_text(encoding='utf-8', errors='replace')
        if re.search(r'monster\.events\s*=\s*\{[^}]*"' + re.escape(event) + '"', text, re.S):
            name = re.search(r'Game\.createMonsterType\("([^"]+)"', text).group(1)
            found.append((name, str(path.relative_to(canary))))
    return found


def soul_war_taint_zones(canary):
    """FourthTaintBossesPrepareDeath: the Soul War hunting monsters' lethal-damage heal (a zone rule)."""
    event = 'FourthTaintBossesPrepareDeath'
    monsters = registrants(canary, event)
    creatures = [ref('Creature', f'canary:creature/{slug(name)}') for name, _ in monsters]
    key = 'canary:encounter/soul_war_taint_zones'
    encounter = {
        'identity': {'key': key, 'revision': REV},
        'display_name': 'Soul War hunting zones: fourth taint lethal heal',
        'scope': 'channel_shared',
        'participants': [{'role': 'hunting_monster', 'creatures': creatures}],
        'anchors': [{'key': 'soul_war_taint_zones', 'kind': 'area',
                     'description': 'The eleven Canary zones of SoulWarQuest.areaZones.monsters (five hunting grounds and six '
                                    'Goshnar boss rooms); to be bound by the map project.'}],
        'phases': [],
        'state': {'counters': [], 'flags': [], 'timers': []},
        'rules': [{
            'key': 'fourth_taint_lethal_heal',
            'trigger': {'kind': 'lethal_damage', 'role': 'hunting_monster'},
            'conditions': [
                {'kind': 'killer_is_player'},
                {'kind': 'killer_progress', 'progress': 'canary:quest-progress/soul_war_taint_4', 'op': '==', 'value': True},
                {'kind': 'in_anchor', 'subject': {'killer': True}, 'anchor': 'soul_war_taint_zones'},
                {'kind': 'chance_percent', 'value': 10}],
            'actions': [
                {'kind': 'say', 'subject': {'role': 'hunting_monster'}, 'text': 'Health restored by the mystic powers of Zarganash!',
                 'mode': 'say'},
                {'kind': 'heal', 'subject': {'role': 'hunting_monster'}, 'amount': 'full'}]}],
        'outcomes': []}
    sources = [{'kind': 'git', 'repository': REPOSITORY, 'revision': REVISION, 'path': MECHANICS,
                'blob_sha1': blob_id((canary / MECHANICS).read_bytes())},
               {'kind': 'git', 'repository': REPOSITORY, 'revision': REVISION, 'path': SOUL_WAR_LIB,
                'blob_sha1': blob_id((canary / SOUL_WAR_LIB).read_bytes())}]
    for _, path in monsters:
        sources.append({'kind': 'git', 'repository': REPOSITORY, 'revision': REVISION, 'path': path,
                        'blob_sha1': blob_id((canary / path).read_bytes())})
    rule = '/encounter/rules/0'
    entries = [
        {'source_index': 0, 'source_lines': [65, 67], 'status': 'mapped', 'destination': rule + '/trigger',
         'resolution': 'CreatureEvent("FourthTaintBossesPrepareDeath") onPrepareDeath: the lethal-damage trigger '
                       '(game.cpp combatChangeHealth runs PREPAREDEATH events when realDamage >= health).'},
        {'source_index': 0, 'source_lines': [68, 69, 70], 'status': 'mapped', 'destination': rule + '/conditions/0',
         'resolution': 'Returns without effect unless the killer is a player.'},
        {'source_index': 0, 'source_lines': [72], 'status': 'mapped', 'destination': rule + '/trigger',
         'resolution': 'creature:getHealth() - realDamage < 1 repeats the lethal-damage condition of the trigger.'},
        {'source_index': 0, 'source_lines': [73], 'status': 'mapped', 'destination': rule + '/conditions/1',
         'resolution': 'killer:getTaintNameByNumber(4): the killer holds the fourth Soul War taint, a quest KV flag '
                       '(soul_war.lua Player:getTaintNameByNumber) that the quest domain publishes read-only (D27).'},
        {'source_index': 0, 'source_lines': [74, 75], 'status': 'mapped', 'destination': rule + '/conditions/2',
         'resolution': 'killer:getSoulWarZoneMonster() ~= nil: the killing player stands in one of the zones of '
                       'SoulWarQuest.areaZones.monsters (soul_war.lua Player:getSoulWarZoneMonster).'},
        {'source_index': 0, 'source_lines': [76, 77], 'status': 'mapped', 'destination': rule + '/conditions/3',
         'resolution': 'math.random(1, 10) == 1: a 10% chance.'},
        {'source_index': 0, 'source_lines': [78], 'status': 'mapped', 'destination': rule + '/actions/0',
         'resolution': 'creature:say(...) with the default talk type (say).'},
        {'source_index': 0, 'source_lines': [79], 'status': 'mapped', 'destination': rule + '/actions/1',
         'resolution': 'creature:addHealth(creature:getMaxHealth()) heals to full. game.cpp then still drains the lethal '
                       'hit capped at the health before the heal, so the creature survives.'},
        {'source_index': 1, 'source_lines': [229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242], 'status': 'mapped',
         'destination': '/encounter/anchors/0',
         'resolution': 'SoulWarQuest.areaZones.monsters lists the zones whose union is the anchor area.'}]
    for index, (name, path) in enumerate(monsters, 2):
        text = (canary / path).read_text(encoding='utf-8', errors='replace').splitlines()
        line = next(n for n, row in enumerate(text, 1) if event in row)
        entries.append({'source_index': index, 'source_lines': [line], 'status': 'mapped',
                        'destination': '/encounter/participants/0/creatures',
                        'resolution': f'{name} registers {event}, so it is a hunting_monster participant.'})
    manifest = {'encounter': key, 'canary_events': [event], 'classification': 'OTS_HYPOTHESIS_ONLY',
                'sources': sources, 'entries': entries}
    catalog = {'definitions': creatures}
    return 'soul_war_taint_zones', encounter, catalog, manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    args = parser.parse_args()
    for build in (soul_war_taint_zones,):
        name, encounter, catalog, manifest = build(args.canary)
        target = ROOT / 'samples' / name
        target.mkdir(parents=True, exist_ok=True)
        for filename, value in (('encounter.json', encounter), ('catalog.json', catalog), ('manifest.json', manifest)):
            (target / filename).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
        print(json.dumps({'encounter': name, 'participants': len(encounter['participants'][0]['creatures'])}))


if __name__ == '__main__':
    main()
