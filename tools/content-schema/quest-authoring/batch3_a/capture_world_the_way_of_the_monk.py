"""Pinned source world callbacks for the_way_of_the_monk; publication is a separate gate."""

from world_common import callback, position


def document():
    quest = "the_way_of_the_monk"
    rows = []
    # Global returns this sphere to the Guild. Use the authored Stone arrival.
    guild_arrival = (32210, 32300, 6)
    for line, origin, appearance, destination in [
        (2, (33605, 31468, 7), 50227, guild_arrival),
        (3, (33601, 31468, 7), 49948, (32456, 32291, 7)),
        (4, (32456, 32290, 7), 49948, (33614, 31494, 7)),
        (5, (32203, 32303, 6), 50310, (33614, 31494, 7)),
    ]:
        rows.append(callback(quest, f"blue-valley-{line}", "use",
                             {"position": position(*origin), "appearance": appearance},
                             position(*destination), [], "actions_blue_valley_teleports.lua", line))
    rows[0]["sources"].extend([
        "https://tibia.fandom.com/wiki/Guarded_Crystal_Sphere",
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/adventurers_guild/adventurers_stone.lua:96-98",
    ])
    return {"schema": 1, "callbacks": rows}
