"""Pinned Albinius contribution and four portal offerings; no ice or rune rite."""
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
UNOWNED = ('buy', 'astral shaper rune', 'ice portal')
PORTALS = (
    ('holy', 'access_golden', 18929, 3, 197),
    ('energy', 'access_violet', 17462, 4, 219),
    ('earth', 'access_earth', 10296, 5, 241),
    ('death', 'access_death', 11481, 6, 263),
)


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
    for name, suffix, raw_item, number, line in PORTALS:
        track = TRACK.rsplit('/', 1)[0] + '/' + suffix
        item = f'oteryn:item.tibia.i{raw_item}'
        storage = lambda op: dict(kind='storage', track=track, op=op, value=1)
        payment = lambda op: dict(kind='item', item=item, count=50, op=op)
        span = f'{line - 3}-{line + 17}'
        rules.extend([
            rule(name + '-offer', name + ' portal', [line], [topic(number)],
                 [guard('eq', 1)], span),
            rule(name + '-prerequisite', name + ' portal', [line + 3], [],
                 [guard('ne', 1)], span),
            # Failure alternatives precede success so the committed access write
            # cannot make an else branch match later in this same source if.
            rule(name + '-already', 'yes', [line + 13], [],
                 [storage('ge')], span, [number]),
            rule(name + '-missing', 'yes', [line + 13], [],
                 [storage('lt'), payment('lt')], span, [number]),
            rule(name + '-accept', 'yes', [line + 10],
                 [dict(kind='take_item', item=item, count=50, optional=False),
                  dict(kind='set', track=track, value=1)],
                 [storage('lt'), payment('ge')], span, [number]),
            rule(name + '-decline', 'no', [line + 16], [], [], span, [number]),
        ])
        # Source elseif yes takes priority; no does not reset the portal topic.
        rules[-1]['exclude_keywords'].append('yes')
    wanted = {TRACK} | {TRACK.rsplit('/', 1)[0] + '/' + row[1] for row in PORTALS}
    track_spans = {TRACK: '116-124', **{TRACK.rsplit('/', 1)[0] + '/' + row[1]: f'{row[4] - 3}-{row[4] + 17}' for row in PORTALS}}
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [cite(track_spans[t['key']])]} for q in state['quests'] if q['quest'] == QUEST
              for t in q['tracks'] if t['key'] in wanted]
    if len(tracks) != len(wanted):
        raise ValueError('ALBINIUS_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
