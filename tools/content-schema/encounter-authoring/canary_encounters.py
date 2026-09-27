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
DREAM_COURTS_LEVERS = 'data-otservbr-global/scripts/quests/the_dream_courts_quest/actions_dreamscarLevers.lua'


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
        build.participant(item, role, boss, event)
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
    build.participant(alptramun, 'alptramun', 'Alptramun', event)
    alptramun['encounter']['state']['counters'].append({'name': 'dreams_killed', 'initial': 0})
    build.entry(alptramun, DREAM_COURTS_LEVERS, [87], 'mapped', '/encounter/state/counters/0',
                'The Dream Scar lever starts the Alptramun fight with AlptramunSummonsKilled at 0.')
    path = build.rule(alptramun, {'key': 'alptramun_resets_dreams', 'trigger': {'kind': 'creature_died', 'role': 'alptramun'},
                                  'conditions': [{'kind': 'has_master', 'role': 'alptramun', 'value': False}],
                                  'actions': [{'kind': 'counter', 'counter': 'dreams_killed', 'operation': 'set', 'value': 0}]})
    build.entry(alptramun, DREAM_COURTS_DEATH, [117, 118, 119], 'mapped', path + '/actions/0',
                'The death of Alptramun sets AlptramunSummonsKilled back to 0.')
    for dream, band in zip(DREAMS, ((0, 9), (9, 18), (18, 27), (27, 36))):
        role = slug(dream)
        build.participant(alptramun, role, dream, event)
        path = build.rule(alptramun, {
            'key': f'{role}_killed', 'trigger': {'kind': 'creature_died', 'role': role},
            'conditions': [{'kind': 'has_master', 'role': role, 'value': False},
                           {'kind': 'counter_compare', 'counter': 'dreams_killed', 'op': '>=', 'value': band[0]},
                           {'kind': 'counter_compare', 'counter': 'dreams_killed', 'op': '<=', 'value': band[1]}],
            'actions': [{'kind': 'counter', 'counter': 'dreams_killed', 'operation': 'add', 'value': 1}]})
        build.entry(alptramun, DREAM_COURTS_DEATH, [92, 93, 94, 95], 'mapped', path + '/conditions/0',
                    'Returns without effect for a creature with a master.')
        build.entry(alptramun, DREAM_COURTS_DEATH, [67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85,
                                                    86, 87, 88, 123, 124, 125, 126, 127, 128, 129, 130, 131], 'mapped',
                    path + '/actions/0',
                    f'The death of an unmastered {dream.lower()} raises AlptramunSummonsKilled by one while it is within '
                    f'{band[0]}-{band[1]}.')
    build.entry(alptramun, ALPTRAMUN_SUMMON, list(range(1, 60)), 'unresolved_semantics', None,
                'The escalation that reads the counter: alptramun_summon.lua summons 1-4 dreams (up to 5 summons, as Alptramun\'s '
                'summons) whose type rises with AlptramunSummonsKilled in bands of nine. No Canary monster casts this spell '
                '(it is not in Alptramun\'s attacks or defenses), and summoned dreams have a master, so this script would not '
                'count them. The reference-date wiki says Alptramun "will initially appear with several summons. These '
                'summons, when killed, will be replaced by even stronger summons" without numbers; defining that ability '
                '(an ability_cast rule, D29) needs its exact rule from the owner or further evidence.')

FORGOTTEN_KNOWLEDGE_KILL = 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua'
# boss, encounter, display, config line, storage, cooldown seconds
FORGOTTEN_KNOWLEDGE_BOSSES = [
    ('Lady Tenebris', 'lady_tenebris', 'Lady Tenebris', 3, 'LadyTenebrisKilled', 20 * 3600),
    ('The Enraged Thorn Knight', 'the_enraged_thorn_knight', 'The Enraged Thorn Knight', 4, 'ThornKnightKilled', 20 * 3600),
    ('Lloyd', 'lloyd', 'Lloyd', 5, 'LloydKilled', 20 * 3600),
    ('Soul of Dragonking Zyrtarch', 'soul_of_dragonking_zyrtarch', 'Soul of Dragonking Zyrtarch', 6, 'DragonkingKilled', 20 * 3600),
    ('Melting Frozen Horror', 'melting_frozen_horror', 'Melting Frozen Horror', 7, 'HorrorKilled', 20 * 3600),
    ('The Time Guardian', 'the_time_guardian', 'The Time Guardian', 8, 'TimeGuardianKilled', 20 * 3600),
    ('The Blazing Time Guardian', 'the_time_guardian', 'The Time Guardian', 9, 'TimeGuardianKilled', 20 * 3600),
    ('The Freezing Time Guardian', 'the_time_guardian', 'The Time Guardian', 10, 'TimeGuardianKilled', 20 * 3600),
    ('The Last Lore Keeper', 'the_last_lore_keeper', 'The Last Lore Keeper', 11, 'LastLoreKilled', 13 * 24 * 3600 + 20 * 3600)]


def forgotten_knowledge(build):
    """ForgottenKnowledgeBossDeath: boss kill outcomes (a cooldown for the reward domain); the Melting Frozen Horror egg
    swap on fixed tiles stays unresolved; an astral glyph death has no effect."""
    event = 'ForgottenKnowledgeBossDeath'
    for boss, name, display, line, storage, cooldown in FORGOTTEN_KNOWLEDGE_BOSSES:
        item = build.get(name, f'Forgotten Knowledge: {display}', 'instance_per_party')
        role = slug(boss)
        build.participant(item, role, boss, event if boss != 'Melting Frozen Horror' else None)
        outcome = f'{role}_defeated'
        item['encounter']['outcomes'].append(outcome)
        path = build.rule(item, {'key': f'{role}_death_outcome', 'trigger': {'kind': 'creature_died', 'role': role},
                                 'conditions': [], 'actions': [{'kind': 'emit_outcome', 'outcome': outcome,
                                                                'credited': 'damage_contributors'}]})
        build.entry(item, FORGOTTEN_KNOWLEDGE_KILL, [16, 18, 19, 20, 21, 22, line], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss}".')
        build.entry(item, FORGOTTEN_KNOWLEDGE_KILL, [24, 25, 26, 27, 28, 29, 30, 35], 'mapped', path + '/actions/0',
                    f'onDeathForDamagingPlayers credits every damaging player (D27): the reward domain sets Storage '
                    f'ForgottenKnowledge.{storage} to now + {cooldown} seconds.')
        if boss == 'The Enraged Thorn Knight':
            build.entry(item, FORGOTTEN_KNOWLEDGE_KILL, [31, 32, 33], 'approved_omission', None,
                        'The PlantCounter/BirdCounter reset sits in an elseif after `if bossConfig.storage`, and the thorn knight '
                        'has a storage, so Canary never runs it (dead branch).')
        item['manifest']['outcome_evidence'].append({
            'outcome': outcome, 'credited': 'damage_contributors',
            'reward_domain': {'canary_storage': f'Storage.Quest.U11_02.ForgottenKnowledge.{storage}', 'cooldown_seconds': cooldown}})
    horror = build.items['melting_frozen_horror']
    build.entry(horror, FORGOTTEN_KNOWLEDGE_KILL, [37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48], 'unresolved_semantics', None,
                'On its death the top creature on tile (32269, 31084, 14) is removed and a baby dragon is created in its place, '
                'and the top creature on (32267, 31071, 14) is removed. Which creatures stand on those tiles is map state, '
                'not visible in the script; modelling it needs those roles confirmed (D25: wiki or map evidence).')
    keeper = build.items['the_last_lore_keeper']
    build.participant(keeper, 'astral_glyph', 'An Astral Glyph', event)
    build.entry(keeper, FORGOTTEN_KNOWLEDGE_KILL, [12, 13], 'approved_omission', None,
                'An astral glyph has an empty config: no storage, not the thorn knight, not the horror, so its death has no '
                'effect in this event.')


ASCENDANT_KILL = 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_bosses_kill.lua'
# boss, config first line, storage, cooldown hours, teleport position, godbreaker position
ASCENDANT_BOSSES = [
    ('The Lord of the Lice', 2, 'TheLordOfTheLiceTimer', 44, (33226, 31478, 12), (33237, 31477, 13)),
    ('Tarbaz', 8, 'TarbazTimer', 44, (33460, 32853, 11), (33427, 32852, 13)),
    ('Ragiaz', 14, 'RagiazTimer', 44, (33482, 32345, 13), (33466, 32392, 13)),
    ('Plagirath', 20, 'PlagirathTimer', 44, (33174, 31511, 13), (33204, 31510, 13)),
    ('Razzagorn', 26, 'RazzagornTimer', 44, (33413, 32467, 14), (33357, 32440, 13)),
    ('Zamulosh', 32, 'ZamuloshTimer', 44, (33644, 32764, 11), (33678, 32758, 13)),
    ('Mazoran', 38, 'MazoranTimer', 44, (33585, 32699, 14), (33614, 32679, 15)),
    ('Shulgrax', 44, 'ShulgraxTimer', 44, (33486, 32796, 13), (33459, 32820, 14)),
    ('Ferumbras Mortal Shell', 50, 'FerumbrasMortalShellTimer', 332, (33392, 31485, 14), (33388, 31414, 14))]


def ascendant(build):
    """AscendantBossesDeath: kill outcomes with cooldowns, and the boss-room teleporter that opens to the Godbreaker for one
    minute. The Ferumbras Mortal Shell crystal reset (quest-wide storages and fixed crystal items) stays unresolved."""
    event = 'AscendantBossesDeath'
    for boss, line, storage, hours, teleport, godbreaker in ASCENDANT_BOSSES:
        name = slug(boss)
        item = build.get(name, f'Ferumbras Ascension: {boss}', 'instance_per_party')
        role = name
        build.participant(item, role, boss, event if boss != 'Ferumbras Mortal Shell' else None)
        outcome = f'{role}_defeated'
        item['encounter']['outcomes'].append(outcome)
        anchors = item['encounter']['anchors']
        anchors += [{'key': 'exit_teleporter', 'kind': 'point', 'description': f'Boss-room teleporter; Canary {teleport}.'},
                    {'key': 'godbreaker', 'kind': 'point', 'description': f'Godbreaker destination; Canary {godbreaker}.'},
                    {'key': 'ascendant_exit', 'kind': 'point', 'description': 'Default teleporter destination; Canary (33319, 32318, 13).'}]
        closed, opened = ref('Item', 'canary:item/1949'), ref('Item', 'canary:item/22761')
        build.define(item, closed)
        build.define(item, opened)
        path = build.rule(item, {
            'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
            'actions': [{'kind': 'emit_outcome', 'outcome': outcome, 'credited': 'damage_contributors'},
                        {'kind': 'map_item', 'operation': 'transform', 'item': closed, 'into': opened, 'anchor': 'exit_teleporter',
                         'destination': 'godbreaker', 'effect': 'canary.appearance:effect/thunder', 'revert_after_ms': 60000,
                         'revert_destination': 'ascendant_exit'}]})
        build.entry(item, ASCENDANT_KILL, [97, 109, 110, 111, 112, 113, line], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss}".')
        build.entry(item, ASCENDANT_KILL, [115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125], 'mapped', path + '/actions/0',
                    f'onDeathForDamagingPlayers credits every damaging player (D27): the reward domain sets '
                    f'FerumbrasAscension.{storage} to now + {hours} h and tells the player when the boss can be fought again.')
        build.entry(item, ASCENDANT_KILL, [128, 129, 130, 131, 132, 133, 134, 135, 89, 90, 91, 92, 93, 94, 95], 'mapped',
                    path + '/actions/1',
                    'The teleporter item 1949 on the teleport position becomes 22761 with a thunder effect and leads to the '
                    'Godbreaker; after 60 s revertTeleport turns it back into 1949 leading to (33319, 32318, 13).')
        build.entry(item, ASCENDANT_KILL, [line + 1, line + 2], 'mapped', '/encounter/anchors',
                    'teleportPos and godbreakerPos become the exit_teleporter and godbreaker anchors (bound by the map project).')
        item['manifest']['outcome_evidence'].append({
            'outcome': outcome, 'credited': 'damage_contributors',
            'reward_domain': {'canary_storage': f'Storage.Quest.U10_90.FerumbrasAscension.{storage}', 'cooldown_seconds': hours * 3600,
                              'message': 'You have defeated <boss>. You can challenge this boss again in <cooldown>.'}})
    shell = build.items['ferumbras_mortal_shell']
    build.entry(shell, ASCENDANT_KILL, list(range(58, 88)) + list(range(137, 148)), 'unresolved_semantics', None,
                'Two minutes after the kill, for each damaging player, the eight Ferumbras crystals are reset: quest-wide global '
                'storages and each player\'s storage go to 0 and crystal items 14961 on fixed tiles turn into 14955. This is '
                'quest-wide state shared by all parties (D27 quest domain) acting on fixed map items; it needs a quest-domain '
                'outcome contract first.')


CULTS = 'data-otservbr-global/scripts/quests/cults_of_tibia/'
CULTS_BOSSES = CULTS + 'creaturescripts_bosses_mission_cults.lua'
CULTS_PILLAR = CULTS + 'creaturescripts_destroyed_pillar.lua'
CULTS_ESSENCE = CULTS + 'creaturescripts_essence_of_malice.lua'
CULTS_LEVERS = CULTS + 'actions_bosses_levers.lua'
CULTS_CHECK_TILE = CULTS + 'creaturescripts_check_tile.lua'
# boss, encounter, config line, quest storage, value
CULTS_MISSION_BOSSES = [
    ('Ravenous Hunger', 'ravenous_hunger', 2, 'Barkless.Mission', 6),
    ('The Souldespoiler', 'the_souldespoiler', 3, 'Misguided.Mission', 4),
    ('Essence of Malice', 'essence_of_malice', 4, 'Humans.Mission', 2),
    ('The Unarmored Voidborn', 'the_unarmored_voidborn', 5, 'Orcs.Mission', 2),
    ('The False God', 'the_false_god', 6, 'Minotaurs.Mission', 4),
    ('The Source of Corruption', 'the_corruptor_of_souls', 9, 'FinalBoss.Mission', 2)]
# pillar, lever spawn line, guardian, stop creature, lever line of the stop creature, pillar script lines
ESSENCE_PILLARS = [
    ('Pillar of Summoning', 285, 'Eshtaba the Conjurer', 'Eshtaba The Conjurer Stop', 291, (23, 30)),
    ('Pillar of Death', 286, 'Malkhar Deathbringer', 'Malkhar Deathbringer Stop', 294, (31, 38)),
    ('Pillar of Protection', 287, 'Eliz the Unyielding', 'Eliz The Unyielding Stop', 292, (47, 54)),
    ('Pillar of Healing', 288, 'Mezlon the Defiler', 'Mezlon The Defiler Stop', 293, (39, 46)),
    ('Pillar of Draining', 289, 'Dorokoll the Mystic', 'Dorokoll The Mystic Stop', 290, (55, 62))]


def cults_of_tibia(build):
    """CultsOfTibiaBossDeath, DestroyedPillar and EssenceOfMaliceSpawnsDeath: mission outcomes of the Cults of Tibia
    bosses, the Essence of Malice pillar room and the Corruptor of Souls hand-over. The Sandking stays unresolved."""
    event = 'CultsOfTibiaBossDeath'
    for boss, name, line, storage, value in CULTS_MISSION_BOSSES:
        item = build.get(name, f'Cults of Tibia: {boss if name != "the_corruptor_of_souls" else "The Corruptor of Souls"}',
                         'instance_per_party')
        role = slug(boss)
        build.participant(item, role, boss, event)
        outcome = f'{role}_defeated'
        item['encounter']['outcomes'].append(outcome)
        path = build.rule(item, {'key': f'{role}_death_outcome', 'trigger': {'kind': 'creature_died', 'role': role},
                                 'conditions': [{'kind': 'has_master', 'role': role, 'value': False}],
                                 'actions': [{'kind': 'emit_outcome', 'outcome': outcome, 'credited': 'damage_contributors'}]})
        build.entry(item, CULTS_BOSSES, [12, 13, 19, 20, 21, 22, 23, line], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss.lower()}".')
        build.entry(item, CULTS_BOSSES, [14, 15, 16, 17], 'mapped', path + '/conditions/0',
                    'Returns without effect for a creature with a master (a player never triggers a monster death event).')
        build.entry(item, CULTS_BOSSES, [46, 47, 48, 49, 50, line], 'mapped', path + '/actions/0',
                    f'onDeathForDamagingPlayers credits every damaging player (D27): the quest domain raises Storage '
                    f'CultsOfTibia.{storage} to {value} when it is lower.')
        item['manifest']['outcome_evidence'].append({
            'outcome': outcome, 'credited': 'damage_contributors',
            'quest_domain': {'canary_storage': f'Storage.Quest.U11_40.CultsOfTibia.{storage}', 'raise_to_at_least': value}})

    # The Corruptor of Souls hands over to The Source of Corruption.
    item = build.items['the_corruptor_of_souls']
    build.participant(item, 'the_corruptor_of_souls', 'The Corruptor of Souls', event)
    build.participant(item, 'zarcorix_of_yalahar', 'Zarcorix Of Yalahar')
    source = creature('The Source of Corruption')
    item['encounter']['anchors'].append({'key': 'source_of_corruption_spawn', 'kind': 'point',
                                         'description': 'The Source of Corruption appears here; Canary (33039, 31922, 15).'})
    path = build.rule(item, {
        'key': 'the_corruptor_of_souls_death', 'trigger': {'kind': 'creature_died', 'role': 'the_corruptor_of_souls'},
        'conditions': [{'kind': 'has_master', 'role': 'the_corruptor_of_souls', 'value': False}],
        'actions': [{'kind': 'spawn', 'creature': source, 'role': 'the_source_of_corruption', 'count': 1,
                     'at': {'anchor': 'source_of_corruption_spawn'}, 'owner': 'none', 'health': 'full'},
                    {'kind': 'remove', 'role': 'zarcorix_of_yalahar'}]})
    build.entry(item, CULTS_BOSSES, [12, 13, 19, 20, 21, 22, 23, 8, 27], 'mapped', path + '/trigger',
                'onDeath of "the corruptor of souls", configured with createNew.')
    build.entry(item, CULTS_BOSSES, [14, 15, 16, 17], 'mapped', path + '/conditions/0',
                'Returns without effect for a creature with a master.')
    build.entry(item, CULTS_BOSSES, [8, 29], 'mapped', path + '/actions/0',
                'Game.createMonster("The Source Of Corruption", Position(33039, 31922, 15)): a new unowned boss with full health.')
    build.entry(item, CULTS_BOSSES, [8, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43], 'mapped', path + '/actions/1',
                'Canary defect: `if removeMonster then` reads an undefined global instead of boss.removeMonster, so the '
                'zarcorix of yalahar is never removed. The reference-date wiki (Cults of Tibia Quest/Spoiler: "Once you kill '
                'The Corruptor of Souls, the Zarcorix of Yalahar will disappear and The Source of Corruption will spawn") '
                'decides (D25): the zarcorix is removed.')
    build.entry(item, CULTS_BOSSES, [28], 'approved_omission', None,
                'Game.setStorageValue("CheckTile", -1) resets the vulnerability deadline that the CheckTile onThink '
                f'({CULTS_CHECK_TILE} lines 1-11) reads for the now dead corruptor; encounter state ends with the instance.')
    build.entry(item, CULTS_BOSSES, [44], 'mapped', path,
                'The corruptor itself credits no mission; the credit comes with The Source of Corruption.')

    sandking = build.get('the_sandking', 'Cults of Tibia: The Sandking', 'instance_per_party')
    build.participant(sandking, 'the_sandking', 'The Sandking')
    sandking['encounter']['state']['counters'].append({'name': 'stage', 'initial': 0})
    sandking['encounter']['outcomes'].append('the_sandking_defeated')
    path = build.rule(sandking, {
        'key': 'the_sandking_death_outcome', 'trigger': {'kind': 'creature_died', 'role': 'the_sandking'},
        'conditions': [{'kind': 'has_master', 'role': 'the_sandking', 'value': False},
                       {'kind': 'counter_compare', 'counter': 'stage', 'op': '>=', 'value': 5}],
        'actions': [{'kind': 'emit_outcome', 'outcome': 'the_sandking_defeated', 'credited': 'damage_contributors'}]})
    build.entry(sandking, CULTS_BOSSES, [12, 13, 19, 20, 21, 22, 23, 7], 'mapped', path + '/trigger',
                'onDeath of the configured boss "the sandking".')
    build.entry(sandking, CULTS_BOSSES, [14, 15, 16, 17], 'mapped', path + '/conditions/0',
                'Returns without effect for a creature with a master.')
    build.entry(sandking, CULTS_BOSSES, [46, 47, 48, 49, 50, 7], 'mapped', path + '/actions/0',
                'onDeathForDamagingPlayers credits every damaging player (D27): the quest domain raises Storage '
                'CultsOfTibia.Life.Mission to 8 when it is lower.')
    build.entry(sandking, CULTS_BOSSES, [7, 24, 25, 26], 'unresolved_semantics', path + '/conditions/1',
                'The credit needs the global "sandking" storage at least 5: the fight stage, set to 1 by '
                'actions_bosses_levers.lua line 481 and advanced by creaturescripts_sandking.lua. It is the stage counter of '
                'this encounter, but the rules that advance it are not transcribed yet.')
    sandking['manifest']['outcome_evidence'].append({
        'outcome': 'the_sandking_defeated', 'credited': 'damage_contributors',
        'quest_domain': {'canary_storage': 'Storage.Quest.U11_40.CultsOfTibia.Life.Mission', 'raise_to_at_least': 8}})

    # Essence of Malice: five pillars guard five mini-bosses; the Essence appears after all five are killed.
    essence = build.items['essence_of_malice']
    malice = essence['encounter']
    malice['anchors'].append({'key': 'essence_spawn', 'kind': 'point', 'description': 'Canary (33098, 31920, 15).'})
    malice['state']['counters'].append({'name': 'guardians_killed', 'initial': 0})
    destroyed = creature('Destroyed Pillar')
    build.define(essence, destroyed)
    for pillar, pillar_line, guardian, stop, stop_line, (first, last) in ESSENCE_PILLARS:
        role, guardian_role, stop_role = slug(pillar), slug(guardian), slug(stop)
        build.participant(essence, role, pillar, 'DestroyedPillar')
        build.participant(essence, stop_role, stop)
        build.participant(essence, guardian_role, guardian, 'EssenceOfMaliceSpawnsDeath')
        anchor = f'{guardian_role}_spot'
        malice['anchors'].append({'key': anchor, 'kind': 'point',
                                  'description': f'The tile of {stop} next to {pillar}; see {CULTS_LEVERS} line {stop_line}.'})
        path = build.rule(essence, {
            'key': f'{role}_destroyed', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
            'actions': [{'kind': 'remove', 'role': stop_role},
                        {'kind': 'spawn', 'creature': destroyed, 'role': 'destroyed_pillar', 'count': 1, 'at': 'death_position',
                         'owner': 'none', 'health': 'full'},
                        {'kind': 'spawn', 'creature': creature(guardian), 'role': guardian_role, 'count': 1,
                         'at': {'anchor': anchor}, 'owner': 'none', 'health': 'full'}]})
        build.entry(essence, CULTS_PILLAR, [1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 17, 18, 19, 20, 21, 22, first], 'mapped', path + '/trigger',
                    f'onDeath of "{pillar.lower()}".')
        build.entry(essence, CULTS_PILLAR, list(range(first + 1, first + 5)), 'mapped', path + '/actions/0',
                    f'The top creature on the adjacent tile is removed: {stop}, placed there with the pillar '
                    f'({CULTS_LEVERS} lines {pillar_line}, {stop_line}).')
        build.entry(essence, CULTS_PILLAR, [last - 1], 'mapped', path + '/actions/1',
                    'Game.createMonster("Destroyed Pillar", position, true, true) at the death position.')
        build.entry(essence, CULTS_PILLAR, [last], 'mapped', path + '/actions/2',
                    f'Game.createMonster("{guardian}", adjacent tile, true, true): the attackable mini-boss replaces {stop}.')
        path = build.rule(essence, {
            'key': f'{guardian_role}_killed', 'trigger': {'kind': 'creature_died', 'role': guardian_role}, 'conditions': [],
            'actions': [{'kind': 'counter', 'counter': 'guardians_killed', 'operation': 'add', 'value': 1}]})
        build.entry(essence, CULTS_ESSENCE, [1, 3, 4, 23], 'mapped', path + '/actions/0',
                    f'onDeath of "{guardian.lower()}" counts towards the Essence of Malice (see the counter rule).')
    path = build.rule(essence, {
        'key': 'essence_of_malice_appears', 'trigger': {'kind': 'counter_reached', 'counter': 'guardians_killed', 'value': 5},
        'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('Essence of Malice'), 'role': 'essence_of_malice',
                                       'count': 1, 'at': {'anchor': 'essence_spawn'}, 'owner': 'none', 'health': 'full'}]})
    build.entry(essence, CULTS_ESSENCE, [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24], 'mapped', path,
                'Canary spawns the Essence at (33098, 31920, 15) when the dying mini-boss is the only one standing in the area '
                '(33087, 31909)-(33112, 31932) (onDeath runs before the corpse creature leaves its tile, creature.cpp '
                'dropCorpse). Mini-bosses killed one after another would each be alone, so the Essence would appear after '
                'the first kill. The reference-date wiki (Cults of Tibia Quest/Spoiler: "After you are done with the 5 mini '
                'bosses, Essence of Malice will spawn") decides (D25): the fifth kill spawns it.')


