"""Source-bound Oldrak NPC steps; no defender-wave or axe reward admission."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = 'dc124275de082d9f39bc2f802b31f823646548153fe0e248417dc54f7b0eba40'
SOURCE = 'data-global/npc/oldrak.lua'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-oldrak.json'
PROGRAM = 'oteryn:quest-npc-program.questpc2-oldrak-demon-oak'
QUEST = 'oteryn:quest.demon_oak'
PREFIX = 'oteryn:quest-progress/quest/u8_2/the_demon_oak/'

def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('OLDRAK_SOURCE_PIN')
    lines = raw.decode().splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    guard = lambda op, value: {'kind': 'storage', 'track': PREFIX + 'done', 'op': op, 'value': value}
    set_value = lambda key, value: {'kind': 'set', 'track': PREFIX + key, 'value': value}
    topic = lambda value: {'kind': 'set_topic', 'value': value}
    def rule(name, words, reply, effects, guards, span, topics=()):
        return dict(key=f'oteryn:quest-npc/demon_oak/oldrak/{name}', quest=QUEST, npc='oteryn:npc.oldrak',
                    trigger='say', keywords=words, guards=guards, reply=reply, effects=effects,
                    sources=[cite(span)], **({'topics': list(topics)} if topics else {}))
    words = ['mission', 'demon oak']
    rules = [rule('offer', words, [quote(104)], [topic(1)], [guard('lt', 1)], '102-105'),
             rule('finish', words, [quote(n) for n in (110, 111, 112)], [set_value('done', 2)], [guard('eq', 1)], '108-114'),
             rule('mission-noop', words, [], [], [guard('ge', 2)], '102-115'),
             rule('mission-accept', ['yes'], [quote(119)], [set_value('progress', 2), topic(0)], [], '116-121', (1,)),
             rule('decline', ['no'], [quote(152)], [topic(0)], [], '151-153', (1,))]
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')}, 'sources': [cite('103-121')]}
              for q in state['quests'] if q['quest'] == QUEST for t in q['tracks'] if not t['key'].endswith('/squares')]
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}

if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
