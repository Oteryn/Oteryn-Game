"""Pinned source world callbacks for bigfoot_burden; publication is a separate gate."""

from build_dialogues import PIN
from world_common import callback, position, storage


def document():
    quest = "bigfoot_burden"
    prefix = "oteryn:quest-progress/quest/u9_60/bigfoots_burden/"
    # This is the complete crystal-free subset. Charged routes are retained in
    # the native transport recipe rather than emitted without their item cost.
    routes = [
        (16, (32795, 31761, 10), (33000, 31870, 13), 1),
        (17, (33000, 31871, 13), (32795, 31762, 10), 1),
        (18, (32803, 31745, 10), (32864, 31844, 11), 1),
        (19, (32864, 31845, 11), (32803, 31746, 10), 1),
        (20, (32796, 31780, 10), (32988, 31862, 9), 120),
        (21, (32986, 31861, 9), (32798, 31783, 10), 120),
        (22, (33001, 31916, 9), (32959, 31953, 9), 120),
        (23, (32959, 31952, 9), (33001, 31915, 9), 120),
        (24, (32980, 31907, 9), (32904, 31894, 13), 480),
        (25, (32904, 31893, 13), (32979, 31907, 9), 480),
        (31, (32789, 31796, 10), (32771, 31800, 10), 25),
        (32, (32772, 31799, 10), (32790, 31795, 10), 25),
    ]
    rows = []
    first_refusal = "You have no idea on how to use this device. Xelvar in Kazordoon might tell you more about it."
    rank_refusal = "Your rank among the Gnomes is too low."
    for line, origin, destination, minimum in routes:
        states = [storage(prefix + "quest_line", minimum if minimum < 100 else 1, 2147483647,
                          failure=rank_refusal if minimum == 25 else first_refusal)]
        if minimum >= 100:
            states.append(storage(prefix + "rank", minimum, 2147483647, failure=rank_refusal))
        rows.append(callback(quest, f"gnomebase-{line}", "step_in", {"position": position(*origin)}, position(*destination), states,
                             "movements_gnomebase_teleport.lua", line, rank_refusal, True))
        rows[-1]["sources"] += [f"{PIN}data-global/scripts/quests/{quest}/movements_gnomebase_teleport.lua:{line}"
                                 for line in (78, 86, 93)]
    return {"schema": 1, "callbacks": rows}