WOTE = 'data-otservbr-global/scripts/quests/wrath_of_the_emperor/'
WOTE_BOSSES = WOTE + 'creaturescripts_bosses_kill.lua'
WOTE_ZALAMON = WOTE + 'creaturescripts_zalamon_kill.lua'
# boss, config lines, statue position
WOTE_BOSS_CONFIG = [
    ('Fury of the Emperor', [2, 3, 4, 5], (33048, 31085, 15)),
    ('Wrath of the Emperor', [6, 7, 8, 9], (33094, 31087, 15)),
    ('Scorn of the Emperor', [10, 11, 12, 13], (33095, 31110, 15)),
    ('Spite of the Emperor', [14, 15, 16, 17], (33048, 31111, 15))]
# form, config lines, next form
ZALAMON_FORMS = [
    ('Snake God Essence', [2, 3, 4, 5], 'Snake Thing', "IT'S NOT THAT EASY MORTALS! FEEL THE POWER OF THE GOD!"),
    ('Snake Thing', [6, 7, 8, 9], 'Lizard Abomination', 'NOOO! NOW YOU HERETICS WILL FACE MY GODLY WRATH!'),
    ('Lizard Abomination', [10, 11, 12, 13], 'Mutated Zalamon', 'YOU ... WILL ... PAY WITH ETERNITY ... OF AGONY!')]


def wrath_of_the_emperor(build):
    """WrathOfTheEmperorBossDeath and ZalamonDeath: the four emperor bosses unseal their statue; Zalamon's forms follow
    one another."""
    event = 'WrathOfTheEmperorBossDeath'
    sealed, unsealed = ref('Item', 'canary:item/10797'), ref('Item', 'canary:item/11427')
    for boss, lines, statue in WOTE_BOSS_CONFIG:
        role = slug(boss)
        item = build.get(role, f'Wrath of the Emperor: {boss}', 'instance_per_party')
        build.participant(item, role, boss, event)
        item['encounter']['anchors'].append({'key': 'statue', 'kind': 'point', 'description': f'Boss statue; Canary {statue}.'})
        build.define(item, sealed)
        build.define(item, unsealed)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'map_item', 'operation': 'transform', 'item': sealed, 'into': unsealed,
                                              'anchor': 'statue'}]})
        build.entry(item, WOTE_BOSSES, [20, 21, 22, 23, 24, 25] + lines[:1], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss.lower()}".')
        build.entry(item, WOTE_BOSSES, [28, 29, 30, 31, 32, 33, 34] + lines[1:2], 'mapped', path + '/actions/0',
                    f'Item 10797 on {statue} becomes 11427 (the sceptre of mission 10 turns it back, '
                    'actions_mission10_a_message_of_freedom_sceptre.lua lines 10-11: quest interaction, not this event).')
        build.entry(item, WOTE_BOSSES, [27] + lines[2:3], 'approved_omission', None,
                    'Game.setStorageValue(Bosses.<boss>, 0): nothing sets this global to 1, and movements_boss_teleport.lua '
                    'lines 55-60 teleport in both branches, differing only by a teleport effect, so the write has no effect.')

    event = 'ZalamonDeath'
    item = build.get('zalamon', 'Wrath of the Emperor: Zalamon', 'instance_per_party')
    for form, lines, next_form, text in ZALAMON_FORMS:
        role, next_role = slug(form), slug(next_form)
        build.participant(item, role, form, event)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'spawn', 'creature': creature(next_form), 'role': next_role, 'count': 1,
                                              'at': 'death_position', 'owner': 'none', 'health': 'full'},
                                             {'kind': 'say', 'subject': {'role': next_role}, 'text': text, 'mode': 'say'}]})
        build.entry(item, WOTE_ZALAMON, [17, 18, 23, 24, 25, 26, 27] + lines[:1] + lines[2:3], 'mapped', path + '/trigger',
                    f'onDeath of the form "{form.lower()}", whose next form is "{next_form.lower()}".')
        build.entry(item, WOTE_ZALAMON, [38, 42] + lines[2:3], 'mapped', path + '/actions/0',
                    f'Game.createMonster("{next_form.lower()}", death position, false, true): the next form with full health.')
        build.entry(item, WOTE_ZALAMON, [39, 40] + lines[1:2], 'mapped', path + '/actions/1',
                    'monster:say(text, TALKTYPE_MONSTER_SAY) by the new form.')
        build.entry(item, WOTE_ZALAMON, [29, 30, 31, 32, 33, 34, 35, 37], 'approved_omission', None,
                    'The next form is not created when a creature of that name is already in view. In one encounter instance '
                    'the next form exists only through this rule, which runs once for the single previous form, so the guard '
                    'never fails.')
    build.participant(item, 'mutated_zalamon', 'Mutated Zalamon', event)
    build.entry(item, WOTE_ZALAMON, [17, 18, 19, 20, 21], 'approved_omission', None,
                'The death of "mutated zalamon" sets the global Mission11 storage to -1, releasing the arena lock that '
                'actions_mission11_payback_time_lever.lua lines 18-24 hold for ten minutes; an instance per party (D26) '
                'replaces the shared-arena lock.')


GHULOSH = 'data-otservbr-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_ghulosh.lua'


def ghulosh(build):
    """ghuloshDeath: the Book of Death and Concentrated Death alternate in the Ghulosh fight."""
    event = 'ghuloshDeath'
    item = build.get('ghulosh', 'The Secret Library: Ghulosh', 'instance_per_party')
    build.participant(item, 'the_book_of_death', 'The Book of Death', event)
    build.participant(item, 'concentrated_death', 'Concentrated Death', event)
    item['encounter']['anchors'].append({'key': 'book_of_death_spawn', 'kind': 'point', 'description': 'Canary (32755, 32716, 10).'})
    item['encounter']['state']['timers'].append({'name': 'book_of_death_return', 'duration_ms': 12000, 'repeat': False})
    path = build.rule(item, {'key': 'the_book_of_death_death', 'trigger': {'kind': 'creature_died', 'role': 'the_book_of_death'},
                             'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('Concentrated Death'),
                                                            'role': 'concentrated_death', 'count': 1, 'at': 'death_position',
                                                            'owner': 'none', 'health': 'full'}]})
    build.entry(item, GHULOSH, [55, 56, 57, 58, 60], 'mapped', path + '/trigger', 'onDeath of "the book of death".')
    build.entry(item, GHULOSH, [61], 'mapped', path + '/actions/0',
                'Game.createMonster("Concentrated Death", death position): full health, no owner.')
    path = build.rule(item, {'key': 'concentrated_death_death', 'trigger': {'kind': 'creature_died', 'role': 'concentrated_death'},
                             'conditions': [], 'actions': [{'kind': 'timer', 'timer': 'book_of_death_return', 'operation': 'start'}]})
    build.entry(item, GHULOSH, [62, 63], 'mapped', path + '/actions/0',
                'onDeath of "concentrated death": addEvent(doSpawn, 4000, ..., k = 1). doSpawn reschedules itself every 2000 ms '
                'while k <= 4 (k = 1..4) and creates the book at k = 5: 4000 + 4 * 2000 = 12000 ms.')
    path = build.rule(item, {'key': 'book_of_death_returns', 'trigger': {'kind': 'timer_elapsed', 'timer': 'book_of_death_return'},
                             'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('The Book of Death'),
                                                            'role': 'the_book_of_death', 'count': 1,
                                                            'at': {'anchor': 'book_of_death_spawn'}, 'owner': 'none',
                                                            'health': 'full'}]})
    build.entry(item, GHULOSH, [45, 50, 51, 52, 53], 'mapped', path + '/actions/0',
                'Game.createMonster("The Book of Death", Position(32755, 32716, 10)): full health, no owner.')
    build.entry(item, GHULOSH, [46, 47, 48, 49], 'approved_omission', None,
                'The teleport magic effect on the spawn tile every 2 s before the book returns is cosmetic.')


DEPTHS = 'data-otservbr-global/scripts/quests/dangerous_depth/creaturescripts_bosses_mission_depths.lua'
# boss, config line, teleport position, destination, destination after the revert
DEPTH_BOSSES = [
    ('The Count of the Core', 2, (33681, 32340, 15), (33682, 32315, 15), (33324, 32111, 15)),
    ('The Duke of the Depths', 3, (33719, 32302, 15), (33691, 32301, 15), (33275, 32318, 15)),
    ('The Baron from Below', 4, (33650, 32312, 15), (33668, 32301, 15), (33462, 32267, 15))]


def dangerous_depth(build):
    """DepthWarzoneBossDeath: a warzone boss opens its room teleporter for 20 minutes."""
    event = 'DepthWarzoneBossDeath'
    closed, opened = ref('Item', 'canary:item/1949'), ref('Item', 'canary:item/22761')
    for boss, line, teleport, destination, back in DEPTH_BOSSES:
        role = slug(boss)
        item = build.get(role, f'Dangerous Depth: {boss}', 'instance_per_party')
        build.participant(item, role, boss, event)
        item['encounter']['anchors'] += [
            {'key': 'exit_teleporter', 'kind': 'point', 'description': f'Boss-room teleporter; Canary {teleport}.'},
            {'key': 'reward_destination', 'kind': 'point', 'description': f'Destination while open; Canary {destination}.'},
            {'key': 'warzone_exit', 'kind': 'point', 'description': f'Destination after the revert; Canary {back}.'}]
        build.define(item, closed)
        build.define(item, opened)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'map_item', 'operation': 'transform', 'item': closed, 'into': opened,
                                              'anchor': 'exit_teleporter', 'destination': 'reward_destination',
                                              'revert_after_ms': 20 * 60 * 1000, 'revert_destination': 'warzone_exit'}]})
        build.entry(item, DEPTHS, [15, 16, 17, 18, 19, 20, line], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss.lower()}".')
        build.entry(item, DEPTHS, [22, 23, 24, 25, 26, 7, 8, 9, 10, 11, 12, 13, line], 'mapped', path + '/actions/0',
                    f'Teleporter 1949 on {teleport} becomes 22761 leading to {destination}; after 20 minutes revert turns it '
                    f'back into 1949 leading to {back}.')
        build.entry(item, DEPTHS, [line], 'mapped', '/encounter/anchors',
                    'teleportPosition, toPosition and toPositionBack become the three point anchors.')


RATHLETON = 'data-otservbr-global/scripts/quests/hero_of_rathleton/'
RATHLETON_KILL = RATHLETON + 'creaturescripts_bosses_kill.lua'
GLOOTH_HORROR = RATHLETON + 'creaturescripts_glooth_horror.lua'
GLOOTH_STAGES = [('Feeble Glooth Horror', 2, 'Weakened Glooth Horror'), ('Weakened Glooth Horror', 3, 'Glooth Horror'),
                 ('Glooth Horror', 4, 'Strong Glooth Horror'), ('Strong Glooth Horror', 5, 'Empowered Glooth Horror')]
# boss, encounter, config lines, teleporter, destination while open, running flag script
RATHLETON_BOSSES = [
    ('Deep Terror', 'deep_terror', [2, 3, 4, 5, 6], (33749, 31952, 14), (33740, 31940, 15), 'movements_deep_terror.lua'),
    ('Empowered Glooth Horror', 'glooth_horror', [7, 8, 9, 10, 11], (33545, 31955, 15), (33534, 31955, 15),
     'movements_glooth_horror.lua'),
    ('Professor Maxxen', 'professor_maxxen', [12, 13, 14, 15, 16], (33718, 32047, 15), (33707, 32107, 15),
     'actions_machines_professor_maxxen.lua')]


def hero_of_rathleton(build):
    """GloothHorror and RathletonBossDeath: the glooth horror splits in two at each stage (the Canary script is broken,
    the wiki decides), and each boss opens its room teleporter for two minutes."""
    item = build.get('glooth_horror', 'Hero of Rathleton: Glooth Horror', 'instance_per_party')
    for stage, line, next_stage in GLOOTH_STAGES:
        role = slug(stage)
        build.participant(item, role, stage, 'GloothHorror')
        path = build.rule(item, {'key': f'{role}_splits', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'spawn', 'creature': creature(next_stage), 'role': slug(next_stage),
                                              'count': 2, 'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
        build.entry(item, GLOOTH_HORROR, [8, 9, 10, 11, 12, 19, line], 'mapped', path + '/trigger', f'onDeath of "{stage}".')
        build.entry(item, GLOOTH_HORROR, [13, 14, 15, 16, 17, 18, line], 'mapped', path + '/actions/0',
                    f'Canary defect: Game.createMonster("{next_stage}", targetMonster:getPosition(), true, true) twice reads '
                    'the undefined global targetMonster, so the script errors and nothing spawns. The reference-date wiki '
                    f'("When slain, the {stage} will turn into 2 {next_stage}s"; 16 Empowered Glooth Horrors in total) decides '
                    '(D25): two of the next stage appear where it died. The teleport effect on each is cosmetic.')
    build.participant(item, 'empowered_glooth_horror', 'Empowered Glooth Horror', 'GloothHorror')
    build.entry(item, GLOOTH_HORROR, [1, 2, 3, 4, 5, 6, 11, 12], 'approved_omission', None,
                'The Empowered Glooth Horror is the last stage and has no config row: its death has no effect in this event.')
    item['encounter']['anchors'].append({'key': 'horror_arena', 'kind': 'area',
                                         'description': 'Tiles within 13 of Canary (33555, 31956, 15) on that floor.'})

    event = 'RathletonBossDeath'
    closed, opened = ref('Item', 'canary:item/1949'), ref('Item', 'canary:item/22761')
    for boss, name, lines, teleport, destination, running in RATHLETON_BOSSES:
        role = slug(boss)
        item = build.get(name, f'Hero of Rathleton: {boss}', 'instance_per_party')
        build.participant(item, role, boss, event)
        item['encounter']['anchors'] += [
            {'key': 'exit_teleporter', 'kind': 'point', 'description': f'Boss-room teleporter; Canary {teleport}.'},
            {'key': 'next_destination', 'kind': 'point', 'description': f'Destination while open; Canary {destination}.'}]
        build.define(item, closed)
        build.define(item, opened)
        conditions = []
        if boss == 'Empowered Glooth Horror':
            conditions = [{'kind': 'creature_present', 'role': slug(stage), 'anchor': 'horror_arena', 'present': False}
                          for stage in [s for s, _, _ in GLOOTH_STAGES] + [boss]]
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role},
                                 'conditions': conditions,
                                 'actions': [{'kind': 'map_item', 'operation': 'transform', 'item': closed, 'into': opened,
                                              'anchor': 'exit_teleporter', 'destination': 'next_destination',
                                              'revert_after_ms': 2 * 60 * 1000}]})
        build.entry(item, RATHLETON_KILL, [48, 49, 50, 51, 52, 53] + lines[:2], 'mapped', path + '/trigger',
                    f'onDeath of the configured boss "{boss.lower()}".')
        if conditions:
            build.entry(item, RATHLETON_KILL, list(range(19, 39)) + [54, 55, 56, 57, 58], 'mapped', path + '/conditions',
                        'checkHorror: nothing happens while a living glooth horror of any stage is within 13 tiles of '
                        '(33555, 31956, 15) on that floor (the horror_arena anchor); the dying one has 0 health and does not '
                        'count.')
        build.entry(item, RATHLETON_KILL, [40, 41, 42, 43, 44, 45, 46, 60, 61, 62, 63, 65, 66, 67, 68, 69, 71, 72] + lines[1:3],
                    'mapped', path + '/actions/0',
                    f'Teleporter 1949 on {teleport} becomes 22761 leading to {destination}; after 2 minutes revertTeleport '
                    'turns it back into 1949 with the destination it had before (oldPos): the revert restores the original '
                    'item and its attributes.')
        build.entry(item, RATHLETON_KILL, [70], 'approved_omission', None,
                    'The thunder effect on the boss position is cosmetic.')
        build.entry(item, RATHLETON_KILL, [73] + lines[3:4], 'approved_omission', None,
                    f'Game.setStorageValue(<running flag>, 0) frees the shared arena that {running} locks; an instance per '
                    'party (D26) replaces the lock.')


AZERUS = 'data-otservbr-global/scripts/quests/in_service_of_yalahar/creaturescritps_azerus_kill.lua'


def azerus(build):
    """AzerusDeath: Azerus leaves a two-minute teleporter where he dies and the arena is cleared of monsters."""
    event = 'AzerusDeath'
    item = build.get('azerus', 'In Service of Yalahar: Azerus', 'instance_per_party')
    for name in ('Azerus', 'Azerus2'):
        build.participant(item, 'azerus', name, event)
    teleporter = ref('Item', 'canary:item/1949')
    build.define(item, teleporter)
    item['encounter']['anchors'] += [
        {'key': 'azerus_escape', 'kind': 'point', 'description': 'Teleporter destination; Canary (32780, 31168, 14).'},
        {'key': 'arena', 'kind': 'area', 'description': 'Tiles within 10 of Canary (32783, 31166, 10) on that floor.'}]
    text = 'Azerus ran into teleporter! It will disappear in 2 minutes. Enter it!'
    path = build.rule(item, {'key': 'azerus_death', 'trigger': {'kind': 'creature_died', 'role': 'azerus'}, 'conditions': [],
                             'actions': [{'kind': 'map_item', 'operation': 'create', 'item': teleporter, 'at': 'death_position',
                                          'destination': 'azerus_escape', 'revert_after_ms': 2 * 60 * 1000},
                                         {'kind': 'say', 'subject': {'role': 'azerus'}, 'text': text, 'mode': 'say'},
                                         {'kind': 'remove', 'all_in': 'arena'}]})
    build.entry(item, AZERUS, [9, 10, 11], 'mapped', path + '/trigger', 'onDeath of Azerus (both registering monster types).')
    build.entry(item, AZERUS, [1, 2, 3, 4, 6, 7, 13, 14, 15, 16, 17, 19, 20], 'mapped', path + '/actions/0',
                'A teleporter 1949 leading to (32780, 31168, 14) is created on the death position and removed after 2 minutes.')
    build.entry(item, AZERUS, [5, 12], 'approved_omission', None, 'Teleport and poff effects on that tile are cosmetic.')
    build.entry(item, AZERUS, [18], 'mapped', path + '/actions/1', 'creature:say(text, TALKTYPE_MONSTER_SAY) at the death position.')
    build.entry(item, AZERUS, [22, 23, 24, 25, 26, 28, 29, 30], 'mapped', path + '/actions/2',
                'Every monster within 10 tiles of (32783, 31166, 10) on that floor is removed (players stay).')
    build.entry(item, AZERUS, [27], 'approved_omission', None, 'The poff effect on each removed monster is cosmetic.')


GORZINDEL = 'data-otservbr-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_gorzindel.lua'
GORZINDEL_LEVER = 'data-otservbr-global/scripts/quests/the_secret_library_quest/library_area/actions_gorzindel.lua'
KNOWLEDGES = [('Stolen Knowledge of Armor', 2, 22), ('Stolen Knowledge of Summoning', 3, 23), ('Stolen Knowledge of Lifesteal', 4, 24),
              ('Stolen Knowledge of Spells', 5, 25), ('Stolen Knowledge of Healing', 6, 26)]


