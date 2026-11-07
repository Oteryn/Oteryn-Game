"""Source Friends and Traders introductions and exact fixed material recipes."""

NPCS = ('a_sweaty_cyclops', 'marina')
# Other ingredients and outputs retain their already admitted richer providers.
ITEMS = {3560, 5886, 12804}


def append(add, state, callbacks, offers, inputs):
    emit = add
    def add(name, quest, npc, words, storage, reply, **kwargs):
        if reply not in inputs[f'data-global/npc/{npc}.lua'].decode():
            raise ValueError('Pinned exchange reply drift: ' + name)
        emit(name, quest, npc, words, storage, reply, **kwargs)
    prefix = 'u7_8/friends_and_traders/'
    def question(npc, words, topic, next_topic, storage, reply):
        if reply not in inputs[f'data-global/npc/{npc}.lua'].decode():
            raise ValueError('Pinned exchange question drift: ' + npc)
        offers.append({'npc':'oteryn:npc.'+npc,'keywords':words,'topic':topic,
                       'next_topic':next_topic,'storage':storage,'reply':[reply]})
    cyclops = 'a_sweaty_cyclops'
    track = prefix+'the_sweaty_cyclops'
    start = [state(prefix+'default_start',-1,1,1),state(track,-1,1,0)]
    question(cyclops,["uth'lokr"],0,1,start,
        "Firy steel it is. Need green ones' breath to melt. Or red even better. Me can make from shield. Lil' one want to trade?")
    question(cyclops,['yes'],1,2,start,"Wait. Me work no cheap is. Do favour for me first, yes?")
    add('exchange-cyclops-start','friends_and_traders',cyclops,['yes'],start,
        'Me need gift for woman. We dance, so me want to give her bast skirt. But she big is. So I need many to make big one. Bring three okay? Me wait.',topic=2)
    callbacks[-1]['next_topic'] = 3
    add('exchange-cyclops-skirts','friends_and_traders',cyclops,['yes'],[state(track,1,2)],
        'Good good! Woman happy will be. Now me happy too and help you.',take=[(3560,3)],topic=4,
        offer=(["uth'lokr"],'Lil\' one bring three bast skirts.'))
    for name, word, source, reward, prompt, topic in [
        ('royal-steel',"uth'kean",3381,5887,'Very noble. Shiny. Me like. But breaks so fast. Me can make from shiny armour. Lil\' one want to trade?',5),
        ('draconian-steel',"uth'lokr",3416,5889,"Firy steel it is. Need green ones' breath to melt. Or red even better. Me can make from shield. Lil' one want to trade?",6),
        ('hell-steel',"za'ralator",3356,5888,'Hellsteel is. Cursed and evil. Dangerous to work with. Me can make from evil helmet. Lil\' one want to trade?',7),
        ('crude-iron',"uth'prta",3281,5892,'Good iron is. Me friends use it much for fight. Me can make from weapon. Lil\' one want to trade?',8),
    ]:
        add('exchange-cyclops-'+name,'friends_and_traders',cyclops,['yes'],
            [state(track,2,2,preserve=True)],'Cling clang!',take=[(source,1)],give=(reward,1),
            topic=topic,offer=([word],prompt),repeatable=True,
            source_text=f'player:removeItem({source}, 1)')
    donor = inputs['data-global/npc/a_sweaty_cyclops.lua'].decode()
    for fact in ('local count = player:getItemCount(5944)', 'player:removeItem(5944, count)',
                 'math.random(100) <= 1', 'total = total + 200', 'total = total + 100',
                 'player:addItem(6528, total)'):
        if fact not in donor:
            raise ValueError('Cyclops bulk source drift: ' + fact)
    add('exchange-cyclops-soul-orbs','friends_and_traders',cyclops,['yes'],
        [state(track,2,2,preserve=True)],'Cling clang!',take=[(5944,1)],give=(6528,100),
        topic=9,repeatable=True,source_text='local count = player:getItemCount(5944)',
        offer=(['soul orb'],'Me can make nasty bolt from soul orbs. Trade all?'))
    callbacks[-1]['bulk'] = {'item':'oteryn:item.tibia.i5944','output':'oteryn:item.tibia.i6528',
                             'base':100,'bonus':100,'chance_percent':1}
    # These source recipes are unconditional, unlike the favour-gated steel menu.
    neutral = [state('exchange/friends_and_traders/metalwork',-1,-1,9223372036854775807,preserve=True)]
    for name, word, source, reward, reply, prompt, topic in [
        ('gold-melt','melt',9058,12804,'Whoooosh There!',"Can melt gold ingot for li'l one. You want?",10),
        ('gear-wheel','gear whell',5880,8775,'Cling clang!','Want to make gear wheel from iron ore?',11),
    ]:
        add('exchange-cyclops-'+name,'friends_and_traders',cyclops,['yes'],neutral,reply,
            take=[(source,1)],give=(reward,1),topic=topic,offer=([word],prompt),repeatable=True,
            source_text=f'player:removeItem({source}, 1)')
        if name == 'gold-melt':
            callbacks[-1]['give']['decay'] = {'duration_millis': 36000000, 'target': None}
    marina='marina'; track=prefix+'the_mermaid_marina'
    start=[state(prefix+'default_start',-1,1,1),state(track,-1,1,0)]
    words=['silk','yarn','silk yarn','spool of yarn','10 pieces of spider silk']
    question(marina,words,0,1,start,"Um. You mean, you really want me to touch that gooey spider silk just because you need yarn? Well... do you think that I'm pretty?")
    question(marina,['yes'],1,2,start,"Well, everyone would say that in your position. Do you think that I'm really, absolutely the most stunning being that you have ever seen?")
    add('exchange-marina-start','friends_and_traders',marina,['yes'],start,
        'I love honey and I haven\'t eaten it in a while, so bring me 50 honeycombs and worship my beauty a little more, then we will see.',topic=2)
    add('exchange-marina-honey','friends_and_traders',marina,['yes'],[state(track,1,2)],
        'Oh goodie! Thank you! Okay... I guess since my fingers are sticky now anyway, I will help you. From now on, if you bring me {10 pieces of spider silk}, I will create one {spool of yarn}.',
        take=[(5902,50)],topic=4,offer=(['honey','honeycomb','50 honeycombs'],
        'Did you bring me the 50 honeycombs I requested and do you absolutely admire my beauty?'))
    add('exchange-marina-yarn','friends_and_traders',marina,['yes'],[state(track,2,2,preserve=True)],
        'Ew... gooey... there you go.',take=[(5879,10)],give=(5886,1),topic=5,
        repeatable=True,offer=(words,'Okay... a deal is a deal, would you like me to create a {spool of yarn} from {10 pieces of spider silk}?'))
