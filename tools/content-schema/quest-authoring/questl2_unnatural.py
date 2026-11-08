"""Unnatural Selection NPC missions reuse the existing dance and crystal owners."""
import hashlib
import json
import re

NPCS = ('lazaran', 'ulala')
# Pot3465 requires the existing fluid-container producer, not the ordinary Item cohort.
ITEMS = {10159, 10198, 10196, 5878, 5876}
FLUID_ITEMS = {3465}
ROOT = 'u8_54/unnatural_selection/'
QUEST = 'unnatural_selection_quest'


def append(add, state, callbacks, offers, inputs, pin):
    def text(npc, prefix):
        lines = inputs[f'data-global/npc/{npc}.lua'].decode().splitlines()
        line = next(line for line in lines if '"' + prefix in line)
        raw = re.search(r'"((?:\\.|[^"\\])*)"', line)[1]
        return json.loads('"' + raw.replace("\\'", "'") + '"'), raw

    def emit(name, npc, before, after, mission, old, new, reply_prefix,
             topic=0, prompt=None, keywords=None, take=(), give=None, maximum=None):
        reply, proof = text(npc, reply_prefix)
        storage = [state(ROOT+'questline', before, after),
                   state(ROOT+f'mission{mission:02}', old, new, maximum)]
        offer = None if prompt is None else (['mission'], text(npc, prompt)[0])
        add('l2-unnatural-'+name, QUEST, npc, keywords or ['yes' if topic else 'mission'],
            storage, reply, topic=topic, offer=offer, take=take, give=give, source_text=proof)
        path = f'data-global/npc/{npc}.lua'
        raw = inputs[path]
        marker = f'Selection.Questline, {after})'
        lines = [i for i, line in enumerate(raw.decode().splitlines(), 1) if marker in line]
        # Lazaran uses its local UnnaturalSelection alias, Ulala the full path.
        if len(lines) != 1:
            raise ValueError('Unnatural Selection phase assignment drift: '+name)
        callbacks[-1]['sources'].append(
            f'crystal:{pin}:{path}:{lines[0]}:sha256:{hashlib.sha256(raw).hexdigest()}')

    emit('recover-skull', 'lazaran', -1, 1, 1, -1, 1,
         'You hero of our tribe if bring back holy skull!', topic=1)
    callbacks[-1]['requires'] = [dict(
        track='oteryn:quest-progress/quest/u8_54/the_new_frontier/mission03',
        minimum=3, maximum=3)]
    # Preserve the source's help -> yes -> mission -> yes recruitment sequence.
    guard = [state(ROOT+'questline', -1, -1),
             state('u8_54/the_new_frontier/mission03', 3, 3)]
    for words, topic, next_topic, prefix in [
        (['help'], 0, 11, 'You mean you want help us?'),
        (['yes'], 11, 12, 'Me have many small task, but also big {mission}.'),
        (['mission'], 12, 1, 'Then one night, green men came with wolves...')]:
        offers.append(dict(npc='oteryn:npc.lazaran', keywords=words, topic=topic,
                           next_topic=next_topic, storage=guard, reply=[text('lazaran', prefix)[0]]))

    emit('return-skull', 'lazaran', 1, 2, 1, 1, 3,
         'Me thank you much! All wisdom safe again now.', topic=2,
         prompt='Oh! You found holy skull?', take=[(10159, 1)], maximum=2)
    emit('issue-tour', 'lazaran', 2, 3, 2, -1, 1,
         'Here take holy skull.', topic=3,
         prompt='What holy skull sees, tribe sees.', give=(10159, 1))
    emit('return-tour', 'lazaran', 3, 4, 2, 12, 13,
         'We make big ritual soon and learn much about world outside.', topic=4,
         prompt='You say you was to where sun is hot and burning?', take=[(10159, 1)])
    emit('great-hunt', 'lazaran', 4, 5, 3, -1, 1,
         'We need to calm and make happy gods.', topic=5,
         prompt='We been weak for too long!')
    emit('dance', 'ulala', 5, 6, 3, 1, 2,
         'Krunus is God for plants and birth.', topic=1, keywords=['krunus'],
         prompt='Do Krunus dance for us!', maximum=3)
    callbacks[-1]['storage'].append(state(ROOT+'dance_status', -1, 1, 17))
    emit('issue-sacrifice', 'ulala', 7, 8, 4, -1, 1,
         'If you bring us 5 teeth of green men,')
    emit('return-sacrifice', 'ulala', 8, 9, 4, 1, 2,
         'Me thank you! Me will bring sacrifice for Pandor.', topic=2,
         prompt='Please help tribe make Pandor happy!',
         take=[(10196, 5), (5878, 5), (5876, 5)])
    emit('issue-crystal', 'ulala', 9, 10, 5, -1, 1,
         'Please find great crystal of Fasuon and pray there for his support!')
    emit('return-crystal', 'ulala', 11, 12, 5, 2, 3,
         'You prayed to Fasuon!')
    emit('gods-pleased', 'ulala', 12, 13, 6, -1, 1,
         'So me will do when you gone.')
    emit('issue-beer', 'lazaran', 13, 14, 6, 1, 2,
         'Now brown water is gone and we sad!')
    emit('return-beer', 'lazaran', 14, 15, 6, 2, 3,
         'We make big ritual soon and learn much about world outside.', topic=6,
         prompt='You bring us big pot of strange water from little men?',
         take=[(3465, 1)], give=(10198, 1))
    callbacks[-1]['take'][0]['fluid_subtype'] = 3
    callbacks[-1]['completes'] = True