def gorzindel(build):
    """gorzindelDeath and gorzindelHealth: Gorzindel is immune until the five stolen knowledges are dead; then the mean
    minions vanish. The Stolen Tome of Portals opens a portal whose per-player room assignment stays unresolved."""
    item = build.get('gorzindel', 'The Secret Library: Gorzindel', 'instance_per_party')
    encounter = item['encounter']
    build.participant(item, 'gorzindel', 'Gorzindel', 'gorzindelHealth')
    build.participant(item, 'mean_minion', 'Mean Minion')
    encounter['state']['flags'].append({'name': 'gorzindel_immune', 'initial': True})
    encounter['anchors'].append({'key': 'knowledge_range', 'kind': 'area',
                                 'description': 'Tiles within 12 of Canary (32687, 32719, 10) on that floor: the main room and the '
                                                'five knowledge rooms.'})
    roles = [slug(k) for k, _, _ in KNOWLEDGES]
    for knowledge, _, _ in KNOWLEDGES:
        build.participant(item, slug(knowledge), knowledge, 'gorzindelDeath')
    for knowledge, line, lever_line in KNOWLEDGES:
        role = slug(knowledge)
        path = build.rule(item, {
            'key': f'{role}_killed', 'trigger': {'kind': 'creature_died', 'role': role}, 'delay_ms': 1000,
            'conditions': [{'kind': 'creature_present', 'role': r, 'anchor': 'knowledge_range', 'present': False} for r in roles],
            'actions': [{'kind': 'remove', 'role': 'mean_minion'}, {'kind': 'flag', 'flag': 'gorzindel_immune', 'value': False}]})
        build.entry(item, GORZINDEL, [11, 13, 16, 17, 37, line], 'mapped', path + '/trigger',
                    f'onDeath of "{knowledge.lower()}" (placed once by {GORZINDEL_LEVER} line {lever_line}) runs a check one '
                    'second later (delay_ms).')
        build.entry(item, GORZINDEL, [9, 18, 19, 20, 21, 22, 23, 24, 25], 'mapped', path + '/conditions',
                    'The check acts only when no stolen knowledge is within 12 tiles of (32687, 32719, 10).')
        build.entry(item, GORZINDEL, [26, 27, 28, 30], 'mapped', path + '/actions/0',
                    'Every mean minion in range is removed; mean minions exist only in the main room.')
        build.entry(item, GORZINDEL, [31, 32], 'mapped', path + '/actions/1',
                    'c:unregisterEvent("gorzindelHealth") ends Gorzindel\'s immunity (the gorzindel_immune flag).')
    build.entry(item, GORZINDEL, [29], 'approved_omission', None, 'The poff effect on each removed minion is cosmetic.')
    path = build.rule(item, {'key': 'gorzindel_immunity', 'trigger': {'kind': 'damage_taken', 'role': 'gorzindel', 'source': 'any'},
                             'conditions': [{'kind': 'flag', 'flag': 'gorzindel_immune', 'value': True}],
                             'actions': [{'kind': 'damage_modifier', 'role': 'gorzindel', 'multiplier_percent': 0, 'sources': 'any',
                                          'until': 'this_hit'}]})
    build.entry(item, GORZINDEL, [55, 57, 58, 59, 60, 63], 'mapped', path,
                'gorzindelHealth (registered by the Gorzindel monster file) zeroes both damage parts of every hit while the '
                'event stays registered; Gorzindel has no healing, so only damage is affected.')
    build.participant(item, 'stolen_tome_of_portals', 'Stolen Tome of Portals')
    build.entry(item, GORZINDEL, [38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49], 'unresolved_semantics', None,
                'The Stolen Tome of Portals leaves a portal (item 1949, action 4952) on its tile and returns there after 10 s. '
                'The portal sends each player who steps in to the next free knowledge room for 10 s '
                '(movements_gorzindel.lua lines 1-38): a per-player room assignment outside the v1 vocabulary.')

HEART = 'data-otservbr-global/scripts/quests/heart_of_destruction/'
HEART_MINION = HEART + 'creaturescripts_heart_minion_death.lua'
HEART_FINAL_LEVER = HEART + 'actions_final_lever.lua'


def heart_minions(build):
    """HeartMinionDeath: minion and boss deaths update the counters of the World Devourer fight and the resonance state of
    the Rupture fight. The summon spells that read the counters are transcribed with those monsters."""
    event = 'HeartMinionDeath'
    item = build.get('world_devourer', 'Heart of Destruction: World Devourer', 'instance_per_party')
    encounter = item['encounter']
    for name in ('rage_summons', 'destruction_summons', 'devourer_summons', 'bosses_killed'):
        encounter['state']['counters'].append({'name': name, 'initial': 0})
    for name in ('the_hunger_killed', 'the_destruction_killed', 'the_rage_killed'):
        encounter['state']['flags'].append({'name': name, 'initial': False})
    build.entry(item, HEART_FINAL_LEVER, [457, 458, 459, 460, 462, 463, 464, 465], 'mapped', '/encounter/state',
                'The final lever starts the fight with every counter at 0 and every boss flag false.')
    minions = [('Frenzy', [7, 8, 9], ['rage_summons', 'devourer_summons']),
               ('Disruption', [12, 13, 14], ['destruction_summons', 'devourer_summons']),
               ('Charged Disruption', [12, 13, 14], ['destruction_summons', 'devourer_summons']),
               ('Overcharged Disruption', [12, 13, 14], ['destruction_summons', 'devourer_summons'])]
    for minion, lines, counters in minions:
        role = slug(minion)
        build.participant(item, role, minion, event)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'counter', 'counter': c, 'operation': 'add', 'value': -1} for c in counters]})
        build.entry(item, HEART_MINION, [1, 2, 3, 4, 5, 6] + lines, 'mapped', path + '/actions',
                    f'The death of a {minion.lower()} lowers the {" and ".join(counters)} counters by one.')
    for boss, lines in (('The Hunger', [15, 16, 17]), ('The Destruction', [18, 19, 20]), ('The Rage', [21, 22, 23])):
        role = slug(boss)
        build.participant(item, role, boss, event)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'counter', 'counter': 'bosses_killed', 'operation': 'add', 'value': 1},
                                             {'kind': 'flag', 'flag': f'{role}_killed', 'value': True}]})
        build.entry(item, HEART_MINION, [1, 2, 3, 4, 5, 6] + lines, 'mapped', path + '/actions',
                    f'The death of {boss.lower()} raises bosses_killed and sets its killed flag.')

    rupture = build.get('rupture', 'Heart of Destruction: Rupture', 'instance_per_party')
    rupture['encounter']['state']['counters'].append({'name': 'resonance_active', 'initial': -1})
    build.entry(rupture, HEART + 'actions_rupture.lua', [25], 'mapped', '/encounter/state/counters/0',
                'The Rupture lever starts the fight with RuptureResonanceActive at -1.')
    build.participant(rupture, 'damage_resonance', 'Damage Resonance', event)
    path = build.rule(rupture, {'key': 'damage_resonance_death', 'trigger': {'kind': 'creature_died', 'role': 'damage_resonance'},
                                'conditions': [], 'actions': [{'kind': 'counter', 'counter': 'resonance_active', 'operation': 'set',
                                                               'value': 0}]})
    build.entry(rupture, HEART_MINION, [1, 2, 3, 4, 5, 6, 10, 11], 'mapped', path + '/actions/0',
                'The death of the damage resonance sets RuptureResonanceActive to 0.')



def heart_chargers(build):
    """ChargerSpawn: a dead charger is replaced after 6 s on one of ten fixed spots."""
    path_ = HEART + 'creaturescripts_charger_spawn.lua'
    item = build.get('heart_chargers', 'Heart of Destruction: charger room', 'instance_per_party')
    build.participant(item, 'charger', 'Charger', 'ChargerSpawn')
    item['encounter']['anchors'].append({
        'key': 'charger_spots', 'kind': 'area',
        'description': 'Exactly ten tiles, Canary (32151, 31356, 14), (32154, 31353, 14), (32153, 31361, 14), (32158, 31362, 14), '
                       '(32161, 31360, 14), (32156, 31357, 14), (32159, 31354, 14), (32163, 31356, 14), (32162, 31352, 14), '
                       '(32158, 31350, 14).'})
    path = build.rule(item, {'key': 'charger_respawn', 'trigger': {'kind': 'creature_died', 'role': 'charger'}, 'delay_ms': 6000,
                             'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('Charger'), 'role': 'charger',
                                                            'count': 1, 'at': {'random_in': 'charger_spots'}, 'owner': 'none',
                                                            'health': 'full'}]})
    build.entry(item, path_, [7, 8, 23, 25, 26], 'mapped', path + '/trigger',
                'onDeath of a charger schedules chargerSpawn 6000 ms later, once per death (delay_ms).')
    build.entry(item, path_, [1, 2, 4, 5, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22], 'mapped', path + '/actions/0',
                'Game.createMonster("charger", one of the ten positions picked uniformly, false, true): a full-health charger '
                'on a uniformly random tile of the charger_spots anchor.')
    build.entry(item, path_, [3, 24], 'approved_omission', None,
                'spawningCharge marks a pending respawn for actions_charges_lever.lua line 65, which keeps the shared room '
                'locked; an instance per party (D26) replaces the lock.')



HEART_BOSS_DEATH = HEART + 'creaturescripts_heart_boss_death.lua'
HEART_SPARK = 'Spark of Destruction'
# encounter (the dying boss), display, lever file, arena corners, vortex tile, HeartBossDeath config lines, access storage, sparks
HEART_BOSS_ROOMS = [
    ('anomaly', 'Anomaly', 'actions_anomaly.lua', ((32258, 31237), (32284, 31262)), (32261, 31250), range(76, 83), 14326,
     [(32267, 31253), (32274, 31255), (32274, 31249), (32267, 31249)]),
    ('rupture', 'Rupture', 'actions_rupture.lua', ((32324, 31239), (32347, 31263)), (32326, 31250), range(83, 90), 14327,
     [(32331, 31254), (32338, 31254), (32330, 31250), (32338, 31250)]),
    ('realityquake', 'Realityquake', 'actions_foreshock.lua', ((32197, 31236), (32220, 31260)), (32199, 31248), range(90, 97), 14328,
     [(32203, 31246), (32205, 31251), (32210, 31251), (32212, 31246)]),
    ('eradicator', 'Eradicator', 'actions_eradicator.lua', ((32297, 31272), (32321, 31296)), (32318, 31284), range(97, 104), 14330,
     [(32304, 31282), (32305, 31287), (32312, 31287), (32314, 31282)]),
    ('outburst', 'Outburst', 'actions_outburst.lua', ((32223, 31273), (32246, 31297)), (32225, 31285), range(104, 111), 14332,
     [(32229, 31282), (32230, 31287), (32237, 31287), (32238, 31282)])]


def heart_sparks():
    return [{'kind': 'spawn', 'creature': creature(HEART_SPARK), 'count': 1, 'at': {'anchor': f'spark_spot_{n}'}, 'owner': 'none',
             'health': 'full'} for n in range(1, 5)]


def heart_stage_rules(build, item, role, stages, actions, path_, lines, text, extra_conditions=(), respawn_check=False):
    """One rule per onThink stage `(threshold, stage)`: crossing the threshold at that stage runs `actions(stage)`. A creature that
    comes back already at or below the threshold of its stage is checked again when it appears (`respawn_check`)."""
    counter = f'{role}_stage'
    for threshold, stage in stages:
        conditions = [{'kind': 'counter_compare', 'counter': counter, 'op': '==', 'value': stage}, *extra_conditions]
        path = build.rule(item, {'key': f'{role}_stage_{stage}', 'trigger': {'kind': 'health_crossed', 'role': role, 'percent': threshold},
                                 'conditions': conditions, 'actions': actions(stage)})
        build.entry(item, path_, lines, 'mapped', path, text.format(threshold=threshold, stage=stage))
        if respawn_check and stage:
            path = build.rule(item, {'key': f'{role}_returns_at_stage_{stage}', 'trigger': {'kind': 'creature_spawned', 'role': role},
                                     'conditions': conditions + [{'kind': 'health_percent', 'role': role, 'op': '<=', 'value': threshold}],
                                     'actions': actions(stage)})
            build.entry(item, path_, lines, 'mapped', path,
                        f'onThink also fires when the {role} comes back at stage {stage} already at or below {threshold}% health.')


