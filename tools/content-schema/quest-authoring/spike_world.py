"""The Middle Spike transformer and repairable monster nests."""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "world-authoring"))
import world_region_codec as codec

SOURCE = "https://www.tibiawiki.com.br/wiki/Spike_Tasks"
SCRIPT = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_spike_tasks/"
BASE = "oteryn:quest-progress/spike/nests/"
WORLD = "content/quests/world-callbacks/spike-objectives.json"
QUEST = "oteryn:quest.spike_task"
PADS = {(32242, 32610, 11), (32242, 32611, 11)}


def dialogue(add, storage, callbacks, offers, declare, elapsed, stamp, fame):
    phase, clock, transformed = (
        BASE + part for part in ("progress", "reported_at", "transformed_at")
    )
    declare(phase, 9, SOURCE)
    declare(clock, 9_223_372_036_854_775_807, SOURCE)
    declare(transformed, 9_223_372_036_854_775_807, SOURCE)
    offers.append(
        {
            "npc": "oteryn:npc.gnombold",
            "topic": 0,
            "keywords": ["nests"],
            "next_topic": 31,
            "minimum_level": 50,
            "maximum_level": 79,
            "storage": [storage(phase, 0, 0, preserve=True)],
            "reply": [
                "Will you use the gnomish transformer and destroy eight monster nests?"
            ],
        }
    )
    add(
        "l12-spike-nests-accept",
        "spike_task",
        "gnombold",
        ["yes"],
        [storage(phase, 0, 1)],
        topic=31,
        source=SOURCE,
    )
    callbacks[-1].update(
        minimum_level=50, maximum_level=79, state_ops=[elapsed(clock, 72000)]
    )
    add(
        "l12-spike-nests-report",
        "spike_task",
        "gnombold",
        ["nests"],
        [
            storage(phase, 9, 0),
            storage(transformed, 0, 0, maximum=9_223_372_036_854_775_807),
        ],
        topic=1,
        source=SOURCE,
    )
    callbacks[-1].update(
        experience=2000,
        state_ops=[stamp(clock), fame(0, 2_147_483_646, 1)],
        reply=["Your report earns one point of Spike fame."],
    )

    release = "oteryn:quest-progress/spike/release/"
    phase, clock, digs = (
        release + part for part in ("progress", "reported_at", "digs")
    )
    declare(phase, 2, SOURCE)
    declare(clock, 9_223_372_036_854_775_807, SOURCE)
    declare(digs, 2_147_483_647, SOURCE)
    offers.append(
        {
            "npc": "oteryn:npc.gnomilly",
            "topic": 0,
            "keywords": ["release"],
            "next_topic": 31,
            "minimum_level": 25,
            "maximum_level": 49,
            "storage": [storage(phase, 0, 0, preserve=True)],
            "reply": ["Will you release an angry spirit from an ominous mound?"],
        }
    )
    add(
        "l12-spike-release-accept",
        "spike_task",
        "gnomilly",
        ["yes"],
        [storage(phase, 0, 1)],
        give=(19203, 1),
        topic=31,
        source=SOURCE,
    )
    callbacks[-1].update(
        minimum_level=25,
        maximum_level=49,
        per_occurrence_mint=True,
        state_ops=[elapsed(clock, 72000)],
    )
    add(
        "l12-spike-release-report",
        "spike_task",
        "gnomilly",
        ["release", "releasing"],
        [storage(phase, 2, 0), storage(digs, 0, 0, maximum=2_147_483_647)],
        topic=1,
        source=SOURCE,
    )
    callbacks[-1].update(
        experience=1000,
        state_ops=[stamp(clock), fame(0, 2_147_483_646, 1)],
        reply=["Your report earns one point of Spike fame."],
    )


