"""Pinned Gnomus ordnance admission; no escort completion or reward lowering."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = '04ce96e16bbfd9f6fa6b114b377643b1bb71c3df33f05c2b1e1d0d5e40bdc5a1'
SOURCE = 'data-global/npc/gnomus.lua'
QUEST = 'oteryn:quest.dangerous_depths_quest'
PROGRAM = 'oteryn:quest-npc-program.questpc2-gnomus-ordnance'
PREFIX = 'oteryn:quest-progress/quest/u11_50/dangerous_depths/'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-gnomus.json'
UNOWNED = ('measurements', 'charting', 'suspicious device', 'status', 'gnomes',
           'dwarves', 'scouts')


def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('GNOMUS_SOURCE_PIN')
    lines = raw.decode().splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    guard = lambda key, op, value: dict(kind='storage', track=PREFIX + key, op=op, value=value)
    write = lambda key, value: dict(kind='set', track=PREFIX + key, value=value)
    topic = lambda value: dict(kind='set_topic', value=value)

    def rule(name, word, reply, effects, guards, span, topics=None):
        return dict(key='oteryn:quest-npc/dangerous_depths/gnomus/' + name, quest=QUEST,
                    npc='oteryn:npc.gnomus', trigger='say', keywords=[word], guards=guards,
                    reply=[quote(n) for n in reply], effects=effects, sources=[cite(span)],
                    exclude_keywords=list(UNOWNED),
                    **({'topics': topics} if topics is not None else {}))

    mission = 'gnomes/ordnance'
    cooldown = 'gnomes/time_task_ordnance'
    rules = [
        rule('cooldown', 'ordnance', [121], [topic(1)],
             [guard(mission, 'eq', 3), guard(cooldown, 'gt', 0)], '119-123'),
        rule('repeat-offer', 'ordnance', list(range(126, 131)), [topic(22)],
             [guard(mission, 'eq', 3), guard(cooldown, 'le', 0)], '124-133'),
        rule('initial-offer', 'ordnance', list(range(136, 141)), [topic(22)],
             [guard(mission, 'lt', 1)], '134-142'),
        rule('pending-first', 'ordnance', [144], [topic(1)],
             [guard(mission, 'eq', 1)], '143-145'),
        rule('pending-escort', 'ordnance', [144], [topic(1)],
             [guard(mission, 'eq', 2), guard('gnomes/gnomes_count', 'lt', 5)], '143-145'),
    ]
    for first, op in ((True, 'lt'), (False, 'ge')):
        effects = ([write('questline', 1)] if first else []) + [
            write(mission, 1), write('gnomes/gnomes_count', 0),
            write('gnomes/crawlers_count', 0), topic(1)]
        entry = rule('accept-first' if first else 'accept-later', 'yes', [167, 168],
                     effects, [guard('questline', op, 1)], '165-176', [22])
        # The preceding ordnance if must consume a compound "ordnance yes".
        entry['exclude_keywords'].append('ordnance')
        rules.append(entry)
    # Lane admission policy, not a claimed donor quest effect: another task's
    # unsupported interaction cannot retain authority to confirm ordnance later.
    for index, word in enumerate(UNOWNED):
        boundary = rule(f'unsupported-task-{index}', word, [], [topic(0)], [], '63-307')
        boundary['exclude_keywords'] = []
        rules.append(boundary)
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    wanted = {PREFIX + key for key in ('questline', mission, cooldown,
              'gnomes/gnomes_count', 'gnomes/crawlers_count')}
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [cite('119-176')]} for q in state['quests'] if q['quest'] == QUEST
              for t in q['tracks'] if t['key'] in wanted]
    if len(tracks) != len(wanted):
        raise ValueError('GNOMUS_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
