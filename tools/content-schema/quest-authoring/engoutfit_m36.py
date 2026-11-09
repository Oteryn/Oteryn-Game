"""Rock Steady's pinned Crystal collection chain; no synthetic reward stages."""
# A new native source owner preserves the published aggregate outfit quest hash.
TRACK = 'oteryn:quest-progress/rock_steady/collection'


def append(add, storage, callbacks, offers, declarations, source):
    declarations.append({'key': TRACK, 'quest': 'oteryn:quest.rock_steady_collection',
                         'initial': -1, 'min': -1, 'max': 5, 'writable': True,
                         'sources': [source]})
    replies = {
        'start': ["If you want anything, you should talk to Old Rock Boy over there. I do {collect} stuff, though. So just ask if you're interested in helping me."],
        'confirm': ['Great! Let me see. Amazing! I will take this, thank you!'],
        'first': ['Great! Let me see. Amazing! I will take this, thank you!'],
        'second': ['Great! Let me see. Amazing! I will take this, thank you!'],
        'weapon': ["Have you got anything... what? You want what? A reward? HAHAHAHAAAA!! ...",
                   "No I'm just teasing you. I'm really happy about my collection now. ...",
                   'Well, I found some kind of weapon a long time ago. I believe it may be especially helpful underwater as it is from the deep folk. In any case it is of more use for you than it would be for me.'],
    }
    for suffix, before, after, item, topic, words in [
        ('start', -1, 1, None, 0, ['addon', 'help']),
        ('confirm', 1, 2, None, 1, ['yes']),
        ('first', 2, 3, 14022, 2, ['yes']),
        ('second', 3, 4, 14023, 3, ['yes']),
        ('weapon', 4, 5, 14021, 0, ['collect']),
    ]:
        states = [storage(TRACK, before, after, 0 if suffix == 'start' else None)]
        add('m36-rock-steady-' + suffix, 'rock_steady_collection', 'rock_steady',
            words, states, take=[] if item is None else [(item, 1)],
            outfit=('deepling', 1) if suffix == 'weapon' else None, topic=topic)
        callbacks[-1].update(reply=replies[suffix], sources=[source])
    for stage, lines in [
        (1, ['I collect everything that reflects light in strange ways. However, I am bored by my collection. And there wasn\'t anything new to add for years. ...',
             "I like pearls for example - but I have already enough. I also like shells - but I can't even count how many I already own. ...",
             "If you find anything of REAL VALUE - bring it to me. I will reward you well. You don't already have something for me by chance?"]),
        (2, ['Have you got anything for me today?']),
        (3, ['Have you got anything for me today?']),
    ]:
        offers.append({'npc': 'oteryn:npc.rock_steady', 'keywords': ['collect'], 'topic': 0,
                       'next_topic': stage, 'storage': [storage(TRACK, stage, stage, preserve=True)],
                       'reply': lines})