def heart_bosses(build):
    """The five Heart of Destruction boss rooms and the World Devourer: HeartBossDeath opens the room vortex and credits every
    player in the room (D27); the stage scripts swap each boss with a charged form or its twin at fixed health thresholds."""
    rooms = {}
    for name, display, lever, ((x1, y1), (x2, y2)), (vx, vy), config, storage, sparks in HEART_BOSS_ROOMS:
        item = rooms[name] = build.get(name, f'Heart of Destruction: {display}', 'instance_per_party')
        anchors = item['encounter']['anchors']
        anchors.append({'key': 'arena', 'kind': 'area', 'description': f'Every tile from Canary ({x1}, {y1}, 14) to ({x2}, {y2}, 14).'})
        anchors.append({'key': 'exit_vortex', 'kind': 'point', 'description': f'Canary ({vx}, {vy}, 14), the vortex of the room.'})
        anchors += [{'key': f'spark_spot_{n}', 'kind': 'point', 'description': f'Canary ({x}, {y}, 14).'} for n, (x, y) in enumerate(sparks, 1)]
        build.define(item, creature(HEART_SPARK))
        build.entry(item, HEART + lever, [13, 14, 15, 16], 'mapped', '/encounter/anchors/0',
                    f'The lever room check area from ({x1}, {y1}, 14) to ({x2}, {y2}, 14) is the arena HeartBossDeath credits.')

    fight_vortex, vortex = ref('Item', 'canary:item/23483'), ref('Item', 'canary:item/23482')

    def boss_death(item, role, boss, lines, tile_line, credit_lines, aid, storage, extra=()):
        outcome = f'{role}_defeated'
        item['encounter']['outcomes'].append(outcome)
        build.define(item, fight_vortex)
        build.define(item, vortex)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'map_item', 'operation': 'transform', 'item': fight_vortex, 'into': vortex,
                                              'anchor': 'exit_vortex', 'interaction': f'canary:interaction/{aid}'},
                                             {'kind': 'emit_outcome', 'outcome': outcome, 'credited': 'players_in_anchor',
                                              'anchor': 'arena'}, *extra]})
        build.entry(item, HEART_BOSS_DEATH, [113, 115, 116, 117, 118, 119, 120, 121, 122, 138, 139, 141], 'mapped', path + '/trigger',
                    f'onDeath of {boss} (the name is looked up in lower case).')
        build.entry(item, HEART_BOSS_DEATH, lines + tile_line, 'mapped', path + '/actions/0',
                    f'The vortex 23483 on the room vortex tile becomes vortex 23482 with action id {aid}, which '
                    f'movements_teleport_heart.lua sends to {"the reward room (32112, 31375, 14)" if aid == 14354 else "the main room (32216, 31380, 14)"} '
                    '(interaction domain). The lever made it 23483 when the fight started.')
        build.entry(item, HEART_BOSS_DEATH, credit_lines, 'mapped', path + '/actions/1',
                    'Every player standing in the room is credited (D27): the quest domain sets their storage '
                    + ', '.join(str(s) for s in storage) + ' to 1.')
        item['manifest']['outcome_evidence'].append({'outcome': outcome, 'credited': 'players_in_anchor',
                                                     'quest_domain': {'canary_storage': [str(s) for s in storage], 'raise_to_at_least': 1}})
        return path

    for name, display, lever, corners, tile, config, storage, sparks in HEART_BOSS_ROOMS:
        item = rooms[name]
        build.participant(item, name, display, 'HeartBossDeath')
        if name == 'eradicator':
            build.participant(item, name, 'Eradicator2', 'HeartBossDeath')
        boss_death(item, name, display, list(config), [123, 124, 125, 126, 127], [51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
                   64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 75, 111, 128], 14325, [storage])
    build.entry(rooms['eradicator'], HEART_BOSS_DEATH, [97], 'mapped', '/encounter/participants/0/creatures',
                'Eradicator2 is also named "Eradicator" (monster.name), so its death runs the same branch.')

    # World Devourer: the room is cleared after the credit.
    item = build.get('world_devourer', 'Heart of Destruction: World Devourer', 'instance_per_party')
    item['encounter']['anchors'] += [
        {'key': 'arena', 'kind': 'area', 'description': 'Every tile from Canary (32260, 31336, 14) to (32283, 31360, 14).'},
        {'key': 'exit_vortex', 'kind': 'point', 'description': 'Canary (32281, 31348, 14), the vortex of the room.'}]
    build.participant(item, 'world_devourer', 'World Devourer', 'HeartBossDeath')
    path = boss_death(item, 'world_devourer', 'World Devourer', [129], [130, 131, 132, 133, 134], [26, 27, 28, 29, 30, 31, 32, 33,
                      34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 135], 14354, [60835, 60814, 60828],
                      [{'kind': 'remove', 'all_in': 'arena'}])
    build.entry(item, HEART_BOSS_DEATH, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 136], 'mapped',
                path + '/actions/2', 'clearDevourer removes every monster in the room.')
    build.entry(item, HEART_BOSS_DEATH, [21, 22, 23, 24], 'approved_omission', None,
                'stopEvent cancels the room-change, room-clear and spark timers of the final lever (actions_final_lever.lua lines '
                '23, 178, 218, 219). Those belong to the lever transcription; the encounter ends with the boss.')

    # Anomaly: four charged anomalies; each death brings the anomaly back at the next quarter of its health.
    item = rooms['anomaly']
    transform, death = HEART + 'creaturescripts_anomaly_transform.lua', HEART + 'creaturescripts_charged_anomaly_death.lua'
    item['encounter']['anchors'].append({'key': 'boss_spot', 'kind': 'point', 'description': 'Canary (32271, 31249, 14).'})
    item['encounter']['state']['counters'].append({'name': 'anomaly_stage', 'initial': 0})
    build.entry(item, HEART + 'actions_anomaly.lua', [23, 24], 'mapped', '/encounter/state/counters/0',
                'The lever starts the fight with ChargedAnomaly 0.')
    build.participant(item, 'anomaly', 'Anomaly', 'AnomalyTransform')
    build.participant(item, 'charged_anomaly', 'Charged Anomaly', 'ChargedAnomalyDeath')
    heart_stage_rules(build, item, 'anomaly', [(75, 0), (50, 1), (25, 2), (5, 3)], lambda stage: [
        {'kind': 'remove', 'role': 'anomaly'}, *heart_sparks(),
        {'kind': 'spawn', 'creature': creature('Charged Anomaly'), 'role': 'charged_anomaly', 'count': 1, 'at': {'anchor': 'boss_spot'},
         'owner': 'none', 'health': 'full'},
        {'kind': 'counter', 'counter': 'anomaly_stage', 'operation': 'set', 'value': stage + 1}],
        transform, [1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 13, 14, 15, 17, 18, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 36],
        'onThink: at or below {threshold}% health with ChargedAnomaly {stage}, the anomaly is removed and four sparks and a '
        'charged anomaly appear; ChargedAnomaly becomes {stage} + 1.')
    for stage, health, lines in ((1, 75, [7]), (2, 50, [8]), (3, 25, [9]), (4, 5, [10])):
        path = build.rule(item, {'key': f'charged_anomaly_death_{stage}', 'trigger': {'kind': 'creature_died', 'role': 'charged_anomaly'},
                                 'conditions': [{'kind': 'counter_compare', 'counter': 'anomaly_stage', 'op': '==', 'value': stage}],
                                 'actions': [{'kind': 'spawn', 'creature': creature('Anomaly'), 'role': 'anomaly', 'count': 1,
                                              'at': {'anchor': 'boss_spot'}, 'owner': 'none', 'health': {'percent': health}}]})
        build.entry(item, death, [1, 2, 3, 4, 6, 11, 13, 15, 16, 17, 18, 20, 21, 22, 23, 25, 26, 27, 28, 30, 31, 33] + lines, 'mapped',
                    path, f'onDeath of a charged anomaly at ChargedAnomaly {stage}: the anomaly comes back with {290000 - health * 2900} '
                    f'of its 290000 health removed, {health}% left.')

    # Rupture: five resonance waves; while a damage resonance lives, each player hit heals the rupture.
    item = rooms['rupture']
    resonance, heal = HEART + 'creaturescripts_rupture_resonance.lua', HEART + 'creaturescripts_rupture_heal.lua'
    item['encounter']['anchors'].append({'key': 'resonance_spot', 'kind': 'point', 'description': 'Canary (32332, 31250, 14).'})
    item['encounter']['state']['counters'].append({'name': 'rupture_stage', 'initial': 0})
    build.entry(item, HEART + 'actions_rupture.lua', [23, 24], 'mapped', '/encounter/state/counters/1',
                'The lever starts the fight with RuptureResonanceStage -1, read as 0.')

    def wave(stage):
        return [*heart_sparks(),
                {'kind': 'spawn', 'creature': creature('Damage Resonance'), 'role': 'damage_resonance', 'count': 1,
                 'at': {'anchor': 'resonance_spot'}, 'owner': 'none', 'health': 'full'},
                {'kind': 'counter', 'counter': 'rupture_stage', 'operation': 'set', 'value': stage + 1},
                {'kind': 'counter', 'counter': 'resonance_active', 'operation': 'set', 'value': 1}]
    build.participant(item, 'rupture', 'Rupture', 'RuptureResonance')
    stages = [(80, 0), (60, 1), (40, 2), (25, 3), (10, 4)]
    inactive = [{'kind': 'counter_compare', 'counter': 'resonance_active', 'op': '!=', 'value': 1}]
    heart_stage_rules(build, item, 'rupture', stages, wave, resonance,
                      [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 13, 14, 15, 16, 18, 19, 21, 22, 23, 24, 25, 26, 27, 29, 30, 31, 32, 33, 34, 35, 37, 38, 40],
                      'onThink: at or below {threshold}% health at resonance stage {stage} with no resonance active, four sparks and a '
                      'damage resonance appear; the stage becomes {stage} + 1 and the resonance is active.', inactive)
    for threshold, stage in stages:
        path = build.rule(item, {'key': f'rupture_stage_{stage}_after_resonance', 'trigger': {'kind': 'creature_died', 'role': 'damage_resonance'},
                                 'conditions': [{'kind': 'counter_compare', 'counter': 'rupture_stage', 'op': '==', 'value': stage},
                                                {'kind': 'health_percent', 'role': 'rupture', 'op': '<=', 'value': threshold}],
                                 'actions': wave(stage)})
        build.entry(item, resonance, [13, 29, 30, 31, 32], 'mapped', path,
                    f'onThink also fires when the resonance dies (RuptureResonanceActive 0) while the rupture is already at or below '
                    f'{threshold}% at stage {stage}.')
    build.participant(item, 'rupture', 'Rupture', 'RuptureHeal')
    path = build.rule(item, {'key': 'rupture_heals_on_player_hit', 'trigger': {'kind': 'damage_taken', 'role': 'rupture', 'source': 'player'},
                             'conditions': [{'kind': 'counter_compare', 'counter': 'resonance_active', 'op': '==', 'value': 1}],
                             'actions': [{'kind': 'heal', 'subject': {'role': 'rupture'}, 'amount': {'min': 5000, 'max': 10000}}]})
    build.entry(item, heal, [1, 3, 4, 5, 6, 7, 9, 10, 11, 13], 'mapped', path,
                'onHealthChange: a player hit while the resonance is active heals the rupture by 5000-10000; the hit itself is '
                'applied unchanged.')
    build.entry(item, heal, [8], 'approved_omission', None, 'The green magic effect is cosmetic.')

    # Foreshock and Aftershock swap at every 20% (then 10%) of their own health; Realityquake follows the second death.
    item = rooms['realityquake']
    shocks, fore, after = HEART + 'creaturescripts_shocks_death.lua', HEART + 'creaturescripts_foreshock_transform.lua', \
        HEART + 'creaturescripts_aftershock_transform.lua'
    item['encounter']['anchors'].append({'key': 'shock_spot', 'kind': 'point', 'description': 'Canary (32208, 31248, 14).'})
    state = item['encounter']['state']
    state['counters'] += [{'name': 'foreshock_stage', 'initial': 0}, {'name': 'aftershock_stage', 'initial': 0}]
    state['flags'].append({'name': 'foreshock_defeated', 'initial': False})
    build.entry(item, HEART + 'actions_foreshock.lua', [23, 24, 25, 26, 27], 'mapped', '/encounter/state',
                'The lever starts both shocks at stage -1 (read as 0) and stores 105000 health for each, their full health.')
    build.define(item, creature('Realityquake'))

    def swap(leaving, coming):
        return lambda stage: [{'kind': 'spawn', 'creature': creature(coming.title()), 'role': coming, 'count': 1, 'at': {'anchor': 'shock_spot'},
                               'owner': 'none', 'health': 'remembered'},
                              {'kind': 'remove', 'role': leaving}, *heart_sparks(),
                              {'kind': 'counter', 'counter': f'{leaving}_stage', 'operation': 'set', 'value': stage + 1}]
    stages = [(80, 0), (60, 1), (40, 2), (20, 3), (10, 4)]
    swap_lines = [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 16, 18, 19, 20, 21, 23, 24, 25, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
                  38, 39, 40, 41, 42, 43, 45]
    build.participant(item, 'foreshock', 'Foreshock', 'ForeshockTransform')
    heart_stage_rules(build, item, 'foreshock', stages, swap('foreshock', 'aftershock'), fore, swap_lines,
                      'onThink: at or below {threshold}% health at stage {stage}, an aftershock appears with the health it last had '
                      '(stored every think, 105000 at the start), the foreshock is removed, four sparks appear and the foreshock '
                      'stage becomes {stage} + 1.', respawn_check=True)
    build.participant(item, 'aftershock', 'Aftershock', 'AftershockTransform')
    heart_stage_rules(build, item, 'aftershock', stages, swap('aftershock', 'foreshock'), after, swap_lines,
                      'onThink: at or below {threshold}% health at stage {stage}, a foreshock appears with the health it last had, the '
                      'aftershock is removed, four sparks appear and the aftershock stage becomes {stage} + 1. The wiki (2026-07-28) '
                      'spawns Realityquake "after defeating Foreshock and Aftershock", so a defeated foreshock does not come back (D25); '
                      'Canary would bring it back with the health of its last think.',
                      [{'kind': 'flag', 'flag': 'foreshock_defeated', 'value': False}], respawn_check=True)
    build.participant(item, 'foreshock', 'Foreshock', 'ShocksDeath')
    path = build.rule(item, {'key': 'foreshock_death', 'trigger': {'kind': 'creature_died', 'role': 'foreshock'}, 'conditions': [],
                             'actions': [{'kind': 'flag', 'flag': 'foreshock_defeated', 'value': True},
                                         {'kind': 'spawn', 'creature': creature('Aftershock'), 'role': 'aftershock', 'count': 1,
                                          'at': {'anchor': 'shock_spot'}, 'owner': 'none', 'health': 'remembered'}, *heart_sparks()]})
    build.entry(item, shocks, [1, 2, 3, 4, 5, 6, 8, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 29, 30, 32], 'mapped', path,
                'onDeath of the foreshock: an aftershock appears with the health it last had and four sparks appear.')
    build.participant(item, 'aftershock', 'Aftershock', 'ShocksDeath')
    path = build.rule(item, {'key': 'aftershock_death', 'trigger': {'kind': 'creature_died', 'role': 'aftershock'}, 'conditions': [],
                             'actions': [{'kind': 'spawn', 'creature': creature('Realityquake'), 'role': 'realityquake', 'count': 1,
                                          'at': 'death_position', 'owner': 'none', 'health': 'full'}, *heart_sparks()]})
    build.entry(item, shocks, [23, 24, 25, 26, 27, 28], 'mapped', path,
                'onDeath of the aftershock: Realityquake appears where it died, with four sparks.')

    # Eradicator: 74 s shielded, then 9 s exposed as Eradicator2 with four sparks, again and again.
    item = rooms['eradicator']
    think = HEART + 'creaturescripts_eradicator_transform.lua'
    item['encounter']['state']['timers'] += [{'name': 'eradicator_shielded', 'duration_ms': 74000, 'repeat': False},
                                             {'name': 'eradicator_exposed', 'duration_ms': 9000, 'repeat': False}]
    path = build.rule(item, {'key': 'eradicator_fight_starts', 'trigger': {'kind': 'encounter_started'}, 'conditions': [],
                             'actions': [{'kind': 'timer', 'timer': 'eradicator_shielded', 'operation': 'start'}]})
    build.entry(item, HEART + 'actions_eradicator.lua', [23, 24, 25, 27, 28, 29], 'mapped', path,
                'The lever sets EradicatorWeak and EradicatorReleaseT to -1 and releases the first change 74000 ms later.')
    build.participant(item, 'eradicator', 'Eradicator', 'EradicatorTransform')
    build.participant(item, 'eradicator', 'Eradicator2', 'EradicatorTransform')
    path = build.rule(item, {'key': 'eradicator_exposed', 'trigger': {'kind': 'timer_elapsed', 'timer': 'eradicator_shielded'}, 'conditions': [],
                             'actions': [{'kind': 'transform', 'role': 'eradicator', 'into': creature('Eradicator2'), 'health': 'keep_absolute'},
                                         *heart_sparks(), {'kind': 'timer', 'timer': 'eradicator_exposed', 'operation': 'start'}]})
    build.entry(item, think, [1, 3, 4, 5, 6, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 30, 31, 32, 34, 35,
                              36, 37, 38, 40], 'mapped', path,
                'onThink after the release with EradicatorWeak not above 0: the eradicator is replaced on its tile by Eradicator2 with '
                'the same health, four sparks appear, and the next release follows 9000 ms later.')
    path = build.rule(item, {'key': 'eradicator_shielded_again', 'trigger': {'kind': 'timer_elapsed', 'timer': 'eradicator_exposed'},
                             'conditions': [], 'actions': [{'kind': 'transform', 'role': 'eradicator', 'into': creature('Eradicator'),
                                                            'health': 'keep_absolute'},
                                                           {'kind': 'timer', 'timer': 'eradicator_shielded', 'operation': 'start'}]})
    build.entry(item, think, [8, 9, 10, 11, 12, 14, 15, 16, 21, 30, 31, 32, 34, 35], 'mapped', path,
                'onThink after the release with EradicatorWeak 1: Eradicator2 is replaced by the Eradicator with the same health and '
                'the next release follows 74000 ms later. Canary acts on the first think after the release (about one second).')

    # Outburst: four charging outbursts; the outburst comes back with the health it had.
    item = rooms['outburst']
    charge, death = HEART + 'creaturescripts_outburst_charge.lua', HEART + 'creaturescripts_charging_out_death.lua'
    item['encounter']['anchors'] += [{'key': 'boss_spot', 'kind': 'point', 'description': 'Canary (32234, 31284, 14).'},
                                     {'key': 'outburst_return', 'kind': 'point', 'description': 'Canary (32234, 31285, 14).'}]
    item['encounter']['state']['counters'].append({'name': 'outburst_stage', 'initial': 0})
    item['encounter']['state']['flags'].append({'name': 'charging_outburst_exploded', 'initial': False})
    build.entry(item, HEART + 'actions_outburst.lua', [23, 24, 25], 'mapped', '/encounter/state',
                'The lever starts the fight at OutburstStage 0 and stores 290000 health, the full health of the outburst.')
    build.participant(item, 'outburst', 'Outburst', 'OutburstCharge')
    build.define(item, creature('Charging Outburst'))
    heart_stage_rules(build, item, 'outburst', [(80, 0), (60, 1), (40, 2), (20, 3)], lambda stage: [
        {'kind': 'remove', 'role': 'outburst'}, *heart_sparks(),
        {'kind': 'spawn', 'creature': creature('Charging Outburst'), 'role': 'charging_outburst', 'count': 1, 'at': {'anchor': 'boss_spot'},
         'owner': 'none', 'health': 'full'},
        {'kind': 'counter', 'counter': 'outburst_stage', 'operation': 'set', 'value': stage + 1},
        {'kind': 'flag', 'flag': 'charging_outburst_exploded', 'value': False}],
        charge, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 15, 16, 17, 19, 20, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 39],
        'onThink: at or below {threshold}% health at OutburstStage {stage}, the outburst (its health stored every think) is removed, '
        'four sparks and a charging outburst appear, the stage becomes {stage} + 1 and the explosion mark is cleared.',
        respawn_check=True)
    build.participant(item, 'charging_outburst', 'Charging Outburst', 'ChargingOutDeath')
    path = build.rule(item, {'key': 'charging_outburst_death', 'trigger': {'kind': 'creature_died', 'role': 'charging_outburst'},
                             'conditions': [{'kind': 'flag', 'flag': 'charging_outburst_exploded', 'value': False}],
                             'actions': [{'kind': 'spawn', 'creature': creature('Outburst'), 'role': 'outburst', 'count': 1,
                                          'at': {'anchor': 'outburst_return'}, 'owner': 'none', 'health': 'remembered'}]})
    build.entry(item, death, [1, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15], 'mapped', path,
                'onDeath of a charging outburst that has not exploded: the outburst comes back one tile south of its start with the '
                'health it had. The explosion (spell outburst_explode, transcribed with the charging outburst) sets the mark and '
                'brings the outburst back itself.')



FORGOTTEN = 'data-otservbr-global/scripts/quests/forgotten_knowledge/'


def forgotten_knowledge_fights(build):
    """HealthForgotten, ThornKnightDeath, LloydPrepareDeath and the energy prism events: the fight mechanics of Lady Tenebris,
    the Thorn Knight and Lloyd."""
    health = FORGOTTEN + 'creaturescripts_healthchange_forgotten.lua'
    for encounter, bosses, guard, guard_lines in (
            ('lady_tenebris', ['Lady Tenebris'], 'Shadow Tentacle', [4, 5, 6, 7, 8, 9, 10, 11]),
            ('the_enraged_thorn_knight', ['Mounted Thorn Knight', 'The Shielded Thorn Knight', 'The Enraged Thorn Knight'],
             'Possessed Tree', [12, 13, 14, 15, 16, 17, 18, 19, 20])):
        item = build.items[encounter]
        build.participant(item, slug(guard), guard)
        for boss in bosses:
            role = slug(boss)
            build.participant(item, role, boss, 'HealthForgotten')
            rules = []
            for source_kind in ('damage_taken', 'heal_received'):
                rules.append(build.rule(item, {
                    'key': f'{role}_unguarded_{source_kind}', 'trigger': {'kind': source_kind, 'role': role, 'source': 'any'},
                    'conditions': [{'kind': 'creature_present', 'role': slug(guard), 'near': {'role': role, 'radius': 7}, 'present': False}],
                    'actions': [{'kind': 'damage_modifier', 'role': role, 'multiplier_percent': 200, 'component': 'primary', 'sources': 'any',
                                 'until': 'this_hit'}]}))
            build.entry(item, health, [1, 2, 3, 21, 22, 23, 25], 'mapped', rules[0],
                        f'onHealthChange of {boss}: the primary part of every change is doubled (primary + 100/100 * primary).')
            build.entry(item, health, [1, 2], 'mapped', rules[1],
                        'The same handler runs for heals (game.cpp combatChangeHealth), so a heal of the boss is doubled too; in a '
                        'heal_received rule the this_hit modifier scales that heal (D31).')
            build.entry(item, health, guard_lines, 'mapped', rules[0] + '/conditions/0',
                        f'Unless a {guard.lower()} stands within 7 tiles on the same floor (getSpectators 7/7/7/7, not multi-floor); '
                        'the spectator is found by name, which only that creature carries.')
            build.entry(item, health, guard_lines, 'mapped', rules[1] + '/conditions/0', 'The same guard for heals.')

    # The Thorn Knight dismounts, then loses its shield.
    item = build.items['the_enraged_thorn_knight']
    death = FORGOTTEN + 'creaturescripts_thorn_knight_death.lua'
    build.define(item, creature('Thorn Steed'))
    for boss, following, extra, lines in (
            ('Mounted Thorn Knight', 'The Shielded Thorn Knight', True, [13, 14, 15, 16, 18]),
            ('The Shielded Thorn Knight', 'The Enraged Thorn Knight', False, [19, 20, 22])):
        role = slug(boss)
        build.participant(item, role, boss, 'ThornKnightDeath')
        actions = [{'kind': 'spawn', 'creature': creature(following), 'role': slug(following), 'count': 1, 'at': 'death_position',
                    'owner': 'none', 'health': 'full'}]
        if extra:
            actions = [{'kind': 'say', 'subject': {'role': role}, 'text': 'The thorn knight unmounts!', 'mode': 'say'}, *actions,
                       {'kind': 'spawn', 'creature': creature('Thorn Steed'), 'count': 1, 'at': 'death_position', 'owner': 'none',
                        'health': 'full'}]
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': actions})
        build.entry(item, death, [11, 12, 23, 24, 25, 27] + lines, 'mapped', path,
                    f'onDeath of {boss.lower()}: {following} appears (forced) where it died'
                    + (', after its line, with a thorn steed.' if extra else '.'))
    build.entry(item, death, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 17, 21], 'approved_omission', None,
                'checkBlood removes the unmovable items (the blood splash) from the death tile 1 ms later; cosmetic.')

    # Lloyd: four times he refocuses on a prism instead of dying; the prism can be killed for 10 s.
    item = build.items['lloyd']
    prepare, prism_health, prism_death = (FORGOTTEN + name for name in ('creaturescripts_lloyd_preparedeath.lua',
                                                                        'creaturescripts_energy_prism.lua',
                                                                        'creaturescripts_energy_prism_death.lua'))
    spots = [('a', 32801, 32827, 2), ('b', 32798, 32827, 3), ('c', 32803, 32826, 4), ('d', 32796, 32826, 5)]
    item['encounter']['anchors'] += [
        {'key': 'lloyd_center', 'kind': 'point', 'description': 'Canary (32799, 32826, 14), between the four prisms.'},
        {'key': 'lloyd_center_tile', 'kind': 'area', 'description': 'Exactly the one tile Canary (32799, 32826, 14).'},
        {'key': 'lloyd_return', 'kind': 'point', 'description': 'Canary (32799, 32829, 14).'}]
    item['encounter']['anchors'] += [{'key': f'prism_spot_{letter}', 'kind': 'point', 'description': f'Canary ({x}, {y}, 14).'}
                                     for letter, x, y, _ in spots]
    item['encounter']['state']['counters'].append({'name': 'prisms_destroyed', 'initial': 0})
    item['encounter']['state']['timers'].append({'name': 'lloyd_refocus', 'duration_ms': 10000, 'repeat': False})
    build.entry(item, prepare, [32, 33, 34, 35, 36, 37, 38], 'mapped', '/encounter/state/counters/0',
                'prismCount is 1 plus the prism tiles left empty: the prisms killed so far. The lever places the four invulnerable '
                'prisms.')
    build.participant(item, 'lloyd', 'Lloyd', 'LloydPrepareDeath')
    for index, (letter, x, y, line) in enumerate(spots):
        prism, invulnerable = f'cosmic_energy_prism_{letter}', f'cosmic_energy_prism_{letter}_invu'
        build.participant(item, invulnerable, f'Cosmic Energy Prism {letter.upper()} Invu')
        path = build.rule(item, {
            'key': f'lloyd_refocuses_on_prism_{letter}', 'trigger': {'kind': 'lethal_damage', 'role': 'lloyd'},
            'conditions': [{'kind': 'counter_compare', 'counter': 'prisms_destroyed', 'op': '==', 'value': index}],
            'actions': [{'kind': 'prevent_death', 'role': 'lloyd'}, {'kind': 'remove', 'role': invulnerable},
                        {'kind': 'spawn', 'creature': creature(f'Cosmic Energy Prism {letter.upper()}'), 'role': prism, 'count': 1,
                         'at': {'anchor': f'prism_spot_{letter}'}, 'owner': 'none', 'health': 'full'},
                        {'kind': 'teleport', 'who': {'role': 'lloyd'}, 'to': 'lloyd_center'},
                        {'kind': 'heal', 'subject': {'role': 'lloyd'}, 'amount': 'full'},
                        {'kind': 'say', 'subject': {'role': 'lloyd'}, 'text': 'The cosmic energies in the chamber refocus on Lloyd.',
                         'mode': 'say'},
                        {'kind': 'timer', 'timer': 'lloyd_refocus', 'operation': 'start'}]})
        build.entry(item, prepare, [1, line, 6, 30, 31, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 50, 51, 52, 53, 54, 55, 57], 'mapped', path,
                    f'onPrepareDeath with {index} prisms killed: the invulnerable prism {letter.upper()} becomes the normal one, '
                    'Lloyd is teleported between the prisms and healed by 300000 (more than his 64000 health) and says his line; '
                    'the revert follows 10 s later.')
        path = build.rule(item, {
            'key': f'lloyd_returns_from_prism_{letter}', 'trigger': {'kind': 'timer_elapsed', 'timer': 'lloyd_refocus'},
            'conditions': [{'kind': 'counter_compare', 'counter': 'prisms_destroyed', 'op': '==', 'value': index}],
            'actions': [{'kind': 'teleport', 'who': {'role': 'lloyd'}, 'to': 'lloyd_return'}, {'kind': 'remove', 'role': prism},
                        {'kind': 'spawn', 'creature': creature(f'Cosmic Energy Prism {letter.upper()} Invu'), 'role': invulnerable,
                         'count': 1, 'at': {'anchor': f'prism_spot_{letter}'}, 'owner': 'none', 'health': 'full'}]})
        build.entry(item, prepare, [8, 9, 10, 11, 12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28], 'mapped', path,
                    f'revertLloyd: Lloyd (the creature on the centre tile) goes back to (32799, 32829, 14) and prism {letter.upper()} '
                    'becomes invulnerable again.')
        build.participant(item, prism, f'Cosmic Energy Prism {letter.upper()}', 'EnergyPrismDeath')
        path = build.rule(item, {'key': f'prism_{letter}_death', 'trigger': {'kind': 'creature_died', 'role': prism}, 'conditions': [],
                                 'actions': [{'kind': 'timer', 'timer': 'lloyd_refocus', 'operation': 'stop'},
                                             {'kind': 'counter', 'counter': 'prisms_destroyed', 'operation': 'add', 'value': 1},
                                             {'kind': 'teleport', 'who': {'role': 'lloyd'}, 'to': 'lloyd_return'}]})
        build.entry(item, prism_death, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 16], 'mapped', path,
                    'onDeath of the prism: the revert is cancelled and Lloyd goes back to (32799, 32829, 14); the empty tile counts '
                    'as a killed prism.')
        build.participant(item, prism, f'Cosmic Energy Prism {letter.upper()}', 'EnergyPrismHealthChange')
        rules = []
        for source_kind in ('damage_taken', 'heal_received'):
            rules.append(build.rule(item, {
                'key': f'prism_{letter}_recharges_on_{source_kind}', 'trigger': {'kind': source_kind, 'role': prism, 'source': 'any'},
                'conditions': [{'kind': 'creature_present', 'role': 'lloyd', 'anchor': 'lloyd_center_tile', 'present': False},
                               {'kind': 'health_percent', 'role': prism, 'op': '<', 'value': 100}],
                'actions': [{'kind': 'say', 'subject': {'role': prism}, 'text': '*zap!*', 'mode': 'say'},
                            {'kind': 'heal', 'subject': {'role': prism}, 'amount': 10000}]}))
        build.entry(item, prism_health, [1, 2, 4, 5, 7, 8, 9, 10, 11, 13], 'mapped', rules[0],
                    'onHealthChange of the prism, read before the change: when not at full health it says "*zap!*" and heals 10000 '
                    '(ten times its 1000 health).')
        build.entry(item, prism_health, [1, 2], 'mapped', rules[1], 'The same handler for heals.')
        build.entry(item, prism_health, [3], 'mapped', rules[0] + '/conditions/0',
                    'Only while the centre tile is empty. The creature there is Lloyd: his wiki page (2026-07-28) says a prism can only '
                    'be killed after he teleports between them (D29, D25).')
        build.entry(item, prism_health, [6], 'approved_omission', None, 'The energy hit effect is cosmetic.')
    build.entry(item, prepare, [14, 49], 'approved_omission', None, 'The teleport effects are cosmetic.')
    build.entry(item, prism_death, [11], 'approved_omission', None, 'The teleport effect is cosmetic.')



