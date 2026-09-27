"""Transcribe Canary creature events into Encounter authoring documents (D20, D26-D28).

Evidence tooling only: every output is OTS_HYPOTHESIS_ONLY source evidence, never Game truth. Each
event is transcribed by hand from pinned Canary source lines (cited in the manifests); participants
are read from the monster files. One event can feed several encounters and one encounter collects
rules from several events. Every manifest `covers` the (event, creature) pairs whose whole effect it
transcribes; the monster converter relocates exactly those. Keys use the provisional `canary:`
namespace, like the monster bundles.

Usage: python canary_encounters.py --canary <checkout of opentibiabr/canary at REVISION>
"""
import argparse
import hashlib
import json
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPOSITORY = 'opentibiabr/canary'
REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
REV = 'canary-47dfd51f'
MONSTER_DIR = 'data-otservbr-global/monster'
SOUL_WAR_MECHANICS = 'data-otservbr-global/scripts/quests/soul_war/soul_war_mechanics.lua'
SOUL_WAR_LIB = 'data-otservbr-global/lib/quests/soul_war.lua'
DREAM_COURTS_DEATH = 'data-otservbr-global/scripts/quests/the_dream_courts_quest/creaturescripts_dreamCourtsDeath.lua'
ALPTRAMUN_SUMMON = 'data-otservbr-global/scripts/spells/monster/alptramun_summon.lua'


def slug(name):
    return re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')


def ref(family, key):
    return {'family': family, 'key': key, 'revision': REV}


def creature(name):
    return ref('Creature', f'canary:creature/{slug(name)}')


def blob_id(data):
    return hashlib.sha1(b'blob %d\0' % len(data) + data).hexdigest()


class Encounters:
    def __init__(self, canary):
        self.canary = canary
        self.items = {}
        self.monster_files = {}
        for path in sorted((canary / MONSTER_DIR).rglob('*.lua')):
            text = path.read_text(encoding='utf-8', errors='replace')
            match = re.search(r'Game\.createMonsterType\("([^"]+)"', text)
            if match:
                self.monster_files[match.group(1).lower()] = (match.group(1), str(path.relative_to(canary)), text)

    def registrants(self, event):
        return [(name, path) for name, path, text in self.monster_files.values()
                if re.search(r'monster\.events\s*=\s*\{[^}]*"' + re.escape(event) + '"', text, re.S)]

    def get(self, name, display, scope):
        if name not in self.items:
            key = f'canary:encounter/{name}'
            self.items[name] = {
                'encounter': {'identity': {'key': key, 'revision': REV}, 'display_name': display, 'scope': scope,
                              'participants': [], 'anchors': [], 'phases': [],
                              'state': {'counters': [], 'flags': [], 'timers': []}, 'rules': [], 'outcomes': []},
                'manifest': {'encounter': key, 'classification': 'OTS_HYPOTHESIS_ONLY', 'covers': {}, 'sources': [],
                             'entries': [], 'outcome_evidence': []},
                'definitions': []}
        return self.items[name]

    def source(self, item, path):
        sources = item['manifest']['sources']
        for index, existing in enumerate(sources):
            if existing['path'] == path:
                return index
        sources.append({'kind': 'git', 'repository': REPOSITORY, 'revision': REVISION, 'path': path,
                        'blob_sha1': blob_id((self.canary / path).read_bytes())})
        return len(sources) - 1

    def participant(self, item, role, name, event=None):
        """Adds `name` under `role` with the evidence line of its monster file; `event` marks it covered."""
        monster_name, path, text = self.monster_files[name.lower()]
        ref_ = creature(monster_name)
        roles = item['encounter']['participants']
        entry = next((p for p in roles if p['role'] == role), None)
        if entry is None:
            entry = {'role': role, 'creatures': []}
            roles.append(entry)
        if ref_ not in entry['creatures']:
            entry['creatures'].append(ref_)
        self.define(item, ref_)
        index = roles.index(entry)
        line = next((n for n, row in enumerate(text.splitlines(), 1) if event and event in row), 1)
        self.entry(item, path, [line], 'mapped', f'/encounter/participants/{index}/creatures',
                   f'{monster_name} is the {role} participant' + (f'; its file registers {event}.' if event else '.'))
        if event:
            item['manifest']['covers'].setdefault(event, [])
            if ref_['key'] not in item['manifest']['covers'][event]:
                item['manifest']['covers'][event].append(ref_['key'])
        return ref_

    def define(self, item, reference):
        if reference not in item['definitions']:
            item['definitions'].append(reference)

    def entry(self, item, path, lines, status, destination, resolution):
        entry = {'source_index': self.source(item, path), 'source_lines': lines, 'status': status, 'resolution': resolution}
        if destination:
            entry['destination'] = destination
        item['manifest']['entries'].append(entry)

    def rule(self, item, rule):
        item['encounter']['rules'].append(rule)
        return f'/encounter/rules/{len(item["encounter"]["rules"]) - 1}'

    def write(self):
        out = ROOT / 'samples'
        if out.exists():
            shutil.rmtree(out)
        for name, item in sorted(self.items.items()):
            target = out / name
            target.mkdir(parents=True)
            for filename, value in (('encounter.json', item['encounter']), ('catalog.json', {'definitions': item['definitions']}),
                                    ('manifest.json', item['manifest'])):
                (target / filename).write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
        return {name: sum(len(p['creatures']) for p in item['encounter']['participants']) for name, item in sorted(self.items.items())}


