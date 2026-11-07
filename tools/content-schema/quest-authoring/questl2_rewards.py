"""Source-backed L2 exchanges using QENGINE-2's existing transaction owner."""
import hashlib
import re
import questl2_exchange_prayers as prayers
import questl2_exchange_recipes as recipes


NPCS = tuple(sorted({'amber', 'norma', 'storkus', 'an_old_dragon_lord', 'dorian', 'gamon', 'hairycles', 'jorge', 'kroox', 'rabaz', *prayers.PRIESTS, *recipes.NPCS}))
NPC_FILES = {'oteryn:npc.an_old_dragonlord': 'an_old_dragon_lord'}


def source_path(npc):
    return 'data-global/npc/' + NPC_FILES.get(npc, npc.removeprefix('oteryn:npc.')) + '.lua'
ITEMS = {
    8191, 8192, 131, 4852, 11699, 227, 5941, 4827, 4828, 4829, 4831, 4832, 7870, 7871, 7933,
    7935, 7936, 19083, 19150, 19151, 19152, 19153, 19154, 19155,
    19156, 22026, 22027, 22028, 23676, 23677, 23678, 23679, 23680,
    23681, 23682, 23683, 25979, 25980, 25981, 25982, 28894, 28895,
    28896, 28897, 31265, 31266, 31267, 31268, 34210, 37058, 37059,
    37060, 43510, 43511, 43517,
}


ITEMS |= recipes.ITEMS

