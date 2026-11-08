"""Pinned Gerimor assignment starts; cult boss reports and rewards stay unqualified."""
import hashlib
import json
from pathlib import Path
import re
import tarfile

REV = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
SHA = 'bf0fba86d07bb8a70d6242c2cc748fbb137cc7b583b47f0886530b6b5c584b2f'
SOURCE = 'data-global/npc/gerimor.lua'
QUEST = 'oteryn:quest.cults_of_tibia_quest'
PROGRAM = 'oteryn:quest-npc-program.questpc2-gerimor-assignments'
PREFIX = 'oteryn:quest-progress/quest/u11_40/cults_of_tibia/'
OUTPUT = 'content/dialogues/quest-rules-supplemental/questpc2-gerimor.json'
CULTS = ('minotaurs', 'prosperity', 'barkless', 'orcs', 'life', 'misguided', 'humans')


def outputs(root):
    with tarfile.open(root / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry = next(r for r in manifest['files'] if r['revision'] == REV and r['path'] == SOURCE)
        raw = archive.extractfile(entry['cache_path']).read()
    if entry['sha256'] != SHA or hashlib.sha256(raw).hexdigest() != SHA:
        raise ValueError('GERIMOR_SOURCE_PIN')
    lines = raw.decode().splitlines()
    quote = lambda n: json.loads(re.search(r'"(?:[^"\\]|\\.)*"', lines[n - 1]).group())
    cite = lambda span: f'crystal:{REV}:{SOURCE}:{span}'
    guard = lambda key, op, value: dict(kind='storage', track=PREFIX + key, op=op, value=value)
    write = lambda key, value: dict(kind='set', track=PREFIX + key, value=value)
    topic = lambda value: dict(kind='set_topic', value=value)
    rules = []

    def rule(name, word, reply, effects, guards, span, topics=None, exact=False):
        rules.append(dict(key='oteryn:quest-npc/cults_of_tibia/gerimor/' + name, quest=QUEST,
                          npc='oteryn:npc.gerimor', trigger='say', keywords=[word],
                          keyword_mode='exact' if exact else 'msg_contains', guards=guards,
                          reply=[quote(n) for n in reply], effects=effects, sources=[cite(span)],
                          **({'topics': topics} if topics is not None else {})))

    # A selected topic encodes the Lua's volatile per-player storage[playerId].
    # No selected cult is persisted until the source confirmation commits.
    selected = []
    for index, cult in enumerate(CULTS):
        begin = next(i for i, line in enumerate(lines) if f'["{cult}"] = {{' in line)
        end = next(i for i in range(begin, len(lines)) if 'rewardExp =' in lines[i])
        block = '\n'.join(lines[begin:end])
        suffix = 'mot_a' if cult == 'prosperity' else cult
        track = suffix + '/mission'
        goal = int(re.search(r'value = (\d+)', block).group(1))
        quotes = [i + 1 for i in range(begin + 2, end) if lines[i].lstrip().startswith('"')]
        # Only the first text array is the assignment offer, never completeText.
        offer_end = next(i for i in range(begin + 2, end) if lines[i].strip() == '},')
        quotes = [i for i in quotes if i <= offer_end]
        chosen = 300 + index
        selected.append(chosen)
        rule('menu-' + cult, 'mission', [388], [topic(2)],
             [guard('final_boss/mission', 'lt', 2), guard(track, 'ne', goal + 1)], '355-390')
        rule('offer-' + cult, cult, quotes, [topic(chosen)], [guard(track, 'lt', goal)],
             f'{begin + 1}-{end + 1}', [2], exact=True)
        for first, op in ((True, 'lt'), (False, 'ge')):
            rule('accept-' + cult + ('-first' if first else '-later'), 'yes', [414],
                 [write(track, 1)] + ([write('questline', 1)] if first else []) + [topic(2)],
                 [guard(track, 'lt', 1), guard('questline', op, 1)], '411-421', [chosen])
        rule('pending-' + cult, 'yes', [421], [topic(2)], [guard(track, 'gt', 0)],
             '420-425', [chosen])
    # Topic 2 enters the table lookup branch, including for the word "no".
    rule('decline', 'no', [450], [topic(0)], [], '449-452', [0, *selected])
    rules[-1]['exclude_keywords'] = ['mission', 'yes']
    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    wanted = {PREFIX + 'questline', PREFIX + 'final_boss/mission',
              *(PREFIX + ('mot_a' if cult == 'prosperity' else cult) + '/mission' for cult in CULTS)}
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [cite('255-452')]} for q in state['quests'] if q['quest'] == QUEST
              for t in q['tracks'] if t['key'] in wanted]
    if len(tracks) != len(wanted):
        raise ValueError('GERIMOR_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
