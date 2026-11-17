"""QB: Vampire Hunter Quest token ranks at Storkus; registered by the reward producer.

Global parity (TibiaWiki `Vampire_Hunter_Quest`, `Vampire_Lord_Token`, `Vampire_Doll`,
`Blood_Skull`): after The Inquisition's vampire mission Storkus takes vampire lord tokens in
six ranks of 1, 4, 5, 10, 30 and 50 tokens (100 in all). Each rank shares experience; the last
rank also gives a vampire doll and opens the door in Vengoth's lower castle, behind which the
blood skull chest waits. The Crystal donor gates the token talk on Inquisition questline 11.
"""

DONOR_NPCS = ('storkus',)

QUEST = 'vampire_hunter_quest'
TOKEN = 8192
VAMPIRE_DOLL = 8191
RANKS = (
    (1, 1000, '143-149', "Ye' brought the token needed to advance to the first vampire hunter rank. "
     "I consider that a fluke, but still, congrats! Let me share some of my experience with ye'."),
    (4, 5000, '151-158', "Ye' brought the four tokens needed to advance to the second vampire hunter "
     "rank. Pretty lucky ye' are! Let me share some of my experience with ye'."),
    (5, 10000, '160-167', "Ye' brought the five tokens needed to advance to the third vampire hunter "
     "rank. Wow, you're pretty determined! Let me share some of my experience with ye'."),
    (10, 20000, '169-176', "Ye' brought the ten tokens needed to advance to the fourth vampire hunter "
     "rank. You're absolutely painstaking! Let me share some of my experience with ye'."),
    (30, 50000, '178-185', "Ye' brought the thirty tokens needed to advance to the fifth vampire "
     "hunter rank. You're completely obliterative, kid! Let me share some of my experience with ye'."),
    (50, 100000, '187-196', "Ye' brought the fifty tokens needed to advance to the last vampire hunter "
     "rank. Now that's something. You're razing-amazing! Let me share some of my experience and a "
     "little something with ye'!"),
)


def append(add, storage, offers, callbacks, declarations, prefix, requirement, source):
    root = prefix + 'u8_2/vampire_hunter_quest/'
    rank, door = root + 'rank', root + 'door'
    for key, maximum, lines in [(rank, len(RANKS), '100-196'), (door, 1, '187-196')]:
        declarations.append({'key': key, 'quest': 'oteryn:quest.' + QUEST, 'initial': 0, 'min': 0,
                             'max': maximum, 'writable': True, 'sources': [source('storkus', lines)]})
    inquisition = [requirement(prefix + 'u8_2/the_inquisition_quest/questline', 11, 25)]
    for reached, (count, experience, lines, reply) in enumerate(RANKS):
        topic = 3 + reached
        offers.append({'npc': 'oteryn:npc.storkus', 'keywords': ['vampire lord token'], 'topic': 0,
                       'next_topic': topic, 'storage': [storage(rank, reached, reached, preserve=True)],
                       'requires': inquisition, 'reply': ["Would ye' like to give me vampire tokens?"]})
        states = [storage(rank, reached, reached + 1)]
        last = reached + 1 == len(RANKS)
        if last:
            states.append(storage(door, 0, 1))
        add(f'qb-vampire-hunter-rank-{reached + 1}', QUEST, 'storkus', ['yes'], states,
            take=[(TOKEN, count)], give=(VAMPIRE_DOLL, 1) if last else None, topic=topic,
            requires=inquisition)
        callbacks[-1].update(experience=experience, reply=[reply],
                             failure_reply=["Ye' don't have enought tokens."],
                             sources=[source('storkus', lines)])
