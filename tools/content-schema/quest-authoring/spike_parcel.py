"""Gnomargery's parcel commission and the distinct Lower Spike outposts."""

import json


def append(root, add, storage, callbacks, offers, declare, elapsed, stamp, fame_change):
    quest = "spike_task"
    npc = "gnomargery"
    source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/"
    pickup = source + "gnomargery.lua:92-103,150-179"
    delivery = source + "gnome_trooper.lua:65-108"
    base = "oteryn:quest-progress/spike/lower/parcel/"
    phase, reported = base + "progress", base + "reported_at"
    declare(phase, 5, pickup)
    declare(reported, 9_223_372_036_854_775_807, pickup)
    sites = []
    directory = root / "content/world/npc-placements"
    for shard in sorted(directory.glob("placements-*.json")):
        for record in json.loads(shard.read_bytes())["records"]:
            npc_placement = record["declaration"]
            cell = npc_placement["cell"]
            if (
                npc_placement["npc"] == "oteryn:npc.gnome_trooper"
                and 13 <= cell["floor"] <= 15
            ):
                sites.append((cell["x"], cell["y"], cell["floor"]))
    if len(set(sites)) != 5 or len(sites) != len(set(sites)):
        raise ValueError("Lower Spike needs five distinct parcel outposts")
    receipts = []
    for x, y, floor in sorted(sites):
        track = base + f"{x}-{y}-{floor}"
        receipts.append(track)
        declare(track, 1, delivery)
        add(
            f"l12-spike-parcel-{x}-{y}-{floor}",
            quest,
            "gnome_trooper",
            ["something"],
            [storage(track, 0, 1)],
            take=[(19219, 1)],
            source=delivery,
        )
        callback = callbacks[-1]
        callback["state_ops"] = [
            {
                "track": phase,
                "from": {"kind": "between", "minimum": 1, "maximum": 4},
                "effect": {"kind": "add", "value": 1},
            }
        ]
        callback.update(
            npc_area={
                "minimum": {"x": x - 5, "y": y - 5, "floor": -floor},
                "maximum": {"x": x + 5, "y": y + 5, "floor": -floor},
            },
            reply=["Thank you for the parcel from the Spike!"],
        )

    offers.append(
        {
            "npc": "oteryn:npc." + npc,
            "keywords": ["deliver", "delivery"],
            "topic": 0,
            "next_topic": 31,
            "minimum_level": 80,
            "storage": [storage(phase, 0, 0, preserve=True)],
            "reply": ["Will you deliver supplies to four different gnome outposts?"],
        }
    )
    add(
        "l12-spike-parcel-accept",
        quest,
        npc,
        ["yes"],
        [storage(phase, 0, 1)],
        topic=31,
        give=(19219, 4),
        source=pickup,
    )
    callbacks[-1].update(
        minimum_level=80,
        per_occurrence_mint=True,
        state_ops=[elapsed(reported, 72000)],
        reply=["Here are the four parcels. Give one to each outpost."],
    )
    add(
        "l12-spike-parcel-report",
        quest,
        npc,
        ["delivery", "deliver"],
        [storage(phase, 5, 0), *[storage(key, 0, 0, maximum=1) for key in receipts]],
        topic=1,
        source=pickup,
    )
    callbacks[-1].update(
        experience=3500,
        state_ops=[stamp(reported), fame_change(0, 2_147_483_646, 1)],
        reply=["Your deliveries earned one point of Spike fame."],
    )


def append_surveys(
    add, storage, callbacks, offers, declare, elapsed, stamp, fame_change
):
    # The Global minimap marks use sector.offset coordinates. Each sector spans 256 cells.
    presences = [
        [(32197, 32527), (32327, 32572), (32217, 32661)],
        [(32277, 32586), (32171, 32669)],
        [(32280, 32582), (32305, 32527), (32096, 32737)],
    ]
    pools = [
        (32263, 32481, 32274, 32490, 13),
        (32163, 32558, 32173, 32567, 13),
        (32201, 32667, 32211, 32672, 13),
        (32135, 32606, 32143, 32614, 14),
        (32330, 32519, 32339, 32521, 14),
        (32260, 32697, 32272, 32705, 14),
        (32176, 32493, 32186, 32502, 15),
        (32341, 32577, 32347, 32586, 15),
        (32220, 32643, 32223, 32652, 15),
    ]
    traces = [
        [(x - 4, y - 4, x + 4, y + 4, depth) for x, y in sites]
        for depth, sites in enumerate(presences, start=8)
    ]
    for label, npc, words, topic, item, xp, lo, hi, steps, selection in (
        (
            "temperature",
            "gnomargery",
            ["temperature"],
            32,
            19206,
            3500,
            80,
            None,
            [pools],
            "world_reset",
        ),
        (
            "tracking",
            "gnomilly",
            ["track", "tracking"],
            32,
            19205,
            1000,
            25,
            49,
            traces,
            "personal",
        ),
    ):
        source = "https://tibia.fandom.com/wiki/Spike_Tasks_Quest/Spoiler"
        phase = "oteryn:quest-progress/spike/" + label + "/progress"
        clock = "oteryn:quest-progress/spike/" + label + "/reported_at"
        declare(phase, len(steps) + 1, source)
        declare(clock, 9_223_372_036_854_775_807, source)
        levels = {"minimum_level": lo}
        if hi is not None:
            levels["maximum_level"] = hi
        offers.append(
            {
                "npc": "oteryn:npc." + npc,
                "keywords": words,
                "topic": 0,
                "next_topic": topic,
                **levels,
                "storage": [storage(phase, 0, 0, preserve=True)],
                "reply": [
                    "Will you track the malevolent presence?"
                    if selection == "personal"
                    else "Will you find the hottest lava pool?"
                ],
            }
        )
        name = "l12-spike-" + label + "-"
        add(
            name + "accept",
            "spike_task",
            npc,
            ["yes"],
            [storage(phase, 0, 1)],
            give=(item, 1),
            topic=topic,
            source=source,
        )
        callbacks[-1].update(
            **levels, per_occurrence_mint=True, state_ops=[elapsed(clock, 72000)]
        )
        for step, sites in enumerate(steps, start=1):
            last = step == len(steps)
            add(
                name + f"survey-{step}",
                "spike_task",
                npc,
                ["use"],
                [storage(phase, step, step + 1)],
                take=[(item, 1)] if last else [],
                present=[] if last else [(item, 1)],
                source=source,
            )
            callbacks[-1].update(
                carried_item=f"oteryn:item.tibia.i{item}",
                survey={
                    "selection": selection,
                    "areas": [
                        {
                            "minimum": {"x": x, "y": y, "floor": -z},
                            "maximum": {"x": xx, "y": yy, "floor": -z},
                        }
                        for x, y, xx, yy, z in sites
                    ],
                },
                reply=[
                    "Report your findings."
                    if last
                    else "The detector signals another presence."
                ],
            )
        add(
            name + "report",
            "spike_task",
            npc,
            words,
            [storage(phase, len(steps) + 1, 0)],
            topic=1,
            source=source,
        )
        callbacks[-1].update(
            experience=xp,
            state_ops=[stamp(clock), fame_change(0, 2_147_483_646, 1)],
            reply=["Your report earns one point of Spike fame."],
        )
