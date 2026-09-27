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


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    args = parser.parse_args()
    build = Encounters(args.canary)
    for transcribe in (soul_war_taint_zones, dream_courts, forgotten_knowledge, ascendant, cults_of_tibia, wrath_of_the_emperor, ghulosh, dangerous_depth, hero_of_rathleton, azerus, gorzindel, heart_minions, heart_chargers, small_boss_events, urmahlullu, megalomania_splinters):
        transcribe(build)
    print(json.dumps(build.write()))


if __name__ == '__main__':
    main()
