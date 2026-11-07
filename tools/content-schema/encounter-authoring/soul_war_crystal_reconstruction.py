from __future__ import annotations
import json
import sys
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
      {"key":"weak_soul_becomes_strong","trigger":{"kind":"creature_spawned","role":"weak_soul"},"delay_ms":8000,"conditions":[{"kind":"in_anchor","subject":{"triggering":True},"anchor":"arena"}],
       "actions":[{"kind":"spawn","creature":cref("Strong Soul"),"role":"strong_soul","count":1,"at":{"anchor":"strong_soul_return"},"owner":"none","health":"full"},{"kind":"remove","triggering":True}]},
      {"key":"strong_soul_becomes_powerful","trigger":{"kind":"creature_spawned","role":"strong_soul"},"delay_ms":8000,"conditions":[{"kind":"in_anchor","subject":{"triggering":True},"anchor":"arena"}],
       "actions":[{"kind":"spawn","creature":cref("Powerful Soul"),"role":"powerful_soul","count":1,"at":{"anchor":"powerful_soul_return"},"owner":"none","health":"full"},{"kind":"remove","triggering":True}]},
      {"key":"powerful_soul_buffs_boss","trigger":{"kind":"creature_spawned","role":"powerful_soul"},"delay_ms":8000,"conditions":[{"kind":"in_anchor","subject":{"triggering":True},"anchor":"arena"}],
       "actions":[
         {"kind":"remove","triggering":True},
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
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
    ]
    entries=[
      entry(1,[138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189],'mapped','Boss immunity, five-greedbeast phase and nominal 34 s timeout/reset intent are recorded; exact think-interval alignment remains unresolved below.','/encounter/rules'),
      entry(0,list(range(198,220)),"mapped","GreedMonsterDeath and SoulSphereDeath counters are represented by death triggers and the greedbeast counter.","/encounter/rules"),
      entry(2,[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104],'mapped','A surviving Weak Soul spawns a new Strong Soul at its fixed GreedMonsters position and removes the original; it does not transform the existing actor.','/encounter/rules/6'),
      entry(3,[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104],'mapped','A surviving Strong Soul spawns a new Powerful Soul at its fixed GreedMonsters position and removes the original; it does not transform the existing actor.','/encounter/rules/7'),
      entry(4,[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113],'mapped','Only a surviving original Powerful Soul is removed and emits the +10 defense/current-health heal intent. The delayed Weak Soul replacement and max-health/reflection effects remain explicit holds below.','/encounter/rules/8'),
      entry(4,[98, 99, 100, 101, 102, 103, 104, 105, 106],'unresolved_semantics','Powerful Soul also adds +10000 maximum health and +10 reflect per configured damage type; the current Encounter vocabulary has no max-health mutation or additive reflect stack.'),
      entry(5,[94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113],'unresolved_semantics','Soulsnatcher damages every player in the boss zone for 500-1000 after 8 s, removes itself and schedules its return through the 10 s helper; current Encounter damage action has no players-in-area subject.'),
      entry(6,[90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124],'unresolved_semantics',"Soul Sphere walks one tile west every 3000 ms and fully heals Goshnar's Greed if the next tile contains the boss; scripted per-tile movement/reach is not in Encounter v1."),
      entry(0,[206],"mapped","Each Greed minion death schedules the same minion back after 10 s at its fixed GreedMonsters position.","/encounter/rules"),
    ]
    entries.extend([
      entry(7,[954, 955, 956, 957, 958, 959, 960, 961, 962, 963, 964, 965, 966, 967, 968, 969, 970, 971, 972, 973, 974, 975, 976, 977],'mapped','GreedMonsters defines the fixed return positions; CreateGoshnarsGreedMonster schedules an actual new creature after 10000 ms.','/encounter/rules'),
      entry(4,[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108],'unresolved_semantics','A surviving Powerful Soul schedules a fresh Weak Soul 10 s after its 8 s removal/buff through the helper (not an immediate transform). This delayed per-original-entity replacement obligation is intentionally not emitted by the core; it must be authored without collapsing overlapping callbacks before admission.'),
      entry(1,[156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175],'unresolved_semantics','The donor immunity counter includes the think interval that enters the sphere phase. A fresh 34000 ms timer is a nominal core only, not proof of exact callback/deadline alignment.'),
      entry(2,[92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104],'unresolved_semantics','Soul callbacks share source-type counters, reset on spawn and compare accumulated time with ==8000. Their exact multi-instance scheduling and reset equivalence is not established by per-spawn delayed rules.'),
    ])
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
      entry(3,[979, 980, 981, 982, 983, 984, 985, 986, 987, 988, 989, 990, 991, 992, 993, 994, 995, 996, 997, 998, 999, 1000, 1001, 1002, 1003, 1004],'unresolved_semantics','Crystal resolves Soul Cage by name when the 40 s callback fires, so an old callback can target a replacement. It suppresses spawning while a cage exists and schedules 23 s after actual removal. The draft role/flag/63 s chain is not proof of overlapping-cycle equivalence; original-instance versus donor name-wide behavior requires an explicit resolution before admission.'),
      entry(3,[984, 985, 986, 987, 988, 989, 990, 991],'unresolved_semantics','Timeout also adds +10 reflection for every SoulWarReflectDamageMap damage type; Encounter v1 reflect_damage has no additive stack operation.'),
      entry(1,[139, 140, 141, 142, 143, 144, 145, 146, 147, 148],'unresolved_semantics','Every 40 s Malice creates connected random white safe-tile groups. Procedural connected tile selection is not in Encounter v1.'),
      entry(3,[1071, 1072, 1073, 1074, 1075, 1076, 1077, 1078, 1079, 1080, 1081, 1082, 1083, 1084, 1085, 1086, 1087, 1088, 1089, 1090, 1091, 1092, 1093, 1094, 1095, 1096, 1097, 1098, 1099, 1100, 1101, 1102, 1103, 1104, 1105, 1106, 1107, 1108],'unresolved_semantics','White-tile reversion examines each tile top creature and its current ground id (safe id 409), then deals 8000 death damage only to a player outside safe ground; Encounter v1 has no complement-of-generated-tiles player selector.'),
    ]
    entries.extend([
      entry(3,[349, 350, 351],'mapped','The lever schedules the first Soul Cage after 23000 ms.','/encounter/rules/2'),
      entry(3,[1006, 1007, 1008, 1009, 1010, 1011, 1012, 1013, 1014, 1015, 1016, 1017, 1018, 1019, 1020, 1021, 1022, 1023, 1024, 1025, 1026, 1027, 1028, 1029, 1030, 1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040, 1041, 1042, 1043, 1044, 1045, 1046, 1047, 1048, 1049, 1050, 1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1110, 1111, 1112, 1113, 1114, 1115, 1116, 1117, 1118, 1119, 1120, 1121, 1122, 1123, 1124, 1125, 1126, 1127, 1128, 1129, 1130, 1131, 1132, 1133, 1134, 1135, 1136, 1137, 1138, 1139, 1140, 1141, 1142, 1143, 1144, 1145, 1146, 1147, 1148],'unresolved_semantics','Connected safe-tile selection and its random group construction are source algorithms, not implemented by this Encounter core.'),
      entry(2,[48, 49, 50, 51, 52],'unresolved_semantics','Soul Cage registers Heal Malice with 2000 ms interval and 90 percent chance. Its existing Ability/Creature implementation and exact healing behavior have not been qualified by this Encounter packet.'),
    ])
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
      entry(1,[415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429, 430, 431, 432],'unresolved_semantics','The 14 s fire event runs on a global donor clock, independently of encounter/boss presence. The instance-started timer preserves a nominal cadence only; source phase/lifecycle equivalence and timed removal must be reconciled before admission.'),
      entry(1,list(range(400,414)),"unresolved_semantics","If an unstomped Searing Fire survives until removal, Spite gains +10 defense. Encounter v1 has no item-still-present condition tied to a timed map-item revert."),
      entry(1,list(range(434,481)),"unresolved_semantics","A player may stomp Searing Fire only once per 56 seconds; this per-player world-item step cooldown is outside Encounter v1 participant state."),
      entry(1,[364, 365, 366, 367, 368, 369, 370, 371, 372, 373, 374, 375, 376, 377, 378, 379, 380, 381, 382, 383, 384, 385, 386, 387, 388, 389, 390, 391, 392, 393, 394, 395, 396, 397, 398],'unresolved_semantics','Stepping on a Weeping Soul corpse is ignored if the player already has any outfit condition; otherwise it consumes the corpse, applies a 14 s outfit and has a 10% chance to heal Spite for 10% max health. These player corpse-step/prerequisite and percent-max heal semantics remain held.'),
      entry(1,list(range(93,119)),"mapped","SoulWarBossesDeath credits contributors; the encounter exposes boss_defeated while quest taint/progress remains in Quest state.","/encounter/rules/2"),
    ]
    entries.extend([
      entry(0,[602, 603, 604, 605, 606, 607, 608, 609, 610, 611, 612, 613, 614, 615, 616, 617],'unresolved_semantics','The Weeping Soul helper defines the outfit, probability and max-health heal; the preceding any-outfit-condition rejection remains part of the player-local hold.'),
    ])
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
      entry(0,[32, 33, 34, 35],'mapped','Source constants declare +2 defense, a 15 s base delay and the elemental cast chance. Shared deadline behavior is separately unresolved; constants alone do not prove an unconditional repeating cadence.','/encounter/state/timers'),
      entry(2,[138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154],'unresolved_semantics','Cruelty calls the shared timestamp-gated helper after its initial delay and removes Greedy Eye on disappearance. The repeating core timer and death-only cleanup do not establish either exact gate or non-death disappearance equivalence.'),
      entry(1,[860, 861, 862, 863, 864, 865, 866, 867, 868, 869, 870, 871, 872, 873, 874, 875, 876, 877, 878, 879, 880, 881, 882, 883, 884, 885, 886, 887, 888, 889, 890, 891, 892, 893, 894, 895, 896, 897, 898, 899, 900, 901, 902, 903, 904, 905, 906, 907, 908, 909, 910, 911],'unresolved_semantics','Greedy Maw use has a per-player 30 s cooldown, consumes Mortal Essence and may reduce defense by 2. It stores use-time+15 s; the growth helper adds another 15 s, so the next growth is no earlier than use-time+30 s (subject to think scheduling). Player use/cooldown and the shared timestamp gate remain unbound.'),
      entry(1,list(range(1089,1104)),"unresolved_semantics","GoshnarsCrueltyBuff scales incoming player damage from the shared defense-drain value. Encounter v1 has no damage multiplier sourced dynamically from a counter/KV value."),
      entry(2,[107],"unresolved_semantics","The custom 'cruelty transform elemental' ability casts every 7 seconds at 50% chance; its native transformation behavior remains a dedicated Ability/engine gap."),
      entry(3,[64],'mapped','A Greedy Eye registers the greedy eye beam attack. Registration alone does not certify canonical Ability behavior or facing geometry.','/encounter/participants/1'),
      entry(1,list(range(93,119)),"mapped","SoulWarBossesDeath credits contributors; the encounter exposes boss_defeated while quest taint/progress remains in Quest state.","/encounter/rules/2"),
    ]
    entries.extend([
      entry(0,[1477, 1478, 1479, 1480, 1481, 1482, 1483, 1484, 1485, 1486, 1487, 1488, 1489, 1490, 1491, 1492, 1493, 1494],'unresolved_semantics','The actual defense helper tests a shared stored timestamp plus 15 s and updates it on successful growth. This gating must replace the unconditional draft repeating timer before admission.'),
      entry(4,[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 37],'unresolved_semantics','The directional eye beam area and Poor Soul interaction are in the spell implementation; canonical Ability equivalence is not established by the Creature attack registration.'),
    ])
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
           flags=[{"name":"purple_phase","initial":True}],
           timers=[{"name":"green_to_blue","duration_ms":60000,"repeat":False},{"name":"blue_to_purple","duration_ms":7000,"repeat":False}],
           outcomes=["boss_defeated"])
    e["rules"]=[
      {"key":"reset_aspect_count","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"counter","counter":"aspect_deaths","operation":"set","value":0}]},
      {"key":"aspect_death_counts","trigger":{"kind":"creature_died","role":"aspect_of_power"},
       "conditions":[{"kind":"flag","flag":"purple_phase","value":True},{"kind":"has_master","role":"aspect_of_power","value":False}],
       "actions":[{"kind":"counter","counter":"aspect_deaths","operation":"add","value":1}]},
      {"key":"aspect_returns_after_five_seconds","trigger":{"kind":"creature_died","role":"aspect_of_power"},"delay_ms":5000,"conditions":[{"kind":"has_master","role":"aspect_of_power","value":False}],
       "actions":[{"kind":"spawn","creature":cref("Aspect of Power"),"role":"aspect_of_power","count":1,
                   "at":{"role_position":"goshnars_megalomania","otherwise":"death_position"},"owner":"none","health":"full"}]},
      {"key":"four_aspects_make_boss_green","trigger":{"kind":"counter_reached","counter":"aspect_deaths","value":4},"conditions":[],
       "actions":[
         {"kind":"counter","counter":"aspect_deaths","operation":"set","value":0},
         {"kind":"flag","flag":"purple_phase","value":False},
         {"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Green"),"health":"keep_absolute"},
         {"kind":"timer","timer":"green_to_blue","operation":"start"}]},
      {"key":"green_phase_becomes_blue","trigger":{"kind":"timer_elapsed","timer":"green_to_blue"},"conditions":[],
       "actions":[
         {"kind":"teleport","who":{"role":"goshnars_megalomania"},"to":"boss"},{"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Blue"),"health":"keep_absolute"},
         {"kind":"timer","timer":"blue_to_purple","operation":"start"}]},
      {"key":"blue_phase_returns_purple","trigger":{"kind":"timer_elapsed","timer":"blue_to_purple"},"conditions":[],
       "actions":[{"kind":"flag","flag":"purple_phase","value":True},
                  {"kind":"transform","role":"goshnars_megalomania","into":cref("Goshnar's Megalomania Purple"),"health":"keep_absolute"}]},
      {"key":"boss_death","trigger":{"kind":"creature_died","role":"goshnars_megalomania"},"conditions":[{"kind":"flag","flag":"purple_phase","value":False}],
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
      entry(1,[913, 914, 915, 916, 917, 918, 919, 920, 921, 922, 923, 924, 925, 926, 927, 928, 929, 930, 931, 932, 933, 934, 935, 936, 937, 938, 939, 940],'unresolved_semantics','Unmastered Aspect deaths count only in Purple and schedule a replacement after 5 s. The donor captures the boss position at death; delayed role_position resolves it later and cannot certify that captured-position behavior.'),
      entry(0,list(range(588,601)),"mapped","The scheduled blue phase lasts 7 seconds before returning Purple.","/encounter/rules/5"),
      entry(0,list(range(1454,1475)),"mapped","Four Aspect deaths transform Purple to Green and schedule Blue after 60 seconds. The 70 second Green-only fallback is source safety logic and is not duplicated because the 60+7 second path returns Purple first.","/encounter/rules"),
      entry(6,list(range(144,175)),"mapped","Monster:setType(..., restoreHealth=false) preserves current absolute health/max-health fields; encounter transforms therefore use keep_absolute.","/encounter/rules"),
      entry(1,list(range(942,1030)),"unresolved_semantics","Dead Aspect corpse interactions, Cleansed Sanity, Necromantic Remains and their player-local torment/defense cooldowns require player item-step/use state not present in Encounter v1."),
      entry(2,[117, 118, 119, 120, 121, 122, 123, 124, 125, 126],'unresolved_semantics','Purple onThink drives torment, procedural white tiles and defense state. These hooks remain separate hard holds; they do not prove elemental spell behavior.'),
      entry(1,[1049, 1050, 1051, 1052, 1053, 1054, 1055, 1056, 1057, 1058, 1059, 1060, 1061, 1062, 1063, 1064],'mapped','MegalomaniaDeath is registered by Green and Blue, not Purple. The core exposes its outcome only outside the Purple phase; persistent flags and final credit remain Quest-owned.','/encounter/rules/6'),
    ]
    entries.extend([
      entry(3,[141, 142, 143, 144, 145, 146, 147, 148, 149, 150],'unresolved_semantics','Green onThink torment, procedural white tiles and defense behavior remain unqualified in this core.'),
      entry(4,[130, 131, 132, 133, 134, 135, 136, 137, 138, 139],'unresolved_semantics','Blue onThink torment, procedural white tiles and defense behavior remain unqualified in this core.'),
      entry(2,[71],'unresolved_semantics','Purple registers the custom elemental transformation spell. Its Ability/native behavior is not represented by the onThink hook; Blue has no such attack registration.'),
      entry(3,[112],'unresolved_semantics','Green also registers the custom elemental transformation spell; its native semantics remain held.'),
      entry(1,[1031, 1032, 1033, 1034, 1035, 1036, 1037, 1038, 1039, 1040, 1041, 1042, 1043, 1044, 1045, 1046, 1047],'unresolved_semantics','Necromantic Focus schedules a strength callback after 300 s. Source identifier necromanticRemainsId differs from the declared necromanticRemainsItemId; this source defect and delayed item/strength behavior must be resolved, not guessed.'),
      entry(0,[1370, 1371, 1372, 1373, 1374, 1375, 1376, 1377, 1378, 1379, 1380, 1381, 1382, 1383],'unresolved_semantics','The delayed Megalomania strength callback depends on item existence and defense state; the source identifier mismatch is not repaired implicitly.'),
      entry(0,[37],'unresolved_semantics','The source declares a 70 s Green-only fallback. Its separate callback and gating are not represented by the draft normal 60+7 s timer path.'),
    ])
    m=manifest(e["identity"]["key"],sources,entries,{"SoulWarAspectOfPowerDeath":[cref("Aspect of Power")["key"]],"MegalomaniaDeath":[cref("Goshnar's Megalomania Green")["key"],cref("Goshnar's Megalomania Blue")["key"]]})
    return e,catalog(e),m


