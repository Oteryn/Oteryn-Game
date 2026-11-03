"""Pinned source world callbacks for the_way_of_the_monk; publication is a separate gate."""

from world_common import callback, position


def document():
    quest = "the_way_of_the_monk"
    rows = []
    for line, origin, destination in [
        (2, (33605, 31468, 7), (32456, 32291, 7)),
        (3, (33601, 31468, 7), (32456, 32291, 7)),
        (4, (32456, 32290, 7), (33614, 31494, 7)),
        (5, (32203, 32303, 6), (33614, 31494, 7)),
    ]:
        rows.append(callback(quest, f"blue-valley-{line}", "use", {"position": position(*origin)},
                             position(*destination), [], "actions_blue_valley_teleports.lua", line))
    return {"schema": 1, "callbacks": rows}