ASURAS = 'data-otservbr-global/scripts/quests/the_secret_library_quest/the_lament_asuras/creaturescripts_asuras_mechanic.lua'
ASURA_RED_ARMORS = [3566, 3379, 3388, 8039, 8053, 8064, 22534, 3381, 7991, 3380, 10439, 3564]
FACELESS = 'data-otservbr-global/scripts/quests/the_dream_courts_quest/creaturescripts_facelessBane.lua'


def eleventh_slice(build):
    """facelessHealth (Alptramun, Plagueroot), AsurasMechanic and GreedMonsterDeath."""
    for boss, damage_type, lines in (('Alptramun', 'death', [48, 49, 50, 51, 52]), ('Plagueroot', 'earth', [55, 56, 57, 58, 59])):
        role = slug(boss)
        item = build.items[role]
        build.participant(item, role, boss, 'facelessHealth')
        rules = []
        for source_kind in ('damage_taken', 'heal_received'):
            rules.append(build.rule(item, {
                'key': f'{role}_absorbs_{damage_type}_on_{source_kind}', 'trigger': {'kind': source_kind, 'role': role, 'source': 'any'},
                'conditions': [], 'actions': [{'kind': 'convert_damage_to_heal', 'role': role, 'damage_types': [damage_type],
                                               'component': 'primary'}]}))
        build.entry(item, FACELESS, [36, 37, 38, 39, 40, 41, 42, 60, 61, 62, 63, 65] + lines, 'mapped', rules[0],
                    f'onHealthChange of {boss}: primary {damage_type} damage heals it by that amount and deals none; the secondary '
                    'part is unchanged.')
        build.entry(item, FACELESS, [36, 37, 38], 'mapped', rules[1],
                    'The same handler for heals; a heal is never typed as damage, so it passes unchanged.')

    # The three Asura queens take damage only from a player carrying their counter.
    names = {'The Diamond Blossom': 'the_diamond_blossom', 'The Blazing Rose': 'the_blazing_rose', 'The Lily of Night': 'the_lily_of_night'}
    armors = [ref('Item', f'canary:item/{item_id}') for item_id in ASURA_RED_ARMORS]
    chimes = ref('Item', 'canary:item/28494')
    counters = {
        'The Diamond Blossom': ([{'kind': 'attacker_wears', 'item': armor, 'wears': False, 'slot': 'armor'} for armor in armors],
                                [24, 25, 26, 27, 28, 29, 30, 31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
                                'a player without one of the twelve red armours in the armour slot (red robe, doublet, demon armor, '
                                'dragon robe, fireborn giant armor, ethno coat, firemind raiment, crown armor, magician\'s robe, '
                                'noble armor, Zaoan robe, red tunic)'),
        'The Blazing Rose': ([{'kind': 'attacker_wears', 'item': chimes, 'wears': False, 'slot': 'right_hand'}],
                             [33, 34, 35, 36, 37], 'a player without silver chimes (28494) in the right hand'),
        'The Lily of Night': ([{'kind': 'killer_progress', 'progress': 'canary:quest-progress/secret_library_asura_fragrance',
                                'op': '==', 'value': True}], [38, 39, 40, 41, 42],
                              'a player whose fragrance has run out')}
    for boss, (conditions, lines, who) in counters.items():
        role = names[boss]
        item = build.get(role, f'The Secret Library: {boss}', 'instance_per_party')
        build.participant(item, role, boss, 'AsurasMechanic')
        if boss == 'The Diamond Blossom':
            for armor in armors:
                build.define(item, armor)
        elif boss == 'The Blazing Rose':
            build.define(item, chimes)
        else:
            # The fragrance is the only condition, so the rule acts when it is absent.
            conditions = [{**conditions[0], 'value': False}]
        immune = [{'kind': 'damage_modifier', 'role': role, 'multiplier_percent': 0, 'sources': 'any', 'until': 'this_hit'}]
        first = None
        for source_kind in ('damage_taken', 'heal_received'):
            path = build.rule(item, {'key': f'{role}_ignores_{source_kind}_from_unprepared_players',
                                     'trigger': {'kind': source_kind, 'role': role, 'source': 'player'},
                                     'conditions': conditions, 'actions': immune})
            build.entry(item, ASURAS, [16, 17, 18, 19, 20, 21, 22, 23, 43, 44, 45, 46, 47, 48, 50] + lines, 'mapped', path,
                        f'onHealthChange of {boss}: a {"hit" if source_kind == "damage_taken" else "heal"} by {who} is zeroed, '
                        'both parts' + ('; Canary runs the handler for heals too, with the healer as attacker.'
                                        if source_kind == 'heal_received' else '.'))
            first = first or path
            path = build.rule(item, {'key': f'{role}_ignores_{source_kind}_from_creatures',
                                     'trigger': {'kind': source_kind, 'role': role, 'source': 'non_player'},
                                     'conditions': [], 'actions': immune})
            build.entry(item, ASURAS, [18, 19, 20, 21, 23, 43, 45, 46, 47, 48], 'mapped', path,
                        'A change by any other creature (a summon, a monster) is zeroed too; one without an attacker passes.')
        if boss == 'The Lily of Night':
            build.entry(item, ASURAS, [39], 'mapped', first + '/conditions/0',
                        'Storage Asuras.Fragrance holds the end of the fragrance (actions_fragrance.lua: now + 10 minutes); the '
                        'quest domain publishes whether it is still active, read-only (D27).')

    # Goshnar's Greed: each greed creature returns 10 s after its death; greedbeast deaths feed the boss's immunity.
    item = build.get('goshnars_greed', "Soul War: Goshnar's Greed", 'instance_per_party')
    positions = {'Greedbeast': (33744, 31666), 'Soulsnatcher': (33747, 31668), 'Weak Soul': (33750, 31666),
                 'Strong Soul': (33750, 31666), 'Powerful Soul': (33750, 31666)}
    item['encounter']['state']['counters'].append({'name': 'greedbeast_kills', 'initial': 0})
    build.entry(item, SOUL_WAR_LIB, [885], 'mapped', '/encounter/state/counters/0',
                "GreedbeastKills starts at 0; goshnars_greed.lua reads and resets it (transcribed with Goshnar's Greed).")
    for name, (x, y) in positions.items():
        role = slug(name)
        anchor = f'{role}_return'
        item['encounter']['anchors'].append({'key': anchor, 'kind': 'point', 'description': f'Canary ({x}, {y}, 14).'})
        build.participant(item, role, name, 'GreedMonsterDeath')
        if name == 'Greedbeast':
            path = build.rule(item, {'key': 'greedbeast_killed', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                     'actions': [{'kind': 'counter', 'counter': 'greedbeast_kills', 'operation': 'add', 'value': 1}]})
            build.entry(item, SOUL_WAR_MECHANICS, [196, 198, 200, 201, 202, 205, 207], 'mapped', path,
                        'onDeath of a greedbeast raises GreedbeastKills.')
        path = build.rule(item, {'key': f'{role}_returns', 'trigger': {'kind': 'creature_died', 'role': role}, 'delay_ms': 10000,
                                 'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature(name), 'role': role, 'count': 1,
                                                                'at': {'anchor': anchor}, 'owner': 'none', 'health': 'full'}]})
        build.entry(item, SOUL_WAR_MECHANICS, [196, 198, 199, 204, 205, 207], 'mapped', path,
                    f'onDeath: CreateGoshnarsGreedMonster brings a new {name.lower()} to its fixed tile 10 s later.')
        build.entry(item, SOUL_WAR_LIB, [905, 906, 907, 908, 909, 910, 911, 913, 918, 919, 920, 921, 927, 928], 'mapped', path + '/actions/0',
                    f'GreedMonsters places the {name.lower()} on ({x}, {y}, 14); createMonster(name, position, true, false) '
                    'is extended, not forced.')
    build.entry(item, SOUL_WAR_LIB, [914, 915, 916, 923, 924, 925], 'approved_omission', None,
                'The teleport effects at 7, 8 and 9 s are cosmetic.')



def heart_minion_forms(build):
    """DisruptionTransform, ChargedDisruptionTransform, CracklerTransform and DepolarizedTransform: minions that change form
    after a number of thinks, or while the crackler room is polarized."""
    item = build.items['world_devourer']
    for minion, following, event, script, seconds, lines in (
            ('Disruption', 'Charged Disruption', 'DisruptionTransform', 'creaturescripts_disruption_transform.lua', 12,
             [1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 30]),
            ('Charged Disruption', 'Overcharged Disruption', 'ChargedDisruptionTransform',
             'creaturescripts_charged_disruption_transform.lua', 18,
             [1, 2, 4, 5, 6, 7, 9, 10, 12, 13, 14, 15, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 30])):
        role = slug(minion)
        build.participant(item, role, minion, event)
        build.define(item, creature(following))
        path = build.rule(item, {'key': f'{role}_charges', 'trigger': {'kind': 'creature_spawned', 'role': role},
                                 'delay_ms': {'min': seconds * 1000, 'max': seconds * 1000 + 1000}, 'conditions': [],
                                 'actions': [{'kind': 'transform', 'role': role, 'into': creature(following), 'health': 'full'}]})
        build.entry(item, HEART + script, lines, 'mapped', path,
                    f'onThink counts the thinks of each {minion.lower()} (one per second); the first think records 1 and at '
                    f'{seconds} a fresh {following.lower()} replaces it on its tile. The first think falls within the first second, '
                    f'so the change comes {seconds}-{seconds + 1} s after the {minion.lower()} appears.')

    # Cracklers: the room's polarization is read on every think.
    item = build.get('heart_cracklers', 'Heart of Destruction: crackler room', 'instance_per_party')
    state = item['encounter']['state']
    state['flags'].append({'name': 'room_polarized', 'initial': False})
    state['timers'].append({'name': 'crackler_think', 'duration_ms': 1000, 'repeat': True})
    lever = HEART + 'actions_cracklers_lever.lua'
    build.entry(item, lever, [125], 'mapped', '/encounter/state/flags/0',
                'The lever and every new vortex pattern clear cracklerTransform. movements_vortex_crackler.lua sets it while '
                'players hold all four vortex tiles of the current pattern and clears it when one steps off; that stepped-on '
                'mechanic is transcribed with the room (D31 adds no stepped-on trigger yet).')
    path = build.rule(item, {'key': 'crackler_think_starts', 'trigger': {'kind': 'encounter_started'}, 'conditions': [],
                             'actions': [{'kind': 'timer', 'timer': 'crackler_think', 'operation': 'start'}]})
    build.entry(item, lever, [125], 'mapped', path, 'Creature thinks run once per second for the whole fight.')
    for minion, following, event, script, value, lines in (
            ('Crackler', 'Depolarized Crackler', 'CracklerTransform', 'creaturescripts_crackler_transform.lua', True,
             [1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 13, 14, 16]),
            ('Depolarized Crackler', 'Crackler', 'DepolarizedTransform', 'creaturescripts_depolarized_transform.lua', False,
             [1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13, 15])):
        role = slug(minion)
        build.participant(item, role, minion, event)
        build.define(item, creature(following))
        path = build.rule(item, {'key': f'{role}_turns', 'trigger': {'kind': 'timer_elapsed', 'timer': 'crackler_think'},
                                 'conditions': [{'kind': 'flag', 'flag': 'room_polarized', 'value': value}],
                                 'actions': [{'kind': 'transform', 'role': role, 'into': creature(following), 'health': 'keep_absolute'}]})
        build.entry(item, HEART + script, lines, 'mapped', path,
                    f'onThink: while cracklerTransform is {str(value).lower()}, every {minion.lower()} is replaced on its tile by a '
                    f'{following.lower()} with the same health.')


def replica_servants(build):
    """ReplicaServantDeath: each golden or diamond servant replica counts for the world and for its top damage dealer."""
    path_ = FORGOTTEN + 'creaturescripts_replica_servants.lua'
    item = build.get('replica_servants', 'Forgotten Knowledge: servant replicas', 'channel_shared')
    for servant, storage, counter, config in (('Golden Servant Replica', 'GoldenServant', 'GoldenServantCounter', [2, 3, 4, 5]),
                                               ('Diamond Servant Replica', 'DiamondServant', 'DiamondServantCounter', [6, 7, 8, 9])):
        role = slug(servant)
        outcome = f'{role}_defeated'
        build.participant(item, role, servant, 'ReplicaServantDeath')
        item['encounter']['outcomes'].append(outcome)
        path = build.rule(item, {'key': f'{role}_death', 'trigger': {'kind': 'creature_died', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'emit_outcome', 'outcome': outcome, 'credited': 'damage_contributors'}]})
        build.entry(item, path_, [1, 10, 11, 12, 13, 14, 15, 16, 37, 38, 40] + config, 'mapped', path + '/trigger',
                    f'onDeath of a {servant.lower()}.')
        build.entry(item, path_, [18, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36], 'mapped',
                    path + '/actions/0',
                    f'The quest domain raises the world count ForgottenKnowledge.{storage}, opens the teleport (item 10840, action '
                    '26665) on (32815, 32870, 13) once both world counts reach 5, and raises '
                    f'ForgottenKnowledge.{counter} of the top damage dealer (D27).')
        item['manifest']['outcome_evidence'].append({
            'outcome': outcome, 'credited': 'damage_contributors',
            'quest_domain': {'world_counter': f'Storage.Quest.U11_02.ForgottenKnowledge.{storage}',
                             'player_counter': f'Storage.Quest.U11_02.ForgottenKnowledge.{counter}',
                             'player': 'the top damage contributor (Canary mostDamageKiller)',
                             'teleport': 'item 10840 with action id 26665 on Canary (32815, 32870, 13) once GoldenServant and '
                                         'DiamondServant both reach 5'}})
    build.entry(item, path_, [17, 19], 'approved_omission', None,
                'The guard compares the storage key, not its value, with 0, so it never runs; the world count starts at 0 in the '
                'quest domain.')



