"""Capture full literal NPC speech for the four owned quests, without Lua execution."""
import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
FAMILIES = {
    'wrath_of_the_emperor': 'Storage.Quest.U8_6.WrathOfTheEmperor',
    'the_new_frontier': 'Storage.Quest.U8_54.TheNewFrontier',
    'the_postman_missions_quest': 'Storage.Quest.U7_24.ThePostmanMissions',
    'the_gravedigger_of_drefia': 'Storage.Quest.U10_10.TheGravediggerOfDrefia',
}


def first_argument(tokens, start):
    depth = 0
    result = []
    for token in tokens[start:]:
        if token[0] == 'comment':
            continue
        value = token[1]
        if value == ',' and depth == 0:
            return result
        if value in ('(', '[', '{'):
            depth += 1
        elif value in (')', ']', '}'):
            if depth == 0:
                return result
            depth -= 1
        result.append(token)
    raise ValueError('Unclosed NPC speech argument')


def speech(lua, tokens, start, offset=0):
    argument = first_argument(tokens, start + offset)
    literals = [{'line': token[2], 'text': lua._string(token)}
                for token in argument if token[0] in ('string', 'lstring')]
    expression = ' '.join(token[1] for token in argument)
    parser = lua.Parser(expression)
    try:
        parsed = lua.as_python(parser.value())
        exact = parser.i == len(parser.tokens) and (
            isinstance(parsed, str) or isinstance(parsed, list) and all(isinstance(x, str) for x in parsed))
    except lua.LuaError:
        exact = False
        parsed = None
    return {'literal_messages': [parsed] if exact and isinstance(parsed, str) else parsed if exact else None,
            'literal_parts': literals, 'expression': None if exact else expression}


def static_keywords(text, root):
    """Keep keyword/child/greeting source packets, without running their modules."""
    sys.path.insert(0, str(root / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    tokens = [t for t in lua.tokenize(text) if t[0] not in ('comment', 'lcomment')]
    captured = []
    methods = {'addKeyword': 'keyword', 'addChildKeyword': 'child_keyword',
               'addAliasKeyword': 'keyword_alias', 'addGreetKeyword': 'greet_keyword'}
    for index in range(len(tokens) - 4):
        head = [t[1] for t in tokens[index:index + 4]]
        if head[1] != ':' or head[2] not in methods or head[3] != '(':
            continue
        first = first_argument(tokens, index + 4)
        parser = lua.Parser(' '.join(t[1] for t in first))
        try:
            keywords = lua.as_python(parser.value())
        except lua.LuaError:
            keywords = None
        row = {'kind': methods[head[2]], 'receiver': head[0],
               'line': tokens[index][2], 'keywords': keywords, 'source_only': True,
               'literal_messages': None, 'expression': None}
        if head[2] != 'addAliasKeyword':
            second_start = index + 5 + len(first)
            second = first_argument(tokens, second_start)
            if head[2] == 'addGreetKeyword':
                parameters = second
            else:
                parameters = first_argument(tokens, second_start + len(second) + 1)
                row['module'] = ''.join(t[1] for t in second)
            row['parameters'] = ' '.join(t[1] for t in parameters)
            depth = 0
            for j, token in enumerate(parameters):
                if token[1] in ('{', '[', '('):
                    depth += 1
                elif token[1] in ('}', ']', ')'):
                    depth -= 1
                if depth == 1 and token[1] == 'text' and j + 1 < len(parameters) and parameters[j + 1][1] == '=':
                    row.update(speech(lua, parameters, j + 2))
                    break
        captured.append(row)
    return captured


def source_voices(text, root):
    sys.path.insert(0, str(root / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    tokens = [t for t in lua.tokenize(text) if t[0] not in ('comment', 'lcomment')]
    for index in range(len(tokens) - 4):
        if [t[1] for t in tokens[index:index + 5]] == ['npcConfig', '.', 'voices', '=', '{']:
            parser = lua.Parser(' '.join(t[1] for t in tokens[index + 4:]))
            table = lua.as_python(parser.value())
            return {'line': tokens[index][2], 'source_only': True, 'parameters': table}
    return None


def capture(crystal, root, *, families=None, schema='OTERYN_QUESTS_2_EMPEROR_DIALOGUE_CAPTURE/v1'):
    sys.path.insert(0, str(root / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    selected = FAMILIES if families is None else families
    entries = []
    files = subprocess.check_output(['git', '-C', str(crystal), 'ls-tree', '-r', '--name-only',
                                     PIN, '--', 'data-global/npc'], text=True).splitlines()
    for relative in sorted(files):
        if not relative.endswith('.lua'):
            continue
        raw = subprocess.check_output(['git', '-C', str(crystal), 'show', f'{PIN}:{relative}'])
        text = raw.decode()
        quests = sorted(quest for quest, family in selected.items() if family in text)
        if not quests:
            continue
        tokens = [t for t in lua.tokenize(text) if t[0] not in ('comment', 'lcomment')]
        replies = []
        for i in range(len(tokens) - 4):
            if [t[1] for t in tokens[i:i+4]] == ['npcHandler', ':', 'say', '(']:
                replies.append({'kind': 'say', 'line': tokens[i][2], **speech(lua, tokens, i + 4)})
            if [t[1] for t in tokens[i:i+4]] == ['npcHandler', ':', 'setMessage', '(']:
                first = first_argument(tokens, i + 4)
                start = i + 4 + len(first) + 1
                replies.append({'kind': 'set_message', 'message_type': ''.join(t[1] for t in first),
                                'line': tokens[i][2], **speech(lua, tokens, start)})
        entries.append({'npc': Path(relative).stem, 'quests': quests,
                        'source': f'zimbadev/crystalserver@{PIN}:{relative}:1',
                        'path': relative, 'blob_sha256': hashlib.sha256(raw).hexdigest(),
                        'replies': replies,
                        'static_keywords': static_keywords(text, root),
                        'voices': source_voices(text, root),
                        'source_string_fragments': [
                            {'line': token[2], 'text': lua._string(token)}
                            for token in tokens if token[0] in ('string', 'lstring')]})
    return {'schema': schema,
            'source_revision': PIN, 'classification': 'OTS_HYPOTHESIS_ONLY',
            'scope': ('Source speech, static keywords, child replies, greetings, aliases, voices and literal '
                      'fragments; dynamic expressions retained explicitly, not executed.'),
            'npcs': entries}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    result = capture(args.crystal, args.root)
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if args.out.read_text() != encoded:
            raise SystemExit('EMPEROR_DIALOGUE_CAPTURE_DRIFT')
    else:
        args.out.write_text(encoded)
    replies = [r for npc in result['npcs'] for r in npc['replies']]
    print(json.dumps({'npcs': len(result['npcs']), 'replies': len(replies),
                      'literal_messages': sum(len(r['literal_messages'] or []) for r in replies),
                      'dynamic_expressions': sum(r['expression'] is not None for r in replies)}))
