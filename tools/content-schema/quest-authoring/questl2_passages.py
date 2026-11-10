"""L2 passage hand-ins reuse the fenced reward and paid travel owners."""
import hashlib

NPCS = ('dalbrect',)
ITEMS = {3205, 3212, 3214}


def append(add, state, callbacks, offers, inputs, pin):
    root = 'oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/'
    quest = 'the_white_raven_monastery_quest'

    def witness(npc, lines):
        path = f'data-global/npc/{npc}.lua'
        return f'crystal:{pin}:{path}:{lines}:sha256:{hashlib.sha256(inputs[path]).hexdigest()}'

    def offer(npc, words, topic, following, storage, reply, requires=()):
        offers.append({'npc': 'oteryn:npc.' + npc, 'keywords': words,
                       'topic': topic, 'next_topic': following, 'storage': storage,
                       'reply': [reply], **({'requires': list(requires)} if requires else {})})

    passage = [state(root + 'passage', 0, 2, 1), state(root + 'quest_log', 0, 1)]
    add('l2-white-raven-brooch', quest, 'dalbrect', ['yes'], passage,
        take=[(3205, 1)], topic=2)
    callbacks[-1].update(sources=[witness('dalbrect', '61-98')],
        reply=['Thank you! I shall consider you my friend from now on! Just let me know if you {need} something!'])
    pending = [state(root + 'passage', 0, 0, 1, preserve=True)]
    offer('dalbrect', ['brooch'], 0, 1, pending, 'What? You want me to examine a brooch?')
    offer('dalbrect', ['yes'], 1, 2, pending,
          "Can it be? I recognize my family's arms! Please give that brooch to me.")

    # The current Dalbrect source writes WhiteRaven QuestLog, while Costello
    # still names the obsolete FamilyBrooch flag. Use the atomic completion
    # written by the actual hand-in as the coherent native prerequisite.
    friendship = [{'track': root + 'quest_log', 'minimum': 1, 'maximum': 1}]
    entrance = [state(root + 'diary', 0, 1), state(root + 'door', 0, 1)]
    add('l2-white-raven-catacombs', quest, 'costello', ['yes'], entrance,
        topic=1, requires=friendship)
    callbacks[-1].update(sources=[witness('costello', '52-84'), witness('dalbrect', '97-98')],
        reply=['Thank you very much! From now on you may open the warded doors to the catacombs.'])
    offer('costello', ['fugio'], 0, 1,
          [state(root + 'diary', 0, 0, preserve=True)],
          'Perhaps you are willing to investigate this matter?', friendship)
    diary = [state(root + 'diary', 1, 2)]
    add('l2-white-raven-diary', quest, 'costello', ['yes'], diary,
        take=[(3212, 1)], give=(3214, 1), topic=2, requires=friendship)
    callbacks[-1].update(sources=[witness('costello', '63-93')],
        reply=["You have done our order a great favour by giving this diary to me! Take this blessed Ankh."])
    offer('costello', ['diary'], 0, 2,
          [state(root + 'diary', 1, 1, preserve=True)],
          'Do you want me to inspect a diary?', friendship)
