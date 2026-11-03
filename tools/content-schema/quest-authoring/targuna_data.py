"""Source-pinned authoring input; copy into the repository tools before publication."""

import argparse
import hashlib
import json
import re
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--donor", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument(
    "--check", action="store_true", help="Compare exact output bytes without writing"
)
args = parser.parse_args()
CRYSTAL = args.donor
PIN = "201189f871d23597b82caf88fd611454a2ddefb8"
SOURCE_BLOBS = {
    "data-global/lib/core/storages.lua": "1a9a7e931eb984a75b285be01df6b1dba36476d8",
    "data-global/npc/adrian.lua": "075b3cffe4574c2a1e88a7fc2265586674d625bc",
    "data-global/npc/camilla.lua": "14dd037141cc7ec6a27b1d45e02b5d3356774485",
    "data-global/npc/emiliana.lua": "06a0197536d6e4fbd77e71cfb9dd55a1c801f1e8",
    "data-global/npc/lizzie.lua": "40e8bbc7e6e0ad8bfd71ebd6fbf6c3c6c0115988",
    "data-global/npc/morla.lua": "e2dc06a2333662902d12a612ed6d5024f71ebcce",
    "data-global/npc/sterling.lua": "e3204853d188107b57462112fa014f41a61f86b3",
    "data-global/scripts/quests/targuna/actions_tortoise_eggs.lua": "7762b23da8dd700edb91bf37c0d5b94d2cb615ff",
    "data-global/scripts/quests/targuna/actions_treasure_chest.lua": "7606f86f0f637169720386847c46ca1665be9f89",
    "data-global/scripts/quests/targuna/creaturescripts_lizard_commander.lua": "cfcd67e7eff75ff2379d4db5ce9ded1d04011738",
    "data-global/scripts/quests/targuna/creaturescripts_pirates.lua": "bb3dac8199d2e6031062d1e7fe4148954fb7c59e",
    "data-global/scripts/quests/targuna/eventcallbacks_secondary_tasks.lua": "1ef140ce61f19bc40fdcf880bc513a0a1365a489",
    "data-global/scripts/quests/targuna/movements_aragonia.lua": "d5e04b5ed70b15d7db0346eccbb4819f1298aca6",
    "data-global/scripts/quests/targuna/movements_main_continent_hint.lua": "eb5e69830f683706f3989ba3a7dff3feac36ed84",
    "data-global/scripts/quests/targuna/movements_sandcastles.lua": "882550261f543b0d7ed77e78cc1c4391947720e4",
}
for relative, expected in SOURCE_BLOBS.items():
    raw = (CRYSTAL / relative).read_bytes()
    actual = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if actual != expected:
        raise ValueError(
            f"pinned donor blob mismatch: {relative}; expected {expected}, got {actual}"
        )
QUEST = "oteryn:quest.targuna_quest"
PREFIX = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/"
TRANSITION = "oteryn:quest-transition/targuna/"


def source(path, line):
    return f"zimbadev/crystalserver@{PIN}:{path}:{line}"


def s(name, line):
    return source("data-global/scripts/quests/targuna/" + name + ".lua", line)


def n(name, line):
    return source("data-global/npc/" + name + ".lua", line)


def effect(track, op, value, kind, result):
    return {
        "track": PREFIX + track,
        "from": {"op": op, "value": value},
        "effect": {"op": kind, "value": result},
    }


def set_(track, op, value, result):
    return effect(track, op, value, "set", result)


def keep(track, op, value):
    return effect(track, op, value, "add", 0)


transitions = []
ACTIVE_NPC = {
    "start-ancient-new",
    "start-ancient-existing",
    "start-treasure-new",
    "start-treasure-existing",
    "start-burning-new",
    "start-burning-existing",
    "report-commander",
    "camilla-reward",
    "mark-treasure",
    "crimson-briefing",
}
HELD_REWARDS = {"morla-reward", "turnips-reward", "treasure-reward"}


