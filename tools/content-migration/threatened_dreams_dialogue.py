"""Pinned Threatened Dreams speech and explicitly reviewed native transitions.

No donor code is executed. Speech uses the repository's literal Lua tokenizer;
the finite transition list below is a source-backed curation, not inferred guards.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys

from quests_2_emperor_dialogues import capture, first_argument, source_voices, static_keywords

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
QUEST = 'oteryn:quest.threatened_dreams_quest'
BASE = 'oteryn:quest-progress/quest/u11_40/threatened_dreams/'
CRYSTAL = 'oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/'
TR = 'oteryn:quest-transition/threatened-dreams/'
TRACKS = {
    'questline': BASE + 'quest_line',
    'animals': CRYSTAL + 'mission01/troubled_animals',
    'animals_log': BASE + 'mission01_1',
    'poacher_chest': BASE + 'mission01/poacher_chest',
    'couple': BASE + 'mission03_1',
    'couple_progress': CRYSTAL + 'mission03/unlikely_couple',
    'tired': BASE + 'mission04/tired_tree',
    'grumpy': BASE + 'mission04/grumpy_stone',
    'teeth': BASE + 'mission04/teeth_collection',
    'tooth_fairy': BASE + 'mission04/tooth_fairy',
    'cloak': BASE + 'mission05/swan_feather_cloak',
    'nightmare': CRYSTAL + 'mission02/nightmare_intruders',
    'nightmare_log': BASE + 'mission02_1',
    'kroazur_access': CRYSTAL + 'mission02/kroazur_access',
    'enfeebled': CRYSTAL + 'mission02/enfeebled_count',
    'frazzlemaws': CRYSTAL + 'mission02/frazzlemaws_count',
    'fairies': CRYSTAL + 'mission02/fairies_counter',
    'moon_mirror': CRYSTAL + 'mission02/dark_moon_mirror',
}

STORAGES = 'data-global/lib/core/storages.lua'
# Existing canonical paths are kept; source IDs are witnesses, never native keys.
# The six read dependencies have no new item writer in this batch. Their source
# reward/rake owners remain held under PR2844 ANSWER6589521017 (5),(6).
SOURCE_TRACKS = (
    ('poacher_chest', 'PoacherChest', 2510, 45753, 1, 'data-global/startup/tables/chest.lua', 1352),
    ('couple_progress', 'UnlikelyCouple', 2554, 45793, 4, 'data-global/npc/aurita.lua', 103),
    ('moon_mirror', 'DarkMoonMirror', 2526, 45767, 1, 'data-global/startup/tables/chest.lua', 1359),
) + tuple((f'stone{i}', f'Stone{i}', 2568 + i, 45805 + i, 1,
           'data-global/scripts/actions/tools/rake.lua', 25 + i) for i in range(1, 6))
TRACKS.update({f'stone{i}': BASE + f'mission04/stone{i}' for i in range(1, 6)})

# These are the source NPCs with root StdModule.say registrations before their
# default quest callbacks. The generated data only prevents callback fallthrough;
# it does not activate the captured lore or mutate a conversation/QuestState owner.
INTERCEPTED_NPCS = {'alkestios', 'chief_grarkharok', 'grumpy_stone', 'tired_tree', 'tooth_fairy'}


def intercepted_keywords(text, root):
    """Qualify only unconditional literal root speech and aliases, without Lua execution."""
    sys.path.insert(0, str(root / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    tokens = [t for t in lua.tokenize(text) if t[0] not in ('comment', 'lcomment')]
    result = []
    for row in static_keywords(text, root):
        if row['receiver'] != 'keywordHandler' or row['kind'] not in ('keyword', 'keyword_alias'):
            raise ValueError('THREATENED_DREAMS_INTERCEPTION_SHAPE')
        keywords = row['keywords']
        if not isinstance(keywords, list) or not 1 <= len(keywords) <= 3 or any(
                not isinstance(word, str) or not word or len(word) > 64 or
                any(not ('a' <= char <= 'z' or char in " '") for char in word)
                for word in keywords):
            raise ValueError('THREATENED_DREAMS_INTERCEPTION_LITERAL_KEYWORDS')
        method = 'addKeyword' if row['kind'] == 'keyword' else 'addAliasKeyword'
        calls = [i for i in range(len(tokens) - 4) if tokens[i][2] == row['line'] and
                 [t[1] for t in tokens[i:i + 4]] == ['keywordHandler', ':', method, '(']]
        if len(calls) != 1:
            raise ValueError('THREATENED_DREAMS_INTERCEPTION_SOURCE_CALL')
        cursor = calls[0] + 4
        arguments = []
        while tokens[cursor][1] != ')':
            argument = first_argument(tokens, cursor)
            arguments.append(argument)
            cursor += len(argument)
            if tokens[cursor][1] == ',':
                cursor += 1
            elif tokens[cursor][1] != ')':
                raise ValueError('THREATENED_DREAMS_INTERCEPTION_ARGUMENTS')
        if row['kind'] == 'keyword_alias':
            # keyword_handler.lua:100-111 copies the preceding root node's
            # parameters and callback; every preceding admitted node is plain say.
            if len(arguments) != 1 or not result:
                raise ValueError('THREATENED_DREAMS_INTERCEPTION_ALIAS')
        else:
            if len(arguments) != 3 or row.get('module') != 'StdModule.say':
                raise ValueError('THREATENED_DREAMS_INTERCEPTION_CONDITION_OR_ACTION')
            parser = lua.Parser(row['parameters'])
            parameters = lua.as_python(parser.value())
            if parser.i != len(parser.tokens) or not isinstance(parameters, dict) or set(parameters) != {'npcHandler', 'text'}:
                raise ValueError('THREATENED_DREAMS_INTERCEPTION_PARAMETERS')
            messages = parameters['text']
            if (parameters['npcHandler'] != {'expr': 'npcHandler'} or
                    not row.get('literal_messages') or
                    not (isinstance(messages, str) or isinstance(messages, list) and
                         all(isinstance(line, str) for line in messages))):
                raise ValueError('THREATENED_DREAMS_INTERCEPTION_LITERAL_SPEECH')
        result.append({'line': row['line'], 'keywords': keywords})
    return result


def pinned(crystal, path):
    return subprocess.check_output(['git', '-C', str(crystal), 'show', f'{PIN}:{path}'])


def source_track(name, maximum):
    key = TRACKS[name]
    source_key = key.removeprefix('oteryn:').replace('quest-progress/crystalserver/', 'quest-progress/', 1)
    return {'key': key, 'quest': QUEST, 'initial': -1, 'min': -1, 'max': maximum,
            'bounds_basis': 'observed_values', 'source_key': 'crystalserver:' + source_key}


def verify_source(crystal):
    paths = {STORAGES, 'src/creatures/players/player.cpp',
             'data/npclib/npc_system/npc_handler.lua',
             'data/npclib/npc_system/keyword_handler.lua',
             'data/npclib/npc_system/modules.lua',
             'data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua',
             'data-global/scripts/quests/threatened_dreams_quest/movement_poacher_notes.lua'}
    paths.update(row[5] for row in SOURCE_TRACKS)
    raw = {path: pinned(crystal, path) for path in paths}
    lines = {path: data.decode().splitlines() for path, data in raw.items()}
    for _, symbol, declaration, source_id, _, writer, writer_line in SOURCE_TRACKS:
        if not lines[STORAGES][declaration - 1].strip().startswith(f'{symbol} = {source_id},'):
            raise ValueError('THREATENED_DREAMS_SOURCE_DECLARATION:' + symbol)
        if '.' + symbol not in lines[writer][writer_line - 1]:
            raise ValueError('THREATENED_DREAMS_SOURCE_WRITER:' + symbol)
    if lines['src/creatures/players/player.cpp'][1341].strip() != 'int32_t value = -1;':
        raise ValueError('THREATENED_DREAMS_UNSET_STORAGE')
    for path, line, exact in (
        ('data/npclib/npc_system/npc_handler.lua', 448, 'local ret = self.keywordHandler:processMessage(npc, player, msg)'),
        ('data/npclib/npc_system/npc_handler.lua', 449, 'if not ret then'),
        ('data/npclib/npc_system/keyword_handler.lua', 59, 'local a, b = string.find(message, keyword)'),
        ('data/npclib/npc_system/keyword_handler.lua', 60, 'if a == nil or b == nil or a < last then'),
        ('data/npclib/npc_system/keyword_handler.lua', 66, 'last = a'),
        ('data/npclib/npc_system/modules.lua', 5, 'FOCUS_GREETWORDS = { "hi", "hello" }'),
        ('data/npclib/npc_system/modules.lua', 6, 'FOCUS_FAREWELLWORDS = { "bye", "farewell" }'),
        ('data/npclib/npc_system/modules.lua', 8, 'FOCUS_TRADE_MESSAGE = { "trade", "offers" }'),
        ('data/npclib/npc_system/modules.lua', 382, 'if string.find(message, word) and not string.find(message, "[%w+]" .. word) and not string.find(message, word .. "[%w+]") then'),
    ):
        if lines[path][line - 1].strip() != exact:
            raise ValueError('THREATENED_DREAMS_KEYWORD_PRECEDENCE_SOURCE')
    rake = lines['data-global/scripts/actions/tools/rake.lua']
    if rake[36].strip() != 'player:setStorageValue(positionStorages[i], 1)':
        raise ValueError('THREATENED_DREAMS_RAKE_FLAG_VALUE')
    kroazur = lines['data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua']
    for line, fragment in (
        (4, '[25029] = {'), (9, 'exitPos = Position(33619, 32306, 9)'),
        (30, 'if player:getPosition() == Position(33591, 32321, 10) then'),
        (32, 'player:teleportTo(room.exitPos)'), (34, 'return true'),
        (37, 'if player:getStorageValue(room.access) < 1 then'),
        (77, 'kroazurRoom:aid(25029)'),
    ):
        if fragment not in kroazur[line - 1]:
            raise ValueError('THREATENED_DREAMS_KROAZUR_EXIT_SOURCE')
    discovery = lines['data-global/scripts/quests/threatened_dreams_quest/movement_poacher_notes.lua']
    for line, exact in (
        (8, 'if player:getStorageValue(Storage.Quest.U11_40.ThreatenedDreams.Mission01.TroubledAnimals) == 6 then'),
        (10, 'player:setStorageValue(Storage.Quest.U11_40.ThreatenedDreams.Mission01.TroubledAnimals, 7)'),
        (15, 'foundPoacherBody:position({ x = 32949, y = 31811, z = 7 })'),
        (16, 'foundPoacherBody:position({ x = 32950, y = 31811, z = 7 })'),
        (17, 'foundPoacherBody:position({ x = 32951, y = 31811, z = 7 })'),
    ):
        if discovery[line - 1].strip() != exact:
            raise ValueError('THREATENED_DREAMS_POACHER_DISCOVERY_SOURCE')
    return [{'repository': 'zimbadev/crystalserver', 'revision': PIN, 'path': path,
             'sha256': hashlib.sha256(blob).hexdigest()} for path, blob in sorted(raw.items())]


def effect(track, comparison, value, result):
    return {'track': TRACKS[track], 'from': {'op': comparison, **({'value': value} if value is not None else {})},
            'from_exact': True, 'effect': {'kind': 'SET', 'value': result}}


def transitions():
    # Each tuple names the exact source writer, and all mutating source branches
    # here have no inventory, outfit, teleport or implicit reward side effect.
    rules = [
        ('alkestios-start', 'alkestios', 96, [effect('questline', 'NE', 1, 1), effect('animals', 'ANY', None, 1), effect('animals_log', 'ANY', None, 1)]),
        ('alkestios-real-animals', 'alkestios', 105, [effect('animals', 'ANY', None, 4), effect('animals_log', 'ANY', None, 4)]),
        ('alkestios-access', 'alkestios', 77, [effect('animals', 'EQ', 15, 16), effect('animals_log', 'ANY', None, 16)]),
        ('ikassis-swan', 'ikassis', 71, [effect('animals', 'EQ', 10, 11), effect('animals_log', 'ANY', None, 11)]),
        ('wolf-search', 'ghostly_wolf', 90, [effect('animals', 'ANY', None, 6), effect('animals_log', 'ANY', None, 6)]),
        ('wolf-peace', 'ghostly_wolf', 75, [effect('animals', 'EQ', 9, 10), effect('animals_log', 'ANY', None, 10)]),
        ('swan-search', 'a_swan', 87, [effect('animals', 'ANY', None, 12), effect('animals_log', 'ANY', None, 12)]),
        ('aurita-start', 'aurita', 102, [effect('couple', 'ANY', None, 1), effect('couple_progress', 'ANY', None, 1)]),
        ('taegen-start', 'taegen', 155, [effect('couple', 'ANY', None, 1), effect('couple_progress', 'ANY', None, 1)]),
        ('tree-start', 'tired_tree', 75, [effect('tired', 'ANY', None, 1)]),
        ('stone-start', 'grumpy_stone', 96, [effect('grumpy', 'ANY', None, 1)]),
        ('cloak-offer', 'valindara', 80, [effect('couple', 'EQ', 4, 4), effect('cloak', 'ANY', None, 1)]),
        ('maelyrra-start', 'maelyrra', 87, [effect('nightmare', 'ANY', None, 1),
            effect('nightmare_log', 'ANY', None, 1), effect('kroazur_access', 'ANY', None, 1),
            effect('enfeebled', 'ANY', None, 0), effect('frazzlemaws', 'ANY', None, 0),
            effect('questline', 'ANY', None, 2)]),
        ('maelyrra-fairies', 'maelyrra', 115, [effect('nightmare', 'ANY', None, 3),
            effect('nightmare_log', 'ANY', None, 3)]),
        ('maelyrra-fairies-report', 'maelyrra', 121, [effect('fairies', 'EQ', 5, 5),
            effect('moon_mirror', 'EQ', 1, 1), effect('nightmare', 'ANY', None, 4),
            effect('nightmare_log', 'ANY', None, 4)]),
        ('maelyrra-vessels', 'maelyrra', 134, [effect('nightmare', 'ANY', None, 5),
            effect('nightmare_log', 'ANY', None, 5)]),
        ('maelyrra-gather-lights', 'maelyrra', 142, [effect('nightmare', 'ANY', None, 6),
            effect('nightmare_log', 'ANY', None, 6)]),
        ('fairy-spell', 'dancing_fairy', 63, [effect('couple', 'EQ', 1, 1),
            effect('couple_progress', 'EQ', 1, 2)]),
        ('tooth-collection-offer', 'tooth_fairy', 113, [effect('tooth_fairy', 'EQ', 2, 2),
            effect('teeth', 'LT', 1, 1)]),
    ]
    return [{'key': TR + key, 'quest': QUEST, 'completes': False, 'effects': effects,
             'requested_by': {'npc': 'crystal:npc/' + npc, 'keywords': [], 'topics': []},
             'source': {'repository': 'zimbadev/crystalserver', 'revision': PIN,
                        'path': f'data-global/npc/{npc}.lua', 'line': line}}
            for key, npc, line, effects in rules]


def state(root):
    document = json.loads((root/'content/quests/missions/quest-state.json').read_bytes())
    existing = next(q for q in document['quests'] if q['quest'] == QUEST)
    additions = [source_track(name, maximum) for name, _, _, _, maximum, _, _ in SOURCE_TRACKS]
    current = {t['key']: t for t in existing['tracks']}
    for track in additions:
        if track['key'] in current and current[track['key']] != track:
            raise ValueError('THREATENED_DREAMS_SOURCE_TRACK_CONFLICT:' + track['key'])
    wanted = {e['track'] for transition in transitions() for e in transition['effects']}
    wanted.update(t['key'] for t in additions)
    tracks = [copy.deepcopy(t) for t in existing['tracks'] if t['key'] in wanted]
    tracks.extend(t for t in additions if t['key'] not in current)
    if {t['key'] for t in tracks} != wanted:
        raise ValueError('THREATENED_DREAMS_NATIVE_TRACK_CLOSURE')
    return {'schema': 'OTERYN_QUEST_STATE_LOWERING/v1', 'classification': 'OTS_HYPOTHESIS_ONLY',
            'family': 'Quest', 'contract': 'existing fenced QuestState owner',
            'authoring_sources': [{'repository': 'zimbadev/crystalserver', 'revision': PIN,
                                   'path': 'data-global/lib/core/storages.lua', 'line': 2506},
                                  {'repository': 'zimbadev/crystalserver', 'revision': PIN,
                                   'path': 'src/creatures/players/player.cpp', 'line': 1342}],
            'counts': {}, 'quests': [{'quest': QUEST, 'source_quest': 'crystalserver:quest/threatened_dreams_quest',
                                     'completion': 'NOT_LOWERED_MULTI_TRACK', 'tracks': sorted(tracks, key=lambda t:t['key']),
                                     'transitions': transitions()}]}


def kroazur_exit():
    return {'schema': 1, 'callbacks': [{
        'key': 'oteryn:quest-callback/threatened-dreams-kroazur-exit', 'quest': QUEST,
        'trigger': 'step_in',
        'selector': {'position': {'x': 33591, 'y': 32321, 'floor': -10},
                     'unique': None, 'action': 25029, 'appearance': None},
        'lever': None, 'reversible': False, 'passages': [],
        'teleport': {'x': 33619, 'y': 32306, 'floor': -9},
        'storage': [], 'offerings': [], 'item_on_target': None,
        'return_on_failure': False, 'failure_message': None,
        'sources': [f'crystal:{PIN}:data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:3-35',
                    f'crystal:{PIN}:data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:76-78'],
    }]}


def poacher_discovery():
    # Both canonical tracks represent source storage45752: Crystal storages.lua
    # line2509 and Canary47dfd51 storages.lua line2344. Preserve the qualified
    # journal key while using the exact Crystal stage as the permission guard.
    return {'schema': 1, 'callbacks': [{
        'key': f'oteryn:quest-callback/threatened-dreams-poacher-discovery-{x}',
        'quest': QUEST, 'trigger': 'step_in',
        'selector': {'position': {'x': x, 'y': 31811, 'floor': -7},
                     'unique': None, 'action': None, 'appearance': None},
        'lever': None, 'reversible': False, 'passages': [], 'teleport': None,
        'storage': [
            {'track': TRACKS['animals'], 'minimum': 6, 'maximum': 6, 'set': 7},
            {'track': TRACKS['animals_log'], 'minimum': 0, 'maximum': 16, 'set': 7},
        ],
        'offerings': [], 'item_on_target': None, 'return_on_failure': False,
        'failure_message': None,
        'sources': [f'crystal:{PIN}:data-global/scripts/quests/threatened_dreams_quest/movement_poacher_notes.lua:8-17',
                    f'crystal:{PIN}:data-global/lib/core/storages.lua:2509',
                    'canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/lib/core/storages.lua:2344'],
    } for x in (32949, 32950, 32951)]}


def outputs(crystal, root):
    source_bindings = verify_source(crystal)
    packet = capture(crystal, root, families={'threatened_dreams_quest': 'Storage.Quest.U11_40.ThreatenedDreams'},
                     schema='OTERYN_THREATENED_DREAMS_SPEECH/v1')
    packet['source_bindings'] = source_bindings
    packet['scope'] = ('Source speech, static keywords, aliases and voices; dynamic expressions retained '
                       'explicitly. Only closed native selector branches are executable.')
    # The generator retains every source fragment; only complete literal replies
    # are executable. Dynamic source expressions remain reference-only.
    branches = {'schema': 1, 'source_revision': PIN, 'npcs': []}
    for npc in packet['npcs']:
        raw = pinned(crystal, npc['path'])
        if hashlib.sha256(raw).hexdigest() != npc['blob_sha256']:
            raise ValueError('THREATENED_DREAMS_SPEECH_SOURCE_BINDING')
        npc['static_keywords'] = static_keywords(raw.decode(), root)
        npc['voices'] = source_voices(raw.decode(), root)
        branches['npcs'].append({'npc': 'oteryn:npc.' + npc['npc'], 'source': npc['source'],
                                 'intercepted_keywords': intercepted_keywords(raw.decode(), root)
                                 if npc['npc'] in INTERCEPTED_NPCS else [],
                                 'branches': [{'line': reply['line'], 'reply': reply['literal_messages']}
                                              for reply in npc['replies']
                                              if reply['kind'] == 'say' and reply['literal_messages']]})
    return {
        'content/dialogues/quests-2/threatened-dreams-source.json': packet,
        'content/dialogues/quests-2/threatened-dreams.json': branches,
        'content/quests/threatened-dreams-npc.json': state(root),
        'tools/content-schema/quest-authoring/quests-2/world-callbacks/threatened-dreams-kroazur-exit.json': kroazur_exit(),
        'tools/content-schema/quest-authoring/quests-2/world-callbacks/threatened-dreams-poacher-discovery.json': poacher_discovery(),
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for name, value in outputs(args.crystal, args.root).items():
        raw = json.dumps(value, ensure_ascii=False, indent=2) + '\n'
        target = args.root/name
        if args.check:
            if target.read_text() != raw:
                raise SystemExit('THREATENED_DREAMS_DIALOGUE_DRIFT:' + name)
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(raw)
