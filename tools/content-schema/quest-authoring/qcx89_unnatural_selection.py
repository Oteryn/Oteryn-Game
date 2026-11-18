"""Lower Lazaran's sourced Skulled entry into the existing quest reward writer."""

import hashlib
import json
import re

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
NPC = 'lazaran'
PATH = f'data-global/npc/{NPC}.lua'
WIKI = 'https://tibia.fandom.com/?curid=41846&oldid=684058'
SPOILER = 'https://tibia.fandom.com/?curid=41847&oldid=1103829'
QUEST = 'unnatural_selection_quest'
TRACK = 'oteryn:quest-progress/quest/u8_54/unnatural_selection/'
ZAO = 'oteryn:quest-progress/quest/u8_54/the_new_frontier/mission03'


def append(root, add, callbacks, offers, storage, greeting_source):
    packet = json.loads((root / 'content/dialogues/quest-sources/unnatural_selection.json').read_bytes())
    entry = next(row for row in packet['entries'] if row['npc_source_name'] == NPC)
    raw = entry['source_lua'].encode()
    digest = hashlib.sha256(raw).hexdigest()
    if (packet['source_repository_revision'] != f'zimbadev/crystalserver@{PIN}'
            or digest != entry['source_blob_sha256'] or raw != greeting_source):
        raise ValueError('Skulled Lazaran source drift')
    lines = raw.decode().splitlines()

    def reply(first, last):
        return [re.search(r'"([^"]+)"', line)[1] for line in lines[first - 1:last]]

    # The native lowered track uses 0 for an unstarted quest. Also accept the
    # donor's historical unset value without accepting any started stage.
    unfinished = storage(TRACK + 'questline', -1, 0, 0, preserve=True)
    zao = [{'track': ZAO, 'minimum': 3, 'maximum': 3}]
    for word, topic, next_topic, first, last in [
        ('help', 0, 11, 112, 112),
        ('yes', 11, 12, 164, 164),
        ('mission', 12, 1, 70, 71),
    ]:
        offer = {
            'npc': f'oteryn:npc.{NPC}', 'keywords': [word],
            'topic': topic, 'next_topic': next_topic,
            'storage': [unfinished], 'reply': reply(first, last),
        }
        if word == 'mission':
            offer['requires'] = zao
        offers.append(offer)

    add('unnatural-selection-skulled-start', QUEST, NPC, ['yes'], [
        storage(TRACK + 'questline', -1, 1, 0),
        storage(TRACK + 'mission01', -1, 1, 0),
    ], topic=1, requires=zao)
    callbacks[-1].update(
        reply=reply(116, 116),
        requires_premium=True,
        sources=[f'crystal:{PIN}:{PATH}:63-74:sha256:{digest}',
                 f'crystal:{PIN}:{PATH}:111-119:sha256:{digest}',
                 f'crystal:{PIN}:{PATH}:163-165:sha256:{digest}',
                 WIKI, SPOILER],
    )