def small_boss_events(build):
    """AstralGlyphDeath, DragonEssenceDeath, DisgustingOozeDeath and FeroxaTransform."""
    glyph_death = 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_astral_glyph_death.lua'
    keeper = build.items['the_last_lore_keeper']
    build.participant(keeper, 'astral_glyph', 'An Astral Glyph', 'AstralGlyphDeath')
    path = build.rule(keeper, {'key': 'astral_glyph_death', 'trigger': {'kind': 'creature_died', 'role': 'astral_glyph'}, 'conditions': [],
                               'actions': [{'kind': 'spawn', 'creature': creature('The Last Lore Keeper'), 'role': 'the_last_lore_keeper',
                                            'count': 1, 'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
    build.entry(keeper, glyph_death, [1, 2], 'mapped', path + '/trigger', 'onDeath of an astral glyph.')
    build.entry(keeper, glyph_death, [3], 'mapped', path + '/actions/0',
                'Game.createMonster("the last lore keeper", death position, true, true): full health, no owner.')

    essence_death = 'data-otservbr-global/scripts/quests/the_first_dragon/creaturescripts_death_dragon_essence.lua'
    item = build.get('the_first_dragon', 'The First Dragon', 'instance_per_party')
    build.participant(item, 'dragon_essence', 'Dragon Essence', 'DragonEssenceDeath')
    item['encounter']['anchors'].append({'key': 'lair_centre', 'kind': 'point', 'description': 'Canary (33617, 31023, 14).'})
    build.define(item, creature('The First Dragon'))
    text = 'BEWARE! THE FIRST DRAGON APROACHES!'
    path = build.rule(item, {'key': 'dragon_essence_death', 'trigger': {'kind': 'creature_died', 'role': 'dragon_essence'}, 'conditions': [],
                             'actions': [{'kind': 'remove', 'role': 'dragon_essence'},
                                         {'kind': 'spawn', 'creature': creature('The First Dragon'), 'role': 'the_first_dragon',
                                          'count': 1, 'at': {'anchor': 'lair_centre'}, 'owner': 'none', 'health': 'full'},
                                         {'kind': 'say', 'subject': {'role': 'dragon_essence'}, 'text': text, 'mode': 'say'}]})
    build.entry(item, essence_death, [1, 3], 'mapped', path + '/trigger', 'onDeath of a dragon essence.')
    build.entry(item, essence_death, [4, 5, 6, 7, 8, 9, 10], 'mapped', path + '/actions/0',
                'Every other dragon essence within 14 tiles of (33617, 31023, 14), the whole lair, is removed.')
    build.entry(item, essence_death, [11], 'mapped', path + '/actions/1',
                'Game.createMonster("The First Dragon", (33617, 31023, 14), true, true): full health, no owner.')
    build.entry(item, essence_death, [12], 'mapped', path + '/actions/2',
                'The dying essence says the warning (Canary places the text on the lair centre).')

    ooze_death = 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_disgusting_ooze_death.lua'
    item = build.items['plagirath']
    build.participant(item, 'disgusting_ooze', 'Disgusting Ooze', 'DisgustingOozeDeath')
    ooze = creature('Disgusting Ooze')
    path = build.rule(item, {'key': 'disgusting_ooze_splits', 'trigger': {'kind': 'creature_died', 'role': 'disgusting_ooze'},
                             'conditions': [{'kind': 'chance_percent', 'value': 10}],
                             'actions': [{'kind': 'spawn', 'creature': ooze, 'role': 'disgusting_ooze', 'count': 2, 'at': 'death_position',
                                          'owner': 'death_master', 'health': 'full'},
                                         {'kind': 'say', 'subject': {'role': 'disgusting_ooze'}, 'text': 'The ooze splits and regenerates.',
                                          'mode': 'say'}]})
    build.entry(item, ooze_death, [1, 2, 3, 4, 5], 'mapped', path + '/trigger',
                'onDeath of a disgusting ooze (Plagirath summons them, plagirath_summon.lua); summons run death events too.')
    build.entry(item, ooze_death, [7], 'mapped', path + '/conditions/0', 'math.random(20) < 3: 2 of 20 values, a 10% chance.')
    build.entry(item, ooze_death, [8, 9, 10, 11, 12, 13, 14], 'mapped', path + '/actions/0',
                'Two new oozes on the death position with the master of the dying ooze (setMaster(creature:getMaster())).')
    build.entry(item, ooze_death, [15], 'mapped', path + '/actions/1', 'creature:say(..., TALKTYPE_MONSTER_SAY).')

    feroxa_path = 'data-otservbr-global/scripts/quests/grimvale/creaturescripts_feroxa_transform.lua'
    item = build.get('feroxa', 'Grimvale: Feroxa', 'channel_shared')
    build.participant(item, 'feroxa', 'Feroxa', 'FeroxaTransform')
    build.participant(item, 'feroxa2', 'Feroxa2', 'FeroxaTransform')
    build.define(item, creature('Feroxa3'))
    build.define(item, creature('Feroxa4'))
    path = build.rule(item, {'key': 'feroxa_second_form', 'trigger': {'kind': 'health_crossed', 'role': 'feroxa', 'percent': 50},
                             'conditions': [], 'actions': [{'kind': 'transform', 'role': 'feroxa', 'into': creature('Feroxa2'),
                                                            'health': 'full'}]})
    build.entry(item, feroxa_path, [1, 2, 3, 4, 5, 6, 7, 12], 'mapped', path + '/trigger',
                'onThink of a creature named "Feroxa" with 100000 maximum health: at 50000 health or less, 50%.')
    build.entry(item, feroxa_path, [9, 10, 11], 'mapped', path + '/actions/0',
                'Feroxa is removed and Feroxa2 (displayed as "Feroxa", 50000 health) is created in its place with full health.')
    path = build.rule(item, {'key': 'feroxa_third_form', 'trigger': {'kind': 'health_crossed', 'role': 'feroxa2', 'percent': 50},
                             'conditions': [], 'actions': [{'kind': 'transform', 'role': 'feroxa2',
                                                            'into': {'random_of': [creature('Feroxa3'), creature('Feroxa4')]},
                                                            'health': 'full'}]})
    build.entry(item, feroxa_path, [13, 14, 23, 24], 'mapped', path + '/trigger',
                'Feroxa2 carries the display name "Feroxa" and 50000 maximum health, so this branch runs: at 25000 or less, 50%.')
    build.entry(item, feroxa_path, [16, 17, 18, 19, 20, 21, 22], 'mapped', path + '/actions/0',
                'Feroxa3 or Feroxa4, picked uniformly, replaces it with full health.')
    build.entry(item, feroxa_path, [8, 15], 'approved_omission', None, 'The poff effect is cosmetic.')



URMAHLULLU = 'data-otservbr-global/scripts/quests/kilmaresh_quest/creaturescripts_urmahlullu_change.lua'
URMAHLULLU_STAGES = ['Urmahlullu the Immaculate', 'Wildness of Urmahlullu', 'Urmahlullu the Tamed', 'Wisdom of Urmahlullu',
                     'Urmahlullu the Weakened']


def urmahlullu(build):
    """UrmahlulluChanges: five forms killed one after another (the wiki decides the shape, D25)."""
    item = build.get('urmahlullu', 'Kilmaresh: Urmahlullu', 'instance_per_party')
    for index, form in enumerate(URMAHLULLU_STAGES[:-1]):
        role, following = slug(form), URMAHLULLU_STAGES[index + 1]
        build.participant(item, role, form, 'UrmahlulluChanges')
        build.define(item, creature(following))
        path = build.rule(item, {'key': f'{role}_changes', 'trigger': {'kind': 'lethal_damage', 'role': role}, 'conditions': [],
                                 'actions': [{'kind': 'prevent_death', 'role': role},
                                             {'kind': 'transform', 'role': role, 'into': creature(following), 'health': 'full'}]})
        build.entry(item, URMAHLULLU, [9, 11, 12, 13, 14, 15, 16, 17, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60,
                                       64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 86, 87, 90],
                    'mapped', path,
                    f'{form} becomes {following} with full health. Canary changes forms at absolute thresholds of one '
                    '512000-health scale (515000 * RATE_BOSS_HEALTH for the first, a typo against the 512000 monster files). '
                    'The reference-date wiki gives each form its own health (130,000, then 70,000, ...), no experience or loot '
                    'for the first four and describes the fight as killing the five forms one after another (Kilmaresh '
                    'Quest/Spoiler); with the wiki health adopted (D15) the forms change on the lethal hit (D25).')
    build.entry(item, URMAHLULLU, [1, 2, 19, 20, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41,
                                   42, 43, 44, 45, 61, 83, 84, 85, 86], 'approved_omission', None,
                'Canary reverts a form to the previous one when the next threshold is not reached within 60 s (its second '
                'revert reads the id of the removed creature and never runs). The reverted form comes back with full health on '
                'the shared 512000 scale; the wiki, whose per-form health replaces that scale (D15), describes five '
                'consecutive kills and no revert, so the revert is not reproduced (D25).')
    build.entry(item, URMAHLULLU, [4, 6, 7], 'approved_omission', None,
                'RATE_BOSS_HEALTH scales both the thresholds and the monster health on the Canary server; Oteryn has no such '
                'server rate.')



SPLINTERS = 'data-otservbr-global/monster/quests/soul_war/normal_monsters/megalomania_room/'


def megalomania_splinters(build):
    """mType.onSpawn of the Splinters of Madness: each stage grows into the next after 120 s (the wiki decides, D25)."""
    item = build.get('goshnars_megalomania', "Soul War: Goshnar's Megalomania", 'instance_per_party')
    stages = [('Lesser Splinter of Madness', 'Greater Splinter of Madness', 'lesser_splinter_of_madness.lua'),
              ('Greater Splinter of Madness', 'Mighty Splinter of Madness', 'greater_splinter_of_madness.lua')]
    for stage, following, filename in stages:
        role = slug(stage)
        build.participant(item, role, stage, 'mType.onSpawn')
        build.define(item, creature(following))
        path = build.rule(item, {'key': f'{role}_grows', 'trigger': {'kind': 'creature_spawned', 'role': role}, 'delay_ms': 120000,
                                 'conditions': [], 'actions': [{'kind': 'transform', 'role': role, 'into': creature(following),
                                                                'health': 'full'}]})
        build.entry(item, SPLINTERS + filename, [97, 98, 99, 100, 101, 102, 103, 104], 'mapped', path,
                    f'onSpawn schedules, 120 s later, setType("{following}", true) on the same creature if it still exists: '
                    'the next stage with its full health.')
    build.entry(item, SPLINTERS + 'greater_splinter_of_madness.lua', [97], 'mapped', '/encounter/rules/1/trigger',
                'Canary runs onSpawn only for a creature that is spawned, and setType does not run it again '
                '(monster_functions.cpp luaMonsterSetType), so a splinter that grew from a lesser one never becomes mighty. '
                'The reference-date wiki (Soul War Quest/Spoiler: lesser splinters "will become Greater Splinters of Madness '
                'and then Mighty Splinters of Madness") decides (D25): creature_spawned also fires for a creature transformed '
                'into the role.')
    build.participant(item, 'mighty_splinter_of_madness', 'Mighty Splinter of Madness')
    build.entry(item, SPLINTERS + 'mighty_splinter_of_madness.lua', list(range(97, 111)), 'unresolved_semantics', None,
                'After 120 s a mighty splinter is absorbed and Goshnar\'s Megalomania grows stronger (the wiki: "If a Mighty '
                'Splinter of Madness is not removed in due time, it will be absorbed"). Canary\'s callback calls say and '
                'remove on the undefined global `creature` and fails before increaseHatredDamageMultiplier(5); the boss '
                'strength change (its hatred multiplier) is outside the vocabulary.')

def world_boss_events(build):
    """Pythius, The First Dragon, the white deer, Dark Trails, The Shatterer, The Primal Menace, Gaz'haragoth, Tirecz and
    the Order of the Lion skirmish."""
    path_ = 'data-otservbr-global/scripts/quests/the_hidden_city_of_beregar/creaturescripts_pythius_the_rotten_kill.lua'
    item = build.get('pythius_the_rotten', 'The Hidden City of Beregar: Pythius the Rotten', 'instance_per_party')
    build.participant(item, 'pythius_the_rotten', 'Pythius the Rotten', 'PythiusTheRottenDeath')
    item['encounter']['outcomes'].append('pythius_the_rotten_defeated')
    text = 'NICE FIGHTING LITTLE WORM, YOUR VICTORY SHALL BE REWARDED!'
    path = build.rule(item, {'key': 'pythius_death', 'trigger': {'kind': 'creature_died', 'role': 'pythius_the_rotten'}, 'conditions': [],
                             'actions': [{'kind': 'say', 'subject': {'role': 'pythius_the_rotten'}, 'text': text, 'mode': 'say'},
                                         {'kind': 'emit_outcome', 'outcome': 'pythius_the_rotten_defeated', 'credited': 'damage_contributors'}]})
    build.entry(item, path_, [1, 2], 'mapped', path + '/trigger', 'onDeath of Pythius the Rotten.')
    build.entry(item, path_, [3], 'mapped', path + '/actions/0',
                'creature:say(..., TALKTYPE_MONSTER_SAY); Canary shows the text on (32572, 31405, 15), a display position only.')
    build.entry(item, path_, [5, 6, 7], 'mapped', path + '/actions/1',
                'The player with the most damage is teleported to the reward room (32577, 31403, 15): a reward room belongs to the '
                'reward domain (D27), which picks the top damage contributor. The teleport effect is cosmetic.')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'pythius_the_rotten_defeated', 'credited': 'damage_contributors',
        'reward_domain': {'recipient': 'the top damage contributor (Canary mostDamageKiller)',
                          'reward_room': 'Canary (32577, 31403, 15)'}})

    path_ = 'data-otservbr-global/scripts/quests/the_first_dragon/creaturescripts_death_first_dragon.lua'
    item = build.items['the_first_dragon']
    build.participant(item, 'the_first_dragon', 'The First Dragon', 'FirstDragonDeath')
    item['encounter']['anchors'] += [
        {'key': 'lair', 'kind': 'area', 'description': 'Every tile within 14 tiles of Canary (33617, 31023, 14) on that floor.'},
        {'key': 'lair_exit', 'kind': 'point', 'description': 'Canary (33617, 31020, 13).'}]
    item['encounter']['outcomes'].append('the_first_dragon_defeated')
    path = build.rule(item, {'key': 'the_first_dragon_death', 'trigger': {'kind': 'creature_died', 'role': 'the_first_dragon'},
                             'conditions': [],
                             'actions': [{'kind': 'emit_outcome', 'outcome': 'the_first_dragon_defeated', 'credited': 'players_in_anchor',
                                          'anchor': 'lair'},
                                         {'kind': 'teleport', 'who': {'players_in': 'lair'}, 'to': 'lair_exit'}]})
    build.entry(item, path_, [1, 21], 'mapped', path + '/trigger', 'onDeath of The First Dragon.')
    build.entry(item, path_, [22, 23, 24, 25, 28, 29, 30], 'mapped', path + '/actions/0',
                'Every player within 14 tiles of the lair centre gets TheFirstDragon.Feathers = 1 unless already set (D27 quest '
                'domain). The outcome is emitted before the teleport so the players are still in the lair.')
    build.entry(item, path_, [26, 27], 'mapped', path + '/actions/1',
                'Those players are teleported to (33617, 31020, 13); the teleport effect is cosmetic.')
    build.entry(item, path_, list(range(3, 20)), 'approved_omission', None, 'The destinations table is never read.')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'the_first_dragon_defeated', 'credited': 'players_in_anchor',
        'quest_domain': {'canary_storage': 'Storage.Quest.U11_02.TheFirstDragon.Feathers', 'raise_to_at_least': 1}})

    deer = 'data/scripts/creaturescripts/monster/white_deer_death.lua'
    scouts = 'data/scripts/creaturescripts/monster/white_deer_scouts.lua'
    item = build.get('white_deer', 'White Deer', 'channel_shared')
    build.participant(item, 'white_deer', 'White Deer', 'WhiteDeerDeath')
    build.participant(item, 'enraged_white_deer', 'Enraged White Deer', 'WhiteDeerScoutsDeath')
    for name in ('Desperate White Deer', 'Elf Scout'):
        build.define(item, creature(name))
    item['encounter']['state']['flags'].append({'name': 'deer_variant_chosen', 'initial': False})
    unmastered = {'kind': 'has_master', 'role': 'white_deer', 'value': False}
    first = build.rule(item, {'key': 'white_deer_roll', 'trigger': {'kind': 'creature_died', 'role': 'white_deer'},
                              'conditions': [unmastered], 'actions': [{'kind': 'flag', 'flag': 'deer_variant_chosen', 'value': False}]})
    enraged = build.rule(item, {
        'key': 'white_deer_turns_to_fight', 'trigger': {'kind': 'creature_died', 'role': 'white_deer'},
        'conditions': [unmastered, {'kind': 'chance_percent', 'value': 30}],
        'actions': [{'kind': 'flag', 'flag': 'deer_variant_chosen', 'value': True},
                    {'kind': 'spawn', 'creature': creature('Enraged White Deer'), 'role': 'enraged_white_deer', 'count': 1,
                     'at': 'death_position', 'owner': 'none', 'health': 'full'},
                    {'kind': 'say', 'subject': {'role': 'white_deer'}, 'text': 'The white deer summons all his strength and turns to fight!',
                     'mode': 'say'}]})
    desperate = build.rule(item, {
        'key': 'white_deer_escapes', 'trigger': {'kind': 'creature_died', 'role': 'white_deer'},
        'conditions': [unmastered, {'kind': 'flag', 'flag': 'deer_variant_chosen', 'value': False}],
        'actions': [{'kind': 'spawn', 'creature': creature('Desperate White Deer'), 'count': 1, 'at': 'death_position', 'owner': 'none',
                     'health': 'full'},
                    {'kind': 'say', 'subject': {'role': 'white_deer'}, 'text': 'The white deer desperately tries to escape!', 'mode': 'say'}]})
    build.entry(item, deer, [6, 8], 'mapped', first + '/trigger', 'onDeath of a white deer.')
    build.entry(item, deer, [9, 10, 11, 12], 'mapped', first + '/conditions/0', 'A summoned white deer has no effect.')
    build.entry(item, deer, [14, 15, 16, 22], 'mapped', first + '/actions/0',
                'One roll of math.random(100) walks the config in order and stops at the first match: 30% the first entry, '
                'otherwise the second. The flag, cleared first and set by the 30% rule, makes the two rules exclusive (rules run '
                'in listed order, §9.2).')
    build.entry(item, deer, [1, 2, 17, 20], 'mapped', enraged,
                'chance 30: an Enraged White Deer on the death position (extended, forced) and the message.')
    build.entry(item, deer, [3, 17, 20], 'mapped', desperate, 'Otherwise a Desperate White Deer and its message.')
    build.entry(item, deer, [18, 19], 'approved_omission', None,
                'The teleport effect is cosmetic; the message only follows a successful spawn, and a forced spawn does not fail.')
    path = build.rule(item, {
        'key': 'elves_avenge_the_deer', 'trigger': {'kind': 'creature_died', 'role': 'enraged_white_deer'},
        'conditions': [{'kind': 'has_master', 'role': 'enraged_white_deer', 'value': False}, {'kind': 'chance_percent', 'value': 10}],
        'actions': [{'kind': 'spawn', 'creature': creature('Elf Scout'), 'count': 2, 'at': 'death_position', 'owner': 'none', 'health': 'full'},
                    {'kind': 'say', 'subject': {'role': 'enraged_white_deer'},
                     'text': 'The elves came too late to save the deer, however they might avenge it.', 'mode': 'say'}]})
    build.entry(item, scouts, [1, 3], 'mapped', path + '/trigger', 'onDeath of an enraged white deer.')
    build.entry(item, scouts, [4, 5, 6, 7], 'mapped', path + '/conditions/0', 'A summoned deer has no effect.')
    build.entry(item, scouts, [9, 10], 'mapped', path + '/conditions/1', 'math.random(100) <= 10: a 10% chance.')
    build.entry(item, scouts, [11, 12, 13, 15, 16], 'mapped', path + '/actions/0', 'Two Elf Scouts on the death position (extended, forced).')
    build.entry(item, scouts, [14], 'approved_omission', None, 'The teleport effect is cosmetic.')
    build.entry(item, scouts, [18], 'mapped', path + '/actions/1', 'targetMonster:say(..., TALKTYPE_MONSTER_SAY).')

    path_ = 'data-otservbr-global/scripts/quests/dark_trails/creaturescripts_kill_the_ravager.lua'
    item = build.get('the_ravager', 'Dark Trails: The Ravager', 'instance_per_party')
    build.participant(item, 'the_ravager', 'The Ravager', 'TheRavagerDeath')
    item['encounter']['anchors'] += [{'key': 'exit_teleporter', 'kind': 'point', 'description': 'Canary (33496, 32070, 8).'},
                                     {'key': 'ravager_exit', 'kind': 'point', 'description': 'Canary (33459, 32083, 8).'}]
    teleporter = ref('Item', 'canary:item/1949')
    build.define(item, teleporter)
    item['encounter']['outcomes'].append('the_ravager_defeated')
    path = build.rule(item, {'key': 'the_ravager_death', 'trigger': {'kind': 'creature_died', 'role': 'the_ravager'}, 'conditions': [],
                             'actions': [{'kind': 'map_item', 'operation': 'create', 'item': teleporter, 'anchor': 'exit_teleporter',
                                          'destination': 'ravager_exit', 'revert_after_ms': 5 * 60 * 1000},
                                         {'kind': 'emit_outcome', 'outcome': 'the_ravager_defeated', 'credited': 'damage_contributors'}]})
    build.entry(item, path_, [9, 10], 'mapped', path + '/trigger', 'onDeath of The Ravager.')
    build.entry(item, path_, [13, 14, 15, 16, 22, 1, 2, 3, 4, 6], 'mapped', path + '/actions/0',
                'A teleporter 1949 on (33496, 32070, 8) leading to (33459, 32083, 8), removed after 5 minutes.')
    build.entry(item, path_, [17, 18, 19, 20, 21], 'mapped', path + '/actions/1',
                'Every damaging player gets DarkTrails.Mission11 = 1 unless already set (D27 quest domain).')
    build.entry(item, path_, [5, 11, 12], 'approved_omission', None, 'Magic effects are cosmetic.')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'the_ravager_defeated', 'credited': 'damage_contributors',
        'quest_domain': {'canary_storage': 'Storage.Quest.U10_50.DarkTrails.Mission11', 'raise_to_at_least': 1}})

    path_ = 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_the_shatterer_kill.lua'
    item = build.get('the_shatterer', 'Ferumbras Ascension: The Shatterer', 'instance_per_party')
    build.participant(item, 'the_shatterer', 'The Shatterer', 'TheShattererDeath')
    item['encounter']['anchors'] += [{'key': 'exit_teleporter', 'kind': 'point', 'description': 'Canary (33393, 32438, 14).'},
                                     {'key': 'shatterer_exit', 'kind': 'point', 'description': 'Canary (33436, 32443, 15).'}]
    closed, opened = ref('Item', 'canary:item/1949'), ref('Item', 'canary:item/22761')
    build.define(item, closed)
    build.define(item, opened)
    item['encounter']['outcomes'].append('the_shatterer_defeated')
    path = build.rule(item, {'key': 'the_shatterer_death', 'trigger': {'kind': 'creature_died', 'role': 'the_shatterer'}, 'conditions': [],
                             'actions': [{'kind': 'emit_outcome', 'outcome': 'the_shatterer_defeated', 'credited': 'damage_contributors'},
                                         {'kind': 'map_item', 'operation': 'transform', 'item': closed, 'into': opened,
                                          'anchor': 'exit_teleporter', 'destination': 'shatterer_exit', 'revert_after_ms': 2 * 60 * 1000}]})
    build.entry(item, path_, [85, 86], 'mapped', path + '/trigger', 'onDeath of The Shatterer.')
    build.entry(item, path_, [87, 88, 89], 'mapped', path + '/actions/0',
                'Every damaging player gets FerumbrasAscension.TheShatterer = 1 (D27 quest domain).')
    build.entry(item, path_, [82, 83, 91, 92, 93, 94, 95, 96, 97, 99, 100, 1, 2, 3, 4, 5, 6, 7], 'mapped', path + '/actions/1',
                'Teleporter 1949 on (33393, 32438, 14) becomes 22761 leading to (33436, 32443, 15); after 2 minutes it turns back '
                'into 1949 with its original destination.')
    build.entry(item, path_, [98], 'approved_omission', None, 'The thunder effect on the boss position is cosmetic.')
    build.entry(item, path_, [101] + list(range(9, 81)), 'approved_omission', None,
                'revert() puts back the chains of the four corridors and resets the four levers for the next group '
                '(actions_the_shatterer_levers.lua removes them); an instance per party starts from the map state (D26).')
    build.entry(item, path_, [103, 104], 'approved_omission', None,
                'Global TheShattererLever and TheShattererTimer reset the shared room; an instance per party replaces them (D26).')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'the_shatterer_defeated', 'credited': 'damage_contributors',
        'quest_domain': {'canary_storage': 'Storage.Quest.U10_90.FerumbrasAscension.TheShatterer', 'set': 1}})

    path_ = 'data-otservbr-global/scripts/quests/the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua'
    item = build.get('the_primal_menace', 'The Primal Ordeal: The Primal Menace', 'instance_per_party')
    build.participant(item, 'the_primal_menace', 'The Primal Menace', 'ThePrimalMenaceDeath')
    item['encounter']['outcomes'].append('the_primal_menace_defeated')
    path = build.rule(item, {'key': 'the_primal_menace_death', 'trigger': {'kind': 'creature_died', 'role': 'the_primal_menace'},
                             'conditions': [], 'actions': [{'kind': 'emit_outcome', 'outcome': 'the_primal_menace_defeated',
                                                            'credited': 'damage_contributors'}]})
    build.entry(item, path_, [1, 3], 'mapped', path + '/trigger', 'onDeath of The Primal Menace.')
    build.entry(item, path_, [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16], 'mapped', path + '/actions/0',
                'Every damaging player gets PrimalOrdeal.Bosses.ThePrimalMenaceKilled = 1 (quest domain) and, when the hazard '
                'points of the fight equal the player\'s highest Gnomprona Gardens hazard level, that level rises by one '
                '(hazard progression domain, D27).')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'the_primal_menace_defeated', 'credited': 'damage_contributors',
        'quest_domain': {'canary_storage': 'Storage.Quest.U12_90.PrimalOrdeal.Bosses.ThePrimalMenaceKilled', 'raise_to_at_least': 1},
        'hazard_domain': {'hazard': 'hazard.gnomprona-gardens',
                          'level_up_when': 'the fight hazard points (Hazard:getHazardPlayerAndPoints of the damage map) equal the '
                                           'player maximum hazard level'}})

    heal = 'data-otservbr-global/scripts/creaturescripts/monster/gaz_haragoth_heal.lua'
    vortex_path = 'data-otservbr-global/scripts/creaturescripts/monster/minion_gaz_haragoth_vortex.lua'
    item = build.get('gaz_haragoth', "Roshamuul: Gaz'haragoth", 'channel_shared')
    build.participant(item, 'gaz_haragoth', "Gaz'haragoth", 'GazHaragothHeal')
    build.participant(item, 'minion_of_gaz_haragoth', "Minion of Gaz'haragoth", 'MinionGazDeath')
    item['encounter']['state']['flags'].append({'name': 'heal_pending', 'initial': False})
    item['encounter']['state']['timers'].append({'name': 'nightmare_heal', 'duration_ms': 7000, 'repeat': False})
    path = build.rule(item, {
        'key': 'gaz_haragoth_draws_on_nightmares', 'trigger': {'kind': 'health_crossed', 'role': 'gaz_haragoth', 'percent': 12.5},
        'conditions': [{'kind': 'flag', 'flag': 'heal_pending', 'value': False}],
        'actions': [{'kind': 'flag', 'flag': 'heal_pending', 'value': True},
                    {'kind': 'say', 'subject': {'role': 'gaz_haragoth'}, 'text': "Gaz'haragoth begins to draw on the nightmares to HEAL himself!",
                     'mode': 'yell'},
                    {'kind': 'timer', 'timer': 'nightmare_heal', 'operation': 'start'}]})
    healed = build.rule(item, {
        'key': 'gaz_haragoth_heals', 'trigger': {'kind': 'timer_elapsed', 'timer': 'nightmare_heal'}, 'conditions': [],
        'actions': [{'kind': 'heal', 'subject': {'role': 'gaz_haragoth'}, 'amount': 300000},
                    {'kind': 'say', 'subject': {'role': 'gaz_haragoth'}, 'text': "Gaz'haragoth HEALS himself!", 'mode': 'yell'},
                    {'kind': 'flag', 'flag': 'heal_pending', 'value': False}]})
    build.entry(item, heal, [7, 8, 9, 10], 'mapped', path + '/trigger',
                'onThink: below 12.5% health (Canary compares strictly) while the marker condition is absent.')
    build.entry(item, heal, [1, 2, 3, 4, 5, 10, 11], 'mapped', path + '/conditions/0',
                'The regeneration condition (subid 88888, 7 s) is only a marker: CONDITION_PARAM_HEALTHGAIN 0.01 is read as the '
                'integer 0 (ConditionRegeneration::setParam takes int32_t), so it heals nothing. The flag, cleared when the heal '
                'lands 7 s later, plays that marker.')
    build.entry(item, heal, [12], 'mapped', path + '/actions/1', 'creature:say(..., TALKTYPE_MONSTER_YELL).')
    build.entry(item, heal, [13, 14, 15, 16, 17, 21, 22], 'mapped', path + '/actions/2', 'addEvent(..., 7000): the heal follows 7 s later.')
    build.entry(item, heal, [18], 'mapped', healed + '/actions/0', 'creature:addHealth(300000).')
    build.entry(item, heal, [19], 'approved_omission', None, 'The magic effect is cosmetic.')
    build.entry(item, heal, [20], 'mapped', healed + '/actions/1', 'creature:say(..., TALKTYPE_MONSTER_YELL).')
    vortex = ref('Item', 'canary:item/20121')
    build.define(item, vortex)
    path = build.rule(item, {'key': 'minion_leaves_a_vortex', 'trigger': {'kind': 'creature_died', 'role': 'minion_of_gaz_haragoth'},
                             'conditions': [], 'actions': [{'kind': 'map_item', 'operation': 'create', 'item': vortex, 'at': 'death_position',
                                                            'revert_after_ms': 60000, 'interaction': 'canary:interaction/33542'}]})
    build.entry(item, vortex_path, [8, 10, 11, 12, 13, 14], 'mapped', path + '/trigger', "onDeath of a minion of Gaz'haragoth.")
    build.entry(item, vortex_path, [16, 17, 18, 19, 20, 22, 1, 2, 3, 4, 5, 6], 'mapped', path + '/actions/0',
                'Vortex 20121 with action id 33542 on the death position, removed after one minute. Action id 33542 is '
                'movements/roshamuul/strange_vortex_tp.lua: a player stepping in is teleported into the nightmare '
                '(33542, 32421, 15) (interaction domain).')

    path_ = 'data-otservbr-global/scripts/quests/the_new_frontier/creaturescripts_tirecz_kill.lua'
    item = build.get('tirecz', 'The New Frontier: Tirecz', 'instance_per_party')
    build.participant(item, 'tirecz', 'Tirecz', 'TireczDeath')
    item['encounter']['anchors'] += [
        {'key': 'arena', 'kind': 'area', 'description': 'Every tile within 10 tiles of Canary (33063, 31034, 3) on that floor.'},
        {'key': 'arena_exit', 'kind': 'point', 'description': 'Canary (33062, 31029, 7).'}]
    item['encounter']['outcomes'].append('tirecz_defeated')
    path = build.rule(item, {'key': 'tirecz_death', 'trigger': {'kind': 'creature_died', 'role': 'tirecz'}, 'conditions': [],
                             'actions': [{'kind': 'emit_outcome', 'outcome': 'tirecz_defeated', 'credited': 'players_in_anchor', 'anchor': 'arena'},
                                         {'kind': 'teleport', 'who': {'players_in': 'arena'}, 'to': 'arena_exit'},
                                         {'kind': 'remove', 'all_in': 'arena'}]})
    build.entry(item, path_, [17, 19], 'mapped', path + '/trigger', 'onDeath of Tirecz.')
    build.entry(item, path_, [2, 20, 21, 22, 25, 26, 27, 28, 29, 30], 'mapped', path + '/actions/0',
                'Every player in the arena gets the achievement "Champion of Chazorai" and, at TheNewFrontier.Questline 25, '
                'Mission09 = 2 and Questline = 26; each player shows the reward text. Achievement, quest and reward text belong '
                'to their domains (D27).')
    build.entry(item, path_, [1, 23], 'mapped', path + '/actions/1', 'Those players are teleported to (33062, 31029, 7).')
    build.entry(item, path_, [24], 'approved_omission', None, 'The teleport effect is cosmetic.')
    build.entry(item, path_, [34, 4, 5, 6, 7, 11, 12, 13, 14, 15], 'mapped', path + '/actions/2',
                'clearArena removes every creature left in the arena.')
    build.entry(item, path_, [8, 9, 10], 'approved_omission', None,
                'The player branch of clearArena never runs: the players were already teleported to floor 7 (its effect constant '
                'COsNST_ME_TELEPORT is also misspelt).')
    build.entry(item, path_, [33], 'approved_omission', None,
                'The global Mission09 storage frees the shared arena; an instance per party replaces it (D26).')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'tirecz_defeated', 'credited': 'players_in_anchor',
        'achievement_domain': {'achievement': 'Champion of Chazorai'},
        'quest_domain': {'canary_storage': 'Storage.Quest.U8_54.TheNewFrontier.Questline', 'when': 25, 'set': 26,
                         'also': {'Storage.Quest.U8_54.TheNewFrontier.Mission09[1]': 2}},
        'message': 'You have won! As new champion take the ancient armor as reward before you leave.'})

    lion = 'data-otservbr-global/scripts/quests/the_order_of_lion/creatureevent-commander_kills.lua'
    lever = 'data-otservbr-global/scripts/quests/the_order_of_lion/action-drume.lua'
    item = build.get('drume', 'The Order of the Lion: Drume', 'instance_per_party')
    build.participant(item, 'usurper_commander', 'Usurper Commander', 'UsurperCommanderDeath')
    build.participant(item, 'kesar', 'Kesar', 'KesarImmortal')
    build.define(item, creature('Drume'))
    item['encounter']['anchors'] += [{'key': 'kesar_spawn', 'kind': 'point', 'description': 'Canary (32444, 32515, 7).'},
                                     {'key': 'drume_spawn', 'kind': 'point', 'description': 'Canary (32444, 32516, 7).'}]
    item['encounter']['state']['counters'].append({'name': 'usurper_commanders', 'initial': 3})
    build.entry(item, lever, [7, 8, 9, 10, 11, 83, 94, 95, 96, 97, 98, 99, 100, 101, 102, 115], 'mapped', '/encounter/state/counters',
                'The lever spawns three usurper commanders and stores their number in a global storage; the counter of the '
                'party instance starts at three (D26).')
    path = build.rule(item, {'key': 'usurper_commander_falls', 'trigger': {'kind': 'lethal_damage', 'role': 'usurper_commander'},
                             'conditions': [{'kind': 'counter_compare', 'counter': 'usurper_commanders', 'op': '>', 'value': 0}],
                             'actions': [{'kind': 'counter', 'counter': 'usurper_commanders', 'operation': 'add', 'value': -1}]})
    build.entry(item, lion, [30, 31, 40], 'mapped', path + '/trigger', 'onPrepareDeath of a usurper commander; the death is not prevented.')
    build.entry(item, lion, [32, 33, 34], 'mapped', path + '/actions/0', 'The count drops by one while above zero.')
    path = build.rule(item, {'key': 'kesar_and_drume_appear', 'trigger': {'kind': 'counter_reached', 'counter': 'usurper_commanders', 'value': 0},
                             'conditions': [],
                             'actions': [{'kind': 'spawn', 'creature': creature('Kesar'), 'role': 'kesar', 'count': 1, 'at': {'anchor': 'kesar_spawn'},
                                          'owner': 'none', 'health': 'full'},
                                         {'kind': 'spawn', 'creature': creature('Drume'), 'count': 1, 'at': {'anchor': 'drume_spawn'},
                                          'owner': 'none', 'health': 'full'}]})
    build.entry(item, lion, [35, 36, 37, 38], 'mapped', path,
                'When the last one falls (the count was 1), Kesar and Drume are created (forced) on their tiles.')
    path = build.rule(item, {'key': 'kesar_is_immortal', 'trigger': {'kind': 'creature_spawned', 'role': 'kesar'}, 'conditions': [],
                             'actions': [{'kind': 'damage_modifier', 'role': 'kesar', 'multiplier_percent': 0, 'sources': 'any', 'until': 'reset'}]})
    build.entry(item, lion, [45, 46, 47, 48], 'mapped', path,
                'onHealthChange returns zero for every hit: Kesar takes no damage. Kesar has no healing spell.')