def mounds(pos, ordinal, shard):
    phase = "oteryn:quest-progress/spike/release/progress"
    digs = "oteryn:quest-progress/spike/release/digs"
    for outcome in ("empty", "zombie", "demon_skeleton", "ghost", "enraged_soul"):
        soul = outcome == "enraged_soul"
        row = {
            "key": f"oteryn:quest-callback/spike/mound-{pos['x']}-{pos['y']}-{-pos['floor']}/{outcome}",
            "quest": QUEST,
            "trigger": "use",
            "selector": {"position": pos, "ordinal": ordinal, "appearance": 19211},
            "lever": [19211, 19212],
            "requires_world_admission": True,
            "reversible": False,
            "passages": [],
            "return_on_failure": False,
            "offerings": [],
            "storage": [
                {
                    "track": phase,
                    "minimum": 1,
                    "maximum": 1,
                    "set": 2 if soul else 1,
                    "preserve": not soul,
                },
                {
                    "track": digs,
                    "minimum": 0,
                    "maximum": 2_147_483_646,
                    "set": 1,
                    "increment": True,
                },
            ],
            "item_on_target": {
                "item": "oteryn:item.tibia.i19203",
                "count": 1,
                "selected_instance": True,
                "reusable": not soul,
            },
            "sources": [SOURCE, SCRIPT + "actions_spirit_shovel.lua", str(shard)],
            "success_message": "You've found nothing special."
            if outcome == "empty"
            else "You unearthed a spirit's anger!"
            if soul
            else "You have disturbed the dead.",
        }
        if outcome == "empty":
            row["effects"] = {"reset_after_ms": 120000}
        else:
            row["effects"] = {
                "encounter": {
                    "minimum_level": 25,
                    "players": [],
                    "monsters": [
                        {"position": pos, "creature": "oteryn:creature." + outcome}
                    ],
                    "room_min": pos,
                    "room_max": pos,
                    "timing": {"spawn_after_ms": 1, "reset_after_ms": 120000},
                }
            }
        yield row


def world(root):
    folder = root / "content/world/placements"
    palette = json.loads((folder / "index.json").read_bytes())["palette"]
    phase, transformed = BASE + "progress", BASE + "transformed_at"
    callbacks = []
    pads = set()
    for depth in range(8, 13):
        for rx in (125, 126):
            for ry in (126, 127):
                shard = folder / codec.region_name(depth, rx, ry)
                if not shard.exists():
                    continue
                for _, sector in codec.decode_region(shard.read_bytes())[3]:
                    for x, y, _, _, _, stack in sector:
                        if not (32100 <= x <= 32400 and 32450 <= y <= 32720):
                            continue
                        pos = {"x": x, "y": y, "floor": -depth}
                        pad = (x, y, depth) in PADS
                        for ordinal, (index, nested, _) in enumerate(stack):
                            appearance = palette[index]["source_item_id"]
                            if not nested and appearance == 19211 and depth <= 10:
                                callbacks.extend(
                                    mounds(pos, ordinal, shard.relative_to(root))
                                )
                            if nested or not (
                                appearance == 19209 or (pad and ordinal == 0)
                            ):
                                continue
                            prefix = "transform" if pad else "nest"
                            row = {
                                "key": f"oteryn:quest-callback/spike/{prefix}-{x}-{y}-{depth}",
                                "quest": QUEST,
                                "trigger": "step_in" if pad else "use",
                                "selector": {
                                    "position": pos,
                                    "appearance": appearance,
                                    "ordinal": ordinal,
                                },
                                "requires_world_admission": True,
                                "reversible": False,
                                "passages": [],
                                "return_on_failure": False,
                                "offerings": [],
                                "sources": [
                                    SOURCE,
                                    SCRIPT
                                    + (
                                        "movements_geomantic_charges.lua"
                                        if pad
                                        else "actions_nests.lua"
                                    ),
                                    str(shard.relative_to(root)),
                                ],
                                "storage": [
                                    {
                                        "track": phase,
                                        "minimum": 1,
                                        "maximum": 8,
                                        "set": 1,
                                        ("preserve" if pad else "increment"): True,
                                    }
                                ],
                                "state_ops": [
                                    {
                                        "track": transformed,
                                        "from": {
                                            "kind": "elapsed_at_least"
                                            if pad
                                            else "elapsed_less_than",
                                            "seconds": 120,
                                        },
                                        "effect": {"kind": "set_now"}
                                        if pad
                                        else {"kind": "add", "value": 0},
                                    }
                                ],
                            }
                            if pad:
                                pads.add((x, y, depth))
                            else:
                                row.update(
                                    lever=[19209, 19210],
                                    effects={"reset_after_ms": 120000},
                                    success_message="You have destroyed a monster nest.",
                                )
                            callbacks.append(row)
    if pads != PADS or not any(row.get("lever") for row in callbacks):
        raise ValueError("Missing Middle Spike transformer or nests")
    return {"schema": 1, "callbacks": callbacks}
