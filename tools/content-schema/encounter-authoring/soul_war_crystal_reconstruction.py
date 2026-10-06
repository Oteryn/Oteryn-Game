from __future__ import annotations
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
OUT = ROOT / "samples" / "soul-war-crystal-reconstruction"
REV = "canary-47dfd51f"
CRYSTAL = "00ce02a57ca5a12e48f32a3476e37471167e4c3f"

def cref(name):
    slug = "".join(c if c.isalnum() else "_" for c in name.lower()).strip("_")
    while "__" in slug:
        slug = slug.replace("__", "_")
    return {"family":"Creature","key":f"canary:creature/{slug}","revision":REV}

def point(key,x,y,z,description=None):
    return {"key":key,"kind":"point","description":description or f"Crystal ({x}, {y}, {z}).","location":{"x":x,"y":y,"floor":z}}

def area(key,x0,x1,y0,y1,z,description=None):
    return {"key":key,"kind":"area","description":description or f"Crystal x {x0}-{x1}, y {y0}-{y1}, z {z}.",
            "location":{"boxes":[{"x":[x0,x1],"y":[y0,y1],"floor":z}]}}

def base(name, display, participants, anchors, counters=None, flags=None, timers=None, outcomes=None):
    return {
        "identity":{"key":f"crystal:encounter/{name}","revision":"crystal-00ce02a5"},
        "display_name":display,
        "scope":"instance_per_party",
        "participants":[{"role":r,"creatures":[cref(n) for n in names]} for r,names in participants],
        "anchors":anchors,
        "phases":[],
        "state":{"counters":counters or [],"flags":flags or [],"timers":timers or []},
        "rules":[],
        "outcomes":outcomes or [],
    }

def catalog(enc):
    seen=[]
    for p in enc["participants"]:
        for ref in p["creatures"]:
            if ref not in seen: seen.append(ref)
    for rule in enc["rules"]:
        def walk(v):
            if isinstance(v,dict):
                if set(v)=={"family","key","revision"} and v not in seen: seen.append(v)
                for x in v.values(): walk(x)
            elif isinstance(v,list):
                for x in v: walk(x)
        walk(rule)
    return {"definitions":seen}

def src(path, sha):
    return {"kind":"git","repository":"zimbadev/crystalserver","revision":CRYSTAL,"path":path,"blob_sha1":sha}

def manifest(key, sources, entries, covers):
    return {"encounter":key,"classification":"SOURCE_BACKED_OTERYN_RECONSTRUCTION",
            "covers":covers,"sources":sources,"entries":entries,"outcome_evidence":[]}

def entry(source_index, lines, status, resolution, destination=None):
    row={"source_index":source_index,"source_lines":lines,"status":status,"resolution":resolution}
    if destination: row["destination"]=destination
    return row