def quest_room_events(build):
    """Splash (Wine Cask), MakeshiftHomeDeath, OrganicMatterDeath, the Ferumbras soul splinter and essence, TentacleDeep."""
    path_ = 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_splash.lua'
    item = build.items['ravenous_hunger']
    build.participant(item, 'wine_cask', 'Wine Cask', 'Splash')
    build.define(item, creature('Liquor Spirit'))
    item['encounter']['anchors'].append({'key': 'liquor_spirit_spawn', 'kind': 'point', 'description': 'Canary (33160, 31945, 15).'})
    path = build.rule(item, {'key': 'wine_cask_splashes', 'trigger': {'kind': 'health_crossed', 'role': 'wine_cask', 'percent': 85},
                             'delay_ms': 100, 'conditions': [],
                             'actions': [{'kind': 'spawn', 'creature': creature('Liquor Spirit'), 'count': 1,
                                          'at': {'anchor': 'liquor_spirit_spawn'}, 'owner': 'none', 'health': 'full'},
                                         {'kind': 'say', 'subject': {'role': 'wine_cask'}, 'text': 'SPLASH!', 'mode': 'yell'},
                                         {'kind': 'heal', 'subject': {'role': 'wine_cask'}, 'amount': 'full'}]})
    build.entry(item, path_, [1, 3, 4, 5], 'mapped', path + '/trigger', 'onThink: below 85% health (Canary compares strictly).')
    build.entry(item, path_, [6, 14, 15], 'mapped', path + '/delay_ms', 'addEvent(..., 100): the effect follows 100 ms later.')
    build.entry(item, path_, [7, 11], 'mapped', path + '/actions/0', 'A liquor spirit on (33160, 31945, 15), not forced.')
    build.entry(item, path_, [12], 'mapped', path + '/actions/1', 'creature:say("SPLASH!", TALKTYPE_MONSTER_YELL).')
    build.entry(item, path_, [13], 'mapped', path + '/actions/2', 'The cask heals to full, so it crosses 85% again only after new damage.')
    build.entry(item, path_, [8, 9, 10], 'approved_omission', None,
                'The closure tests the captured creature object, never nil; a delayed action on a creature that no longer exists '
                'does nothing (§9.2).')

    path_ = 'data-otservbr-global/scripts/quests/dangerous_depth/creaturescripts_lost_exile_task.lua'
    item = build.get('dangerous_depth_makeshift_homes', 'Dangerous Depth: makeshift homes', 'channel_shared')
    build.participant(item, 'makeshift_home', 'Makeshift Home', 'MakeshiftHomeDeath')
    trash = ref('Item', 'canary:item/398')
    build.define(item, trash)
    path = build.rule(item, {'key': 'makeshift_home_leaves_rubble', 'trigger': {'kind': 'creature_died', 'role': 'makeshift_home'},
                             'conditions': [], 'actions': [{'kind': 'map_item', 'operation': 'create', 'item': trash, 'at': 'death_position',
                                                            'interaction': 'canary:interaction/57233'}]})
    build.entry(item, path_, [68, 70], 'mapped', path + '/trigger', 'onDeath of a makeshift home.')
    build.entry(item, path_, [71, 72], 'mapped', path + '/actions/0',
                'Wooden trash 398 with action id 57233 on the death position. Action id 57233 is dangerous_depth/'
                'actions_wooden_trash.lua: searching it may start a prisoner escort (quest interaction).')

    path_ = 'data-otservbr-global/scripts/quests/dangerous_depth/creaturescripts_the_baron_from_below.lua'
    item = build.items['the_baron_from_below']
    build.participant(item, 'organic_matter', 'Organic Matter', 'OrganicMatterDeath')
    build.define(item, creature('Aggressive Matter'))
    path = build.rule(item, {'key': 'organic_matter_bursts', 'trigger': {'kind': 'creature_died', 'role': 'organic_matter'},
                             'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('Aggressive Matter'), 'count': 4,
                                                            'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
    build.entry(item, path_, [63, 64], 'mapped', path + '/trigger', 'onDeath of organic matter.')
    build.entry(item, path_, [65, 66, 67], 'mapped', path + '/actions/0', 'Four aggressive matters on the death position (extended).')

    path_ = 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_ferumbras_soul_splinter.lua'
    item = build.items['ferumbras_mortal_shell']
    build.participant(item, 'ferumbras_soul_splinter', 'Ferumbras Soul Splinter', 'FerumbrasSoulSplinterDeath')
    build.participant(item, 'ferumbras_essence', 'Ferumbras Essence', 'FerumbrasEssenceImmortal')
    path = build.rule(item, {'key': 'soul_splinter_death', 'trigger': {'kind': 'creature_died', 'role': 'ferumbras_soul_splinter'},
                             'conditions': [], 'actions': [{'kind': 'spawn', 'creature': creature('Ferumbras Essence'),
                                                            'role': 'ferumbras_essence', 'count': 1, 'at': 'death_position',
                                                            'owner': 'none', 'health': 'full'}]})
    build.entry(item, path_, [1, 3], 'mapped', path + '/trigger', 'onDeath of a Ferumbras soul splinter.')
    build.entry(item, path_, [4, 5, 6, 7, 8, 9], 'mapped', path + '/actions/0',
                'A Ferumbras essence on the death position (extended, forced); the failure log has no game effect.')
    path = build.rule(item, {'key': 'ferumbras_essence_is_immortal', 'trigger': {'kind': 'creature_spawned', 'role': 'ferumbras_essence'},
                             'conditions': [], 'actions': [{'kind': 'damage_modifier', 'role': 'ferumbras_essence', 'multiplier_percent': 0,
                                                            'sources': 'any', 'until': 'reset'}]})
    build.entry(item, path_, [14, 15, 16, 17], 'mapped', path,
                'onHealthChange returns zero for every hit: the essence takes no damage. It also zeroes the essence\'s own heal '
                '(15-25), which changes nothing on a creature that never loses health.')

    path_ = 'data-otservbr-global/scripts/quests/hero_of_rathleton/creaturescripts_tentacle.lua'
    item = build.items['deep_terror']
    build.participant(item, 'tentacle', 'Tentacle of the Deep Terror', 'TentacleDeep')
    item['encounter']['anchors'] += [
        {'key': 'tentacle_room', 'kind': 'area', 'description': 'Every tile within 13 tiles of Canary (33740, 31953, 14) on that floor.'},
        {'key': 'tentacle_spawns', 'kind': 'area', 'description': 'Canary x 33736-33746, y 31948-31957, z 14.'},
        {'key': 'deep_terror_spawn', 'kind': 'point', 'description': 'Canary (33741, 31953, 14).'}]
    item['encounter']['state']['flags'].append({'name': 'deep_terror_risen', 'initial': False})
    respawn = build.rule(item, {
        'key': 'tentacle_regrows', 'trigger': {'kind': 'creature_died', 'role': 'tentacle'}, 'delay_ms': 10000,
        'conditions': [{'kind': 'creature_present', 'role': 'tentacle', 'anchor': 'tentacle_room', 'present': True}],
        'actions': [{'kind': 'spawn', 'creature': creature('Tentacle of the Deep Terror'), 'role': 'tentacle', 'count': 1,
                     'at': {'random_in': 'tentacle_spawns'}, 'owner': 'none', 'health': 'full'}]})
    rise = build.rule(item, {
        'key': 'deep_terror_rises', 'trigger': {'kind': 'creature_died', 'role': 'tentacle'}, 'delay_ms': 5000,
        'conditions': [{'kind': 'creature_present', 'role': 'tentacle', 'anchor': 'tentacle_room', 'present': False},
                       {'kind': 'flag', 'flag': 'deep_terror_risen', 'value': False}],
        'actions': [{'kind': 'spawn', 'creature': creature('Deep Terror'), 'role': 'deep_terror', 'count': 1,
                     'at': {'anchor': 'deep_terror_spawn'}, 'owner': 'none', 'health': 'full'},
                    {'kind': 'flag', 'flag': 'deep_terror_risen', 'value': True}]})
    build.entry(item, path_, [20, 21, 22, 23, 24, 25], 'mapped', respawn + '/trigger', 'onDeath of a tentacle.')
    build.entry(item, path_, [26], 'mapped', respawn + '/delay_ms',
                'Canary checks 5 s after each death. The reference-date wiki (Hero of Rathleton Quest/Spoiler) gives a dead '
                'tentacle "a 10-second window before it spawns again with full health" unless all four die in it (D15, D25).')
    build.entry(item, path_, [1, 2, 4, 5, 6], 'mapped', respawn + '/conditions/0',
                'A living tentacle within 13 tiles of (33740, 31953, 14). Canary returns after the first spectator of the list, '
                'so the outcome depends on spectator order (a player listed first spawns the Deep Terror while tentacles live); '
                'the wiki decides: tentacles regrow while any live, the Deep Terror comes when all are dead (D25).')
    build.entry(item, path_, [3, 7, 8], 'mapped', respawn + '/actions/0',
                'A new tentacle, forced, on a random tile of x 33736-33746, y 31948-31957.')
    build.entry(item, path_, [9, 26], 'mapped', rise + '/delay_ms', 'The Deep Terror check runs 5 s after a death (the wiki gives no delay).')
    build.entry(item, path_, [10, 11, 12], 'mapped', rise + '/conditions/1',
                'GlobalStorage HeroRathleton.DeepRunning == 2 makes the Deep Terror appear once; an instance flag replaces the '
                'global (D26).')
    build.entry(item, path_, [13, 15], 'mapped', rise + '/actions/0', 'The Deep Terror, forced, on (33741, 31953, 14).')
    build.entry(item, path_, [14], 'mapped', rise + '/actions/1', 'DeepRunning = 2.')

def secret_library_knowledges(build):
    """lokathmorDeath, mazzinorDeath, mazzinorHealth: the knowledge drops check a name their registrant never has; the wiki
    decides (D25)."""
    path_ = 'data-otservbr-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_lokathmor.lua'
    item = build.get('lokathmor', 'The Secret Library: Lokathmor', 'instance_per_party')
    build.participant(item, 'lokathmor', 'Lokathmor', 'lokathmorDeath')
    build.participant(item, 'dark_knowledge', 'Dark Knowledge')
    parchment = ref('Item', 'canary:item/28488')
    build.define(item, parchment)
    path = build.rule(item, {'key': 'dark_knowledge_leaves_parchment', 'trigger': {'kind': 'creature_died', 'role': 'dark_knowledge'},
                             'conditions': [], 'actions': [{'kind': 'map_item', 'operation': 'create', 'item': parchment,
                                                            'at': 'death_position'}]})
    build.entry(item, path_, [3, 5, 6, 7, 10], 'approved_omission', None,
                'The event is registered only by Lokathmor, but its body acts only on a creature named "dark knowledge", so '
                'Lokathmor\'s death has no effect; Dark Knowledge (corpse 0) registers no event, so in Canary no parchment ever '
                'appears.')
    build.entry(item, path_, [1, 8, 9], 'mapped', path,
                'The reference-date wiki (Dark Knowledge: "When it dies, it becomes a Parchment of Dark Knowledge"; The Secret '
                'Library Quest/Spoiler: its parchment is used on the writing desk to free Lokathmor) decides (D25): parchment '
                '28488 (actions_parchment.lua) on the death position of a dark knowledge.')

    path_ = 'data-otservbr-global/scripts/quests/the_secret_library_quest/library_area/creaturescripts_mazzinor.lua'
    item = build.get('mazzinor', 'The Secret Library: Mazzinor', 'instance_per_party')
    build.participant(item, 'mazzinor', 'Mazzinor', 'mazzinorDeath')
    build.participant(item, 'mazzinor', 'Mazzinor', 'mazzinorHealth')
    build.participant(item, 'wild_knowledge', 'Wild Knowledge')
    vortex = ref('Item', 'canary:item/28673')
    build.define(item, vortex)
    path = build.rule(item, {'key': 'wild_knowledge_leaves_vortex', 'trigger': {'kind': 'creature_died', 'role': 'wild_knowledge'},
                             'conditions': [], 'actions': [{'kind': 'map_item', 'operation': 'create', 'item': vortex,
                                                            'at': 'death_position', 'revert_after_ms': 60000,
                                                            'interaction': 'canary:interaction/4951'}]})
    build.entry(item, path_, [4, 6, 7, 8, 19, 20, 22], 'approved_omission', None,
                'mazzinorDeath is registered only by Mazzinor, but its body acts only on a creature named "wild knowledge", so '
                'Mazzinor\'s death has no effect; Wild Knowledge (corpse 0) registers no event, so in Canary no vortex appears.')
    build.entry(item, path_, [1, 2, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18], 'mapped', path,
                'The reference-date wiki (The Secret Library Quest/Spoiler: "Once you kill them, they will become a blue vortex") '
                'decides (D25): vortex 28673 with action id 4951 on the death position of a wild knowledge, removed after one '
                'minute. Action id 4951 (movements_mazzinor.lua) gives the player stepping in a 30 s outfit condition and '
                'removes the vortex (interaction domain).')
    build.entry(item, path_, [24, 26, 27, 28, 29, 30, 31, 33], 'approved_omission', None,
                'mazzinorHealth re-applies the primary part of every change to Mazzinor through creature:addHealth without an '
                'attacker and zeroes the original (damage arrives negative, game.cpp combatChangeHealth): the secondary part of '
                'each hit is lost and no player is credited with any damage, so the reward boss would leave no rewards. The '
                'reference-date wiki describes an ordinary fight in which players attack Mazzinor directly and lists its loot, '
                'so the change is not reproduced (D25); Crystal (D30) has the same script.')

def d31_events(build):
    """Events that need the D31 vocabulary: player messages, removing only the triggering creature, weighted choices, heal
    triggers, party credit and circular areas."""
    lion = 'data-otservbr-global/scripts/quests/the_order_of_lion/creatureevent-commander_kills.lua'
    lever = 'data-otservbr-global/scripts/quests/the_order_of_lion/action-drume.lua'
    item = build.items['drume']
    build.participant(item, 'lion_commander', 'Lion Commander', 'LionCommanderDeath')
    item['encounter']['anchors'] += [
        {'key': 'skirmish', 'kind': 'area', 'description': 'Canary x 32417-32461, y 32507-32539, z 7 (22 and 16 tiles around (32439, 32523, 7)).'},
        {'key': 'skirmish_exit', 'kind': 'point', 'description': 'Canary (32453, 32503, 7).'}]
    item['encounter']['state']['counters'].append({'name': 'lion_commanders', 'initial': 3})
    build.entry(item, lever, [2, 3, 4, 5, 6, 82, 85, 86, 87, 88, 89, 90, 91, 92, 93, 114], 'mapped', '/encounter/state/counters',
                'The lever spawns three lion commanders and stores their number; the party instance counter starts at three (D26).')
    lost = build.rule(item, {
        'key': 'skirmish_lost', 'trigger': {'kind': 'lethal_damage', 'role': 'lion_commander'},
        'conditions': [{'kind': 'counter_compare', 'counter': 'lion_commanders', 'op': '<=', 'value': 1}],
        'actions': [{'kind': 'remove', 'all_in': 'skirmish', 'keep_summons': True},
                    {'kind': 'message', 'to': {'players_in': 'skirmish'}, 'text': 'You lost the skirmish.'},
                    {'kind': 'teleport', 'who': {'players_in': 'skirmish'}, 'to': 'skirmish_exit'}]})
    falls = build.rule(item, {
        'key': 'lion_commander_falls', 'trigger': {'kind': 'lethal_damage', 'role': 'lion_commander'},
        'conditions': [{'kind': 'counter_compare', 'counter': 'lion_commanders', 'op': '>', 'value': 1}],
        'actions': [{'kind': 'counter', 'counter': 'lion_commanders', 'operation': 'add', 'value': -1}]})
    build.entry(item, lion, [8, 9, 25, 26], 'mapped', falls + '/trigger', 'onPrepareDeath of a lion commander; the death is not prevented.')
    build.entry(item, lion, [10, 11, 12], 'mapped', falls, 'While more than one is left, the count drops by one.')
    build.entry(item, lion, [1, 2, 3, 4, 5, 6, 13, 14], 'mapped', lost + '/conditions/0',
                'The last one (count 1 or less) loses the skirmish. The rule stands before the count rule so both read the count '
                'before this death.')
    build.entry(item, lion, [15, 16, 17], 'mapped', lost + '/actions/0',
                'Every monster without a master in the room is removed; players\' summons stay.')
    build.entry(item, lion, [18, 19], 'mapped', lost + '/actions/1', 'MESSAGE_EVENT_ADVANCE to every player in the room.')
    build.entry(item, lion, [20, 21, 22], 'mapped', lost + '/actions/2', 'Those players are teleported to (32453, 32503, 7).')
    build.entry(item, lion, [23], 'approved_omission', None, 'The teleport effect is cosmetic.')

    evaporate = 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_evaporate.lua'
    spawn_boss = 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_spawn_boss.lua'
    item = build.items['ravenous_hunger']
    build.participant(item, 'liquor_spirit', 'Liquor Spirit', 'Evaporate')
    build.participant(item, 'leiden', 'Leiden')
    item['encounter']['state']['flags'].append({'name': 'leiden_evaporated', 'initial': False})
    near = lambda role: {'kind': 'creature_present', 'role': role, 'near': {'role': 'liquor_spirit', 'radius': 3, 'shape': 'circle'},
                         'present': True}
    trigger = {'kind': 'health_crossed', 'role': 'liquor_spirit', 'percent': 60}
    say = build.rule(item, {'key': 'liquor_spirit_evaporates', 'trigger': trigger, 'delay_ms': 100, 'conditions': [],
                            'actions': [{'kind': 'say', 'subject': {'role': 'liquor_spirit'}, 'text': 'The liquor spirit evaporates!',
                                         'mode': 'yell'}]})
    hurt = build.rule(item, {'key': 'evaporation_hurts_leiden', 'trigger': trigger, 'delay_ms': 100, 'conditions': [near('leiden')],
                             'actions': [{'kind': 'damage', 'subject': {'role': 'leiden'}, 'amount': {'min': 3000, 'max': 6000},
                                          'damage_type': 'none'},
                                         {'kind': 'flag', 'flag': 'leiden_evaporated', 'value': True}]})
    feed = build.rule(item, {'key': 'evaporation_feeds_ravenous_hunger', 'trigger': trigger, 'delay_ms': 100,
                             'conditions': [near('ravenous_hunger')],
                             'actions': [{'kind': 'heal', 'subject': {'role': 'ravenous_hunger'}, 'amount': {'min': 3000, 'max': 6000}}]})
    gone = build.rule(item, {'key': 'liquor_spirit_gone', 'trigger': trigger, 'delay_ms': 100, 'conditions': [],
                             'actions': [{'kind': 'remove', 'triggering': True}]})
    build.entry(item, evaporate, [31, 32, 33, 34], 'mapped', say + '/trigger', 'onThink: below 60% health (Canary compares strictly).')
    build.entry(item, evaporate, [35, 36, 37, 38, 39, 45, 46], 'mapped', say + '/delay_ms',
                'addEvent(..., 100) for the same creature; its first run removes the spirit, later ones find no creature.')
    build.entry(item, evaporate, [40], 'mapped', say + '/actions/0', 'creature:say(..., TALKTYPE_MONSTER_YELL).')
    build.entry(item, evaporate, [1, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 29, 41, 42], 'mapped', hurt + '/conditions/0',
                'The combat area around the spirit is Canary\'s radius-3 circle (every tile with dx^2 + dy^2 <= 10, the rows of '
                'lines 5-11); onTargetTile acts on the monster standing on each tile.')
    build.entry(item, evaporate, [18, 20], 'mapped', hurt + '/actions/0',
                'Leiden loses 3000-6000 health through creature:addHealth: a direct change without attacker or type, so no '
                'resistance applies (damage_type none, D31).')
    build.entry(item, evaporate, [19], 'mapped', hurt + '/actions/1',
                'registerEvent("SpawnBoss") arms Leiden\'s death rule below; the flag records it.')
    build.entry(item, evaporate, [21, 22], 'mapped', feed + '/actions/0', 'Ravenous Hunger in the area gains 3000-6000 health.')
    build.entry(item, evaporate, [23, 24, 25, 26, 27], 'approved_omission', None, 'Other tiles and creatures are left alone.')
    build.entry(item, evaporate, [2], 'approved_omission', None, 'The purple smoke effect is cosmetic; the combat has no damage formula.')
    build.entry(item, evaporate, [43, 44], 'mapped', gone + '/actions/0', 'creature:remove(): only this spirit disappears (D31).')
    path = build.rule(item, {'key': 'leiden_turns_into_ravenous_hunger', 'trigger': {'kind': 'creature_died', 'role': 'leiden'},
                             'conditions': [{'kind': 'flag', 'flag': 'leiden_evaporated', 'value': True}],
                             'actions': [{'kind': 'spawn', 'creature': creature('Ravenous Hunger'), 'role': 'ravenous_hunger', 'count': 1,
                                          'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
    build.entry(item, spawn_boss, [1, 2, 3, 4, 5, 7], 'mapped', path + '/trigger',
                'SpawnBoss is registered on Leiden by the evaporation only (creaturescripts_evaporate.lua line 19).')
    build.entry(item, spawn_boss, [8, 12, 13, 15], 'mapped', path + '/actions/0', 'Ravenous Hunger on Leiden\'s death position, not forced.')
    build.entry(item, spawn_boss, [9, 10, 11], 'approved_omission', None,
                'The Sinister Hermit branch belongs to movements_geyser.lua, which registers SpawnBoss on the hermit.')

    path_ = 'data-otservbr-global/scripts/creaturescripts/monster/the_welter_egg.lua'
    item = build.get('the_welter', 'Raid: The Welter', 'channel_shared')
    build.participant(item, 'egg', 'Egg', 'TheWelterEgg')
    build.define(item, creature('Spawn of the Welter'))
    path = build.rule(item, {'key': 'egg_hatches', 'trigger': {'kind': 'creature_spawned', 'role': 'egg'}, 'delay_ms': 10000, 'conditions': [],
                             'actions': [{'kind': 'spawn', 'creature': creature('Spawn of the Welter'), 'count': 1, 'at': 'subject_position',
                                          'owner': 'none', 'health': 'full'},
                                         {'kind': 'remove', 'triggering': True}]})
    build.entry(item, path_, [1, 2, 3, 16, 17], 'mapped', path + '/trigger',
                'onThink schedules the hatching 10 s after the egg\'s first think (monsters think every second, so within a second '
                'of appearing); later thinks schedule more, which find the egg gone.')
    build.entry(item, path_, [4, 5, 6, 7, 8, 11, 12, 13, 14, 15], 'mapped', path + '/actions/0',
                'A Spawn of the Welter on the egg\'s tile (forced, no master).')
    build.entry(item, path_, [10], 'mapped', path + '/actions/1', 'Only this egg is removed (D31).')
    build.entry(item, path_, [9], 'approved_omission', None, 'The poison effect is cosmetic.')

    path_ = 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_possessed_tree.lua'
    item = build.items['the_enraged_thorn_knight']
    build.participant(item, 'possessed_tree', 'Possessed Tree', 'PossessedTree')
    unbound = ['Unbound Blightwalker', 'Unbound Demon', 'Unbound Demon Outcast', 'Unbound Defiler']
    branches = []
    for name in unbound:
        build.define(item, creature(name))
        branches.append({'weight': 1, 'actions': [
            {'kind': 'spawn', 'creature': creature(name), 'count': 1, 'at': 'death_position', 'owner': 'none', 'health': 'full'},
            {'kind': 'say', 'subject': {'spawned': True}, 'text': f'The destruction of the tree unleashes the {name.lower()}!',
             'mode': 'say'}]})
    unleash = build.rule(item, {'key': 'possessed_tree_unleashes', 'trigger': {'kind': 'creature_died', 'role': 'possessed_tree'},
                                'conditions': [], 'actions': [{'kind': 'one_of', 'branches': branches}]})
    regrow = build.rule(item, {'key': 'possessed_tree_regrows', 'trigger': {'kind': 'creature_died', 'role': 'possessed_tree'},
                               'delay_ms': 60000, 'conditions': [],
                               'actions': [{'kind': 'spawn', 'creature': creature('Possessed Tree'), 'role': 'possessed_tree', 'count': 1,
                                            'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
    build.entry(item, path_, [2, 3, 4, 5, 6, 7], 'mapped', unleash + '/trigger', 'onDeath of a possessed tree.')
    build.entry(item, path_, [1, 9, 10, 11, 12], 'mapped', unleash + '/actions/0',
                'One of the four unbound creatures, picked uniformly, appears forced on the death position and names itself.')
    build.entry(item, path_, [13], 'mapped', regrow, 'addEvent: a new possessed tree, forced, on the same position after 60 s.')
    build.entry(item, path_, [8], 'approved_omission', None, 'The plant effect is cosmetic.')

    path_ = 'data-otservbr-global/scripts/quests/grave_danger_quest/cobra_bastion/creaturescripts_ugly_monster.lua'
    item = build.get('ugly_monster', 'Cobra Bastion: Ugly Monster', 'channel_shared')
    build.participant(item, 'ugly_monster', 'Ugly Monster', 'UglyMonsterDrop')
    loot = [3577, 3582, 836, 3587, 3591, 3593, 3586, 3601, 30059, 30060, 30061]
    for item_id in loot:
        build.define(item, ref('Item', f'canary:item/{item_id}'))
    drops = [{'weight': 97 if index < 8 else 8, 'actions': [{'kind': 'drop_item', 'item': ref('Item', f'canary:item/{item_id}'),
                                                             'at': 'subject_position'}]} for index, item_id in enumerate(loot)]
    rules = []
    for source_kind in ('damage_taken', 'heal_received'):
        rules.append(build.rule(item, {'key': f'ugly_monster_drops_on_{source_kind}', 'trigger': {'kind': source_kind, 'role': 'ugly_monster',
                                                                                                  'source': 'any'},
                                       'conditions': [{'kind': 'chance_percent', 'value': 1}],
                                       'actions': [{'kind': 'one_of', 'branches': drops}]}))
    build.entry(item, path_, [68, 69, 81, 82, 84], 'mapped', rules[0] + '/trigger',
                'onHealthChange runs for every change with an origin, damage and heals alike (game.cpp combatChangeHealth); '
                'one rule per trigger kind.')
    build.entry(item, path_, [68, 69], 'mapped', rules[1] + '/trigger', 'The same handler for heals.')
    build.entry(item, path_, [70, 71], 'mapped', rules[0] + '/conditions/0', 'math.random(100) == 100: a 1% chance.')
    build.entry(item, path_, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 72, 73, 74, 75, 76, 77, 78, 79, 80], 'mapped',
                rules[0] + '/actions/0',
                'Then 3% (math.random(100) >= 98) one of the last three items, uniformly, else one of the first eight: weights '
                '8 each for 30059-30061 and 97 each for the others (0.03/3 = 8/800, 0.97/8 = 97/800), dropped on the monster\'s tile.')

    path_ = 'data-otservbr-global/scripts/quests/forgotten_knowledge/creaturescripts_soulcatcher_summon.lua'
    item = build.get('soulcatcher', 'Forgotten Knowledge: Soulcatcher', 'instance_per_party')
    build.participant(item, 'soulcatcher', 'Soulcatcher', 'SoulcatcherSummon')
    build.define(item, creature('Corrupted Soul'))
    item['encounter']['anchors'].append({'key': 'corrupted_soul_spawn', 'kind': 'point', 'description': 'Canary (33358, 31187, 10).'})
    rules = []
    for source_kind in ('damage_taken', 'heal_received'):
        rules.append(build.rule(item, {
            'key': f'soulcatcher_releases_a_soul_on_{source_kind}', 'trigger': {'kind': source_kind, 'role': 'soulcatcher', 'source': 'any'},
            'conditions': [{'kind': 'health_percent', 'role': 'soulcatcher', 'op': '<', 'value': 100},
                           {'kind': 'chance_percent', 'value': 15}],
            'actions': [{'kind': 'spawn', 'creature': creature('Corrupted Soul'), 'count': 1, 'at': {'anchor': 'corrupted_soul_spawn'},
                         'owner': 'none', 'health': 'full'}]}))
    build.entry(item, path_, [1, 2, 3, 11, 12, 14], 'mapped', rules[0] + '/trigger',
                'onHealthChange of the Soulcatcher: every change with an origin, including its own heals (healing spell '
                '100-145 and life drain); one rule per trigger kind.')
    build.entry(item, path_, [1, 2, 3], 'mapped', rules[1] + '/trigger', 'The same handler for heals.')
    build.entry(item, path_, [4, 9], 'mapped', rules[0] + '/conditions/0',
                'getHealth() <= 49999 of 50000: not at full health, read before the change (§9.2).')
    build.entry(item, path_, [5, 6, 8], 'mapped', rules[0] + '/conditions/1', 'math.random(1, 100) <= 15.')
    build.entry(item, path_, [7], 'mapped', rules[0] + '/actions/0', 'A corrupted soul, forced, on (33358, 31187, 10).')

    path_ = 'data-otservbr-global/scripts/quests/dark_trails/creaturescripts_kill_death_priest_shargon.lua'
    item = build.get('death_priest_shargon', 'Dark Trails: Death Priest Shargon', 'instance_per_party')
    build.participant(item, 'death_priest_shargon', 'Death Priest Shargon', 'DeathPriestShargonDeath')
    item['encounter']['anchors'] += [{'key': 'exit_teleporter', 'kind': 'point', 'description': 'Canary (33487, 32101, 9).'},
                                     {'key': 'shargon_exit', 'kind': 'point', 'description': 'Canary (33489, 32088, 9).'}]
    teleporter = ref('Item', 'canary:item/1949')
    build.define(item, teleporter)
    item['encounter']['outcomes'].append('death_priest_shargon_defeated')
    path = build.rule(item, {'key': 'death_priest_shargon_death', 'trigger': {'kind': 'creature_died', 'role': 'death_priest_shargon'},
                             'conditions': [],
                             'actions': [{'kind': 'map_item', 'operation': 'create', 'item': teleporter, 'anchor': 'exit_teleporter',
                                          'destination': 'shargon_exit', 'revert_after_ms': 5 * 60 * 1000},
                                         {'kind': 'emit_outcome', 'outcome': 'death_priest_shargon_defeated', 'credited': 'party'}]})
    build.entry(item, path_, [18, 20], 'mapped', path + '/trigger', 'onDeath of Death Priest Shargon.')
    build.entry(item, path_, [1, 2, 3, 4, 23, 24, 25, 26, 36, 10, 11, 12, 13, 15, 16], 'mapped', path + '/actions/0',
                'A teleporter 1949 on (33487, 32101, 9) leading to (33489, 32088, 9), removed after 5 minutes.')
    build.entry(item, path_, [5, 6, 7, 28, 29, 30, 31, 32, 33, 34], 'mapped', path + '/actions/1',
                'onDeathForParty credits the party of the top damage dealer (functions.lua lines 1087-1096; party.lua Participants '
                'tests a misspelt requiredSharedExperience, so shared experience is never required): DarkTrails.Mission18 = 1 unless '
                'already set (D27 quest domain).')
    build.entry(item, path_, [14, 21, 22], 'approved_omission', None, 'Magic effects are cosmetic.')
    item['manifest']['outcome_evidence'].append({
        'outcome': 'death_priest_shargon_defeated', 'credited': 'party',
        'quest_domain': {'canary_storage': 'Storage.Quest.U10_50.DarkTrails.Mission18', 'raise_to_at_least': 1}})

    path_ = 'data-otservbr-global/scripts/quests/dangerous_depth/creaturescripts_snail_slime_kill.lua'
    item = build.items['the_count_of_the_core']
    build.participant(item, 'snail_slime', 'Snail Slime', 'SnailSlimeDeath')
    build.participant(item, 'snail_slime', 'Snail Slime', 'SnailSlimeThink')
    cry = build.rule(item, {'key': 'snail_slime_bursts', 'trigger': {'kind': 'creature_died', 'role': 'snail_slime'}, 'conditions': [],
                            'actions': [{'kind': 'say', 'subject': {'role': 'snail_slime'}, 'text': '!!', 'mode': 'yell'}]})
    feed = build.rule(item, {'key': 'snail_slime_feeds_the_count', 'trigger': {'kind': 'creature_died', 'role': 'snail_slime'},
                             'conditions': [{'kind': 'creature_present', 'role': 'the_count_of_the_core',
                                             'near': {'role': 'snail_slime', 'radius': 2, 'shape': 'circle'}, 'present': True}],
                             'actions': [{'kind': 'heal', 'subject': {'role': 'the_count_of_the_core'}, 'amount': {'min': 0, 'max': 2000}}]})
    build.entry(item, path_, [60, 61, 65, 66, 68], 'mapped', cry + '/trigger', 'onDeath of a snail slime.')
    build.entry(item, path_, [62], 'mapped', cry + '/actions/0', 'creature:say("!!", TALKTYPE_MONSTER_YELL).')
    build.entry(item, path_, [5, 6, 12, 13, 14, 15, 16, 17, 18, 19, 22, 23, 24, 25, 27, 63, 64], 'mapped', feed + '/conditions/0',
                'AREA_CIRCLE2X2 around the slime (every tile with dx^2 + dy^2 <= 5); onTargetTile acts on The Count of the Core '
                'standing there.')
    build.entry(item, path_, [8, 9, 10, 20, 21], 'mapped', feed + '/actions/0', 'The Count gains math.random(0, 2000) health.')
    build.entry(item, path_, [1, 2, 3], 'approved_omission', None,
                'The fire combat has no damage formula, so it deals nothing; its effect is cosmetic.')
    build.entry(item, path_, [29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55,
                              56, 58], 'approved_omission', None,
                'SnailSlimeThink is registered only by the snail slime, but acts only on a creature named "the count of the core", '
                'so it never does anything.')

def respawn_and_remains(build):
    """CarlinVortexDeath (the cult soul remains) and DeathDragon (Ragiaz's dragons revive; the wiki decides, D25)."""
    path_ = 'data-otservbr-global/scripts/quests/cults_of_tibia/creaturescripts_carlin_vortex_spawn.lua'
    item = build.get('cult_soul_remains', 'Cults of Tibia: soul remains of the cultists', 'channel_shared')
    for name in ('Cult Believer', 'Cult Enforcer', 'Cult Scholar'):
        build.participant(item, 'cultist', name, 'CarlinVortexDeath')
    branches = []
    for item_id in (32414, 32415):
        remains = ref('Item', f'canary:item/{item_id}')
        build.define(item, remains)
        branches.append({'weight': 1, 'actions': [{'kind': 'map_item', 'operation': 'create', 'item': remains, 'at': 'death_position',
                                                   'revert_after_ms': 60000, 'interaction': 'canary:interaction/5580'}]})
    path = build.rule(item, {'key': 'cultist_leaves_soul_remains', 'trigger': {'kind': 'creature_died', 'role': 'cultist'}, 'conditions': [],
                             'actions': [{'kind': 'one_of', 'branches': branches}]})
    build.entry(item, path_, [1, 2, 15], 'mapped', path + '/trigger', 'onDeath of a cult believer, enforcer or scholar.')
    build.entry(item, path_, [3, 4, 5, 6, 7, 8, 9, 10, 11, 12], 'mapped', path + '/actions/0',
                'Remains 32414 or 32415, picked uniformly, with action id 5580 on the death position, removed after one minute. '
                'Action id 5580 (movements_task_teleport.lua) lets a player absorb the remains for a Cults of Tibia mission '
                'counter (interaction and quest domains).')

    path_ = 'data-otservbr-global/scripts/quests/ferumbras_ascension/creaturescripts_death_dragon.lua'
    item = build.items['ragiaz']
    build.participant(item, 'death_dragon', 'Death Dragon', 'DeathDragon')
    revive = build.rule(item, {'key': 'death_dragon_revives', 'trigger': {'kind': 'creature_died', 'role': 'death_dragon'}, 'delay_ms': 1000,
                               'conditions': [],
                               'actions': [{'kind': 'spawn', 'creature': creature('Death Dragon'), 'role': 'death_dragon', 'count': 1,
                                            'at': 'death_position', 'owner': 'none', 'health': 'full'}]})
    speak = build.rule(item, {'key': 'ragiaz_revives_his_minion', 'trigger': {'kind': 'creature_died', 'role': 'death_dragon'},
                              'conditions': [{'kind': 'creature_present', 'role': 'ragiaz', 'near': {'role': 'death_dragon', 'radius': 10},
                                              'present': True}],
                              'actions': [{'kind': 'say', 'subject': {'role': 'ragiaz'}, 'text': 'Ragiaz power revives his minion!',
                                           'mode': 'say'}]})
    build.entry(item, path_, [1, 2, 3, 4, 5, 21, 23], 'mapped', revive + '/trigger',
                'onDeath of a death dragon. The guard reads the undefined global targetMonster, so Canary always returns here and '
                'the rest never runs. The reference-date wiki (Ferumbras\' Ascension Quest/Spoiler: "The Death Dragons respawn '
                'immediately after being killed") decides (D25): the body below is transcribed.')
    build.entry(item, path_, [7, 8, 9, 10], 'mapped', revive + '/actions/0',
                'A new death dragon, forced, on the death position one second later (addEvent returns an event id, so the nil check '
                'never stops it).')
    build.entry(item, path_, [12, 13, 14, 15, 16, 17, 18, 19, 20], 'mapped', speak,
                'Ragiaz within 10 tiles of the dead dragon says the line.')


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    args = parser.parse_args()
    build = Encounters(args.canary)
    for transcribe in (soul_war_taint_zones, dream_courts, forgotten_knowledge, ascendant, cults_of_tibia, wrath_of_the_emperor, ghulosh, dangerous_depth, hero_of_rathleton, azerus, gorzindel, heart_minions, heart_chargers, heart_bosses, small_boss_events, urmahlullu, megalomania_splinters, world_boss_events, quest_room_events, secret_library_knowledges, d31_events, respawn_and_remains, forgotten_knowledge_fights, eleventh_slice, heart_minion_forms, replica_servants):
        transcribe(build)
    print(json.dumps(build.write()))


if __name__ == '__main__':
    main()
