"""Pinned Yonan assembly, reused from questl2b's qualified Kilmaresh exchange."""
import hashlib
import re

NPCS = ('yonan',)
ITEMS = set(range(31572, 31577))
SOURCE = 'data-global/npc/yonan.lua'
SHA256 = 'ab32c38fec7fc99be97b91ed19d79e64880c9ff955d58d8eee3504282f6e3848'
SHARD = 'content/quests/npc-callbacks/l2-n2-stories.json'


def append(add, callbacks, offers, inputs, pin):
    raw = inputs[SOURCE]
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('L2_YONAN_SOURCE_PIN')
    text = raw.decode()
    required = [f'player:getItemById({item}, 1)' for item in range(31572, 31576)]
    required += [f'player:removeItem({item}, 1)' for item in range(31572, 31576)]
    required += ['player:addItem(31576, 1)', 'npcHandler:setTopic(playerId, 5)']
    if not all(value in text for value in required):
        raise ValueError('L2_YONAN_ASSEMBLY_SOURCE')
    add('l2-regalia-combine', 'kilmaresh_quest', 'yonan', ['yes'], [],
        take=[(item, 1) for item in range(31572, 31576)], give=(31576, 1), topic=5)
    witness = f'crystal:{pin}:{SOURCE}:198-209:sha256:{SHA256}'
    callbacks[-1].update(sources=[witness], reply=['Well then, let me have a look.'])
    question = re.findall(r'"([^"\n]+)"', text.splitlines()[198])
    if len(question) != 1:
        raise ValueError('L2_YONAN_SOURCE_QUESTION')
    offers.append({'npc': 'oteryn:npc.yonan', 'keywords': ['regalia of suon'],
        'topic': 0, 'next_topic': 5, 'storage': [], 'reply': question})


def preserve_weight(definition, record):
    """Keep the accepted canonical weight; source and canonical values are exact."""
    if definition['identity']['key'] != 'oteryn:item.tibia.i31576':
        return
    canonical = definition['semantics']['physical']['value']['weight']
    observed = record['semantics']['physical']['value']['weight']
    if (canonical != {'state': 'KNOWN', 'value': 3400}
            or observed != {'state': 'KNOWN', 'value': 850}):
        raise ValueError('L2_REGALIA_WEIGHT_PRECEDENCE_DRIFT')
    record['semantics']['physical']['value']['weight'] = canonical
