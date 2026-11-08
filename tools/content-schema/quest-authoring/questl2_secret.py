"""Secret Service chief branches, bound to the pinned NPC assignments and replies.

World objectives remain separate native callbacks: reporting cannot manufacture
their required mission progress. The same global Quest and faction guards apply
to all three chiefs, including the shared final mission.
"""
import json
import re

NPCS = ('chester_kahs', 'emma', 'uncle')
PREFIX = 'u8_1/secret_service/'
FACTIONS = {
    'tbi': dict(npc='chester_kahs', join=5, topics=[6, 7, 8, 9, 10, 11, 13],
                before=[2, 1, 2, 1, 2, 2, 1], after=[3, 2, 3, 2, 3, 3, 2],
                take={2: 5956, 3: 5952, 4: 348, 7: 396}, give={5: 349, 6: 397}, reward=897),
    'cgb': dict(npc='emma', join=1, topics=[2, 3, 4, 5, 6, 7, 8],
                before=[1, 1, 2, 1, 1, 1, 1], after=[2, 2, 3, 2, 2, 2, 2],
                take={1: 648, 2: 652, 4: 399, 5: 400, 6: 401, 7: 396},
                give={3: 350}, reward=898),
    'avin': dict(npc='uncle', join=1, topics=[2, 3, 4, 5, 6, 7, 8],
                 before=[3, 1, 3, 2, 1, 2, 1], after=[4, 2, 4, 3, 2, 3, 2],
                 take={2: 403, 5: 406, 7: 396}, give={1: 402, 3: 404, 6: 405}, reward=899),
}
ITEMS = {i for spec in FACTIONS.values()
         for i in [*spec['take'].values(), *spec['give'].values(), spec['reward']]}


def _speech(lines, start):
    active = False
    for number in range(start, len(lines)):
        line = lines[number]
        if 'npcHandler:say(' in line:
            active = True
            line = line.split('npcHandler:say(', 1)[1].lstrip()
        if active:
            match = re.match(r'\s*"((?:\\.|[^"\\])*)"', line)
            if match:
                raw = match[1]
                return json.loads('"' + raw.replace("\\'", "'") + '"'), raw, number + 1
    raise ValueError('Secret Service source speech absent')


def _phase(lines, value):
    marker = f'player:setStorageValue(Storage.Quest.U8_1.SecretService.Quest, {value})'
    matches = [i for i, line in enumerate(lines) if marker in line]
    if len(matches) != 1:
        raise ValueError(f'Secret Service source phase assignment changed: {value}')
    return _speech(lines, matches[0] + 1)


def _prompt(lines, field, value):
    marker = f'player:getStorageValue(Storage.Quest.U8_1.SecretService.{field}) == {value}'
    mission = next(i for i, line in enumerate(lines) if 'MsgContains(message, "mission")' in line)
    start = next(i for i in range(mission, len(lines)) if marker in lines[i])
    return _speech(lines, start + 1)


def append(add, state, callbacks, offers, inputs, pin):
    quest = 'secret_service_quest'
    global_track = PREFIX + 'quest'

    def mission_field(faction, number):
        return 'Mission07' if number == 7 else faction.upper() + f'Mission{number:02}'

    def mission_track(faction, number):
        return PREFIX + ('mission07' if number == 7 else faction + f'_mission{number:02}')

    def guard(track, minimum, maximum):
        return dict(track='oteryn:quest-progress/quest/' + track,
                    minimum=minimum, maximum=maximum)

    for faction, spec in FACTIONS.items():
        npc = spec['npc']
        path = f'data-global/npc/{npc}.lua'
        lines = inputs[path].decode().splitlines()
        reply, proof, _ = _phase(lines, 1)
        # A source recruitment confirmation only starts the shared programme;
        # the first issued mission chooses and locks the faction.
        join = next(i for i, line in enumerate(lines) if 'MsgContains(message, "join")' in line)
        proposal, _, _ = _speech(lines, join + 1)
        add(f'l2-secret-{faction}-enlist', quest, npc, ['yes'],
            [state(global_track, -1, 1, 0)], reply, topic=spec['join'],
            offer=(['join'], proposal), source_text=proof)
        if faction != 'tbi':
            callbacks[-1]['achievement'] = 'oteryn:achievement/secret_agent'
        for number in range(1, 8):
            track = mission_track(faction, number)
            give = spec['give'].get(number)
            reply, proof, _ = _phase(lines, number * 2)
            name = 'issue' if number == 1 else f'issue-{number:02}'
            add(f'l2-secret-{faction}-{name}', quest, npc, ['mission'],
                [state(global_track, number * 2 - 1, number * 2), state(track, -1, 1, 0)],
                reply, give=None if give is None else (give, 1), source_text=proof)
            if number == 1:
                callbacks[-1]['requires'] = [guard(mission_track(other, 1), -1, 0)
                                            for other in FACTIONS if other != faction]
            else:
                callbacks[-1]['requires'] = [guard(mission_track(faction, number - 1),
                                                 spec['after'][number - 2], spec['after'][number - 2])]
            before = spec['before'][number - 1]
            after = spec['after'][number - 1]
            reply, proof, _ = _phase(lines, number * 2 + 1)
            prompt, _, _ = _prompt(lines, mission_field(faction, number), before)
            take = spec['take'].get(number)
            name = 'finish' if number == 7 else ('return' if number == 2 else f'report-{number:02}')
            add(f'l2-secret-{faction}-{name}', quest, npc, ['yes'],
                [state(global_track, number * 2, number * 2 + 1), state(track, before, after)],
                reply, take=() if take is None else [(take, 1)], topic=spec['topics'][number - 1],
                offer=(['mission'], prompt), source_text=proof,
                give=(spec['reward'], 1) if number == 7 else None)
            if number != 1:
                callbacks[-1]['requires'] = [guard(mission_track(faction, 1), 1, 4)]
            if number == 7:
                callbacks[-1].update(completes=True, achievement=f'oteryn:achievement/top_{faction}_agent')
