"""Retained Crystal Wizard addon sources; registered by the reward producer."""

DONOR_NPCS = ('the_queen_of_the_banshees', 'lugri')


def append(add, storage, offers, prefix):
    # Lugri's source sacrifices advance one shared Wizard track; the first
    # confirmation is dialogue only, and both source looks receive the skull.
    wizard=prefix+'u7_8/wizard_outfits'
    states=[storage(wizard,0,1),storage('oteryn:quest-progress/outfit_quest/default_start',0,1,1)]
    add('l2-wizard-start','outfit_and_addon_quests','lugri',['yes'],states,topic=2)
    for words,topic,next_topic in [(['outfit','addon'],0,1),(['yes'],1,2)]:
        offers.append({'npc':'oteryn:npc.lugri','keywords':words,'topic':topic,'next_topic':next_topic,
            'storage':states,'reply':['Are you willing to prove your loyalty through these sacrifices?']})
    for stage,suffix,words,item in [(1,'shield',['shield','medusa shield'],3436),
            (2,'mail',['mail','dragon scale mail'],3386),(3,'legs',['legs','crown legs'],3382),
            (4,'skull',['ring','ring of the sky'],3006)]:
        states=[storage(wizard,stage,stage+1)]
        add('l2-wizard-'+suffix,'outfit_and_addon_quests','lugri',['yes'],states,
            take=[(item,1)],topic=stage+2,outfit=('wizard',2) if stage==4 else None)
        offers.append({'npc':'oteryn:npc.lugri','keywords':words,'topic':0,'next_topic':stage+2,
            'storage':states,'reply':['Is it your true wish to sacrifice this to Zathroth?']})
    states=[storage(wizard,5,6)]
    add('l2-wizard-flowers-start','outfit_and_addon_quests','the_queen_of_the_banshees',
        ['yes'],states,topic=10)
    for words,topic,next_topic in [(['addon'],0,9),(['yes'],9,10)]:
        offers.append({'npc':'oteryn:npc.the_queen_of_the_banshees','keywords':words,
            'topic':topic,'next_topic':next_topic,'storage':states,
            'reply':['Could you please bring me 50 holy orchids?']})
    states=[storage(wizard,6,7)]
    add('l2-wizard-flowers','outfit_and_addon_quests','the_queen_of_the_banshees',['yes'],states,
        take=[(5922,50)],topic=11,outfit=('wizard',1),achievement='oteryn:achievement/warlock')
    offers.append({'npc':'oteryn:npc.the_queen_of_the_banshees','keywords':['orchid','holy orchid'],
        'topic':0,'next_topic':11,'storage':states,'reply':['Have you really brought me 50 holy orchids?']})