def soul_war_taint_zones(build):
    """FourthTaintBossesPrepareDeath: the Soul War hunting monsters' lethal-damage heal (a zone rule)."""
    event = 'FourthTaintBossesPrepareDeath'
    item = build.get('soul_war_taint_zones', 'Soul War hunting zones: fourth taint lethal heal', 'channel_shared')
    for name, _ in build.registrants(event):
        build.participant(item, 'hunting_monster', name, event)
    item['encounter']['anchors'].append({
        'key': 'soul_war_taint_zones', 'kind': 'area',
        'description': 'The eleven Canary zones of SoulWarQuest.areaZones.monsters (five hunting grounds and six Goshnar boss '
                       'rooms); to be bound by the map project.'})
    rule = build.rule(item, {
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
            {'kind': 'heal', 'subject': {'role': 'hunting_monster'}, 'amount': 'full'}]})
    for lines, destination, text in (
            ([65, 67], '/trigger', 'CreatureEvent("FourthTaintBossesPrepareDeath") onPrepareDeath: the lethal-damage trigger '
                                   '(game.cpp combatChangeHealth runs PREPAREDEATH events when realDamage >= health).'),
            ([68, 69, 70], '/conditions/0', 'Returns without effect unless the killer is a player.'),
            ([72], '/trigger', 'creature:getHealth() - realDamage < 1 repeats the lethal-damage condition of the trigger.'),
            ([73], '/conditions/1', 'killer:getTaintNameByNumber(4): the killer holds the fourth Soul War taint, a quest KV flag '
                                    '(soul_war.lua Player:getTaintNameByNumber) that the quest domain publishes read-only (D27).'),
            ([74, 75], '/conditions/2', 'killer:getSoulWarZoneMonster() ~= nil: the killing player stands in one of the zones of '
                                        'SoulWarQuest.areaZones.monsters (soul_war.lua Player:getSoulWarZoneMonster).'),
            ([76, 77], '/conditions/3', 'math.random(1, 10) == 1: a 10% chance.'),
            ([78], '/actions/0', 'creature:say(...) with the default talk type (say).'),
            ([79], '/actions/1', 'creature:addHealth(creature:getMaxHealth()) heals to full. game.cpp then still drains the lethal '
                                 'hit capped at the health before the heal, so the creature survives.')):
        build.entry(item, SOUL_WAR_MECHANICS, lines, 'mapped', rule + destination, text)
    build.entry(item, SOUL_WAR_LIB, list(range(229, 243)), 'mapped', '/encounter/anchors/0',
                'SoulWarQuest.areaZones.monsters lists the zones whose union is the anchor area.')


# creaturescripts_dreamCourtsDeath.lua questlog: boss, lines, quest storage, cap, cooldown storage.
DREAM_COURTS_BOSSES = [
    ('Faceless Bane', 'faceless_bane', 'Faceless Bane', [2, 8], 'TheDreamCourts.HauntedHouse.Questline', 4, 'FacelessTimer'),
    ('Maxxenius', 'maxxenius', 'Maxxenius', [9, 15], 'TheDreamCourts.DreamScar.BossCount', 5, 'MaxxeniusTimer'),
    ('Alptramun', 'alptramun', 'Alptramun', [16, 22], 'TheDreamCourts.DreamScar.BossCount', 5, 'AlptramunTimer'),
    ('Izcandar the Banished', 'izcandar', 'Izcandar', [23, 29], 'TheDreamCourts.DreamScar.BossCount', 5, 'IzcandarTimer'),
    ('Izcandar Champion of Winter', 'izcandar', 'Izcandar', [30, 36], 'TheDreamCourts.DreamScar.BossCount', 5, 'IzcandarTimer'),
    ('Izcandar Champion of Summer', 'izcandar', 'Izcandar', [37, 43], 'TheDreamCourts.DreamScar.BossCount', 5, 'IzcandarTimer'),
    ('Plagueroot', 'plagueroot', 'Plagueroot', [44, 50], 'TheDreamCourts.DreamScar.BossCount', 5, 'PlagueRootTimer'),
    ('Malofur Mangrinder', 'malofur_mangrinder', 'Malofur Mangrinder', [51, 57], 'TheDreamCourts.DreamScar.BossCount', 5, 'MalofurTimer'),
    ('The Nightmare Beast', 'the_nightmare_beast', 'The Nightmare Beast', [58, 64], 'TheDreamCourts.WardStones.Questline', 2,
     'NightmareTimer')]