def greed():
    participants=[
      ("goshnars_greed",["Goshnar's Greed"]),("greedbeast",["Greedbeast"]),
      ("soulsnatcher",["Soulsnatcher"]),("weak_soul",["Weak Soul"]),
      ("strong_soul",["Strong Soul"]),("powerful_soul",["Powerful Soul"]),
      ("soul_sphere",["Soul Sphere"]),
    ]
    anchors=[
      area("arena",33737,33755,31658,31673,14),
      point("boss_normal",33746,31666,14), point("boss_shield",33741,31659,14),
      point("soul_sphere_spawn",33752,31659,14),
      point("greedbeast_return",33744,31666,14), point("soulsnatcher_return",33747,31668,14),
      point("weak_soul_return",33750,31666,14), point("strong_soul_return",33750,31666,14),
      point("powerful_soul_return",33750,31666,14),
    ]
    e=base("goshnars_greed_crystal","Soul War: Goshnar's Greed (Crystal reconstruction)",participants,anchors,
           counters=[{"name":"greedbeast_kills","initial":0}],
           flags=[{"name":"boss_immune","initial":True},{"name":"sphere_phase_active","initial":False}],
           timers=[{"name":"sphere_phase","duration_ms":34000,"repeat":False}],
           outcomes=["boss_defeated"])
    rules=e["rules"]
    rules += [
      {"key":"boss_starts_immune","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"flag","flag":"boss_immune","value":True},{"kind":"counter","counter":"greedbeast_kills","operation":"set","value":0}]},
      {"key":"immune_boss_ignores_damage","trigger":{"kind":"damage_taken","role":"goshnars_greed","source":"any"},
       "conditions":[{"kind":"flag","flag":"boss_immune","value":True}],
       "actions":[{"kind":"damage_modifier","role":"goshnars_greed","multiplier_percent":0,"sources":"any","until":"this_hit"}]},
      {"key":"greedbeast_kill_counts","trigger":{"kind":"creature_died","role":"greedbeast"},"conditions":[],
       "actions":[{"kind":"counter","counter":"greedbeast_kills","operation":"add","value":1}]},
      {"key":"five_greedbeasts_start_sphere_phase","trigger":{"kind":"counter_reached","counter":"greedbeast_kills","value":5},"conditions":[],
       "actions":[
         {"kind":"counter","counter":"greedbeast_kills","operation":"set","value":0},
         {"kind":"flag","flag":"boss_immune","value":True},{"kind":"flag","flag":"sphere_phase_active","value":True},
         {"kind":"teleport","who":{"role":"goshnars_greed"},"to":"boss_shield"},
         {"kind":"move_lock","role":"goshnars_greed","locked":True},
         {"kind":"spawn","creature":cref("Soul Sphere"),"role":"soul_sphere","count":1,"at":{"anchor":"soul_sphere_spawn"},"owner":"none","health":"full"},
         {"kind":"timer","timer":"sphere_phase","operation":"start"}]},
      {"key":"soul_sphere_kill_releases_boss","trigger":{"kind":"creature_died","role":"soul_sphere"},"conditions":[],
       "actions":[
         {"kind":"timer","timer":"sphere_phase","operation":"stop"},
         {"kind":"flag","flag":"sphere_phase_active","value":False},{"kind":"flag","flag":"boss_immune","value":False},
         {"kind":"teleport","who":{"role":"goshnars_greed"},"to":"boss_normal"},
         {"kind":"move_lock","role":"goshnars_greed","locked":False}]},
      {"key":"sphere_phase_timeout_releases_boss","trigger":{"kind":"timer_elapsed","timer":"sphere_phase"},
       "conditions":[{"kind":"flag","flag":"sphere_phase_active","value":True}],
       "actions":[
         {"kind":"remove","role":"soul_sphere"},
         {"kind":"flag","flag":"sphere_phase_active","value":False},{"kind":"flag","flag":"boss_immune","value":False},
         {"kind":"teleport","who":{"role":"goshnars_greed"},"to":"boss_normal"},
         {"kind":"move_lock","role":"goshnars_greed","locked":False}]},
      {"key":"weak_soul_becomes_strong","trigger":{"kind":"creature_spawned","role":"weak_soul"},"delay_ms":8000,"conditions":[],
       "actions":[{"kind":"transform","role":"weak_soul","into":cref("Strong Soul"),"health":"full"}]},
      {"key":"strong_soul_becomes_powerful","trigger":{"kind":"creature_spawned","role":"strong_soul"},"delay_ms":8000,"conditions":[],
       "actions":[{"kind":"transform","role":"strong_soul","into":cref("Powerful Soul"),"health":"full"}]},
      {"key":"powerful_soul_buffs_boss","trigger":{"kind":"creature_spawned","role":"powerful_soul"},"delay_ms":8000,"conditions":[],
       "actions":[
         {"kind":"transform","role":"powerful_soul","into":cref("Weak Soul"),"health":"full"},
         {"kind":"attribute","role":"goshnars_greed","attribute":"defense","operation":"add","value":10},
         {"kind":"heal","subject":{"role":"goshnars_greed"},"amount":10000}]},
      {"key":"boss_death","trigger":{"kind":"creature_died","role":"goshnars_greed"},"conditions":[],
       "actions":[{"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    for role,anchor in [("greedbeast","greedbeast_return"),("soulsnatcher","soulsnatcher_return"),
                        ("weak_soul","weak_soul_return"),("strong_soul","strong_soul_return"),("powerful_soul","powerful_soul_return")]:
        rules.append({"key":f"{role}_returns_after_death","trigger":{"kind":"creature_died","role":role},"delay_ms":10000,"conditions":[],
                      "actions":[{"kind":"spawn","creature":next(p["creatures"][0] for p in e["participants"] if p["role"]==role),
                                  "role":role,"count":1,"at":{"anchor":anchor},"owner":"none","health":"full"}]})
    sources=[
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnars_greed.lua","0a869103c09426c6e8816de9b028bc63908681c1"),
      src("data-global/monster/quests/soul_war/weak_soul.lua","b7150efa00f3340240944ffd8233d3e81e2d7a6a"),
      src("data-global/monster/quests/soul_war/strong_soul.lua","ae3615b6eb4937947e5601477a0d8514225bddd7"),
      src("data-global/monster/quests/soul_war/powerful_soul.lua","0ece602eab5b0592925c70b1c96e632c7857154f"),
      src("data-global/monster/quests/soul_war/soulsnatcher.lua","faa819dec6e1b6905f87b5642e454468667dde8f"),
      src("data-global/monster/quests/soul_war/soul_sphere.lua","093eb9e9f2210dd6a93d131bd73b5ab9846e72c6"),
    ]
    entries=[
      entry(1,list(range(138,189)),"mapped","Boss immunity, five-greedbeast phase, sphere timeout and reset are represented by flags, a 34 s timer, fixed anchors and move lock.","/encounter/rules"),
      entry(0,list(range(198,220)),"mapped","GreedMonsterDeath and SoulSphereDeath counters are represented by death triggers and the greedbeast counter.","/encounter/rules"),
      entry(2,list(range(88,103)),"mapped","Weak Soul becomes Strong Soul after 8 s.","/encounter/rules/6"),
      entry(3,list(range(88,103)),"mapped","Strong Soul becomes Powerful Soul after 8 s.","/encounter/rules/7"),
      entry(4,list(range(88,112)),"mapped","Powerful Soul returns to Weak Soul; +10 defense and +10000 current health are represented.","/encounter/rules/8"),
      entry(4,list(range(99,106)),"unresolved_semantics","Powerful Soul also adds +10000 maximum health and +10 reflect per configured damage type; the current Encounter vocabulary has no max-health mutation or additive reflect stack."),
      entry(5,list(range(88,110)),"unresolved_semantics","Soulsnatcher damages every player in the boss zone for 500-1000 after 8 s; current Encounter damage action has no players-in-area subject."),
      entry(6,list(range(90,124)),"unresolved_semantics","Soul Sphere walks one tile west every 3000 ms and fully heals Goshnar's Greed if the next tile contains the boss; scripted per-tile movement/reach is not in Encounter v1."),
      entry(0,[206],"mapped","Each Greed minion death schedules the same minion back after 10 s at its fixed GreedMonsters position.","/encounter/rules"),
    ]
    m=manifest(e["identity"]["key"],sources,entries,{"GreedMonsterDeath":[p["creatures"][0]["key"] for p in e["participants"] if p["role"] in {"greedbeast","soulsnatcher","weak_soul","strong_soul","powerful_soul"}],"SoulSphereDeath":[cref("Soul Sphere")["key"]]})
    return e,catalog(e),m

def malice():
    participants=[("goshnars_malice",["Goshnar's Malice"]),("soul_cage",["Soul Cage"])]
    anchors=[area("arena",33699,33718,31590,31607,14),point("soul_cage_spawn",33709,31596,14)]
    e=base("goshnars_malice_crystal","Soul War: Goshnar's Malice (Crystal reconstruction)",participants,anchors,
           flags=[{"name":"cage_timeout_pending","initial":False}],outcomes=["boss_defeated"])
    e["rules"]=[
      {"key":"malice_reflects_physical_and_death","trigger":{"kind":"damage_taken","role":"goshnars_malice","source":"player"},"conditions":[],
       "actions":[{"kind":"reflect_damage","role":"goshnars_malice","percent":100,"damage_types":["physical","death"]}]},
      {"key":"soul_cage_reflects_player_damage","trigger":{"kind":"damage_taken","role":"soul_cage","source":"player"},"conditions":[],
       "actions":[{"kind":"reflect_damage","role":"soul_cage","percent":10,"damage_types":[]}]},
      {"key":"first_soul_cage","trigger":{"kind":"encounter_started"},"delay_ms":23000,"conditions":[],
       "actions":[{"kind":"spawn","creature":cref("Soul Cage"),"role":"soul_cage","count":1,"at":{"anchor":"soul_cage_spawn"},"owner":"none","health":"full"}]},
      {"key":"destroyed_soul_cage_returns","trigger":{"kind":"creature_died","role":"soul_cage"},"delay_ms":23000,"conditions":[],
       "actions":[{"kind":"spawn","creature":cref("Soul Cage"),"role":"soul_cage","count":1,"at":{"anchor":"soul_cage_spawn"},"owner":"none","health":"full"}]},
      {"key":"surviving_soul_cage_buffs_malice","trigger":{"kind":"creature_spawned","role":"soul_cage"},"delay_ms":40000,
       "conditions":[{"kind":"creature_present","role":"soul_cage","anchor":"arena","present":True}],
       "actions":[{"kind":"remove","role":"soul_cage"},{"kind":"flag","flag":"cage_timeout_pending","value":True},
                  {"kind":"attribute","role":"goshnars_malice","attribute":"defense","operation":"add","value":10}]},
      {"key":"timed_out_soul_cage_returns","trigger":{"kind":"creature_spawned","role":"soul_cage"},"delay_ms":63000,
       "conditions":[{"kind":"flag","flag":"cage_timeout_pending","value":True}],
       "actions":[{"kind":"flag","flag":"cage_timeout_pending","value":False},
                  {"kind":"spawn","creature":cref("Soul Cage"),"role":"soul_cage","count":1,"at":{"anchor":"soul_cage_spawn"},"owner":"none","health":"full"}]},
      {"key":"boss_death","trigger":{"kind":"creature_died","role":"goshnars_malice"},"conditions":[],
       "actions":[{"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    sources=[
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnars_malice.lua","1cda5185bbe9caa2b3b01bc9fc47320005d80305"),
      src("data-global/monster/quests/soul_war/soul_cage.lua","c6c65b32547d17405f5b0e7c8084100e6a46db94"),
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
    ]
    entries=[
      entry(0,list(range(12,56)),"mapped","Malice 100% physical/death reflection and Soul Cage 10% reflection are represented as player-damage rules.","/encounter/rules"),
      entry(0,list(range(57,66)),"mapped","Soul Cage death schedules a new cage after 23 s.","/encounter/rules/3"),
      entry(3,list(range(1130,1162)),"mapped","Soul Cage spawn/timeout lifecycle is represented: first spawn 23 s, timeout 40 s, +10 defense, next spawn 23 s later.","/encounter/rules"),
      entry(3,list(range(1130,1162)),"unresolved_semantics","Timeout also adds +10 reflection for every SoulWarReflectDamageMap damage type; Encounter v1 reflect_damage has no additive stack operation."),
      entry(1,list(range(139,153)),"unresolved_semantics","Every 40 s Malice creates connected random white safe-tile groups. Procedural connected tile selection is not in Encounter v1."),
      entry(3,list(range(1070,1129)),"unresolved_semantics","White-tile revert checks ground id 409 and deals 8000 death damage to players outside safe ground; Encounter v1 has no complement-of-generated-tiles player selector."),
    ]
    m=manifest(e["identity"]["key"],sources,entries,{"Goshnar's-Malice":[cref("Goshnar's Malice")["key"]],"SoulCageHealthChange":[cref("Soul Cage")["key"]],"SoulCageDeath":[cref("Soul Cage")["key"]]})
    return e,catalog(e),m


def spite():
    participants=[("goshnars_spite",["Goshnar's Spite"]),("weeping_soul",["Weeping Soul"])]
    anchors=[
      area("arena",33734,33751,31624,31640,14),
      point("fire_north",33743,31628,14), point("fire_east",33736,31632,14),
      point("fire_west",33750,31632,14), point("fire_south",33742,31637,14),
    ]
    e=base("goshnars_spite_crystal","Soul War: Goshnar's Spite (Crystal reconstruction)",participants,anchors,
           timers=[{"name":"searing_fire_cycle","duration_ms":14000,"repeat":True}],
           outcomes=["boss_defeated"])
    fire={"family":"Item","key":"canary:item/33877","revision":REV}
    e["rules"]=[
      {"key":"start_searing_fire_cycle","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"timer","timer":"searing_fire_cycle","operation":"start"}]},
      {"key":"create_one_searing_fire","trigger":{"kind":"timer_elapsed","timer":"searing_fire_cycle"},"conditions":[],
       "actions":[{"kind":"one_of","branches":[
         {"weight":1,"actions":[{"kind":"map_item","operation":"create","item":fire,"anchor":"fire_north","revert_after_ms":5000}]},
         {"weight":1,"actions":[{"kind":"map_item","operation":"create","item":fire,"anchor":"fire_east","revert_after_ms":5000}]},
         {"weight":1,"actions":[{"kind":"map_item","operation":"create","item":fire,"anchor":"fire_west","revert_after_ms":5000}]},
         {"weight":1,"actions":[{"kind":"map_item","operation":"create","item":fire,"anchor":"fire_south","revert_after_ms":5000}]},
       ]}]},
      {"key":"boss_death","trigger":{"kind":"creature_died","role":"goshnars_spite"},"conditions":[],
       "actions":[{"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    sources=[
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnars_spite.lua","1daedf2a19178f90bc7f6ea6041f8f3fd1bacef6"),
      src("data-global/monster/quests/soul_war/weeping_soul.lua","0c45ec43fff093054def38605a66bc2b90de727f"),
    ]
    entries=[
      entry(0,list(range(619,639)),"mapped","The four fixed Searing Fire positions and 14/5/56 second constants are pinned; the representable core uses the 14 second cadence and 5 second item lifetime.","/encounter/anchors"),
      entry(1,list(range(415,433)),"mapped","Every 14 seconds one of four Searing Fire tiles is chosen uniformly and the item is removed after 5 seconds.","/encounter/rules"),
      entry(1,list(range(400,414)),"unresolved_semantics","If an unstomped Searing Fire survives until removal, Spite gains +10 defense. Encounter v1 has no item-still-present condition tied to a timed map-item revert."),
      entry(1,list(range(434,481)),"unresolved_semantics","A player may stomp Searing Fire only once per 56 seconds; this per-player world-item step cooldown is outside Encounter v1 participant state."),
      entry(1,list(range(364,399)),"unresolved_semantics","Stepping on a Weeping Soul corpse consumes it, applies a 14 second outfit condition and has a 10% chance to heal Spite for 10% max health; player corpse-step and percent-max heal are not representable."),
      entry(1,list(range(93,119)),"mapped","SoulWarBossesDeath credits contributors; the encounter exposes boss_defeated while quest taint/progress remains in Quest state.","/encounter/rules/2"),
    ]
    m=manifest(e["identity"]["key"],sources,entries,{"SoulWarBossesDeath":[cref("Goshnar's Spite")["key"]],"WeepingSoulCorpse":[cref("Weeping Soul")["key"]]})
    return e,catalog(e),m

def cruelty():
    participants=[("goshnars_cruelty",["Goshnar's Cruelty"]),("greedy_eye",["A Greedy Eye"])]
    anchors=[
      area("arena",33847,33864,31858,31874,7),
      point("boss",33856,31866,7), point("greedy_eye",33856,31858,7),
    ]
    e=base("goshnars_cruelty_crystal","Soul War: Goshnar's Cruelty (Crystal reconstruction)",participants,anchors,
           timers=[{"name":"defense_growth","duration_ms":15000,"repeat":True}],
           outcomes=["boss_defeated"])
    e["rules"]=[
      {"key":"start_defense_growth","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"timer","timer":"defense_growth","operation":"start"}]},
      {"key":"unchecked_cruelty_grows_harder","trigger":{"kind":"timer_elapsed","timer":"defense_growth"},"conditions":[],
       "actions":[{"kind":"attribute","role":"goshnars_cruelty","attribute":"defense","operation":"add","value":2}]},
      {"key":"boss_death_removes_eye","trigger":{"kind":"creature_died","role":"goshnars_cruelty"},"conditions":[],
       "actions":[{"kind":"remove","role":"greedy_eye"},{"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    sources=[
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnars_cruelty.lua","468e6a312e446b328771641f2feb7878f101e8b8"),
      src("data-global/monster/quests/soul_war/normal_monsters/furious_crater/a_greedy_eye.lua","8622ed190a2da9030ce1eb111029581d88a0d922"),
      src("data-global/scripts/quests/soul_war/spell-eye_beam.lua","7a73979930a0cdc17950fab7075334322f4cb128"),
    ]
    entries=[
      entry(0,list(range(32,36)),"mapped","Cruelty defense grows by 2 on a 15 second source cadence when Greedy Maw is not refreshed.","/encounter/state/timers"),
      entry(2,list(range(138,154)),"mapped","After the initial 15 seconds the boss invokes the defense-growth helper; the Greedy Eye is removed when Cruelty disappears.","/encounter/rules"),
      entry(1,list(range(860,912)),"unresolved_semantics","Using Some Mortal Essence on Greedy Maw has a per-player 30 second cooldown, pushes the shared defense deadline by 15 seconds, consumes the item and conditionally reduces current defense by 2; a player-targeted world-object action/cooldown is outside Encounter v1."),
      entry(1,list(range(1089,1104)),"unresolved_semantics","GoshnarsCrueltyBuff scales incoming player damage from the shared defense-drain value. Encounter v1 has no damage multiplier sourced dynamically from a counter/KV value."),
      entry(2,[107],"unresolved_semantics","The custom 'cruelty transform elemental' ability casts every 7 seconds at 50% chance; its native transformation behavior remains a dedicated Ability/engine gap."),
      entry(3,[64],"mapped","A Greedy Eye owns the direction-bound 'greedy eye beam' creature ability; the beam stays in Creature/Ability authoring rather than being duplicated as an encounter rule.","/encounter/participants/1"),
      entry(1,list(range(93,119)),"mapped","SoulWarBossesDeath credits contributors; the encounter exposes boss_defeated while quest taint/progress remains in Quest state.","/encounter/rules/2"),
    ]
    m=manifest(e["identity"]["key"],sources,entries,{"SoulWarBossesDeath":[cref("Goshnar's Cruelty")["key"]],"GoshnarsCrueltyBuff":[cref("Goshnar's Cruelty")["key"]]})
    return e,catalog(e),m

def megalomania():
    participants=[
      ("goshnars_megalomania",["Goshnar's Megalomania Purple","Goshnar's Megalomania Green","Goshnar's Megalomania Blue"]),
      ("aspect_of_power",["Aspect of Power"]),
    ]
    anchors=[area("arena",33701,33719,31626,31642,14),point("boss",33710,31634,14),point("aspect_start",33710,31635,14)]
    e=base("goshnars_megalomania_crystal","Soul War: Goshnar's Megalomania (Crystal reconstruction)",participants,anchors,
           counters=[{"name":"aspect_deaths","initial":0}],
           timers=[{"name":"green_to_blue","duration_ms":60000,"repeat":False},{"name":"blue_to_purple","duration_ms":7000,"repeat":False}],
           outcomes=["boss_defeated"])
    e["rules"]=[
      {"key":"reset_aspect_count","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"counter","counter":"aspect_deaths","operation":"set","value":0}]},
      {"key":"aspect_death_counts","trigger":{"kind":"creature_died","role":"aspect_of_power"},"conditions":[],
       "actions":[{"kind":"counter","counter":"aspect_deaths","operation":"add","value":1}]},
      {"key":"aspect_returns_after_five_seconds","trigger":{"kind":"creature_died","role":"aspect_of_power"},"delay_ms":5000,"conditions":[],
       "actions":[{"kind":"spawn","creature":cref("Aspect of Power"),"role":"aspect_of_power","count":1,
                   "at":{"role_position":"goshnars_megalomania","otherwise":"death_position"},"owner":"none","health":"full"}]},
      {"key":"four_aspects_make_boss_green","trigger":{"kind":"counter_reached","counter":"aspect_deaths","value":4},"conditions":[],
       "actions":[
         {"kind":"counter","counter":"aspect_deaths","operation":"set","value":0},
         {"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Green"),"health":"keep_absolute"},
         {"kind":"timer","timer":"green_to_blue","operation":"start"}]},
      {"key":"green_phase_becomes_blue","trigger":{"kind":"timer_elapsed","timer":"green_to_blue"},"conditions":[],
       "actions":[
         {"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Blue"),"health":"keep_absolute"},
         {"kind":"timer","timer":"blue_to_purple","operation":"start"}]},
      {"key":"blue_phase_returns_purple","trigger":{"kind":"timer_elapsed","timer":"blue_to_purple"},"conditions":[],
       "actions":[{"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Purple"),"health":"keep_absolute"}]},
      {"key":"boss_death","trigger":{"kind":"creature_died","role":"goshnars_megalomania"},"conditions":[],
       "actions":[{"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    sources=[
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnar's_megalomania_purple.lua","7781c66516d83abbe905496cff7c3caf44902bf9"),
      src("data-global/monster/quests/soul_war/goshnar's_megalomania_green.lua","787ab94b6627f5785fc9a574e6192f679e933235"),
      src("data-global/monster/quests/soul_war/goshnar's_megalomania_blue.lua","cf81168aa00b063ba9ce379e48b7254a4522e160"),
      src("data-global/monster/quests/soul_war/aspect_of_power.lua","896cb5207226ff460039f2c96b1eee39b5cda3ee"),
      src("src/lua/functions/creatures/monster/monster_functions.cpp","4e6c50984ee2e5881f9a55aa15d82cd42da9979c"),
    ]
    entries=[
      entry(0,list(range(450,477)),"mapped","The lever starts Purple plus one Aspect of Power in the fixed final arena; retry/quest gates remain outside this encounter core.","/encounter/anchors"),
      entry(1,list(range(913,941)),"mapped","Every Aspect death counts while the boss is Purple and schedules a replacement at the boss position after 5 seconds.","/encounter/rules"),
      entry(0,list(range(588,601)),"mapped","The scheduled blue phase lasts 7 seconds before returning Purple.","/encounter/rules/4"),
      entry(0,list(range(1454,1475)),"mapped","Four Aspect deaths transform Purple to Green and schedule Blue after 60 seconds. The 70 second Green-only fallback is source safety logic and is not duplicated because the 60+7 second path returns Purple first.","/encounter/rules"),
      entry(6,list(range(144,175)),"mapped","Monster:setType(..., restoreHealth=false) preserves current absolute health/max-health fields; encounter transforms therefore use keep_absolute.","/encounter/rules"),
      entry(1,list(range(942,1030)),"unresolved_semantics","Dead Aspect corpse interactions, Cleansed Sanity, Necromantic Remains and their player-local torment/defense cooldowns require player item-step/use state not present in Encounter v1."),
      entry(2,list(range(138,170)),"unresolved_semantics","Purple/Green/Blue onThink loops also drive torment, procedural white tiles, defense growth and the custom elemental transformation; these remain separate hard holds."),
      entry(1,list(range(1050,1062)),"mapped","Megalomania death records the quest kill flag for credited killers; the encounter exposes boss_defeated and leaves persistent quest state to Quest authoring.","/encounter/rules/6"),
    ]
    m=manifest(e["identity"]["key"],sources,entries,{"SoulWarAspectOfPowerDeath":[cref("Aspect of Power")["key"]],"SoulWarMegalomaniaDeath":[cref("Goshnar's Megalomania Purple")["key"]]})
    return e,catalog(e),m

def write_one(name,bundle):
    e,c,m=bundle
    d=OUT/name
    d.mkdir(parents=True,exist_ok=True)
    for fn,val in [("encounter.json",e),("catalog.json",c),("manifest.json",m)]:
        (d/fn).write_text(json.dumps(val,ensure_ascii=False,indent=2)+"\n",encoding="utf-8",newline="\n")

def main():
    write_one("goshnars_greed",greed())
    write_one("goshnars_malice",malice())
    write_one("goshnars_spite",spite())
    write_one("goshnars_cruelty",cruelty())
    write_one("goshnars_megalomania",megalomania())
    index={
      "schema":"OTERYN_SOUL_WAR_CRYSTAL_ENCOUNTER_RECONSTRUCTION/v1",
      "source":{"repository":"zimbadev/crystalserver","revision":CRYSTAL,"branch":"summer-update"},
      "runtime_qualified":False,
      "encounters":["goshnars_greed","goshnars_malice","goshnars_spite","goshnars_cruelty","goshnars_megalomania"],
      "note":"Source-backed representable cores only. unresolved_semantics rows are hard holds, not approximations."
    }
    OUT.mkdir(parents=True,exist_ok=True)
    (OUT/"index.json").write_text(json.dumps(index,ensure_ascii=False,indent=2)+"\n",encoding="utf-8",newline="\n")
    print("generated",len(index["encounters"]),"Soul War Crystal encounter cores")

if __name__=="__main__":
    main()