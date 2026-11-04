"""Generate batch-2 classic source speech, guarded transitions and exchange inputs.

This importer reads only Crystal 201189f Git blobs. Literal source expressions
remain evidence; runtime selects only exact literal branches through native owners.
"""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
NPCS = ('mr_morris', 'woblin', 'an_old_dragon_lord', 'rottin_wood',
        'the_queen_of_the_banshees', 'jerom', 'chief_grarkharok', 'menesto',
        'oressa', 'captain_dreadnought', 'inigo', 'mysterious_ornate_chest')
NATIVE_NPCS = ('mr_morris', 'woblin', 'jerom', 'the_queen_of_the_banshees')
DAWN = 'oteryn:quest-progress/quest/u10_55/dawnport/'
DAWN_QUEST = 'oteryn:quest.dawnport_quest'
TROLL = 'oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline'
TROLL_QUEST = 'oteryn:quest.troll_sabotage_quest'
ROTTIN = 'oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/'
ROTTIN_QUEST = 'oteryn:quest.rottin_wood_and_the_married_men_quest'
WALLCOUNT = 'oteryn:quest-progress/crystalserver/kv/rottinwood-wallcount'


def encode(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode()


def source(path, line):
    return {'repository': 'zimbadev/crystalserver', 'revision': PIN, 'path': path, 'line': line}


def pinned(crystal, path):
    return subprocess.check_output(['git', '-C', str(crystal), 'show', f'{PIN}:{path}'])


def speech_capture(crystal, root):
    sys.path.insert(0, str(root / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    documents, greetings = [], []
    for npc in NPCS:
        path = f'data-global/npc/{npc}.lua'
        raw = pinned(crystal, path)
        tokens = [t for t in lua.tokenize(raw.decode()) if t[0] not in ('comment', 'lcomment')]
        branches, messages, message_sources = [], {}, {}
        fragments = [{'line': t[2], 'text': lua._string(t)} for t in tokens
                     if t[0] in ('string', 'lstring')]
        for i, token in enumerate(tokens):
            signature = [t[1] for t in tokens[i:i + 4]]
            if signature not in (['npcHandler', ':', 'say', '('], ['npcHandler', ':', 'setMessage', '(']):
                continue
            start = i + 4
            message = None
            if signature[2] == 'setMessage':
                message = tokens[start][1]
                if tokens[start + 1][1] != ',':
                    raise ValueError('CLASSIC_MESSAGE_SHAPE')
                start += 2
            argument, depth, end = [], 0, start
            for j in range(start, len(tokens)):
                value = tokens[j][1]
                if value in ('(', '{', '['):
                    depth += 1
                if value in (')', '}', ']'):
                    if depth == 0:
                        break
                    depth -= 1
                if depth == 0 and value == ',':
                    break
                argument.append(tokens[j])
                end = j
            parts = [lua._string(t) for t in argument if t[0] in ('string', 'lstring')]
            literal = bool(argument) and (
                argument[0][0] in ('string', 'lstring') and len(argument) == 1
                or argument[0][1] == '{' and all(t[0] in ('string', 'lstring') or t[1] in ('{', '}', ',') for t in argument))
            if message:
                if message in ('MESSAGE_GREET', 'MESSAGE_FAREWELL', 'MESSAGE_WALKAWAY') and npc in NATIVE_NPCS:
                    if not literal or len(parts) != 1 or message in messages:
                        raise ValueError('CLASSIC_NATIVE_GREETING_NOT_LITERAL')
                    messages[message] = parts[0]
                    message_sources[message] = source(path, token[2])
            else:
                branches.append({'key': f'line-{token[2]}', 'line': token[2], 'end_line': tokens[end][2],
                    'reply': parts if literal else [], 'source_text_parts': parts, 'literal': literal,
                    'source_expression': None if literal else ' '.join(t[1] for t in argument),
                    'native_binding': 'SOURCE_BRANCH_SELECTED_BY_NATIVE_ADAPTER_ONLY'})
        key = 'oteryn:npc.' + ('an_old_dragonlord' if npc == 'an_old_dragon_lord' else npc)
        documents.append({'npc': key, 'source': {**source(path, 1), 'sha256': hashlib.sha256(raw).hexdigest()},
                          'branches': branches, 'source_string_fragments': fragments})
        if npc in NATIVE_NPCS:
            if set(messages) != {'MESSAGE_GREET', 'MESSAGE_FAREWELL', 'MESSAGE_WALKAWAY'}:
                raise ValueError('CLASSIC_NATIVE_GREETING_INCOMPLETE')
            greetings.append({'npc': key, 'greet': messages['MESSAGE_GREET'],
                              'farewell': messages['MESSAGE_FAREWELL'], 'walkaway': messages['MESSAGE_WALKAWAY'],
                              'sources': message_sources})
    return documents, greetings


def effect(track, op='ANY', value=None, after=1, kind='SET'):
    guard = {'op': op}
    if value is not None:
        guard['value'] = value
    return {'track': track, 'from': guard, 'from_exact': True, 'effect': {'kind': kind, 'value': after}}


def transition(name, effects, npc, keywords, topic, line, quest=DAWN_QUEST):
    return {'key': 'oteryn:quest-transition/classic/' + name, 'quest': quest,
            'completes': False, 'effects': effects,
            'requested_by': {'npc': 'crystal:npc/' + npc, 'keywords': keywords, 'topics': [topic] if topic else []},
            'source': {**source(f'data-global/npc/{npc}.lua', line), 'scope': 'EXACT_NATIVE_NPC_BRANCH'}}


def state_document(quest, transitions, tracks):
    return {'schema': 'OTERYN_QUEST_STATE_LOWERING/v1', 'classification': 'OTS_HYPOTHESIS_ONLY',
            'family': 'Quest', 'contract': 'docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md',
            'authoring_sources': [t['source'] for t in transitions],
            'counts': {'transitions': len(transitions), 'tracks': len(tracks)},
            'quests': [{'quest': quest, 'source_quest': 'Crystal201189f pinned source',
                        'completion': 'NOT_COMPLETED_BY_START_ADAPTER', 'tracks': tracks, 'transitions': transitions}]}


def states(root):
    base = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    transitions = []
    for name, line, topic in [('the_lost_amulet', 161, 1), ('the_rare_herb', 180, 3), ('the_dorm_key', 185, 4)]:
        effects = [effect(DAWN + name, 'LT', 1)]
        if name == 'the_rare_herb':
            effects.append(effect(DAWN + 'herb_flower'))
        transitions.append(transition('dawnport/start-' + name.replace('_', '-'), effects, 'mr_morris', ['yes'], topic, line))
    transitions.append(transition('dawnport/start-logbook', [effect(DAWN + 'torn_log_book', 'LT', 1, 0, 'ADD'),
        effect(DAWN + 'the_stolen_log_book')], 'mr_morris', ['yes'], 2, 170))
    for active, count, line, topic in [('morrisk_troll', 'morris_troll_count', 200, 6),
                                      ('morris_goblin', 'morris_goblin_count', 210, 7),
                                      ('morris_minos', 'morris_minos_count', 220, 8)]:
        transitions.append(transition('dawnport/start-' + active.replace('_', '-'),
            [effect(DAWN + active, 'LT', 1), effect(DAWN + count, after=0)], 'mr_morris', ['yes'], topic, line))
    transitions.append(transition('dawnport/find-old-nasty', [effect(DAWN + 'the_dorm_key', 'EQ', 1, 2)],
                                  'woblin', ['reward'], 1, 92))
    used = {e['track'] for t in transitions for e in t['effects']}
    tracks = [t for q in base['quests'] if q['quest'] == DAWN_QUEST for t in q['tracks'] if t['key'] in used]
    if not any(t['key'] == DAWN + 'torn_log_book' for t in tracks):
        tracks.append({'bounds_basis': 'explicit_crystal_values', 'initial': 0, 'key': DAWN + 'torn_log_book',
                       'min': 0, 'max': 1, 'quest': DAWN_QUEST,
                       'source_key': 'canary:quest-progress/quest/u10_55/dawnport/torn_log_book'})
    troll = transition('troll-sabotage/start', [effect(TROLL, 'LT', 1)], 'jerom', ['yes'], 1, 83, TROLL_QUEST)
    troll_tracks = [{'bounds_basis': 'explicit_crystal_values', 'initial': 0, 'key': TROLL, 'min': 0, 'max': 3,
                     'quest': TROLL_QUEST, 'source_key': 'canary:quest-progress/quest/u8_2/troll_sabotage_quest/questline'}]
    return state_document(DAWN_QUEST, transitions, tracks), state_document(TROLL_QUEST, [troll], troll_tracks)


def exchanges(crystal):
    lines = pinned(crystal, 'data-global/npc/mr_morris.lua').decode().splitlines()
    for number, expected in [(91, 'player:removeItem(21379, 1)'), (92, 'player:addItem(3031, 50)'),
                             (93, 'player:setStorageValue(Storage.Quest.U10_55.Dawnport.TheLostAmulet, 3)'),
                             (134, 'player:addItem(3031, 50)'),
                             (135, 'player:setStorageValue(Storage.Quest.U10_55.Dawnport.TheRareHerb, 3)')]:
        if lines[number - 1].strip() != expected:
            raise ValueError('CLASSIC_EXCHANGE_SOURCE_DRIFT')
    return {'schema': 1, 'rules': [
        {'key': 'oteryn:quest-transition/classic/dawnport/return-the-lost-amulet',
         'trigger': {'kind': 'npc_command', 'npc': 'oteryn:npc.mr_morris'},
         'consume': [{'item': 'oteryn:item.tibia.i21379', 'count': 1}],
         'grant': [{'item': 'oteryn:item.tibia.i3031', 'count': 50}],
         'source': source('data-global/npc/mr_morris.lua', 85)},
        {'key': 'oteryn:quest-transition/classic/dawnport/return-the-rare-herb',
         'trigger': {'kind': 'npc_command', 'npc': 'oteryn:npc.mr_morris'},
         'consume': [], 'grant': [{'item': 'oteryn:item.tibia.i3031', 'count': 50}],
         'source': source('data-global/npc/mr_morris.lua', 127)},
    ]}


def rottin_state(crystal, root):
    path = 'data-global/npc/rottin_wood.lua'
    lines = pinned(crystal, path).decode().splitlines()
    for number, expected in [(97, 'elseif player:getStorageValue(S.Questline) == 2 then'),
                             (176, 'player:setStorageValue(S.Questline, 3)'),
                             (177, 'player:setStorageValue(S.Mission02, 1) -- Quest Log: Start Mission 02'),
                             (178, 'player:kv():set("rottinwood-wallcount", 0)')]:
        if lines[number - 1].strip() != expected:
            raise ValueError('CLASSIC_ROTTIN_DISREPAIR_SOURCE_DRIFT')
    base = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    required = {ROTTIN + 'questline', ROTTIN + 'mission02'}
    tracks = [t for q in base['quests'] if q['quest'] == ROTTIN_QUEST
              for t in q['tracks'] if t['key'] in required]
    if {t['key'] for t in tracks} != required:
        raise ValueError('CLASSIC_ROTTIN_EXISTING_TRACKS')
    # This batch admits only the literal reset to zero. The deferred hammer
    # owner must qualify its counter writes separately; no old state is migrated.
    tracks.append({'bounds_basis': 'explicit_crystal_kv_reset_only', 'initial': 0,
                   'min': 0, 'max': 0, 'key': WALLCOUNT, 'quest': ROTTIN_QUEST,
                   'source_key': 'crystalserver:kv/rottinwood-wallcount'})
    start = transition('rottin-wood/start-disrepair', [
        effect(ROTTIN + 'questline', 'EQ', 2, 3),
        effect(ROTTIN + 'mission02'), effect(WALLCOUNT, after=0)],
        'rottin_wood', ['yes'], 4, 176, ROTTIN_QUEST)
    return state_document(ROTTIN_QUEST, [start], tracks)


def generate(crystal, root=ROOT):
    documents, greetings = speech_capture(crystal, root)
    dawn, troll = states(root)
    return {
        'content/dialogues/quests-2/classic-donor-dialogues.json': encode({
            'schema': 'OTERYN_CLASSIC_QUEST_DIALOGUE_SOURCE/v1', 'source_revision': PIN, 'npcs': documents}),
        'content/dialogues/classic-quest-branches.json': encode({'schema': 1, 'npcs': [
            {'npc': doc['npc'], 'branches': [{'key': b['key'], 'reply': b['reply']} for b in doc['branches'] if b['literal']]}
            for doc in documents]}),
        'content/dialogues/quests-2/classic-npc-greetings.json': encode({'schema': 1, 'npcs': greetings,
            'placeholder_source': source('data/npclib/npc_system/npc_handler.lua', 383)}),
        'content/quests/dawnport-starts.json': encode(dawn),
        'content/quests/troll-sabotage-starts.json': encode(troll),
        'content/quests/rottin-wood-starts.json': encode(rottin_state(crystal, root)),
        'content/dialogues/quests-2/classic-deferred-exchanges.json': encode({
            'schema': 'OTERYN_CLASSIC_DEFERRED_EXCHANGE_SOURCE/v1',
            'disposition': 'LEFT_OUT_SHARED_OWNER',
            'answer': 'https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017',
            'rules': exchanges(crystal)['rules']}),
    }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    outputs = generate(args.crystal, args.root)
    for relative, raw in outputs.items():
        path = args.root / relative
        if args.check:
            if path.read_bytes() != raw:
                raise SystemExit('CLASSIC_AUTHORING_DRIFT:' + relative)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
    print(json.dumps({'files': len(outputs), 'source_npcs': len(NPCS), 'native_greetings': len(NATIVE_NPCS), 'check': args.check}))
