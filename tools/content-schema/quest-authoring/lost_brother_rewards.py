"""Admit Tarun's Lost Brother branches and gate his existing trade service."""
import json
import re

NPC = 'oteryn:npc.tarun'
QUEST = 'oteryn:quest.adventurers_guild'
TRACK = 'oteryn:quest-progress/quest/u10_80/the_lost_brother_quest'
GUILD = 'oteryn:quest-progress/quest/u9_80/adventurers_guild/quest_line'
PAGE = 'https://tibia.fandom.com/?curid=75165&oldid=933445'
SPOILER = 'https://tibia.fandom.com/?curid=75304&oldid=749371'


def storage(track, before, after):
    return {'track': track, 'minimum': before, 'maximum': before, 'set': after}


def append(root, callbacks, offers, inputs, revision, sha):
    path = 'data-global/npc/tarun.lua'
    raw = inputs[path]
    lines = raw.decode().splitlines()

    def source(first, last):
        return f'crystal:{revision}:{path}:{first}-{last}:sha256:{sha(raw)}'

    def reply(first, last):
        return re.findall(r'"([^"\n]+)"', '\n'.join(lines[first - 1:last]))

    def offer(word, topic, stage, next_topic, messages):
        offers.append({'npc': NPC, 'keywords': [word], 'topic': topic,
                       'next_topic': next_topic, 'storage': [storage(TRACK, stage, stage)],
                       'reply': messages})

    def callback(name, word, topic, states, messages, citation, **reward):
        callbacks.append({
            'key': f'oteryn:quest-reward/lost-brother-{name}', 'quest': QUEST,
            'npc': NPC, 'keywords': [word], 'topic': topic, 'next_topic': 0,
            'storage': states, 'requires_premium': True, 'take': [], 'give': None,
            'outfit': None, 'achievement': None, 'reply': messages,
            'sources': [PAGE, SPOILER, citation], **reward,
        })

    offer('mission', 0, 0, 1, reply(69, 70))
    offer('mission', 1, 0, 1, reply(69, 70))
    offer('no', 1, 0, 0, reply(94, 94))
    offer('mission', 0, 1, 0, reply(74, 74))
    # The group log can already be open from another Adventurers Guild mission.
    callback('start', 'yes', 1, [storage(TRACK, 0, 1), {
        'track': GUILD, 'minimum': 0, 'maximum': 1, 'set': 1,
    }], reply(88, 88), source(86, 96))
    # The Global return transcript has the authoritative wording.
    returned = [
        'So, he is dead as I feared. I warned him to go with this woman, but he gave in to temptation. My heart darkens and moans. But you have my sincere thanks. ...',
        'Without your help I would have stayed in the dark about his fate. Please, take this as a little recompense.',
    ]
    callback('return', 'mission', 0, [storage(TRACK, 2, 3)], returned, source(76, 84),
             give={'item': 'oteryn:item.tibia.i3039', 'count': 1, 'contents': []},
             experience=3000)

    service = 'oteryn:service.trade.tarun'
    document = json.loads((root / 'content/services/trade/trade-00000-00323.json').read_bytes())
    declaration, = (row['declaration'] for row in document['records']
                    if row['declaration']['identity']['key'] == service)
    requirement = {'quest': QUEST, 'track': TRACK, 'initial': 0, 'minimum': 3, 'maximum': 3}
    return {
        'npc': NPC, 'service': service,
        'offers': [{**row, 'requirements': [requirement]} for row in declaration['offers']],
        'sources': [PAGE, source(100, 109)],
    }