def add(key, effects, sources, experience=None):
    transitions.append(
        {
            "key": TRANSITION + key,
            "effects": effects,
            "experience": experience,
            "source": sources,
            "execution": (
                "ready"
                if key in ACTIVE_NPC
                else "held_shared_owner"
                if key in HELD_REWARDS
                else "source_only"
            ),
        }
    )


ancient = "ancient_enemy/mission"
treasure = "long_lost_treasure/mission"
burning = "burning_heart/mission"
pirates = "long_lost_treasure/pirate_kill_count"
eggs = "long_lost_treasure/tortoise_egg_count"
commander = "ancient_enemy/lizard_commander_killed"
morla = "long_lost_treasure/morla_done"
turnips = "burning_heart/turnips_reward"
herald = "burning_heart/herald_killed"
sand = "long_lost_treasure/sandcastle_count"

# Source Questline is advanced only if absent/zero; never reset a started quest.
for suffix, mission, value, origin in [
    ("start-ancient", ancient, 2, n("lizzie", 119)),
    ("start-treasure", treasure, 1, n("sterling", 129)),
    ("start-burning", burning, 1, n("emiliana", 159)),
]:
    for started in [False, True]:
        add(
            suffix + ("-existing" if started else "-new"),
            [
                set_(mission, "lt", value, value),
                keep("questline", "ge", 1)
                if started
                else set_("questline", "lt", 1, 1),
            ],
            [origin],
        )

add(
    "report-commander",
    [set_(ancient, "eq", 2, 3), keep(commander, "eq", 1)],
    [n("lizzie", 92)],
)
add(
    "commander-death",
    [keep(ancient, "eq", 2), set_(commander, "ne", 1, 1)],
    [s("creaturescripts_lizard_commander", 4)],
)
add(
    "camilla-reward",
    [
        set_("ancient_enemy/camilla_reward", "ne", 1, 1),
        keep(commander, "eq", 1),
        set_(ancient, "any", None, 4),
    ],
    [n("camilla", 91)],
    1500,
)
add(
    "morla-reward",
    [
        set_(morla, "ne", 1, 1),
        set_(treasure, "ge", 1, 3),
        keep(pirates, "ge", 20),
        keep(eggs, "ge", 10),
    ],
    [n("morla", 92), n("morla", 110)],
)
add(
    "mark-treasure",
    [
        set_(treasure, "eq", 3, 4),
        keep(morla, "eq", 1),
        set_("long_lost_treasure/treasure_marked", "any", None, 1),
    ],
    [n("sterling", 142)],
)
add(
    "matilda-outbound-first",
    [set_(treasure, "eq", 1, 2)],
    [s("movements_aragonia", 24)],
)
add("matilda-outbound-repeat", [keep(treasure, "ge", 2)], [s("movements_aragonia", 24)])
add(
    "pirate-death",
    [keep(treasure, "eq", 2), effect(pirates, "lt", 20, "add", 1)],
    [s("creaturescripts_pirates", 9)],
)
add(
    "tortoise-egg",
    [keep(treasure, "eq", 2), effect(eggs, "lt", 10, "add", 1)],
    [s("actions_tortoise_eggs", 69)],
)
add(
    "treasure-reward",
    [set_("long_lost_treasure/treasure_chest", "ne", 1, 1), set_(treasure, "ge", 4, 5)],
    [s("actions_treasure_chest", 12)],
)
add(
    "crimson-briefing",
    [set_(burning, "eq", 1, 2), keep(herald, "ne", 1), keep(turnips, "ne", 1)],
    [n("emiliana", 215), n("emiliana", 230)],
)
add(
    "turnips-reward",
    [set_(turnips, "ne", 1, 1), keep(herald, "eq", 1), set_(burning, "any", None, 4)],
    [n("emiliana", 169), n("emiliana", 177)],
    5000,
)
for index in range(13):
    bit = 1 << index
    add(
        "sandcastle-" + str(index + 1),
        [effect(sand, "bit_unset", bit, "add", bit)],
        [s("movements_sandcastles", 57)],
    )
