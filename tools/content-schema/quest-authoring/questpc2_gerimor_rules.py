"""Pinned Gerimor assignments, reports and finale settlement."""
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
FINALE_REV = '201189f871d23597b82caf88fd611454a2ddefb8'
CROWNS = {'druid': 26187, 'knight': 26188, 'monk': 50313,
          'paladin': 26189, 'sorcerer': 26190}


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
    reports = []
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
        reward = int(re.search(r'rewardExp = (\d+)', lines[end]).group(1))
        reports.append((cult, track, goal, reward, f'{begin + 1}-{end + 1}'))
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
    def finale_rule(name, word, reply, effects, guards, span, topics=None, exact=False):
        rules.append(dict(
            key='oteryn:quest-npc/cults_of_tibia/gerimor/' + name,
            quest=QUEST, npc='oteryn:npc.gerimor', trigger='say', keywords=[word],
            keyword_mode='exact' if exact else 'msg_contains', guards=guards,
            reply=[reply], effects=effects,
            sources=[f'crystal:{FINALE_REV}:{SOURCE}:{span}'],
            **({'topics': topics} if topics is not None else {}),
        ))

    completed_reports = []
    for cult, track, goal, reward, span in reports:
        completed_reports.append(guard(track, 'eq', goal + 1))
        finale_rule('report-' + cult, cult, 'Thank you for your report.',
                    [write(track, goal + 1), dict(kind='experience', amount=reward), topic(0)],
                    [guard(track, 'eq', goal)], span, [2], exact=True)
        finale_rule('reported-' + cult, cult, 'You have already reported on that cult.',
                    [topic(2)], [guard(track, 'gt', goal)], '402-404', [2], exact=True)

    mission = 'final_boss/mission'
    door = 'final_boss/access_door'
    finale_rule('finale-access', 'mission', 'The way to the final source is open.',
                [write(mission, 1), write(door, 1), topic(0)],
                [*completed_reports, guard(mission, 'lt', 1)], '356-378')
    finale_rule('finale-in-progress', 'mission', 'The final source still awaits you.',
                [topic(0)], [*completed_reports, guard(mission, 'eq', 1)], '359-378')
    finale_rule('finale-offer', 'mission', 'Are you ready to receive your reward?',
                [topic(4)], [guard(mission, 'eq', 2)], '379-386')
    finale_rule('finale-decline', 'no', 'Return when you are ready.',
                [topic(0)], [], '449-452', [4])
    finale_rule('finale-completed', 'mission', 'You have already claimed your reward.',
                [topic(0)], [guard(mission, 'gt', 2)], '356-358')
    for vocation, crown in [('none', None), *CROWNS.items()]:
        gifts = [dict(kind='give_item', item='oteryn:item.tibia.i26186', count=1)]
        if crown is not None:
            gifts.insert(0, dict(kind='give_item', item=f'oteryn:item.tibia.i{crown}', count=1))
        finale_rule('finale-reward-' + vocation, 'yes', 'Thank you. Here is your reward.',
                    gifts + [dict(kind='experience', amount=50_000),
                             dict(kind='achievement', name='Corruption Contained'),
                             write(mission, 3), topic(0)],
                    [guard(mission, 'eq', 2),
                     dict(kind='vocation', value='oteryn:vocation.' + vocation)],
                    '425-447', [4])
        rules[-1]['inventory_failure_reply'] = ['Please make room for your reward.']

    state = json.loads((root / 'content/quests/missions/quest-state.json').read_bytes())
    wanted = {PREFIX + 'questline', PREFIX + 'final_boss/mission', PREFIX + door,
              *(PREFIX + ('mot_a' if cult == 'prosperity' else cult) + '/mission' for cult in CULTS)}
    tracks = [{**{k: t[k] for k in ('key', 'quest', 'initial', 'min', 'max')},
               'sources': [f'crystal:{FINALE_REV}:{SOURCE}:373-377']
                           if t['key'] == PREFIX + door else [cite('255-452')]}
              for q in state['quests'] if q['quest'] == QUEST
              for t in q['tracks'] if t['key'] in wanted]
    report_bounds = {PREFIX + track: goal + 1 for _, track, goal, _, _ in reports}
    for track in tracks:
        track['max'] = max(track['max'], report_bounds.get(track['key'], track['max']))
    if len(tracks) != len(wanted):
        raise ValueError('GERIMOR_TRACK_SET')
    document = dict(schema=1, program_key=PROGRAM, quest=QUEST, tracks=tracks, rules=rules,
                    source_files=[dict(path=SOURCE, sha256=SHA, revision=revision)
                                  for revision in (REV, FINALE_REV)])
    return {OUTPUT: json.dumps(document, ensure_ascii=False, indent=2) + '\n'}


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[3]
    for path, text in outputs(root).items():
        (root / path).write_text(text)
