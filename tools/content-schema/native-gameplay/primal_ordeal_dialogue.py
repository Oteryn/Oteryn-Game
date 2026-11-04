#!/usr/bin/env python3
"""Capture every Gnomadness speech from Crystal201189f without executing Lua.

The numeric source concatenations remain explicit named text templates. This
source record does not admit a hazard selection, mount, bag or achievement.
PR2844 ANSWER6589521017 reserves the composite runtime owner for a later PR.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
PATH = 'data-global/npc/gnomadness.lua'
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
VARIABLES = {'current': '|HAZARD_CURRENT|', 'maximum': '|HAZARD_MAXIMUM|', 'desiredLevel': '|HAZARD_DESIRED|'}
SPEECHES = (('hazard', 74, 'hazard'), ('invalid_level', 80, 'level'),
            ('level_selected', 85, 'level'), ('prize', 92, 'level'), ('above_maximum', 95, 'level'))


def speech(expression):
    """A closed sequence of string literals and three source numeric locals."""
    tokens = re.split(r'\s*\.\.\s*', expression)
    result = []
    variables = []
    for token in tokens:
        token = token.strip()
        if STRING.fullmatch(token):
            result.append(json.loads(token))
        elif token in VARIABLES:
            result.append(VARIABLES[token])
            variables.append(token)
        else:
            raise ValueError('Unqualified Gnomadness speech expression')
    if not result:
        raise ValueError('Empty Gnomadness speech')
    return ''.join(result), variables


def build(crystal):
    raw = subprocess.run(['git', '-C', str(crystal), 'show', f'{PIN}:{PATH}'],
                         check=True, capture_output=True).stdout
    lines = raw.decode().splitlines()
    keywords, witnesses = [], []
    for key, line, trigger in SPEECHES:
        match = re.fullmatch(r'\s*npcHandler:say\((.*), npc, creature\)', lines[line - 1])
        if not match:
            raise ValueError(f'Pinned Gnomadness speech moved: {line}')
        text, variables = speech(match[1])
        keywords.append({'key': key, 'triggers': [trigger], 'reply': [text]})
        witnesses.append({'key': key, 'line': line, 'source_expression': match[1], 'variables': variables})
    greet = re.fullmatch(r'keywordHandler:addGreetKeyword\(\{ "hi" \}, \{ npcHandler = npcHandler, text = ("(?:[^"\\]|\\.)*") \}\)', lines[101])
    default = re.fullmatch(r'npcHandler:setMessage\(MESSAGE_GREET, ("(?:[^"\\]|\\.)*")\)', lines[104])
    notice = re.fullmatch(r'\s*player:sendTextMessage\(MESSAGE_EVENT_ADVANCE, ("(?:[^"\\]|\\.)*")\)', lines[87])
    if not greet or not default or not notice:
        raise ValueError('Pinned Gnomadness greeting or reward notice moved')
    source = {'repository': 'zimbadev/crystalserver', 'revision': PIN, 'path': PATH,
              'sha256': hashlib.sha256(raw).hexdigest(), 'speech': witnesses,
              'greet_lines': [102, 103, 105], 'reward_notice_line': 88}
    declaration = {'kind': 'Dialogue', 'identity': {'key': 'oteryn:dialogue.npc.gnomadness', 'revision': 'definition-r1'},
                   'greet': [json.loads(greet[1])], 'keywords': keywords,
                   'fields': [
                       {'field_path': 'oteryn:source.quest.primal_ordeal.dialogue',
                        'value': {'type': 'Text', 'value': json.dumps(source, ensure_ascii=False, sort_keys=True)}},
                       {'field_path': 'oteryn:source.quest.primal_ordeal.default_greeting',
                        'value': {'type': 'Text', 'value': json.loads(default[1])}},
                       {'field_path': 'oteryn:source.quest.primal_ordeal.reward_notice',
                        'value': {'type': 'Text', 'value': json.loads(notice[1])}},
                       {'field_path': 'oteryn:runtime.quest.primal_ordeal.hold',
                        'value': {'type': 'Text', 'value': 'https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017 point5: no composite hazard/mount/bag owner in this batch'}},
                   ]}
    return {'schema': 'OTERYN_DIALOGUE_AUTHORING_SHARD/v1', 'family': 'Dialogue',
            'shard': {'index': 0, 'start': 0, 'end': 0, 'count': 1}, 'source_legacy_role': PATH,
            'records': [{'declaration': declaration}]}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = ROOT / 'content/dialogues/primal-ordeal.json'
    expected = json.dumps(build(args.crystal), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if output.read_text() != expected:
            raise SystemExit('Stale Primal Ordeal dialogue')
    else:
        output.write_text(expected)