for task, origin in [
    ("daily_reward", s("eventcallbacks_secondary_tasks", 14)),
    ("stash_item", s("eventcallbacks_secondary_tasks", 32)),
    ("take_from_stash", s("eventcallbacks_secondary_tasks", 44)),
    ("deposit_gold", n("adrian", 77)),
    ("withdraw_gold", n("adrian", 77)),
]:
    add(
        "secondary-" + task.replace("_", "-"),
        [set_("secondary_tasks/" + task, "eq", 1, 2)],
        [origin],
    )

# Each source conditional is represented by a mutually exclusive closed guard.
# This stays inside the existing eight-effects ceiling and one atomic writer call.
arrival_tracks = [
    "questline",
    "secondary_tasks/daily_reward",
    "secondary_tasks/deposit_gold",
    "secondary_tasks/stash_item",
    "secondary_tasks/take_from_stash",
    "secondary_tasks/withdraw_gold",
    ancient,
]
for mask in range(128):
    effects = [set_("main_continent_hint", "ne", 1, 1)]
    for index, track in enumerate(arrival_tracks):
        effects.append(
            keep(track, "ge", 1) if mask & (1 << index) else set_(track, "lt", 1, 1)
        )
    add("arrival-" + str(mask), effects, [s("movements_main_continent_hint", 26)])


def positions(filename, first, last):
    lines = (
        (CRYSTAL / "data-global/scripts/quests/targuna" / filename)
        .read_text()
        .splitlines()
    )
    data = "\n".join(lines[first - 1 : last])
    values = re.findall(r"Position\((\d+),\s*(\d+),\s*(\d+)\)", data)
    if not values:
        values = re.findall(
            r"\{\s*x\s*=\s*(\d+),\s*y\s*=\s*(\d+),\s*z\s*=\s*(\d+)\s*\}", data
        )
    return [{"x": int(x), "y": int(y), "floor": -int(z)} for x, y, z in values]


data = {
    "schema": "OTERYN_TARGUNA/v1",
    "quest": QUEST,
    "source": source("data-global/lib/core/storages.lua", 3152),
    "tracks": [
        {
            "key": PREFIX + "main_continent_hint",
            "initial": 0,
            "min": 0,
            "max": 1,
            "source": s("movements_main_continent_hint", 26),
        }
    ],
    "transitions": transitions,
    "arrival_tracks": [PREFIX + x for x in arrival_tracks],
    "arrival_positions": positions("movements_main_continent_hint.lua", 6, 16),
    "sandcastle_positions": positions("movements_sandcastles.lua", 8, 22),
    "egg_positions": positions("actions_tortoise_eggs.lua", 8, 53),
    "rewards": [
        {
            "key": "morla",
            "source": n("morla", 110),
            "transition": TRANSITION + "morla-reward",
            "item": 53158,
            "contents": [],
            "claim": "oteryn:reward-claim/targuna-morla",
        },
        {
            "key": "turnips",
            "source": n("emiliana", 177),
            "transition": TRANSITION + "turnips-reward",
            "item": 53073,
            "contents": [],
            "claim": "oteryn:reward-claim/targuna-turnips",
        },
        {
            "key": "treasure",
            "source": s("actions_treasure_chest", 23),
            "transition": TRANSITION + "treasure-reward",
            "item": 53074,
            "contents": [3003],
            "claim": "oteryn:reward-claim/targuna-treasure",
        },
    ],
}
assert len(data["arrival_positions"]) == 9
assert len(data["sandcastle_positions"]) == 13
assert len(data["egg_positions"]) == 46
assert len({t["key"] for t in transitions}) == len(transitions)
assert all(len(t["effects"]) <= 8 for t in transitions)
encoded = (json.dumps(data, indent=2) + "\n").encode("utf-8")
if args.check:
    if not args.out.is_file() or args.out.read_bytes() != encoded:
        raise SystemExit(f"generated content drift: {args.out}")
else:
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(encoded)
print(
    f"Targuna input: {len(transitions)} closed compound transitions; 9 arrivals, 13 castles, 46 eggs."
)