DREAMS = ['Unpleasant Dream', 'Horrible Dream', 'Nightmarish Dream', 'Mind-Wrecking Dream']


def dream_courts(build):
    """dreamCourtsDeath: boss kill outcomes (quest progress and a 20 h cooldown for the reward and quest domains), the
    Plagueroot plant attendant, and Alptramun's dream escalation (unresolved: needs an ability-cast trigger)."""
    event = 'dreamCourtsDeath'
    for boss, name, display, lines, questline, cap, timer in DREAM_COURTS_BOSSES:
        item = build.get(name, f'Dream Courts: {display}', 'instance_per_party')
        role = slug(boss)
        covered = boss != 'Alptramun'
        build.participant(item, role, boss, event if covered else None)
        outcome = f'{role}_defeated'
        item['encounter']['outcomes'].append(outcome)
        path = build.rule(item, {'key': f'{role}_death_outcome', 'trigger': {'kind': 'creature_died', 'role': role},
                                 'conditions': [{'kind': 'has_master', 'role': role, 'value': False}],
                                 'actions': [{'kind': 'emit_outcome', 'outcome': outcome, 'credited': 'damage_contributors'}]})
        build.entry(item, DREAM_COURTS_DEATH, [92, 105], 'mapped', path + '/trigger',
                    f'onDeath of the questlog boss "{boss}".')
        build.entry(item, DREAM_COURTS_DEATH, [93, 94, 95], 'mapped', path + '/conditions/0',
                    'Returns without effect for a non-monster or a creature with a master.')
        build.entry(item, DREAM_COURTS_DEATH, [106, 107, 108, 109, 110, 111, 112, 113] + lines, 'mapped', path + '/actions/0',
                    f'Every player in creature:getDamageMap() is credited (D27): the quest domain raises Storage {questline} by 1 '
                    f'while it is <= {cap}, and the reward domain sets the {timer} cooldown to 20 hours.')
        item['manifest']['outcome_evidence'].append({
            'outcome': outcome, 'credited': 'damage_contributors',
            'quest_domain': {'canary_storage': f'Storage.Quest.U12_00.{questline}', 'increment': 1, 'only_while_at_most': cap},
            'reward_domain': {'canary_storage': f'Storage.Quest.U12_00.TheDreamCourts.DreamScarGlobal.{timer}'
                                                if timer != 'FacelessTimer' else
                                                'Storage.Quest.U12_00.TheDreamCourts.BurriedCatedralGlobal.FacelessTimer',
                              'cooldown_seconds': 20 * 60 * 60}})
    plagueroot = build.items['plagueroot']
    build.participant(plagueroot, 'plant_abomination', 'Plant Abomination', event)
    attendant = creature('Plant Attendant')
    build.define(plagueroot, attendant)
    path = build.rule(plagueroot, {
        'key': 'plant_abomination_attendant', 'trigger': {'kind': 'creature_died', 'role': 'plant_abomination'},
        'conditions': [{'kind': 'has_master', 'role': 'plant_abomination', 'value': False}],
        'actions': [{'kind': 'spawn', 'creature': attendant, 'role': 'plant_attendant', 'count': 1, 'at': 'death_position',
                     'owner': 'none', 'health': 'full'}]})
    build.entry(plagueroot, DREAM_COURTS_DEATH, [93, 94, 95], 'mapped', path + '/conditions/0',
                'Returns without effect for a creature with a master.')
    build.entry(plagueroot, DREAM_COURTS_DEATH, [99, 100, 101, 102], 'mapped', path + '/actions/0',
                'Game.createMonster("plant attendant", death position): a new unowned plant attendant with full health.')
    alptramun = build.items['alptramun']
    for dream in DREAMS:
        build.participant(alptramun, 'dream', dream)
    build.entry(alptramun, DREAM_COURTS_DEATH, [117, 118, 119, 123, 124, 125, 126, 127, 128, 129, 130, 131], 'unresolved_semantics', None,
                'Alptramun escalation: its death resets the global AlptramunSummonsKilled counter and each dream death raises it '
                'within its band; alptramun_summon.lua picks the dream type from that counter. Canary never counts: the summon '
                'spell sets Alptramun as master (line 45) and this script returns early for creatures with a master (line 93), '
                'so only unpleasant dreams appear, while the reference-date wiki lists all four dreams in the fight (D25). '
                'Modelling it needs an ability-cast trigger outside the v1 vocabulary (D28): owner decision.')
    build.entry(alptramun, ALPTRAMUN_SUMMON, [20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 45, 46], 'unresolved_semantics',
                None, 'The dream type chosen from the escalation counter; see the entry above.')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    args = parser.parse_args()
    build = Encounters(args.canary)
    for transcribe in (soul_war_taint_zones, dream_courts):
        transcribe(build)
    print(json.dumps(build.write()))


if __name__ == '__main__':
    main()
