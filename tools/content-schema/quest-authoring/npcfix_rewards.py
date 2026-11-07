"""NPCFIX source compositions over the shared fenced reward owner."""
import hashlib
import re

NPCS = ('al_dee', 'a_weakened_forest_fury', 'a_wandering_soul')
ITEMS = {3456, 8225, 8453, 23812}


def append(callbacks, offers, inputs, pin):
    def text(npc, phrase):
        raw = inputs[f'data-global/npc/{npc}.lua'].decode()
        for line in raw.splitlines():
            if phrase in line:
                matches = re.findall(r'"([^"\n]*)"', line)
                found = [m for m in matches if phrase in m]
                if len(found) == 1:
                    return found[0]
        raise ValueError('NPCFIX source literal missing: ' + npc + ':' + phrase)

    def source(npc, phrase):
        path = f'data-global/npc/{npc}.lua'
        raw = inputs[path]
        line = next(i for i, row in enumerate(raw.decode().splitlines(), 1) if phrase in row)
        return f'crystal:{pin}:{path}:{line}:sha256:{hashlib.sha256(raw).hexdigest()}'

    def state(track, minimum, maximum, value, preserve=False):
        return dict(track='oteryn:quest-progress/' + track, minimum=minimum,
                    maximum=maximum, set=value, **({'preserve': True} if preserve else {}))

    def item(number, count=1):
        return dict(item=f'oteryn:item.tibia.i{number}', count=count)

    def offer(npc, words, topic, next_topic, storage, reply):
        offers.append(dict(npc='oteryn:npc.' + npc, keywords=words, topic=topic,
                           next_topic=next_topic, storage=storage, reply=reply))

    def callback(name, quest, npc, words, topic, storage, take, give, phrase, repeat=False,
                 achievement=None):
        row = dict(key='oteryn:quest-reward/npcfix/' + name, quest='oteryn:quest.' + quest,
                   npc='oteryn:npc.' + npc, keywords=words, topic=topic, next_topic=0,
                   storage=storage, take=take, give={**give, 'contents': []}, outfit=None,
                   achievement=achievement, reply=[text(npc, phrase)], sources=[source(npc, phrase)])
        if repeat:
            row['repeatable'] = True
        if achievement:
            row['achievement_once'] = True
        callbacks.append(row)

    npc = 'al_dee'
    receipt = [state('npcfix/pick_quest/exchange', -1, -1, -1, True)]
    offer(npc, ['pick', 'small axe'], 0, 1, receipt, [text(npc, 'Picks are hard to come by.')])
    offer(npc, ['no'], 1, 0, receipt, [text(npc, "Well, then don't.")])
    callback('al-dee-pick', 'pick_quest', npc, ['yes'], 1, receipt,
             [item(3462)], item(3456), 'Splendid! Here, take your pick.', True)

    npc = 'a_weakened_forest_fury'
    cage = 'quest/u11_02/forgotten_knowledge/bird_cage'
    waiting = [state(cage, 0, 0, 1)]
    raw = inputs[f'data-global/npc/{npc}.lua'].decode()
    blocks = re.findall(r'npcHandler:say\(\{(.*?)\}, npc, creature\)', raw, re.S)
    introduction = next(b for b in blocks if 'Will you be our guardian?' in b)
    offer(npc, ['distress', 'mission'], 0, 1, waiting,
          re.findall(r'"([^"\n]*)"', introduction))
    success = next(b for b in blocks if 'Indeed, you will.' in b)
    callback('forest-fury-cage', 'forgotten_knowledge_quest', npc, ['yes'], 1,
             waiting, [], item(23812), 'Indeed, you will.')
    callbacks[-1]['reply'] = re.findall(r'"([^"\n]*)"', success)
    held = [state(cage, 1, 1, 1, True)]
    offer(npc, ['cages'], 0, 2, held,
          [text(npc, 'Crafted generations ago'), text(npc, 'Are you in need of another one?')])
    offer(npc, ['yes'], 2, 0, held, [text(npc, 'I already handed a cage to you.')])
    for keyword, phrase in [('seeds', 'Seeds to give life'), ('intruders', 'The intruders appeared'),
                            ('water', 'The purest water flows'), ('birds', 'Take care, guardian.'),
                            ('phials', 'Phials for the purest water')]:
        offer(npc, [keyword], 1, 0, waiting, [text(npc, phrase)])

    npc = 'a_wandering_soul'
    mission = [state('quest/u8_4/blood_brothers/mission05', 2, 4, 2, True)]
    callback('wandering-crystal', 'blood_brothers_quest', npc, ['blood crystal'], 0,
             mission, [item(8453)], item(8225), "What the...? I don't know anything",
             True, 'oteryn:achievement/ghostwhisperer')
    callbacks[-1]['give']['decay'] = dict(duration_millis=604800000,
        target=dict(family='Item', key='oteryn:item.tibia.i8453', revision='definition-r1'))
    offer(npc, ['blood crystal'], 0, 0,
          [state('quest/u8_4/blood_brothers/mission05', 0, 1, 0, True)],
          [text(npc, 'A crystal? My mind is too clouded')])

    greetings = {}
    for npc in NPCS:
        default = 'data/npclib/npc_system/npc_handler.lua'
        messages = {}
        citations = []
        raw = inputs[f'data-global/npc/{npc}.lua'].decode()
        for field, constant in [('greet', 'GREET'), ('farewell', 'FAREWELL'), ('walkaway', 'WALKAWAY')]:
            lines = re.findall(r'npcHandler:setMessage\(MESSAGE_' + constant + r',\s*"([^"\n]*)"', raw)
            if npc == 'al_dee' and field == 'greet':
                block = re.search(r'npcHandler:setMessage\(MESSAGE_GREET,\s*\{(.*?)\}\)', raw, re.S)
                lines = [' '.join(re.findall(r'"([^"\n]*)"', block.group(1)))]
            elif npc == 'a_wandering_soul' and field == 'greet':
                lines = [text(npc, 'Is that you...? No...')]
            if not lines:
                lines = re.findall(r'\[MESSAGE_' + constant + r'\]\s*=\s*"([^"\n]*)"', inputs[default].decode())
                citations.append(f'crystal:{pin}:{default}:sha256:{hashlib.sha256(inputs[default]).hexdigest()}')
            else:
                citations.append(source(npc, 'MESSAGE_' + constant))
            if len(lines) != 1:
                raise ValueError('NPCFIX ambiguous message: ' + npc + ':' + field)
            messages[field] = lines[0]
        greetings['oteryn:npc.' + npc] = dict(npc='oteryn:npc.' + npc, **messages, sources=citations)
    greetings['oteryn:npc.a_wandering_soul']['conditional'] = [
        dict(track='oteryn:quest-progress/quest/u8_4/blood_brothers/mission05', minimum=2,
             item='oteryn:item.tibia.i8453', text=text('a_wandering_soul', 'I saw you...')),
        dict(track='oteryn:quest-progress/quest/u8_4/blood_brothers/mission05', minimum=2,
             item='oteryn:item.tibia.i8225', text=text('a_wandering_soul', 'The pain is slightly less...')),
    ]
    return greetings
