"""Lane-only chosen programmes extend the adopted main QENGINE2 producer."""
def extend(root, callbacks, offers, inputs, pin, load_module):
    programmes=load_module('questpc7_source_programmes','tools/content-schema/quest-authoring/questpc7_outfit_programmes.py')
    lane_callbacks=[]
    selected = load_module('questpc7_selected_rewards', 'tools/content-schema/quest-authoring/questpc7_selected_rewards.py')
    selected.append_golden(root, lane_callbacks, offers, inputs, pin)
    selected.append_friends(root, lane_callbacks, offers, inputs, pin)
    goblin=load_module('questpc7_goblin_programme','tools/content-schema/quest-authoring/questpc7_goblin_programme.py')
    goblin.append(root,lane_callbacks,inputs,pin)
    selected.append_tokens(root, lane_callbacks, inputs, pin)
    selected.append_eligibility_starts(root, lane_callbacks, inputs, pin)
    programmes.append(root, lane_callbacks, inputs, pin)
    formal_worlds = programmes.append_formal(root, lane_callbacks, inputs, pin)
    signs = load_module('questpc7_goblin_signs', 'tools/content-schema/quest-authoring/questpc7_goblin_signs.py')
    formal_worlds.extend(signs.build(root))
    pond = load_module('questpc7_goblin_pond', 'tools/content-schema/quest-authoring/questpc7_goblin_pond.py')
    formal_worlds.extend(pond.build(root))
    trunks = load_module("questpc7_goblin_trunks", "tools/content-schema/quest-authoring/questpc7_goblin_trunks.py")
    formal_worlds.extend(trunks.build(root))
    stalkers = load_module("questpc7_goblin_stalkers", "tools/content-schema/quest-authoring/questpc7_goblin_stalkers.py")
    stalker_objectives = stalkers.build(root)
    encounters = load_module('questpc7_encounters','tools/content-schema/quest-authoring/questpc7_encounters.py')
    formal_worlds.extend(encounters.worlds(root))
    hoist = load_module('questpc7_hoist_programme','tools/content-schema/quest-authoring/questpc7_hoist_programme.py')
    hoist_worlds, hoist_bindings = hoist.build(root)
    formal_worlds.extend(hoist_worlds)
    hydra = load_module('questpc7_hydra_programme','tools/content-schema/quest-authoring/questpc7_hydra_programme.py')
    hydra_worlds, hydra_objectives = hydra.build(root)
    formal_worlds.extend(hydra_worlds)
    hero = load_module('questpc7_hero_programme','tools/content-schema/quest-authoring/questpc7_hero_programme.py')
    hero_worlds, hero_objectives = hero.build(root)
    formal_worlds.extend(hero_worlds)
    ferumbras = load_module('questpc7_ferumbras_programme','tools/content-schema/quest-authoring/questpc7_ferumbras_programme.py')
    ferumbras_worlds, ferumbras_objectives = ferumbras.build(root)
    formal_worlds.extend(ferumbras_worlds)
    grave = load_module('questpc7_grave_programme','tools/content-schema/quest-authoring/questpc7_grave_programme.py')
    grave_worlds, grave_objectives = grave.build(root)
    formal_worlds.extend(grave_worlds)
    grimvale = load_module('questpc7_grimvale_programme','tools/content-schema/quest-authoring/questpc7_grimvale_programme.py')
    grimvale_worlds, grimvale_objectives, grimvale_callbacks = grimvale.build(root)
    formal_worlds.extend(grimvale_worlds)
    lane_callbacks.extend(grimvale_callbacks)
    heart = load_module('questpc7_heart_programme','tools/content-schema/quest-authoring/questpc7_heart_programme.py')
    heart_worlds, heart_objectives, heart_callbacks = heart.build(root, inputs, pin)
    formal_worlds.extend(heart_worlds)
    lane_callbacks.extend(heart_callbacks)
    forgotten = load_module('questpc7_forgotten_programme','tools/content-schema/quest-authoring/questpc7_forgotten_programme.py')
    forgotten_worlds, forgotten_objectives, forgotten_callbacks = forgotten.build(root, inputs, pin)
    formal_worlds.extend(forgotten_worlds)
    lane_callbacks.extend(forgotten_callbacks)
    smith = load_module('questpc7_smith_programme','tools/content-schema/quest-authoring/questpc7_smith_programme.py')
    smith_worlds, smith_objectives = smith.build(root)
    formal_worlds.extend(smith_worlds)
    feaster = load_module('questpc7_feaster_programme','tools/content-schema/quest-authoring/questpc7_feaster_programme.py')
    feaster_worlds, _, feaster_callbacks = feaster.build(root)
    formal_worlds.extend(feaster_worlds)
    lane_callbacks.extend(feaster_callbacks)
    main_keys={row['key'] for row in callbacks}
    callbacks.extend(row for row in lane_callbacks if row['key'] not in main_keys)
    for row in formal_worlds:
        if row.get('outfit_reward') is not None:
            row.setdefault('effects',{})['outfit_reward']=row.pop('outfit_reward')
    return formal_worlds, encounters.objectives(root)+hydra_objectives+hero_objectives+ferumbras_objectives+grave_objectives+grimvale_objectives+heart_objectives+forgotten_objectives+stalker_objectives+smith_objectives,hoist_bindings

def outputs(root, inputs, pin, load_module, lane):
    import json
    from questpc7_feaster_programme import PHASE_ROUTE_KEYS
    encoded=lambda value:(json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode()
    worlds,objectives,bindings=lane
    phase_routes = [row for row in worlds if row['key'] in PHASE_ROUTE_KEYS]
    if len(phase_routes) != 2 or {row['key'] for row in phase_routes} != PHASE_ROUTE_KEYS:
        raise ValueError('QUESTPC7_FEASTER_PHASE_OWNERSHIP_DRIFT')
    main_worlds = [row for row in worlds if row['key'] not in PHASE_ROUTE_KEYS]
    knowledge=load_module('questpc7_knowledge','tools/content-schema/quest-authoring/questpc7_knowledge.py')
    props=load_module('questpc7_props','tools/content-schema/quest-authoring/questpc7_props.py')
    planned_props=props.outputs(root)
    smith=load_module('questpc7_smith_programme','tools/content-schema/quest-authoring/questpc7_smith_programme.py')
    smith.placements(root,planned_props)
    from questpc7_index_witness import rebind
    rebind(root,worlds,planned_props)
    return {
        'content/quests/questpc7-world-callbacks.json':encoded({'schema':1,'callbacks':main_worlds}),
        'content/quests/world-callbacks/feaster-phase-routes.json':encoded({'schema':1,'callbacks':phase_routes}),
        'content/quests/questpc7-death-objectives.json':encoded({'schema':1,'objectives':objectives}),
        'content/quests/native-world-callback-bindings.json':encoded({'schema':1,'bindings':bindings}),
        'content/quests/imbuement-knowledge.json':encoded(knowledge.build(root,inputs,pin)),
        **planned_props,
    }
