"""QB: mainland Citizen and Warrior addon conversations; registered by the reward producer.

Global parity (TibiaWiki `Citizen_Outfits_Quest/Spoiler`, `Warrior_Outfits_Quest/Spoiler`):
Lubo makes the backpack from 100 minotaur leathers two hours after delivery; Hanna makes
the feather hat from a legion helmet, 100 chicken feathers and 50 honeycombs. Trisha takes
100 hardened bones, 100 turtle shells, a fighting spirit and a dragon claw, then Cornelia
adorns the armour two hours later; Morgan forges the warrior's sword from 100 iron ore
and a piece of royal steel. The Crystal donor gives women a one-hour Cornelia wait; the
Wiki states two hours for everyone, and global wins.
"""

DONOR_NPCS = ('lubo', 'hanna', 'trisha', 'cornelia')

TWO_HOURS = 7201


def append(add, storage, offers, callbacks, prefix):
    general = 'outfit_and_addon_quests'

    def offer(npc, words, topic, next_topic, states, reply):
        offers.append({'npc': 'oteryn:npc.' + npc, 'keywords': words, 'topic': topic,
                       'next_topic': next_topic, 'storage': states, 'reply': [reply]})

    def timer_set(track):
        callbacks[-1]['state_ops'] = [{'track': track, 'from': {'kind': 'any'},
                                       'effect': {'kind': 'set_now'}}]

    def timer_elapsed(track):
        callbacks[-1]['state_ops'] = [{'track': track, 'from': {'kind': 'elapsed_at_least',
                                       'seconds': TWO_HOURS}, 'effect': {'kind': 'add', 'value': 0}}]

    citizen = prefix + 'u7_8/citizen_outfits/'
    backpack, mission, timer = citizen + 'addon_backpack', citizen + 'mission_backpack', citizen + 'addon_backpack_timer'
    states = [storage(backpack, 0, 1), storage(mission, 0, 1)]
    offer('lubo', ['addon', 'outfit'], 0, 1, states,
          "Sorry, the backpack I wear is not for sale. It's handmade from rare minotaur leather.")
    offer('lubo', ['backpack', 'minotaur', 'leather'], 1, 2, states,
          'Well, if you really like this backpack, I could make one for you, but minotaur leather '
          'is hard to come by these days. Are you willing to put some work into this?')
    add('qb-citizen-backpack-start', general, 'lubo', ['yes'], states, topic=2)
    states = [storage(backpack, 1, 2), storage(mission, 1, 2)]
    offer('lubo', ['addon', 'outfit', 'backpack', 'leather'], 0, 3, states,
          'Ah, right, almost forgot about the backpack! Have you brought me 100 pieces of '
          'minotaur leather as requested?')
    add('qb-citizen-backpack-materials', general, 'lubo', ['yes'], states, take=[(5878, 100)], topic=3)
    timer_set(timer)
    add('qb-citizen-backpack', general, 'lubo', ['addon', 'outfit', 'backpack'],
        [storage(backpack, 2, 3), storage(mission, 2, 0)], outfit=('citizen', 1))
    timer_elapsed(timer)

    hat, hat_mission = citizen + 'addon_hat', citizen + 'mission_hat'
    states = [storage(hat, 0, 1), storage(hat_mission, 0, 1)]
    offer('hanna', ['addon', 'outfit', 'hat'], 0, 1, states,
          "Pretty, isn't it? My friend Amber taught me how to make it, but I could help you with "
          'one if you like. What do you say?')
    add('qb-citizen-hat-start', general, 'hanna', ['yes'], states, topic=1)
    states = [storage(hat, 1, 2), storage(hat_mission, 1, 0)]
    offer('hanna', ['addon', 'outfit', 'hat'], 0, 2, states,
          "Oh, you're back already? Did you bring a legion helmet, 100 chicken feathers and "
          '50 honeycombs?')
    add('qb-citizen-hat', general, 'hanna', ['yes'], states,
        take=[(3374, 1), (5902, 50), (5890, 100)], outfit=('citizen', 2), topic=2)

    warrior = prefix + 'u7_8/warrior_outfits/'
    shoulder, shoulder_timer = warrior + 'warrior_shoulder_addon', warrior + 'warrior_shoulder_timer'
    states = [storage(shoulder, 0, 1),
              storage('oteryn:quest-progress/outfit_quest/default_start', 0, 1, 1)]
    offer('trisha', ['earn'], 0, 1, states,
          "I'm not sure if you are enough of a hero to earn them. You could try, though. "
          'What do you think?')
    offer('trisha', ['yes'], 1, 2, states,
          'Bring me 100 hardened bones, 100 turtle shells, fighting spirit and a claw from a '
          'mighty dragon king. Did you understand everything I told you and are willing to '
          'handle this task?')
    add('qb-warrior-shoulder-start', general, 'trisha', ['yes'], states, topic=2)
    for stage, suffix, words, item, count, question in [
            (1, 'bones', ['hardened bones'], 5925, 100,
             'How are you faring with your mission? Have you collected all 100 hardened bones?'),
            (2, 'shells', ['turtle shells'], 5899, 100,
             'Did you get us 100 turtle shells so we can make new shields?'),
            (3, 'spirit', ['fighting spirit'], 5884, 1, 'So, can you show me your fighting spirit?'),
            (4, 'claw', ['dragon claw'], 5919, 1,
             'Have you actually managed to obtain the red dragon claw I asked for?')]:
        states = [storage(shoulder, stage, stage + 1)]
        offer('trisha', words, 0, stage + 2, states, question)
        add('qb-warrior-shoulder-' + suffix, general, 'trisha', ['yes'], states,
            take=[(item, count)], topic=stage + 2)
    words = ['addon', 'armour', 'armor', 'outfit', 'spike', 'trisha']
    add('qb-warrior-shoulder-work', general, 'cornelia', words, [storage(shoulder, 5, 6)])
    timer_set(shoulder_timer)
    add('qb-warrior-shoulder', general, 'cornelia', words, [storage(shoulder, 6, 7)],
        outfit=('warrior', 1))
    timer_elapsed(shoulder_timer)

    sword = warrior + 'warrior_sword_addon'
    add('qb-warrior-sword-start', general, 'morgan', ["warrior's sword"], [storage(sword, 0, 1)])
    callbacks[-1]['forbids_addons'] = [{'outfit': 'oteryn:outfit.warrior', 'addons': 2}]
    offer('morgan', ['forge'], 0, 1, [storage(sword, 1, 1, preserve=True)],
          "What would you like me to forge for you? A knight's sword or a warrior's sword?")
    add('qb-warrior-sword', general, 'morgan', ["warrior's sword"], [storage(sword, 1, 2)],
        take=[(5887, 1), (5880, 100)], outfit=('warrior', 2), topic=1)
    callbacks[-1]['forbids_addons'] = [{'outfit': 'oteryn:outfit.warrior', 'addons': 2}]