def append(callbacks, offers, inputs, pin):
    def state(track, before, after, maximum=None, preserve=False):
        return {'track': 'oteryn:quest-progress/quest/' + track,
                'minimum': before, 'maximum': before if maximum is None else maximum,
                'set': after, **({'preserve': True} if preserve else {})}

    def add(name, quest, npc, keywords, storage, reply, take=(), topic=0, offer=None,
            give=None, repeatable=False, source_text=None, fee=None):
        path = source_path('oteryn:npc.' + npc)
        callback = {
            'key': 'oteryn:quest-reward/' + name,
            'quest': 'oteryn:quest.' + quest, 'npc': 'oteryn:npc.' + npc,
            'keywords': keywords, 'topic': topic, 'next_topic': 0,
            'storage': storage,
            'take': [{'item': f'oteryn:item.tibia.i{item}', 'count': count}
                     for item, count in take],
            'give': None if give is None else {
                'item': f'oteryn:item.tibia.i{give[0]}', 'count': give[1], 'contents': []},
            'outfit': None, 'achievement': None,
            'reply': [reply],
            'sources': [f'crystal:{pin}:{path}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'],
        }
        # Keep literal success text tied to the pinned source, rather than
        # silently inventing a reward branch when the source corpus changes.
        proof = source_text or reply
        if proof not in inputs[path].decode():
            raise ValueError('L2 reward reply is absent from donor: ' + name)
        if fee is not None:
            callback['fee'] = fee
        if repeatable:
            callback['repeatable'] = True
        if source_text:
            line = next(i for i, text in enumerate(inputs[path].decode().splitlines(), 1)
                        if source_text in text)
            callback['sources'] = [
                f'crystal:{pin}:{path}:{line}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}']
        callbacks.append(callback)
        if offer is not None:
            words, text = offer
            if text not in inputs[path].decode():
                raise ValueError('L2 offer is absent from donor: ' + name)
            offers.append({'npc': callback['npc'], 'keywords': words, 'topic': 0,
                           'next_topic': topic, 'storage': storage, 'reply': [text]})

    jorge = inputs['data-global/npc/jorge.lua'].decode()
    for required in ('{ name = "Abacus", id = 19151 }',
                     'getPlayerItemCount(creature, 19083) >= 20',
                     'doPlayerRemoveItem(creature, 19083, 20)',
                     'doPlayerAddItem(creature, items[i].id, 1)'):
        if required not in jorge:
            raise ValueError('Jorge token recipe drift: ' + required)
    souvenirs = re.findall(r'\{ name = "([^"]+)", id = (\d+) \}', jorge)
    if len(souvenirs) != 37:
        raise ValueError('Jorge source souvenir menu drift')
    for index, (name, item) in enumerate(souvenirs):
        key = re.sub(r'[^a-z0-9]+', '-', name.lower()).strip('-')
        add('token-' + key, 'hunting_for_tokens', 'jorge', [name.lower()],
            [state('exchange/hunting_for_tokens/souvenirs', -1, -1, preserve=True)],
            'You just swapped 20 silver raid tokens for 1 ' + name + '.',
            take=[(19083, 20)], give=(int(item), 1), topic=1, repeatable=True,
            source_text='"You just swapped 20 silver raid tokens for 1 " .. items[i].name .. "."',
            offer=(['souvenir'], 'All listed souvenirs are at 20 silver raid tokens each. Which one of them do you want?')
                  if index == 0 else None)
        line = next(i for i, text in enumerate(jorge.splitlines(), 1)
                    if f'name = "{name}", id = {item}' in text)
        callbacks[-1]['sources'].append(
            f'crystal:{pin}:data-global/npc/jorge.lua:{line}:sha256:{hashlib.sha256(inputs["data-global/npc/jorge.lua"]).hexdigest()}')

    friends = 'u7_8/friends_and_traders/'
    add('exchange-quentin-start', 'friends_and_traders', 'quentin', ['yes'],
        [state(friends + 'default_start', -1, 1, 1),
         state(friends + 'the_blessed_stake', -1, 1, 0)],
        "Alright, I guess you need a stake first. Maybe Gamon can help you, the leg of a chair or something could just do. Try asking him for a stake, and if you have one, bring it back to me.",
        topic=1, offer=(['stake'], 'Is this stake really important enough to you so that you are willing to take this burden?'))
    add('exchange-gamon-stake', 'friends_and_traders', 'gamon', ['yes'],
        [state(friends + 'the_blessed_stake', 1, 1, 9223372036854775807, preserve=True)],
        "Argh... my heart aches! Alright... a promise is a promise. Here - take this wooden stake, and now get lost.",
        give=(5941, 1), topic=1, repeatable=True, fee={'gold':5000, 'include_bank':True},
        offer=(['stake'], "Making a stake from a chair? Are you insane??! I won't waste my chairs on you for free! You will have to pay for it, but since I consider your plan a blasphemy, it will cost 5000 gold pieces. Okay?"))

    prayers.append(add, state, callbacks, inputs)
    recipes.append(add, state, callbacks, offers, inputs)

    dragon = inputs['data-global/npc/an_old_dragon_lord.lua'].decode()
    refusal = "AHHHH THE PAIN OF AGESSS! I NEED MUSSSSHRROOOMSSS TO EASSSE MY PAIN! BRRRING ME MUSHRRROOOMSSS!"
    success = "AHHH MUSHRRROOOMSSS! NOW MY PAIN WILL BE EASSSED FOR A WHILE! TAKE THISS AND LEAVE THE DRAGONSSS' CEMETERY AT ONCE!"
    track = 'oteryn:quest-progress/dragonfetish'
    for suffix, minimum, maximum in [('initial', -1, 0), ('later', 2, 9223372036854775807)]:
        add('draconia-fetish-' + suffix, 'draconia_quest', 'an_old_dragonlord',
            ['hi', 'hello', 'hail'],
            [{'track': track, 'minimum': minimum, 'maximum': maximum, 'set': 1}],
            success, take=[(3723, 1)], give=(3206, 1), repeatable=True,
            source_text='player:addItem(3206, 1)')
        callbacks[-1].update(close_after=True, failure_reply=[refusal])
    held = "LEAVE THE DRAGONS' CEMETERY AT ONCE!"
    if held not in dragon or refusal not in dragon or 'player:removeItem(3723, 1)' not in dragon or 'player:addItem(3206, 1)' not in dragon:
        raise ValueError('Draconia greeting recipe drift')
    offers.append({'npc': 'oteryn:npc.an_old_dragonlord',
                   'keywords': ['hi', 'hello', 'hail'], 'topic': 0, 'next_topic': 0,
                   'storage': [{'track': track, 'minimum': 1, 'maximum': 1, 'set': 1}],
                   'reply': [held], 'close_after': True})

    fox = 'u8_1/to_outfox_afox_quest/questline'
    for callback in callbacks:
        if callback['key'] == 'oteryn:quest-reward/outfox-helmet':
            callback['storage'] = [state(fox,1,2)]
            callback['sources'].append('crystal:'+pin+':data-global/npc/budrik.lua:68:sha256:'+hashlib.sha256(inputs['data-global/npc/budrik.lua']).hexdigest())
    add('outfox-start','tibia_tales','budrik',['yes'],
        [state('u8_1/tibia_tales/default_start',-1,1,1),state(fox,-1,1,0)],
        "I knew you have the guts for that task! We presume the hideout of The Horned Fox somewhere in the south-west near the coast. Good luck!",
        topic=1,source_text='player:setStorageValue(Storage.Quest.U8_1.ToOutfoxAFoxQuest.Questline, 1)',
        offer=(['mission','quest'],"It belonged to my father and before that to my grandfather. That helmet is at least 600 years old! I need it back. Are you willing to help me?"))
    botany='u8_6/an_interest_in_botany/'
    add('l2-botany-start','tibia_tales','rabaz',['yes'],
        [state('u8_1/tibia_tales/default_start',-1,1,1),state(botany+'questline',-1,1,0),state(botany+'chest_door',-1,0,0)],
        "Yes? Yes! That's the enthusiasm I need! Remember to bring a sharp knife to gather the samples, plants - even mutated deformed plants - are very sensitive you know. Off you go and be careful out there, Zao is no place for the feint hearted mind you.",
        topic=1,source_text='player:setStorageValue(Storage.Quest.U8_6.AnInterestInBotany.Questline, 1)',
        offer=(['mission'],"Simply return to me after you have done that and we will discuss your reward. What do you say, are you in?"))
    add('l2-botany-return','tibia_tales','rabaz',['yes'],[state(botany+'questline',3,4)],
        "Here, I always wore this comfortable hat when travelling, take it. It may be of use for you on further reconnaissances in Zao. Again you have my thanks, friend.",
        topic=2,take=[(11699,1)],give=(11700,1),source_text='player:addItem(11700, 1)',
        offer=(['mission'],"Well fantastic work, you gathered both samples! Now I can continue my work on the almanach, thank you very much for your help indeed. Can I take a look at my book please?"))
    callbacks[-1]['give']['backpack_outputs']=[{'item':'oteryn:item.tibia.i3035','count':10}]
    callbacks[-1]['experience']=3000

    bone = 'u8_1/tibia_tales/into_the_bone_pit'
    add('l2-bone-start', 'tibia_tales', 'muriel', ['yes'], [state(bone,-1,1,0)],
        "Great! Here is the container for the bone. Once, I used it to collect ectoplasma of ghosts, but it will work here as well. ...",
        topic=1,give=(4852,1),source_text='player:addItem(4852, 1)',
        offer=(['mission'],"Find that pit, dig for a well-preserved human skeleton and conserve a sample in a special container which you receive from me. Are you going to help me?"))
    callbacks[-1]['minimum_level'] = 35
    add('l2-bone-return','tibia_tales','muriel',['mission'],[state(bone,2,3)],
        "Excellent! Now I can try to put my theoretical thoughts into practice and find a cure for the symptoms of undead. Here, take this for your efforts.",
        take=[(131,1)],give=(6299,1),source_text='player:addItem(6299, 1)')
    callbacks[-1].update(minimum_level=35,experience=1000)

    sam = 'u7_5/sams_old_backpack/'
    add('sam-kroox', 'sam_s_old_backpack_quest', 'kroox',
        ['sam sent me', 'sam send me'],
        [state(sam + 'sams_old_backpack_npc', 1, 2),
         state(sam + 'sams_old_backpack_door', -1, 1, 1)],
        'The problem is, some giant spiders made the tunnels where the storage is their new home. Good luck.')

    thieves = 'u8_2/the_thieves_guild_quest/'
    deliveries = [
        ('tusks', 3044, 10, 1, 2,
         "What a fine material. That will be worth a coin or two. So far, so good. Ask me for another mission if you're ready for it."),
        ('vase', 227, 1, 2, 3,
         'What an ugly vase. But who am I to question the taste of our customers? Anyway, I might have another mission in store for you.'),
        ('invitation', 7933, 1, 2, 3,
         "You're too valuable to us. Speaking about your value, I might have some other mission for you."),
        ('bridge', 7871, 1, 7, 8,
         "Excellent, that serves this fool right. I fear in your next mission, you'll have to get your hands dirty. Just ask me to learn more about it."),
        ('goblet', 7870, 1, 1, 2,
         'That goblet is hardly worth all this trouble but we had to insist on our payment. However, I assume you are eager for more missions, so just ask.'),
        ('fish', 7936, 1, 3, 4,
         "This little goldfish will bring us a hefty ransom! Just ask me if you're ready for another mission."),
        ('letter', 7935, 1, 1, 2,
         "Excellent, that little letter will do the trick for sure ..."),
    ]
    for index, (name, item, count, before, after, reply) in enumerate(deliveries, 1):
        add('thieves-' + name, 'the_thieves_guild_quest', 'dorian', ['yes'],
            [state(thieves + f'mission{index:02}', before, after),
             state(thieves + 'questline', index, index + 1)],
            reply, [(item, count)], topic=index + 1,
            offer=(['mission'], 'Have you finished your mission?'))

    ape = 'u7_6/the_ape_city/'
    deliveries = [
        ('moss', 4827, 1, 2, 3,
         "Ah yes! That's it. Thank you for bringing mighty whisper moss to Hairycles. It will help but still much is to be done. Just ask for other mission if you ready.",
         'Oh, you brought me whisper moss? Good hairless ape you are! Can me take it?'),
        ('syrup', 4828, 3, 4, 4,
         'You so good! Brought syrup to me! Thank you, will prepare cure now. Just ask for {mission} if you want help again.',
         'You brought me that cough syrup from human healer me asked for?'),
        ('parchment', 4831, 5, 6, 7,
         'You brought scroll with lizard text? Good! I will see what text tells me! Come back when ready for other mission.',
         'You got scroll from lizard village in south east?'),
        ('egg', 4839, 9, 10, 11,
         'Ah, the egg! Mighty warrior you be! Thank you. Hairycles will put it at safe place immediately.',
         'You bring Hairycles egg of hydra?'),
        ('mushroom', 4829, 11, 12, 13,
         "Incredible, you brought a witches' cap! Now me can prepare mighty charm of life. Yet still other {missions} will await you, friend.",
         "You brought Hairycles witches' cap from Fibula?"),
        ('hair', 4832, 15, 16, 16,
         'Incredible! You got a hair of holy Bong! This will raise the spirit of my people. You are truly a friend. But one last mission awaits you.',
         'You brought hair of holy ape?'),
    ]
    for name, item, before, after, topic, reply, offer in deliveries:
        storage = [state(ape + 'questline', before, after)]
        if name == 'moss':
            storage.append(state('u7_6/whisper_moss', 1, 1, preserve=True))
        elif name == 'hair':
            storage.append(state(ape + 'holy_ape_hair', 1, 1, preserve=True))
        elif name == 'mushroom':
            # The source closes the temporary Fibula door on delivery.
            storage.append(state(ape + 'fibula_door', -1, -1, 1))
        add('ape-' + name, 'the_ape_city_quest', 'hairycles', ['yes'],
            storage, reply, [(item, 1)], topic=topic,
            offer=None if offer is None else (['mission'], offer))
