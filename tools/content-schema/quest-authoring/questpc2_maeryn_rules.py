"""Retained Maeryn helmet preference; no item enchantment or boss admission."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = '43b066a640d3b3d01a73cec683c0c0ddf7fce54e8e8491b0ddd80dc9dec954f8'
SOURCE = 'data-global/npc/maeryn.lua'
QUEST = 'oteryn:quest.grimvale_quest'
PROGRAM = 'oteryn:quest-npc-program.questpc2-maeryn-helmet-choice'
TRACK = 'oteryn:quest-progress/quest/u10_80/grimvale_quest/were_helmet_enchant'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-maeryn.json'
# Only preceding MsgContains branches can overlap a compound helmet/yes input.
# The preceding table.contains branches match the whole message, so cannot.
PRIOR = ('tokens', 'pitiful', 'changed', 'hope', 'tunnels', 'hunch', 'artefacts',
         'moon', 'nightshade', 'name', 'maeryn', 'time', 'job', 'grimvale', 'owin',
         'werewolves', 'gladys', 'cornell')


def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('MAERYN_SOURCE_PIN')
    text = raw.decode()
    lines = text.splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    topic = lambda n: dict(kind='set_topic', value=n)

    def rule(name, word, reply, effects, span, topics=None, exact=False):
        return dict(key='oteryn:quest-npc/grimvale/maeryn/' + name, quest=QUEST,
                    npc='oteryn:npc.maeryn', trigger='say', keywords=[word], guards=[],
                    reply=[quote(reply)], effects=effects, sources=[cite(span)],
                    exclude_keywords=list(PRIOR),
                    **({'keyword_mode': 'exact'} if exact else {}),
                    **({'topics': topics} if topics is not None else {}))

    rules = [rule('offer', 'werewolf helmet', 135, [topic(1)], '134-136'),
             rule('confirm', 'yes', 139, [topic(2)], '137-140', [1]),
             rule('knight', 'knight', 148, [topic(3)], '142-150', [2], True)]
    mapping = text.split('local vocations = {', 1)[1].split('local knightChoice', 1)[0]
    for name in ('sorcerer', 'druid', 'paladin', 'club', 'axe', 'sword'):
        value = int(re.search(r'\["' + name + r'"\]\s*=\s*(\d+)', mapping).group(1))
        weapon = name in ('club', 'axe', 'sword')
        rules.append(rule(name, name, 168 if weapon else 155,
                          [dict(kind='set', track=TRACK, value=value), topic(0)],
                          '161-173' if weapon else '142-159', [3 if weapon else 2], True))
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST,
                    tracks=[dict(key=TRACK, quest=QUEST, initial=-1, min=-1, max=5,
                                 sources=[cite('60-70'), cite('134-173')])], rules=rules,
                    source_files=[dict(path=SOURCE, revision=REV, sha256=SHA)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