def hatred():
    participants=[
      ("goshnars_hatred",["Goshnar's Hatred"]),
      ("burning_hatred",["Ashes of Burning Hatred","Spark of Burning Hatred","Flame of Burning Hatred","Blaze of Burning Hatred"]),
    ]
    anchors=[area("arena",33735,33751,31592,31606,14),point("burning_hatred_start",33743,31599,14)]
    e=base("goshnars_hatred_crystal","Soul War: Goshnar's Hatred (Crystal reconstruction)",participants,anchors,
           counters=[{"name":"hatred","initial":0}],
           timers=[
             {"name":"ashes_burns","duration_ms":45000,"repeat":False},
             {"name":"spark_burns","duration_ms":45000,"repeat":False},
             {"name":"flame_burns","duration_ms":45000,"repeat":False},
             {"name":"blaze_burns","duration_ms":46000,"repeat":False},
           ],
           outcomes=["boss_defeated"])
    sorrow={"family":"Item","key":"canary:item/33793","revision":REV}
    e["rules"]=[
      {"key":"start_burning_cycle","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[{"kind":"timer","timer":"ashes_burns","operation":"start"}]},
      {"key":"ashes_become_spark","trigger":{"kind":"timer_elapsed","timer":"ashes_burns"},"conditions":[],
       "actions":[{"kind":"transform","role":"burning_hatred","into":cref("Spark of Burning Hatred"),"health":"full"},{"kind":"timer","timer":"spark_burns","operation":"start"}]},
      {"key":"spark_becomes_flame","trigger":{"kind":"timer_elapsed","timer":"spark_burns"},"conditions":[],
       "actions":[{"kind":"transform","role":"burning_hatred","into":cref("Flame of Burning Hatred"),"health":"full"},{"kind":"timer","timer":"flame_burns","operation":"start"}]},
      {"key":"flame_becomes_blaze","trigger":{"kind":"timer_elapsed","timer":"flame_burns"},"conditions":[],
       "actions":[{"kind":"transform","role":"burning_hatred","into":cref("Blaze of Burning Hatred"),"health":"full"},{"kind":"timer","timer":"blaze_burns","operation":"start"}]},
      {"key":"blaze_returns_to_ashes_and_empowers_boss","trigger":{"kind":"timer_elapsed","timer":"blaze_burns"},"conditions":[],
       "actions":[
         {"kind":"transform","role":"burning_hatred","into":cref("Ashes of Burning Hatred"),"health":"full"},
         {"kind":"timer","timer":"ashes_burns","operation":"start"},
         {"kind":"counter","counter":"hatred","operation":"add","value":10},
         {"kind":"attribute","role":"goshnars_hatred","attribute":"outgoing_damage_percent","operation":"add","value":10}]},
      {"key":"sorrow_delays_running_fire_form","trigger":{"kind":"item_used","role":"burning_hatred","item":sorrow},"conditions":[],
       "actions":[
         {"kind":"timer","timer":"ashes_burns","operation":"add","ms":10000},
         {"kind":"timer","timer":"spark_burns","operation":"add","ms":10000},
         {"kind":"timer","timer":"flame_burns","operation":"add","ms":10000},
         {"kind":"timer","timer":"blaze_burns","operation":"add","ms":10000}]},
      {"key":"hatred_hardens_on_player_hit","trigger":{"kind":"damage_taken","role":"goshnars_hatred","source":"player"},
       "conditions":[{"kind":"counter_compare","counter":"hatred","op":">","value":0}],
       "actions":[{"kind":"attribute","role":"goshnars_hatred","attribute":"defense","operation":"add","value":{"counter":"hatred"}}]},
      {"key":"boss_death_stops_burning_cycle","trigger":{"kind":"creature_died","role":"goshnars_hatred"},"conditions":[],
       "actions":[
         {"kind":"remove","role":"burning_hatred"},
         {"kind":"timer","timer":"ashes_burns","operation":"stop"},
         {"kind":"timer","timer":"spark_burns","operation":"stop"},
         {"kind":"timer","timer":"flame_burns","operation":"stop"},
         {"kind":"timer","timer":"blaze_burns","operation":"stop"},
         {"kind":"emit_outcome","outcome":"boss_defeated","credited":"damage_contributors"}]},
    ]
    sources=[
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
      src("data-global/scripts/quests/soul_war/soul_war_mechanics.lua","51d42e7e970a2f24fc8438a833aacac2c061dfc2"),
      src("data-global/monster/quests/soul_war/goshnars_hatred.lua","cc6652eb9d91f15b5d182206076bb216c0e43bf9"),
      src("data-global/monster/quests/soul_war/normal_monsters/burning_hatred/ashes_of_burning_hatred.lua","995cad06686c3a5115a884f272a895d1a2333918"),
      src("data-global/monster/quests/soul_war/normal_monsters/burning_hatred/spark_of_burning_hatred.lua","c3c9d73749f95caa7bc2e360b86d9a532ce6bc1c"),
      src("data-global/monster/quests/soul_war/normal_monsters/burning_hatred/flame_of_burning_hatred.lua","20f253efa7960ecdc25afdf8d3ee4f8d05afde9c"),
      src("data-global/monster/quests/soul_war/normal_monsters/burning_hatred/blaze_of_burning_hatred.lua","77b619b70eaf259e124c00e5f7e3951684a74088"),
    ]
    entries=[
      entry(0,list(range(394,421)),"mapped","The lever starts Goshnar's Hatred with Ashes of Burning Hatred and initializes the shared burn timer to 180 seconds.","/encounter/rules/0"),
      entry(0,[489, 490, 491, 492, 493, 494],'mapped','Crystal defines the Ashes/Spark/Flame/Blaze thresholds at 180/135/90/45 seconds.','/encounter/state/timers'),
      entry(1,list(range(577,599)),"mapped","Using Sorrow of Burning Hatred on the active fire consumes the item and adds 10 seconds to the shared burning time.","/encounter/rules/5"),
      entry(1,[600, 601, 602, 603, 604, 605, 606, 607, 608, 609, 610, 611, 612, 613, 614, 615, 616, 617, 618, 619, 620, 621, 622, 623, 624, 625, 626, 627, 628, 629, 630, 631, 632, 633, 634, 635, 636, 637, 638, 639],'unresolved_semantics','BurningChangeForm compares the shared pre-decrement time to form thresholds. The initial Ashes transition occurs on callback 46, and Sorrow can raise time across earlier thresholds. The forward-only draft timers are not exact counter/phase equivalence.'),
      entry(1,list(range(647,654)),"mapped","Player hits add the hatred multiplier to boss defense; the accepted Oteryn counter/attribute model is retained.","/encounter/rules/6"),
      entry(1,list(range(655,663)),"mapped","The hatred multiplier increases boss outgoing primary damage; the accepted Oteryn additive empowerment model is retained.","/encounter/rules/4"),
      entry(1,list(range(672,696)),"unresolved_semantics","Two Condensed Remorse pickups reset a player's torment counter. Encounter v1 has no per-player item-step counter/state for this mechanic."),
      entry(2,list(range(136,146)),"mapped","When Hatred disappears all Burning Hatred forms are removed; boss death stops the authored fire cycle.","/encounter/rules/7"),
      entry(1,list(range(93,119)),"mapped","SoulWarBossesDeath credits contributors; the encounter exposes boss_defeated while persistent taints and boss flags remain Quest state.","/encounter/rules/7"),
    ]
    entries.extend([
      entry(1,[579, 580, 581, 582, 583, 584, 585, 586, 587, 588, 589, 590, 591, 592, 593, 594, 595],'unresolved_semantics','Sorrow adds 10 to the shared remaining counter, which can cross earlier form thresholds and reverse forms. Extending only the currently running forward timer does not cover this behavior.'),
      entry(1,[655, 656, 657, 658, 659, 660, 661, 662],'unresolved_semantics','The source hatred multiplier scales primary outgoing damage only. The draft outgoing_damage_percent attribute is not proof of primary-only composition; no additional accepted exception was established by this audit.'),
      entry(2,[136, 137, 138, 139, 140, 141, 142, 143, 144, 145],'unresolved_semantics','Hatred disappearance removes the current Burning form. Death-only cleanup does not cover non-death disappearance and must be reconciled before admission.'),
    ])
    m=manifest(e["identity"]["key"],sources,entries,{"GoshnarsHatredBuff":[cref("Goshnar's Hatred")["key"]],"BurningChangeForm":[cref("Ashes of Burning Hatred")["key"],cref("Spark of Burning Hatred")["key"],cref("Flame of Burning Hatred")["key"],cref("Blaze of Burning Hatred")["key"]]})
    return e,catalog(e),m


def claustrophobic_raid(number, bounds, sand_positions, brachias, demons, phantoms, exit_position, source_lines, bad_spawn=False):
    roles=[("brachiodemon","Brachiodemon",brachias),("infernal_demon","Infernal Demon",demons),("infernal_phantom","Infernal Phantom",phantoms)]
    participants=[(role,[name]) for role,name,_ in roles]
    anchors=[area("arena",bounds[0],bounds[1],bounds[2],bounds[3],bounds[4])]
    anchors.append({
      "key":"entry_gate","kind":"area",
      "description":f"Crystal Claustrophobic Inferno raid {number} sand-timer entry tiles.",
      "location":{"boxes":[{"x":[x,x],"y":[y,y],"floor":z} for x,y,z in sand_positions]}
    })
    anchors.append(point("source_exit",exit_position[0],exit_position[1],exit_position[2]))
    for role,_,positions in roles:
        for n,(x,y,z) in enumerate(positions,1):
            anchors.append(point(f"{role}_spawn_{n}",x,y,z))

    e=base(
      f"claustrophobic_inferno_raid_{number}_crystal",
      f"Soul War: Claustrophobic Inferno Raid {number} (Crystal reconstruction)",
      participants, anchors,
      counters=[{"name":"wave_count","initial":0}],
      timers=[
        {"name":"wave_tick","duration_ms":20000,"repeat":True},
        {"name":"survival","duration_ms":120000,"repeat":False},
      ],
      outcomes=["raid_survived"],
    )

    def wave_actions():
        actions=[]
        for role,name,positions in roles:
            for n,_ in enumerate(positions,1):
                actions.append({
                  "kind":"spawn","creature":cref(name),"role":role,"count":1,
                  "at":{"anchor":f"{role}_spawn_{n}"},"owner":"none","health":"full"
                })
        return actions

    e["rules"]=[
      {"key":"start_raid_clock","trigger":{"kind":"encounter_started"},"conditions":[],
       "actions":[
         {"kind":"counter","counter":"wave_count","operation":"set","value":1},
         {"kind":"timer","timer":"wave_tick","operation":"start"},
         {"kind":"timer","timer":"survival","operation":"start"}]},
      {"key":"first_wave_after_spawn_warning","trigger":{"kind":"encounter_started"},"delay_ms":3000,"conditions":[],
       "actions":wave_actions()},
      {"key":"next_wave_every_twenty_seconds","trigger":{"kind":"timer_elapsed","timer":"wave_tick"},"delay_ms":3000,
       "conditions":[{"kind":"counter_compare","counter":"wave_count","op":"<","value":6}],
       "actions":wave_actions()+[{"kind":"counter","counter":"wave_count","operation":"add","value":1}]},
      {"key":"survive_two_minutes","trigger":{"kind":"timer_elapsed","timer":"survival"},"conditions":[],
       "actions":[
         {"kind":"timer","timer":"wave_tick","operation":"stop"},
         {"kind":"remove","all_in":"arena"},
         {"kind":"emit_outcome","outcome":"raid_survived","credited":"players_in_anchor","anchor":"arena"}]},
    ]
    sources=[
      src("data-global/lib/quests/soul_war.lua","4c9d3ac502ecbd7ec32f4823a8cab13d7ad6f703"),
      src("data-global/scripts/quests/soul_war/moveevent-claustrophobic-inferno-raid.lua","39f3e156051d8a7fa0344a0a2a4448b56014f70c"),
    ]
    entries=[
      entry(0,source_lines,"mapped",f"Raid {number} zone, sand-timer tiles and every concrete Brachiodemon/Infernal Demon/Infernal Phantom spawn coordinate are pinned as anchors.","/encounter/anchors"),
      entry(0,[259,260],"mapped","Crystal summer-update sets spawnTime=20 seconds and surviveTime=120 seconds. The separate 5 second player kick remains an unresolved reset action below.","/encounter/state/timers"),
      entry(1,[13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41],'mapped','The donor creates six stages (120/20), each spawning all three configured monster groups after a 3 second warning; the core models waves at t=3,23,43,63,83,103 seconds.','/encounter/rules'),
      entry(1,[43, 44, 45, 46, 47, 48, 49, 50, 51],'unresolved_semantics','On reset Crystal removes monsters, waits 5 seconds, refreshes the zone and removes players. Encounter v1 has no delayed players-in-area removal/kick action.'),
      entry(1,[58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81],'unresolved_semantics','Raid start is a sand-timer MoveEvent with direction filtering and a 10 second post-reset cooldown. This belongs to interaction/world entry ownership, not the encounter core.'),
    ]
    if bad_spawn:
        entries.append(entry(0,[208],"unresolved_semantics","Crystal raid 2 contains a bare 'Pos' token in spawnsPhantons. The missing coordinate is deliberately not guessed or copied."))
    covers={"ClaustrophobicInfernoRaid":[cref("Brachiodemon")["key"],cref("Infernal Demon")["key"],cref("Infernal Phantom")["key"]]}
    return e,catalog(e),manifest(e["identity"]["key"],sources,entries,covers)

def claustrophobic_raid_1():
    return claustrophobic_raid(
      1,(33985,34045,31053,31077,9),
      [(34012,31049,9),(34013,31049,9),(34014,31049,9),(34015,31049,9)],
      [(33994,31068,9),(33998,31062,9),(34012,31061,9),(34006,31075,9),(34037,31072,9),(34026,31063,9),(34034,31057,9),(34019,31067,9),(34040,31064,9)],
      [(33994,31073,9),(34015,31075,9),(34034,31070,9)],
      [(34018,31058,9),(33997,31058,9),(34008,31066,9),(34013,31070,9),(34037,31060,9),(34026,31073,9),(34006,31058,9)],
      (34009,31083,9),list(range(126,169)))

def claustrophobic_raid_2():
    return claustrophobic_raid(
      2,(33988,34043,31042,31068,10),
      [(34012,31075,10),(34011,31075,10),(34010,31075,10)],
      [(33999,31048,10),(33996,31053,10),(33996,31061,10),(34006,31056,10),(34010,31066,10),(34019,31068,10),(34023,31063,10),(34013,31046,10),(34022,31045,10),(34036,31063,10),(34035,31047,10)],
      [(34017,31040,10),(34001,31058,10),(34016,31056,10),(34028,31046,10),(34030,31058,10)],
      [(33991,31059,10),(34005,31045,10),(34006,31051,10),(34020,31051,10),(34036,31053,10),(34020,31072,10),(34007,31070,10)],
      (34011,31028,10),list(range(169,215)),True)

def claustrophobic_raid_3():
    return claustrophobic_raid(
      3,(33987,34044,31043,31076,11),
      [(34009,31036,11),(34010,31036,11),(34011,31036,11),(34012,31036,11),(34013,31036,11),(34014,31036,11)],
      [(34016,31047,11),(34007,31047,11),(33998,31051,11),(33993,31057,11),(33995,31065,11),(34006,31069,11),(34007,31060,11),(34020,31060,11),(34032,31058,11),(34032,31067,11),(34039,31067,11)],
      [(33998,31061,11),(34012,31060,11),(34030,31063,11)],
      [(34001,31047,11),(34013,31064,11),(34007,31053,11),(34023,31054,11),(34038,31053,11)],
      (34014,31085,11),list(range(215,259)))

def render(val):
    return (json.dumps(val,ensure_ascii=False,indent=2)+"\n").encode("utf-8")

def write_one(name,bundle,check=None):
    e,c,m=bundle
    d=OUT/name
    for fn,val in [("encounter.json",e),("catalog.json",c),("manifest.json",m)]:
        emit(d/fn,render(val),check)

def emit(path,data,check):
    """Write generated bytes, or in check mode only compare them."""
    if check is None:
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(data)
    elif not path.is_file() or path.read_bytes()!=data:
        check.append(str(path.relative_to(ROOT)))

def main(argv=None):
    argv=sys.argv[1:] if argv is None else argv
    if argv not in ([],["--check"]):
        raise SystemExit("usage: soul_war_crystal_reconstruction.py [--check]")
    check=[] if argv else None
    write_one("claustrophobic_inferno_raid_1",claustrophobic_raid_1(),check)
    write_one("claustrophobic_inferno_raid_2",claustrophobic_raid_2(),check)
    write_one("claustrophobic_inferno_raid_3",claustrophobic_raid_3(),check)
    write_one("goshnars_greed",greed(),check)
    write_one("goshnars_malice",malice(),check)
    write_one("goshnars_spite",spite(),check)
    write_one("goshnars_cruelty",cruelty(),check)
    write_one("goshnars_megalomania",megalomania(),check)
    write_one("goshnars_hatred",hatred(),check)
    index={
      "schema":"OTERYN_SOUL_WAR_CRYSTAL_ENCOUNTER_RECONSTRUCTION/v1",
      "source":{"repository":"zimbadev/crystalserver","revision":CRYSTAL,"branch":"summer-update"},
      "runtime_qualified":False,
      "encounters":["claustrophobic_inferno_raid_1","claustrophobic_inferno_raid_2","claustrophobic_inferno_raid_3","goshnars_greed","goshnars_malice","goshnars_spite","goshnars_cruelty","goshnars_hatred","goshnars_megalomania"],
      "note":"Source-backed representable cores only. unresolved_semantics rows are hard holds, not approximations."
    }
    emit(OUT/"index.json",render(index),check)
    if check is not None:
        if check:
            print("stale generated files:",*check,sep="\n  ")
            raise SystemExit(1)
        print("Soul War Crystal encounter cores are current")
        return
    print("generated",len(index["encounters"]),"Soul War Crystal encounter cores")

if __name__=="__main__":
    main()