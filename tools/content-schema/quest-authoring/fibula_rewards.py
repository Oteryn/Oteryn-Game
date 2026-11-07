"""Source key sale for the authored Fibula path; no source sale limit is invented."""
import hashlib
import re

PHASE = 'oteryn:quest-progress/fibula/supplement'


def append(callbacks, offers, inputs):
    path = 'data-global/npc/dermot.lua'
    raw = inputs[path]
    text = raw.decode()
    if (not re.search(r'removeMoneyBank\(2000\)', text)
            or not re.search(r'player:addItem\(2968,\s*1\)', text)
            or not re.search(r'key:setActionId\(3940\)', text)):
        raise ValueError('Fibula key sale source changed')
    digest = hashlib.sha256(raw).hexdigest()
    source = f'crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:{path}:66-81:sha256:{digest}'
    for repeat in (False, True):
        # The first purchase activates only the optional chosen supplement.
        # Later source sales remain available before and after its completion.
        storage = dict(track=PHASE, minimum=0 if repeat else -1,
                       maximum=1 if repeat else -1, set=0)
        if repeat:
            storage['preserve'] = True
        callbacks.append(dict(
            key='oteryn:quest-reward/fibula-key-' + ('repeat' if repeat else 'first'),
            quest='oteryn:quest.deeper_fibula_quest', npc='oteryn:npc.dermot',
            keywords=['yes'], topic=2, next_topic=0, storage=[storage], take=[],
            fee=dict(gold=2000, include_bank=True),
            **({'repeatable': True} if repeat else {}),
            give=dict(item='oteryn:item.tibia.i2968', count=1, contents=[], key_number=3940),
            outfit=None, achievement=None, reply=['Here it is.'], sources=[source]))
    offers.append(dict(npc='oteryn:npc.dermot', keywords=['key'], topic=0,
                       next_topic=2, storage=[],
                       reply=['Do you want to buy the dungeon key for 2000 gold?']))