def world_callbacks(inputs, pin):
    path = 'data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua'
    raw = inputs[path]
    source = raw.decode()
    for guard in ('player:getItemCount(10159) >= 1',
                  'config[player:getStorageValue(Storage.Quest.U8_54.UnnaturalSelection.Mission02)]',
                  'player:getStorageValue(Storage.Quest.U8_54.UnnaturalSelection.Mission02) + 1'):
        if guard not in source:
            raise ValueError('Unnatural Selection tour predicate drift')
    blocks = list(re.finditer(r'\[(\d+)\] = \{(.*?)(?=\n\t\[\d+\] = \{|\n\}\n)',
                              source.split('local mission2AllAroundTheWorld')[0], re.S))
    if [int(block[1]) for block in blocks] != list(range(1, 12)):
        raise ValueError('Unnatural Selection tour stages drift')
    callbacks = []

    def callback(name, trigger, selector, storage, effects, text, witness):
        return dict(key='oteryn:quest-callback/l2-unnatural-'+name,
                    quest='oteryn:quest.'+QUEST, trigger=trigger, selector=selector,
                    lever=None, reversible=False, passages=[], teleport=None,
                    storage=storage, effects=effects, offerings=[], item_on_target=None,
                    return_on_failure=False, failure_message=None,
                    success_message=text, sources=[witness])

    for block in blocks:
        stage = int(block[1])
        text = re.search(r'text = "((?:\\.|[^"\\])*)"', block[2])[1]
        text = json.loads('"'+text.replace("\\'", "'")+'"')
        positions = re.findall(r'Position\((\d+),\s*(\d+),\s*(\d+)\)', block[2])
        if len(positions) != (16 if stage == 5 else 5 if stage == 6 else 1):
            raise ValueError('Unnatural Selection tour alternatives drift')
        witness = f'crystal:{pin}:{path}:{source[:block.start()].count(chr(10))+1}:sha256:{hashlib.sha256(raw).hexdigest()}'
        for alternative, (x, y, z) in enumerate(positions, 1):
            callbacks.append(callback(f'view-{stage:02}-{alternative:02}', 'step_in',
                dict(position=dict(x=int(x), y=int(y), floor=-int(z)), unique=None,
                     action=None, appearance=None),
                [dict(track='oteryn:quest-progress/quest/'+ROOT+'mission02',
                      minimum=stage, maximum=stage, set=stage+1)],
                dict(present=[dict(item='oteryn:item.tibia.i10159', count=1)]),
                text, witness))
    chest = 'data-global/scripts/actions/other/others/quest_system2.lua'
    raw = inputs[chest]
    source = raw.decode()
    block = re.search(r'\[9259\] = \{(.*?)\n\t\},', source, re.S)
    if block is None or any(value not in block[1] for value in (
            'itemId = 10159', 'UnnaturalSelection.Mission01', 'formerValue = 1', 'newValue = 2')):
        raise ValueError('Unnatural Selection skull cache drift')
    text = re.search(r'say = "([^"]+)"', block[1])[1]
    callbacks.append(callback('recover-skull', 'use',
        dict(position=None, unique=9259, action=None, appearance=None),
        [dict(track='oteryn:quest-progress/quest/'+ROOT+'mission01', minimum=1, maximum=1, set=2)],
        dict(item_reward=dict(item='oteryn:item.tibia.i10159', count=1, contents=[])),
        text, f'crystal:{pin}:{chest}:{source[:block.start()].count(chr(10))+1}:sha256:{hashlib.sha256(raw).hexdigest()}'))
    return dict(schema=1, callbacks=callbacks)
