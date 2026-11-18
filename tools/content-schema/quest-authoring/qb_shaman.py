"""QB: Shaman outfit and addon conversations (Hairycles and Chondur); registered by the reward producer.

Global parity (TibiaWiki `Shaman_Outfits_Quest/Spoiler`): once the Banuta (Ape City) questline is
finished, Hairycles gives the Shaman outfit when asked about "shamanic powers". Chondur then wants
5 pirate voodoo dolls (the Shattered Isles reputation mission, already modelled) before his task:
a mandrake and 5 dworc voodoo dolls give the voodoo staff (addon 2); afterwards 5 tribal masks and
5 banana staffs give the voodoo mask (addon 1). The Way of the Shaman achievement belongs to the
generic full-outfit table, not to these conversations.

Not modelled: Chondur's stage-dependent greeting and the donor's extra Counterspell and Djinn in
Love requirements (the global wiki asks only for the pirate voodoo dolls); his "Maybe next time"
reply to "no"; and Hairycles' "You already are shaman and doctor" and "journey for wisdom" replies.
"""

DONOR_NPCS = ('chondur', 'hairycles')


def append(add, storage, offers, callbacks, prefix):
    general = 'outfit_and_addon_quests'
    shaman = prefix + 'u7_8/shaman_outfits/'
    stage = shaman + 'addon_staff_mask'
    mission_staff, mission_mask = shaman + 'mission_staff', shaman + 'mission_mask'
    ape = prefix + 'u7_6/the_ape_city/'
    sabrehaven = prefix + 'u7_8/the_shattered_isles/reputation_in_sabrehaven'

    def offer(npc, words, topic, next_topic, states, *reply):
        offers.append({'npc': 'oteryn:npc.' + npc, 'keywords': words, 'topic': topic,
                       'next_topic': next_topic, 'storage': states, 'reply': list(reply)})

    def callback(name, quest, npc, states, reply, **kwargs):
        add('qb-shaman-' + name, quest, npc, ['yes'], states, **kwargs)
        callbacks[-1]['reply'] = [reply]
        callbacks[-1]['requires_premium'] = True

    def at(before, after):
        return [storage(stage, before, after)]

    # Hairycles gives the Shaman outfit once the Banuta questline is finished.
    states = [storage(ape + 'shaman_outfit', 0, 1)]
    offer('hairycles', ['shamanic powers', 'shamanic', 'outfit'], 0, 18, [storage(ape + 'shaman_outfit', 0, 0)],
          'Me truly proud of you, friend. You learn many about plants, charms and ape people. '
          'Me want grant you shamanic power now. You ready?')
    callback('hairycles', 'the_ape_city_quest', 'hairycles', states,
             'Friend of the ape people! Take my gift and become me apprentice! Here is shaman '
             'clothing for you!', outfit=('shaman', 0), topic=18,
             requires=[{'track': ape + 'questline', 'minimum': 18, 'maximum': 18}])

    # Chondur's first task: the voodoo staff, after the pirate voodoo dolls of the Sabrehaven mission.
    states = [storage(stage, 0, 1), storage(mission_staff, 0, 1)]
    offer('chondur', ['task', 'addon'], 0, 1, at(0, 0),
          'The time has come, my child. I sense great spiritual wisdom in you and I shall grant '
          'you a sign of your progress if you can fulfil my task.')
    offer('chondur', ['addon', 'task'], 1, 2, at(0, 0),
          'Deep in the Tiquandan jungle a monster lurks which is seldom seen. It is the revenge of '
          'the jungle against humankind. ...',
          'This monster, if slain, carries a rare root called mandrake. If you find it, bring it '
          'to me. Also, gather 5 of the voodoo dolls used by the mysterious dworc voodoomasters. ...',
          'If you manage to fulfil this task, I will grant you your own staff. Have you understood '
          'everything and are you ready for this test?')
    callback('staff-start', general, 'chondur', states,
             "Good! Come back once you've found a mandrake and collected 5 dworcish voodoo dolls.",
             topic=2, requires=[{'track': sabrehaven, 'minimum': 12, 'maximum': 22}])
    callbacks[-1]['requires_outfits'] = ['oteryn:outfit.shaman']
    offer('chondur', ['addon', 'mandrake', 'voodoo dolls', 'voodoo doll'], 0, 5, at(1, 1),
          'Have you gathered the mandrake and the 5 voodoo dolls from the dworcs?')
    callback('staff', general, 'chondur', [storage(stage, 1, 2), storage(mission_staff, 1, 2)],
             "I'm proud of you my child, excellent work. This staff shall be yours from now on!",
             take=[(3002, 5), (5014, 1)], outfit=('shaman', 2), topic=5)

    # Chondur's second task: the voodoo mask.
    offer('chondur', ['addon'], 0, 3, at(2, 2),
          "You have successfully passed the first task. If you fulfil my second task, I'll grant "
          'you a mask like the one I am wearing. Do you want to hear the requirements?')
    offer('chondur', ['yes'], 3, 4, at(2, 2),
          'The dworcs of Tiquanda like to wear certain tribal masks which I would like to take a '
          'look at. Please bring me 5 of these masks. ...',
          'Secondly, the high ape magicians of Banuta use banana staffs. I would love to learn '
          'more about these staffs, so please bring me 5 of them, too. ...',
          'If you manage to fulfil this task, I will grant you your own mask. Have you understood '
          'everything and are you ready for this test?')
    callback('mask-start', general, 'chondur', [storage(stage, 2, 3), storage(mission_mask, 0, 1)],
             'Good! Come back once you have collected 5 tribal masks and 5 banana staffs.', topic=4)
    callbacks[-1]['requires_outfits'] = ['oteryn:outfit.shaman']
    offer('chondur', ['tribal masks', 'tribal mask', 'banana staff', 'banana staffs'], 0, 6, at(3, 3),
          'Have you gathered the 5 tribal masks and the 5 banana staffs?')
    callback('mask', general, 'chondur', [storage(stage, 3, 4), storage(mission_mask, 1, 2)],
             'Well done, my child! I hereby grant you the right to wear a shamanic mask. Do it '
             'proudly.', take=[(3403, 5), (3348, 5)], outfit=('shaman', 1), topic=6)
