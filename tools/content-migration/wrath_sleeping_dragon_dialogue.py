"""Extract the finite dream conversation from the previously captured pinned NPC."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
PATH = 'data-global/npc/a_sleeping_dragon.lua'


def build(crystal, capture_path):
    capture = json.loads(capture_path.read_bytes())
    npc = next(npc for npc in capture['npcs'] if npc['npc'] == 'a_sleeping_dragon')
    raw = subprocess.check_output(['git', '-C', str(crystal), 'show', f'{PIN}:{PATH}'])
    if npc['blob_sha256'] != hashlib.sha256(raw).hexdigest():
        raise ValueError('Sleeping Dragon source differs from speech capture')
    lines = raw.decode().splitlines()
    replies = {row['line']: row['literal_messages'] for row in npc['replies']}
    steps = []
    for index, line in enumerate(lines):
        match = re.search(r'elseif message:lower\(\) == "([^"]+)" and npcHandler:getTopic\(playerId\) == (\d+) then', line)
        if match is None:
            continue
        keyword, topic = match.groups()
        topic = int(topic)
        end = next((j for j in range(index + 1, len(lines))
                    if 'elseif message:lower()' in lines[j] or '\t\tend' == lines[j]), len(lines))
        reply = next(replies[j + 1] for j in range(index + 1, end) if j + 1 in replies)
        target = next(int(re.search(r'setTopic\(playerId, (\d+)\)', lines[j]).group(1))
                      for j in range(index + 1, end) if 'setTopic(playerId,' in lines[j])
        steps.append({'topic': topic, 'keyword': keyword, 'next_topic': target,
                      'messages': reply, 'source_line': index + 1})
    if [step['topic'] for step in steps] != list(range(1, 33)):
        raise ValueError('Sleeping Dragon dream chain differs')
    entry_line = next(i + 1 for i, line in enumerate(lines) if '(message == "SOLOSARASATIQUARIUM")' in line)
    entry_reply = next(reply for line, reply in sorted(replies.items()) if line > entry_line)
    help_line = next(i + 1 for i, line in enumerate(lines) if 'message:lower() == "help"' in line)
    help_reply = next(reply for line, reply in sorted(replies.items()) if line > help_line)
    return {'schema': 1, 'source': f'zimbadev/crystalserver@{PIN}:{PATH}:67-198',
            'source_blob_sha256': npc['blob_sha256'], 'npc': 'oteryn:npc.a_sleeping_dragon',
            'entry_keyword': 'SOLOSARASATIQUARIUM', 'entry_messages': entry_reply,
            'help_messages': help_reply, 'steps': steps}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--crystal', type=Path, required=True)
    parser.add_argument('--capture', type=Path, default=ROOT / 'content/dialogues/quests-2/emperor-donor-dialogues.json')
    parser.add_argument('--out', type=Path, default=ROOT / 'content/dialogues/quests-2/wrath-sleeping-dragon.json')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    packet = build(args.crystal, args.capture)
    encoded = json.dumps(packet, indent=2, ensure_ascii=False) + '\n'
    if args.check:
        if args.out.read_text() != encoded:
            raise SystemExit('SLEEPING_DRAGON_DIALOGUE_DRIFT')
    else:
        args.out.write_text(encoded)
    messages = packet['entry_messages'] + packet['help_messages'] + [message for step in packet['steps'] for message in step['messages']]
    print(json.dumps({'steps': len(packet['steps']), 'messages': len(messages),
                      'max_message_bytes': max(len(message.encode()) for message in messages)}))
