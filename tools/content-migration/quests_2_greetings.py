#!/usr/bin/env python3
"""Extract the closed batch-2 NPC greetings from the unchanged Crystal pin.

Conditional variants are retained separately; native quest callers choose them.
This reads literal source strings and never executes donor code.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
NPCS = (
    'mr_morris', 'woblin', 'jerom', 'the_queen_of_the_banshees',
    'alkestios', 'ikassis', 'ghostly_wolf', 'a_swan', 'chief_grarkharok',
    'aurita', 'taegen', 'valindara', 'tired_tree', 'grumpy_stone',
    'scrutinon', 'spectulus', 'a_sleeping_dragon', 'jack_springer', 'flickering_soul', 'rottin_wood',
    'maelyrra', 'dancing_fairy', 'tooth_fairy',
)
VARIANTS = {
    'spectulus': {'default': 67, 'sea-of-light-complete': 70},
    'a_sleeping_dragon': {'default': 53, 'dream': 51},
    'jack_springer': {'default': 65, 'started': 67},
}


def pinned(crystal, path):
    return subprocess.check_output(['git', '-C', str(crystal), 'show', f'{PIN}:{path}'])


def source(path, line, blob):
    return {'repository': 'zimbadev/crystalserver', 'revision': PIN,
            'path': path, 'line': line, 'sha256': hashlib.sha256(blob).hexdigest()}


def message(raw, kind, path, selected=None):
    sys.path.insert(0, str(ROOT / 'tools/content-schema/quest-authoring'))
    import lua_tables as lua
    tokens = [token for token in lua.tokenize(raw.decode()) if token[0] not in ('comment', 'lcomment')]
    matches = []
    for i, token in enumerate(tokens):
        values = [part[1] for part in tokens[i:i+6]]
        literal = None
        if values == ['npcHandler', ':', 'setMessage', '(', kind, ',']:
            literal = tokens[i+6]
        elif values[:4] == ['[', kind, ']', '=']:
            literal = tokens[i+4]
        if literal and literal[0] in ('string', 'lstring') and (selected is None or token[2] == selected):
            matches.append((token[2], lua._string(literal)))
    if len(matches) != 1:
        raise ValueError(f'QUEST_GREETING_LITERAL:{path}:{kind}:{selected}:{len(matches)}')
    line, text = matches[0]
    return {'text': text, 'source': source(path, line, raw)}


def build(crystal):
    defaults_path = 'data/npclib/npc_system/npc_handler.lua'
    defaults = pinned(crystal, defaults_path)
    entries = []
    for npc in NPCS:
        path = f'data-global/npc/{npc}.lua'
        raw = pinned(crystal, path)
        entry = {'npc': 'oteryn:npc.' + npc, 'variants': {}}
        for field, kind in [('greet', 'MESSAGE_GREET'), ('farewell', 'MESSAGE_FAREWELL'), ('walkaway', 'MESSAGE_WALKAWAY')]:
            selected = VARIANTS.get(npc, {}).get('default') if field == 'greet' else None
            present = re.search(r'setMessage\(\s*' + kind + r'\s*,', raw.decode())
            entry[field] = message(raw, kind, path, selected) if present else message(defaults, kind, defaults_path)
        for name, line in VARIANTS.get(npc, {}).items():
            if name != 'default':
                entry['variants'][name] = message(raw, 'MESSAGE_GREET', path, line)
        entries.append(entry)
    return {'schema': 1, 'source_revision': PIN, 'npcs': entries,
            'placeholder_source': source(defaults_path, 383, defaults)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    text = json.dumps(build(args.crystal), ensure_ascii=False, indent=2) + '\n'
    target = args.root / 'content/dialogues/quests-2/npc-greetings.json'
    if args.check:
        if target.read_text() != text:
            raise SystemExit('QUEST_GREETING_DRIFT')
    else:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
