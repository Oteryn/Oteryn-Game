"""Source-pinned Albinius temple contribution; portal and rune rites are separate."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = '1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18'
SOURCE = 'data-global/npc/albinius.lua'
QUEST = 'oteryn:quest.forgotten_knowledge_quest'
PROGRAM = 'oteryn:quest-npc-program.questpc2-albinius-tomes'
TRACK = 'oteryn:quest-progress/quest/u11_02/forgotten_knowledge/tomes'
ITEM = 'oteryn:item.tibia.i23986'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-albinius.json'
UNOWNED = ('buy', 'astral shaper rune', 'ice portal', 'holy portal', 'energy portal',
           'earth portal', 'death portal')


def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('ALBINIUS_SOURCE_PIN')
    lines = raw.decode().splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    guard = lambda op, value: dict(kind='storage', track=TRACK, op=op, value=value)
    topic = lambda value: dict(kind='set_topic', value=value)
    item_guard = lambda op: dict(kind='item', item=ITEM, count=5, op=op)

    def rule(name, word, reply, effects, guards, span, topics=None):
        return dict(key='oteryn:quest-npc/forgotten_knowledge/albinius/' + name, quest=QUEST,
                    npc='oteryn:npc.albinius', trigger='say', keywords=[word], guards=guards,
                    reply=[quote(n) for n in reply], effects=effects, sources=[cite(span)],
                    continue_matching=True, exclude_keywords=list(UNOWNED),
                    **({'topics': topics} if topics is not None else {}))

    # The source uses independent ifs: "temple yes" can offer and commit together.
    # Success deliberately retains topic1 until the next already-donated reply.
    rules = [
        rule('shapers', 'shapers', [107, 108], [], [], '105-110'),
        rule('temple', 'temple', [113], [topic(1)], [], '112-115'),
        rule('already', 'yes', [118], [topic(0)], [guard('eq', 1)], '116-120', [1]),
        rule('contribute', 'yes', [123],
             [dict(kind='take_item', item=ITEM, count=5, optional=False),
              dict(kind='set', track=TRACK, value=1)],
             [guard('ne', 1), item_guard('ge')], '120-124', [1]),
        rule('missing', 'yes', [126], [], [guard('ne', 1), item_guard('lt')], '120-128', [1]),
        rule('decline', 'no', [130], [dict(kind='end_conversation', delay_ms=0)], [], '129-132', [1]),
        rule('tomes', 'tomes', [135], [topic(7)], [guard('lt', 1)], '134-137'),
    ]
    rules[5]['exclude_keywords'].append('yes')
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [cite('116-124')]} for q in state['quests'] if q['quest'] == QUEST
              for t in q['tracks'] if t['key'] == TRACK]
    if len(tracks) != 1:
        raise ValueError('ALBINIUS_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
