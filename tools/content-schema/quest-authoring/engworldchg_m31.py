"""Compile the ten selected personal world-change recipes from explicit native bindings.

Global calendars, restoration thresholds and optional rare bosses are deliberately
not substituted for the chosen on-demand recipes. Native map cells are witnesses,
not new world terminals. The Rust journal owner consumes this finite programme file.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = "content/quests/world-change-journals.json"
I64_MAX = 2**63 - 1
QUESTS = {
    "fire_from_the_earth": "oteryn:quest.authored.fire_from_the_earth_mini_world_change",
    "fury_gates": "oteryn:quest.fury_gates_mini_world_change",
    "grimvale": "oteryn:quest.grimvale_mini_world_change",
    "hive_born": "oteryn:quest.authored.hive_born_world_change",
    "hive_outpost": "oteryn:quest.authored.hive_outpost_mini_world_change",
    "horse_station": "oteryn:quest.horse_station_world_change",
    "insectoid_invasion": "oteryn:quest.authored.insectoid_invasion_world_change",
    "jungle_camp": "oteryn:quest.authored.jungle_camp_mini_world_change",
    "kingsday": "oteryn:quest.authored.kingsday_mini_world_change",
    "lumberjack": "oteryn:quest.authored.lumberjack_mini_world_change",
}


def track(key, suffix="phase"):
    return f"oteryn:quest-progress/world-change-journal/{key}/{suffix}"


def position(x, y, z):
    return {"x": x, "y": y, "floor": -z}


def state(key, minimum, maximum, value, *, preserve=False):
    return {"track": key, "minimum": minimum, "maximum": maximum,
            "set": value, **({"preserve": True} if preserve else {})}


def add_track(program, suffix, maximum, *, initial=0, minimum=0):
    key = track(program["key"], suffix)
    declaration = {"key": key, "quest": program["quest"], "initial": initial,
                   "min": minimum, "max": maximum}
    if declaration not in program["tracks"]:
        program["tracks"].append(declaration)
    return key


def bind(program, stages, event):
    for stage in stages:
        program["stage_bindings"].setdefault(stage, []).append(event)


def world(program, event, at, appearance, before, after, stages, *, trigger="use",
          guards=(), writes=(), teleport=None, encounter=None, completes=False):
    key = f"oteryn:quest-callback/worldchg-m31/{program['key']}/{event}"
    callback = {
        "key": key, "quest": program["quest"], "trigger": trigger,
        "selector": {"position": at, "unique": None, "action": None,
                     "appearance": appearance},
        "lever": None, "reversible": False, "passages": [], "teleport": teleport,
        "storage": [state(program["phase_track"], *before, after), *guards, *writes],
        "requires_world_admission": True,
        "offerings": [], "item_on_target": None, "return_on_failure": False,
        "failure_message": "The personal journey requirements are not met.",
        "success_message": "Your personal journal has been updated.",
        "sources": ["oteryn-native:world-change-journal:v1", *program["sources"]],
    }
    if program["minimum_level"] > 0:
        callback["minimum_level"] = program["minimum_level"]
    if encounter is not None:
        callback["effects"] = {"transform": None, "facing": None, "magic": None,
                               "portals": None, "item_cycle": None, "encounter": encounter}
    program["world_callbacks"].append(callback)
    if completes:
        program["completion"]["callbacks"].append(key)
    bind(program, stages, key)
    return callback


def npc(program, event, who, keyword, before, after, stages, *, achievement=None,
        participants=(), completes=False, guards=()):
    key = f"oteryn:quest-reward/worldchg-m31/{program['key']}/{event}"
    callback = {
        "key": key, "quest": program["quest"], "npc": f"oteryn:npc.{who}",
        "keywords": [keyword], "topic": 0, "next_topic": 0,
        "storage": [state(program["phase_track"], *before, after), *guards],
        "take": [], "give": None, "outfit": None, "achievement": achievement,
        "reply": ["Your personal world-change journey has been recorded."],
        "sources": ["oteryn-native:world-change-journal:v1", *program["sources"]],
    }
    if participants:
        callback["participants"] = list(participants)
    if completes:
        callback["completes"] = True
        program["completion"]["callbacks"].append(key)
    program["npc_callbacks"].append(callback)
    bind(program, stages, key)
    return callback


def kills(program, creatures, count, stages, area, *, suffix="kills", gate="enabled",
          completes=False):
    progress = add_track(program, suffix, count)
    active = add_track(program, gate, 1)
    for creature in creatures:
        key = f"oteryn:quest-transition/native-kills/m31/{program['key']}/{creature}"
        program["counters"].append({
            "transition": key, "creature": f"oteryn:creature.{creature}",
            "quest": program["quest"], "track": progress, "gate": active,
            "maximum": count, "area": area,
        })
        bind(program, stages, key)
    if completes:
        program["completion"].update(counter_track=progress, counter_target=count)
    return progress, active


def wave(program):
    nonce = add_track(program, "waves", I64_MAX)
    return {"track": nonce, "minimum": 0, "maximum": I64_MAX - 1, "set": 1, "increment": True}


def encounter(staging, landing, room_min, room_max, monsters):
    return {"minimum_level": 1, "players": [{"from": staging, "to": landing}],
            "room_min": room_min, "room_max": room_max,
            "monsters": [{"creature": f"oteryn:creature.{key}", "position": at}
                         for key, at in monsters]}


def expected(root=ROOT):
    catalogue = json.loads((root / "content/encounters/world-changes/owner-catalogue.json").read_text())
    sources = {e["key"]: [r["url"] for r in e.get("source_refs", [])]
               for e in catalogue["entries"]}
    definitions = {}
    for shard in (root / "content/quests/definitions").glob("quests-*.json"):
        for row in json.loads(shard.read_text())["records"]:
            d = row["definition"]
            if d["identity"]["key"] in QUESTS.values():
                definitions[d["identity"]["key"]] = d
    programs = {}
    for key, quest in sorted(QUESTS.items()):
        definition = definitions[quest]
        recipe = definition.get("recipe") or definition["oteryn_recipe"]["payload"]["recipe"]
        stages = recipe.get("stages", recipe.get("steps"))
        program = {"key": key, "quest": quest, "mode": "personal_on_demand", "repeat": "once",
                   "requires_premium": recipe["requirements"]["premium"],
                   "minimum_level": recipe["requirements"]["min_level"],
                   "journal": {"name": definition["display_name"], "objectives": [s["objective"] for s in stages]},
                   "runtime_enabled": key in {"fury_gates", "grimvale", "hive_outpost", "kingsday"},
                   "phase_track": track(key), "tracks": [], "world_callbacks": [],
                   "npc_callbacks": [], "counters": [], "stage_bindings": {},
                   "completion": {"callbacks": [], "counter_track": None, "counter_target": None},
                   "achievement": None, "taming": None, "sources": sources[key]}
        add_track(program, "phase", 7, minimum=-1)
        programs[key] = program

    p = programs["fury_gates"]
    progress, active = kills(p, ["fury"], 10, ["s3"], [33257, 33342, 31784, 31867, 13, 15])
    access = world(p, "entry", position(32265, 32164, 7), 7066, (-1, 5), 1, ["s1"],
                   teleport=position(33290, 31786, 13))
    access["storage"][0]["at_least"] = True  # the earned hunting access never resets completion
    dungeon = world(p, "dungeon", position(33290, 31786, 13), 10549, (1, 5), 2, ["s2"],
                    trigger="step_in", writes=[state(active, 0, 1, 1)])
    dungeon["storage"][0]["at_least"] = True
    world(p, "seal", position(33290, 31785, 13), 776, (2, 2), 5, ["s4", "s5"],
          guards=[state(progress, 10, 10, 10, preserve=True)],
          writes=[state(active, 1, 1, 0)], completes=True)

    p = programs["grimvale"]
    progress, active = kills(p, ["werewolf"], 10, ["s2"], [33300, 33360, 31665, 31730, 0, 15])
    cells = [position(x, y, 7) for y in (31710, 31711) for x in range(33310, 33313)]
    landing = position(33312, 31710, 7)
    cells.remove(landing)
    cohort = encounter(position(33312, 31709, 7), landing, position(33310, 31710, 7),
                       position(33312, 31711, 7), [("werewolf", at) for at in cells])
    world(p, "island", position(33312, 31709, 7), 4409, (-1, 1), 1, ["s1"],
          trigger="step_in", guards=[state(progress, 0, 9, 0, preserve=True)],
          writes=[state(active, 0, 1, 1), wave(p)], encounter=cohort)
    world(p, "survey", position(33311, 31709, 7), 1866, (1, 1), 4, ["s3", "s4"],
          guards=[state(progress, 10, 10, 10, preserve=True)],
          writes=[state(active, 1, 1, 0)], completes=True)

    p = programs["hive_outpost"]
    progress, active = kills(p, ["crawler", "hive_overseer", "insectoid_worker", "kollos", "spidris",
                                "spidris_elite", "spitter", "swarmer", "waspoid"],
                             20, ["s2"], [32130, 32190, 32810, 32880, 0, 12])
    world(p, "outpost", position(32159, 32845, 7), 12851, (-1, 0), 1, ["s1"],
          trigger="step_in", writes=[state(active, 0, 0, 1)])
    world(p, "survey", position(32159, 32844, 7), 3733, (1, 1), 4, ["s3", "s4"],
          guards=[state(progress, 20, 20, 20, preserve=True)],
          writes=[state(active, 1, 1, 0)], completes=True)

    p = programs["insectoid_invasion"]
    _, active = kills(p, ["insectoid_scout"], 10, ["s2", "s3"],
                      [32200, 32220, 32050, 32070, 7, 7], completes=True)
    world(p, "incursion", position(32208, 32059, 7), 4407, (-1, 0), 1, ["s1"],
          trigger="step_in", writes=[state(active, 0, 0, 1)])

    p = programs["jungle_camp"]
    hunted, hunting = kills(p, ["hunter", "poacher"], 5, ["s2"],
                           [32870, 32960, 32720, 32820, 0, 15], suffix="hunters", gate="hunt-active")
    _, dworcs = kills(p, ["dworc_fleshhunter", "dworc_venomsniper", "dworc_voodoomaster"],
                      5, ["s4", "s5"], [32800, 32860, 32830, 32900, 8, 12],
                      suffix="dworcs", gate="dworc-active", completes=True)
    world(p, "hunter-camp", position(32900, 32792, 7), 1019, (-1, 0), 1, ["s1"],
          trigger="step_in", writes=[state(hunting, 0, 0, 1)])
    world(p, "dworc-camp", position(32831, 32866, 10), 351, (1, 1), 3, ["s3"],
          trigger="step_in", guards=[state(hunted, 5, 5, 5, preserve=True)],
          writes=[state(hunting, 1, 1, 0), state(dworcs, 0, 0, 1)])

    p = programs["kingsday"]
    p["achievement"] = "oteryn:achievement/loyal_subject"
    world(p, "royal-visit", position(32309, 32173, 6), 464, (-1, 0), 1, ["s1"], trigger="step_in")
    npc(p, "greeting", "king_tibianus", "hail the king", (1, 1), 3, ["s2", "s3"],
        achievement=p["achievement"], completes=True)

    p = programs["lumberjack"]
    p["achievement"] = "oteryn:achievement/whistle_blower"
    world(p, "clearing", position(32418, 31701, 7), 4518, (-1, 0), 1, ["s1"], trigger="step_in")
    npc(p, "chip", "chip", "work", (1, 1), 2, ["s2"])
    npc(p, "report", "queen_eloise", "lumberjack", (2, 2), 4, ["s3", "s4"],
        achievement=p["achievement"], completes=True)

    p = programs["fire_from_the_earth"]
    p["achievement"] = "oteryn:achievement/fire_from_the_earth"
    fiery = ["diabolic_imp", "dragon", "dragon_hatchling", "dragon_lord",
             "dragon_lord_hatchling", "dragonling", "hellfire_fighter"]
    progress, active = kills(p, fiery, 50, ["s2", "s3"], [32030, 32090, 32585, 32680, 0, 12], completes=True)
    cells = [position(x, y, 7) for y in (32589, 32590) for x in range(32037, 32041)]
    landing = cells.pop(0)
    cohort = encounter(position(32037, 32588, 7), landing, position(32037, 32589, 7),
                       position(32040, 32590, 7), list(zip(fiery, cells)))
    world(p, "volcano", position(32037, 32587, 7), 103, (-1, 1), 1, ["s1"],
          guards=[state(progress, 0, 49, 0, preserve=True)],
          writes=[state(active, 0, 1, 1), wave(p)], encounter=cohort)

    p = programs["hive_born"]
    progress, active = kills(p, ["crawler", "spidris", "spidris_elite", "kollos"],
                             10, ["s5"], [33430, 33630, 31150, 31320, 0, 14])
    npc(p, "ask", "orockle", "hive defence", (-1, 0), 1, ["s1"])
    npc(p, "permit", "orockle", "personal scent", (1, 1), 3, ["s2", "s3"], participants=[{
        "quest": "oteryn:quest.hive_native_shared", "storage": [], "state_ops": [{
            "track": "oteryn:quest-progress/hive/favour",
            "from": {"kind": "between", "minimum": 50, "maximum": I64_MAX},
            "effect": {"kind": "add", "value": -50},
        }],
    }])
    world(p, "eastern-inner", position(33599, 31228, 1), 12854, (3, 3), 4, ["s4"],
          trigger="step_in", writes=[state(active, 0, 0, 1)])
    world(p, "western-inner", position(33494, 31197, 1), 13038, (4, 4), 7, ["s6", "s7"],
          trigger="step_in", guards=[state(progress, 10, 10, 10, preserve=True)],
          writes=[state(active, 1, 1, 0)], completes=True)

    p = programs["horse_station"]
    cohort = encounter(position(32462, 32213, 7), position(32463, 32214, 7),
                       position(32462, 32214, 7), position(32463, 32214, 7),
                       [("wild_horse", position(32462, 32214, 7))])
    world(p, "pasture", position(32462, 32213, 7), 4519, (-1, 1), 1, ["s1"],
          trigger="step_in", writes=[wave(p)], encounter=cohort)
    taming = {
        "transition": "oteryn:quest-transition/worldchg-m31/horse_station/tamed",
        "item": "oteryn:item.tibia.i12802", "creature": "oteryn:creature.wild_horse",
        "mount": "oteryn:mount.war_horse", "guaranteed_once": True,
        "position": position(32462, 32214, 7), "from_phase": 1, "to_phase": 4,
    }
    p["taming"] = taming
    bind(p, ["s2", "s3", "s4"], taming["transition"])

    return {"schema": 1, "programs": list(programs.values())}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    text = json.dumps(expected(), indent=2, ensure_ascii=False, sort_keys=True) + "\n"
    path = ROOT / OUTPUT
    if args.check:
        if not path.exists() or path.read_text() != text:
            raise SystemExit(f"{OUTPUT} is stale")
    else:
        path.write_text(text)


if __name__ == "__main__":
    main()
