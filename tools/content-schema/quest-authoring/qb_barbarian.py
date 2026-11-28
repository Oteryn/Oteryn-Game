"""QB: Barbarian addon conversations (Bron and Ajax); registered by the reward producer.

Global parity (TibiaWiki `Barbarian_Outfits_Quest/Spoiler`): Bron asks the player to teach his
brother Ajax that violence is not the answer; Ajax is provoked with "MINE", "no" and
"say please" and, after one hour, reports back. Bron then wants a fighting spirit, 50 red and
50 green pieces of cloth, 10 spools of yarn and a warrior's sweat before he gives the wig
(addon 2). Ajax wants 100 iron ore, a huge chunk of crude iron, 50 perfect behemoth fangs and
50 lizard leathers, and forges the axe (addon 1) two hours later with the Brutal Politeness
achievement (Crystal 201189f8, data-global/npc/ajax.lua:134-141).

Not modelled: Ajax setting the player on fire when he says please, the one hour of silence
that restarts if the player talks earlier, and the stage-dependent greetings of both NPCs.
The Ajax report is bound to the words "back" and "brother" instead of his greeting.
"""

DONOR_NPCS = ("ajax", "bron")

ONE_HOUR = 3601
TWO_HOURS = 7201


def append(add, storage, offers, callbacks, prefix):
    general = "outfit_and_addon_quests"
    barbarian = prefix + "u7_8/barbarian_outfits/"
    stage, timer = (
        barbarian + "barbarian_addon",
        barbarian + "barbarian_addon_wait_timer",
    )

    def offer(npc, words, topic, next_topic, states, *reply):
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": words,
                "topic": topic,
                "next_topic": next_topic,
                "storage": states,
                "reply": list(reply),
            }
        )

    def callback(name, npc, words, states, reply, **kwargs):
        add("qb-barbarian-" + name, general, npc, words, states, **kwargs)
        callbacks[-1].update(reply=[reply], requires_premium=True)

    def timer_set():
        callbacks[-1]["state_ops"] = [
            {"track": timer, "from": {"kind": "any"}, "effect": {"kind": "set_now"}}
        ]

    def timer_elapsed(seconds):
        callbacks[-1]["state_ops"] = [
            {
                "track": timer,
                "from": {"kind": "elapsed_at_least", "seconds": seconds},
                "effect": {"kind": "add", "value": 0},
            }
        ]

    def at(before, after):
        return [storage(stage, before, after)]

    # Bron asks for help with his brother.
    states = at(0, 1)
    offer(
        "bron",
        ["recruitment", "violence", "outfit", "addon"],
        0,
        1,
        states,
        "Convincing Ajax that it is not always necessary to use brute force... this would be "
        "such an achievement. Definitely a hard task though. ...",
        "Listen, I simply have to ask, maybe a stranger can influence him better than I can. "
        "Would you help me with my brother?",
    )
    offer(
        "bron",
        ["yes"],
        1,
        2,
        states,
        "Really! That is such an incredibly nice offer! I already have a plan. You have to teach "
        "him that sometimes words are stronger than fists. ...",
        "Maybe you can provoke him with something to get angry, like saying... 'MINE!' or "
        "something. But beware, I'm sure that he will try to hit you. ...",
        "Don't do this if you feel weak or ill. He will probably want to make you leave by using "
        "violence, but just stay strong and refuse to give up. ...",
        "If he should ask what else is necessary to make you leave, tell him to 'say please'. "
        "Afterwards, do leave and return to him one hour later. ...",
        "This way he might learn that violence doesn't always help, but that a friendly word "
        "might just do the trick. ...",
        "Have you understood everything I told you and are really willing to take this risk?",
    )
    callback(
        "bron-start",
        "bron",
        ["yes"],
        states,
        "You are indeed not only well educated, but also very courageous. I wish you good "
        "luck, you are my last hope.",
        topic=2,
    )

    # Ajax is provoked; the one-hour wait starts at "yes" after "say please".
    states = at(1, 2)
    offer(
        "ajax",
        ["mine"],
        0,
        1,
        at(1, 1),
        "YOURS? WHAT IS YOURS! NOTHING IS YOURS! IS MINE! GO AWAY, YES?!",
    )
    offer(
        "ajax",
        ["no"],
        1,
        2,
        at(1, 1),
        "YOU STUPID! STUBBORN! I KILL YOU! WILL LEAVE NOW?!",
    )
    offer(
        "ajax",
        ["no"],
        2,
        3,
        at(1, 1),
        "ARRRRRRRRRR! YOU ME DRIVE MAD! HOW I MAKE YOU GO??",
    )
    offer("ajax", ["no"], 3, 4, at(1, 1), "I GIVE YOU NO!")
    offer(
        "ajax",
        ["say please"],
        4,
        5,
        at(1, 1),
        "Please? What you mean please? Like I say please you say bye? Please?",
    )
    callback(
        "ajax-please",
        "ajax",
        ["yes"],
        states,
        "Oh. Easy. Okay. Please is good. Now don't say anything. Head aches.",
        topic=5,
    )
    timer_set()
    callback(
        "ajax-wait",
        "ajax",
        ["back", "brother"],
        at(2, 3),
        "You back. You know, you right. Brother is right. Fist not always good. Tell him that!",
    )
    timer_elapsed(ONE_HOUR)

    # Bron hears the report and sends the player back to Ajax about Gelagos.
    offer(
        "bron",
        ["brother is right. fist not always good."],
        0,
        3,
        at(3, 3),
        "Oh! He really said that? I am so proud of you. These are really good news. Everything "
        "would be great... if only there wasn't this person near my house.",
    )
    offer(
        "bron",
        ["person"],
        3,
        4,
        at(3, 3),
        "This... person... makes me want to... say something bad... must... control myself. "
        "<sweats> I really don't know what to do anymore. ...",
        "I wonder if Ajax has an idea. Could you ask him about Gelagos?",
    )
    callback(
        "bron-person",
        "bron",
        ["yes"],
        at(3, 4),
        "Again, I have to thank you for your selfless offer to help me. I hope that Ajax can "
        "come up with something, now that he has experienced the power of words.",
        topic=4,
    )
    offer(
        "ajax",
        ["gelagos"],
        0,
        6,
        at(4, 4),
        "Annoying kid. Bro hates him, but talking no help. Bro needs fighting spirit!",
    )
    callback(
        "ajax-gelagos",
        "ajax",
        ["fighting spirit"],
        at(4, 5),
        "If you want to help bro, bring him fighting spirit. Magic fighting spirit. Ask Djinn.",
        topic=6,
    )

    # The fighting spirit extracted by the djinn turns Bron mighty for a moment.
    callback(
        "bron-spirit",
        "bron",
        ["fighting spirit"],
        at(5, 6),
        "Fighting spirit? What am I supposed to do with this fi... - oh! I feel strange... ME "
        "MIGHTY! ME WILL CHASE OFF ANNOYING KIDS! GROOOAARR!! RRRRRRRRRRRRAAAAAAAGE!!",
        take=[(5884, 1)],
    )
    offer(
        "bron",
        ["yes"],
        0,
        6,
        at(6, 6),
        "I'm impressed... I am sure this was Ajax' idea. I would love to give him a present, but "
        "if I leave my hut to gather ingredients, he will surely notice. ...",
        "Would you maybe help me again, one last time, my friend? I assure you that your efforts "
        "will not be in vain.",
    )
    offer(
        "bron",
        ["yes"],
        6,
        7,
        at(6, 6),
        "Great! You see, I really would love to sew a nice shirt for him. I just need a few "
        "things for that, so please listen closely: ...",
        "He loves green and red, so I will need about 50 pieces of red cloth - like the material "
        "heroes make their capes of - and 50 pieces of the green cloth Djinns like. ...",
        "Secondly, I need about 10 rolls of spider silk yarn. I think mermaids can yarn silk of "
        "large spiders to create a smooth thread. ...",
        "The only remaining thing needed would be a bottle of warrior's sweat to spray it over "
        "the shirt... he just loves this smell. ...",
        "Have you understood everything I told you and are willing to handle this task?",
    )
    callback(
        "bron-task",
        "bron",
        ["yes"],
        at(6, 7),
        "Thank you, my friend! Come back to me once you have collected 50 pieces of red cloth "
        "and 50 pieces of green cloth.",
        topic=7,
    )

    # Bron's shirt materials, one stage each, and then the wig.
    for suffix, words, reply, topic, before in [
        (
            "cloth",
            ["red piece of cloth"],
            (
                "Terrific! I will start to trim it while you gather 10 rolls of spider silk. "
                "I'm sure that Ajax will love it."
            ),
            8,
            7,
        ),
        (
            "yarn",
            ["spider silk", "rolls of spider silk"],
            (
                "I'm impressed! You really managed to get spider silk yarn for me! I will immediately "
                "start to work on this shirt. Please don't forget to bring me warrior's sweat!"
            ),
            9,
            8,
        ),
        (
            "sweat",
            ["warrior's sweat", "warriors sweat"],
            (
                "Good work! Now I can finally finish this present for Ajax. Because you were such a "
                "great help, I have also a present for you. Will you accept it?"
            ),
            10,
            9,
        ),
    ]:
        question = {
            "cloth": "Have you really managed to fulfil the task and brought me 50 pieces of red "
            "cloth and 50 pieces of green cloth?",
            "yarn": "Oh, did you bring 10 rolls of spider silk yarn for me?",
            "sweat": "Were you able to get hold of a flask with pure warrior's sweat?",
        }[suffix]
        take = {
            "cloth": [(5910, 50), (5911, 50)],
            "yarn": [(5886, 10)],
            "sweat": [(5885, 1)],
        }[suffix]
        offer("bron", words, 0, topic, at(before, before), question)
        callback(
            "bron-" + suffix,
            "bron",
            ["yes"],
            at(before, before + 1),
            reply,
            take=take,
            topic=topic,
        )
    callback(
        "bron-wig",
        "bron",
        ["yes"],
        at(10, 11),
        "I have kept this traditional barbarian wig safe for many years now. It is now yours! "
        "I hope you will wear it proudly, friend.",
        outfit=("barbarian", 2),
    )

    # Ajax's own present: the axe, forged two hours after the last delivery.
    offer(
        "ajax",
        ["present"],
        0,
        6,
        at(11, 11),
        "Bron gave me present. Ugly, but nice from him. Me want to give present too. You help me?",
    )
    offer(
        "ajax",
        ["yes"],
        6,
        7,
        at(11, 11),
        "Good! Me make shiny weapon. If you help me, I make one for you too. Like axe I wear. I "
        "need stuff. Listen. ...",
        "Me need 100 iron ore. Then need crude iron. Then after that 50 behemoth fangs. And 50 "
        "lizard leather. You understand?",
    )
    callback(
        "ajax-axe-start",
        "ajax",
        ["yes"],
        at(11, 12),
        "Good. You get 100 iron ore first. Come back.",
        topic=7,
    )
    for suffix, words, question, item, count, reply, before in [
        (
            "ore",
            ["iron ore"],
            "You bring 100 iron ore?",
            5880,
            100,
            "Good! Now bring crude iron.",
            12,
        ),
        (
            "crude-iron",
            ["crude iron"],
            "You bring crude iron?",
            5892,
            1,
            "Good! Now bring 50 behemoth fangs.",
            13,
        ),
        (
            "fangs",
            ["behemoth fang", "behemoth fangs"],
            "You bring 50 behemoth fangs?",
            5893,
            50,
            "Good! Now bring 50 lizard leather.",
            14,
        ),
        (
            "leather",
            ["lizard leather"],
            "You bring 50 lizard leather?",
            5876,
            50,
            "Ah! All stuff there. I will start making axes now. Come later and ask me for axe.",
            15,
        ),
    ]:
        offer("ajax", words, 0, before - 4, at(before, before), question)
        callback(
            "ajax-" + suffix,
            "ajax",
            ["yes"],
            at(before, before + 1),
            reply,
            take=[(item, count)],
            topic=before - 4,
        )
    timer_set()
    callback(
        "ajax-axe",
        "ajax",
        ["axe"],
        at(16, 17),
        "Axe is done! For you. Take. Wear like me.",
        outfit=("barbarian", 1),
        achievement="oteryn:achievement/brutal_politeness",
    )
    timer_elapsed(TWO_HOURS)
