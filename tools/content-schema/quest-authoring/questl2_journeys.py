"""L2 source mission issuance and item lifecycle branches through native owners."""

# The owner explicitly requires XP for Into The Bone Pit; pinned Muriel has no
# addExperience call. Conservative local completion rule: 1,000 XP only when the
# actual filled sample is returned. Lost-sample completion grants nothing.
BONE_PIT_RETURN_EXPERIENCE = 1000
BONE_PIT_LEVEL_REPLY = 'Indeed there is something to be done, but I need someone more experienced. Come back later if you want to.'


def append(add, state, callbacks, inputs):
    add('outfox-start', 'tibia_tales', 'budrik', ['yes'],
        [state('u8_1/tibia_tales/default_start', 0, 1, 1),
         state('u8_1/to_outfox_afox_quest', 0, 1)],
        'I knew you have the guts for that task! We presume the hideout of The Horned Fox somewhere in the south-west near the coast. Good luck!',
        topic=1, offer=(['mission', 'quest'],
            'It belonged to my father and before that to my grandfather. That helmet is at least 600 years old! I need it back. Are you willing to help me?'))
    track = 'u8_1/tibia_tales/into_the_bone_pit'
    add('bone-pit-issue', 'tibia_tales', 'muriel', ['yes'],
        [state(track, 0, 1)],
        'Great! Here is the container for the bone. Once, I used it to collect ectoplasma of ghosts, but it will work here as well. ...',
        give=(4852, 1), topic=1, offer=(['mission'],
            'Find that pit, dig for a well-preserved human skeleton and conserve a sample in a special container which you receive from me. Are you going to help me?'))
    warning = BONE_PIT_LEVEL_REPLY
    if warning not in inputs['data-global/npc/muriel.lua'].decode():
        raise ValueError('Bone Pit level warning drift')
    callbacks[-1].update(minimum_level=35, failure_reply=[warning])
    add('bone-pit-return', 'tibia_tales', 'muriel', ['mission'],
        [state(track, 2, 3)],
        'Excellent! Now I can try to put my theoretical thoughts into practice and find a cure for the symptoms of undead. Here, take this for your efforts.',
        take=[(131, 1)], give=(6299, 1))
    callbacks[-1].update(minimum_level=35, experience=BONE_PIT_RETURN_EXPERIENCE, failure_reply=[BONE_PIT_LEVEL_REPLY],
        inventory=[{'item': 'oteryn:item.tibia.i131', 'minimum': 1, 'maximum': 4294967295}])
    add('bone-pit-lost-sample', 'tibia_tales', 'muriel', ['mission'],
        [state(track, 2, 3)],
        'I am so glad you are still alive. Benjamin found the container with the bone sample inside. Fortunately, I inscribe everything with my name, so he knew it was mine. ...')
    callbacks[-1].update(minimum_level=35, failure_reply=[BONE_PIT_LEVEL_REPLY],
        inventory=[{'item': 'oteryn:item.tibia.i131', 'minimum': 0, 'maximum': 0}])
