"""Pinned Henricus recruitment/interrogation dialogue using the existing source owner."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = '1b98530e7f7ae4bef44e9d55373254e6590a160c7c8f5d8d9dd41b578bf7977d'
SOURCE = 'data-global/npc/henricus.lua'
QUEST = 'oteryn:quest.the_inquisition_quest'
PROGRAM = 'oteryn:quest-npc-program.questpc2-henricus-inquisition'
PREFIX = 'oteryn:quest-progress/quest/u8_2/the_inquisition_quest/'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-henricus.json'
FLAGS = ('walter_guard', 'kulag_guard', 'grof_guard', 'miles_guard', 'tim_guard')


def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('HENRICUS_SOURCE_PIN')
    lines = raw.decode().splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    guard = lambda track, op, value: {'kind': 'storage', 'track': PREFIX + track, 'op': op, 'value': value}
    set_value = lambda track, value: {'kind': 'set', 'track': PREFIX + track, 'value': value}
    topic = lambda value: {'kind': 'set_topic', 'value': value}
    def rule(name, words, quoted, effects, guards, span, topics=()):
        return dict(key=f'oteryn:quest-npc/the_inquisition/henricus/{name}', quest=QUEST,
                    npc='oteryn:npc.henricus', trigger='say', keywords=words, guards=guards,
                    reply=[quote(n) for n in quoted], effects=effects, sources=[cite(span)],
                    **({'topics': list(topics)} if topics else {}))
    words = ['mission', 'report']
    rules = [
        rule('inquisitor', ['inquisitor'], [66], [], [], '65-66'),
        rule('join-offer', ['join'], [69], [topic(2)], [guard('questline', 'lt', 1)], '67-71'),
        rule('join-noop', ['join'], [], [], [guard('questline', 'ge', 1)], '67-71'),
        rule('bless-refuse', ['blessing', 'bless'], [77], [topic(0)], [guard('questline', 'ne', 25)], '72-79'),
        rule('flask-refuse', ['flask', 'special flask'], [85], [topic(0)], [guard('questline', 'lt', 12)], '80-87'),
        rule('mission-offer', words, [90], [topic(2)], [guard('questline', 'lt', 1)], '88-91'),
        rule('mission-start', words, [94, 95], [set_value('questline', 2), set_value('mission01', 1), topic(0)], [guard('questline', 'eq', 1)], '92-99'),
        rule('mission-report', words, [101], [topic(3)], [guard('questline', 'eq', 2)], '100-102'),
        rule('join-accept', ['yes'], [205], [set_value('questline', 1), topic(0)], [], '203-207', (2,)),
        rule('report-accept', ['yes'], [217, 218], [set_value('questline', 3), set_value('mission01', 7), topic(0)], [guard(flag, 'eq', 1) for flag in FLAGS], '208-225', (3,)),
    ]
    rules.extend(rule('report-missing-' + flag, ['yes'], [223], [topic(0)],
                      [guard(flag, 'ne', 1)], '208-225', (3,)) for flag in FLAGS)
    rules.append(rule('decline', ['no'], [288], [topic(0)], [], '286-290', (2, 3)))
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    wanted = {PREFIX + key for key in ('questline', 'mission01', *FLAGS)}
    tracks = [{**{key: track[key] for key in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [cite('65-225')]} for quest in state['quests'] if quest['quest'] == QUEST
              for track in quest['tracks'] if track['key'] in wanted]
    if len(tracks) != len(wanted):
        raise ValueError('HENRICUS_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
