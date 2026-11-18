"""Lower the source-bound Ape City exchanges and outfit confirmation."""

NPC = 'oteryn:npc.padreia'
QUESTLINE = 'oteryn:quest-progress/quest/u7_6/the_ape_city/questline'
GLOBAL = 'https://tibia.fandom.com/?curid=5846&oldid=1023981'
CRYSTAL = (
    'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'
    'data-global/npc/padreia.lua:61-92:'
    'sha256:1b521a3ff677c3c4db585886f43d233f999a945dcd0b6a11fb0fd1c3ce19f63a'
)


def append(callbacks, offers):
    offers.append({
        'npc': NPC, 'keywords': ['cough syrup'], 'topic': 0, 'next_topic': 1,
        'storage': [], 'reply': ['Do you want to buy a bottle of cough syrup for 50 gold?'],
    })
    callbacks.append({
        'key': 'oteryn:quest-reward/ape-city-cough-syrup',
        'quest': 'oteryn:quest.the_ape_city_quest',
        'npc': NPC, 'keywords': ['yes'], 'topic': 1, 'next_topic': 0,
        # The paid MINT owner records the existing stage without advancing it.
        'storage': [{'track': QUESTLINE, 'minimum': 0, 'maximum': 18, 'set': 0, 'preserve': True}],
        'take': [],
        'fee': {'gold': 50, 'include_bank': True},
        'repeatable': True,
        'give': {'item': 'oteryn:item.tibia.i4828', 'count': 1, 'contents': []},
        'outfit': None, 'achievement': None,
        'reply': ['Thank you. Here it is.'],
        'failure_reply': ["You don't have enough money."],
        'sources': [GLOBAL, CRYSTAL],
    })

    # The already imported questline is also the entrance portal's prerequisite.
    # Use the exact report and confirmation sequence from the Mission 9 transcript.
    quest = 'oteryn:quest.the_ape_city_quest'
    npc = 'oteryn:npc.hairycles'
    track = 'oteryn:quest-progress/quest/u7_6/the_ape_city/'
    shaman = track + 'shaman_outfit'
    donor = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/hairycles.lua'
    digest = 'b2307616dfa9b6bdff899c60daf02a09980299c80b211d73c948d8681732df73'

    offers.append({
        'npc': npc, 'keywords': ['mission'], 'topic': 0, 'next_topic': 17,
        'storage': [],
        'requires': [{'track': QUESTLINE, 'minimum': 16, 'maximum': 16}],
        'reply': [
            'You have proven yourself a friend, me will grant you permission to enter the deepest catacombs under Banuta which we have sealed in the past ...',
            'Me still can sense the evil presence there. We did not dare to go deeper and fight creatures of evil there ...',
            'You may go there, fight the evil and find the monument of the serpent god and destroy it with hammer me give to you ...',
            'Only then my people will be safe. Please tell Hairycles, will you go there?',
        ],
    })
    callbacks.append({
        'key': 'oteryn:quest-reward/ape-city-catacombs-accept',
        'quest': quest, 'npc': npc, 'keywords': ['yes'], 'topic': 17, 'next_topic': 0,
        'storage': [{'track': QUESTLINE, 'minimum': 16, 'maximum': 16, 'set': 17}],
        'take': [],
        'give': {'item': 'oteryn:item.tibia.i4835', 'count': 1, 'contents': []},
        'outfit': None, 'achievement': None,
        'reply': ['Hairycles sure you will make it. Just use hammer on all that looks like snake or lizard. Tell Hairycles if you succeed with mission.'],
        'sources': [GLOBAL, f'{donor}:174-181:sha256:{digest}', f'{donor}:442-450:sha256:{digest}'],
    })

    shaman_ready = {'track': QUESTLINE, 'minimum': 18, 'maximum': 18}
    shaman_unclaimed = {'track': shaman, 'minimum': 0, 'maximum': 0}
    offers.append({
        'npc': npc, 'keywords': ['outfit', 'shamanic powers'], 'topic': 0, 'next_topic': 18,
        'storage': [], 'requires': [shaman_ready, shaman_unclaimed],
        'reply': ['Me truly proud of you, friend. You learn many about plants, charms and ape people. Me want grant you shamanic power now. You ready?'],
    })
    callbacks.append({
        'key': 'oteryn:quest-reward/ape-city-shaman',
        'quest': quest, 'npc': npc, 'keywords': ['yes'], 'topic': 18, 'next_topic': 0,
        'storage': [{**shaman_unclaimed, 'set': 1}], 'requires': [shaman_ready],
        'take': [], 'give': None,
        'outfit': {'outfit': 'oteryn:outfit.shaman', 'addons': 0},
        'achievement': None,
        'reply': ['Friend of the ape people! Take my gift and become me apprentice! Here is shaman clothing for you!'],
        'sources': [GLOBAL, f'{donor}:212-222:sha256:{digest}', f'{donor}:451-461:sha256:{digest}'],
    })
