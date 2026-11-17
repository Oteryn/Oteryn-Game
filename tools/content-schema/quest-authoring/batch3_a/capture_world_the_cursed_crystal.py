"""Pinned source world callbacks for the_cursed_crystal; publication is a separate gate."""

from build_dialogues import PIN
from world_common import callback, position


def document():
    quest = "the_cursed_crystal"
    rows = []
    for line, origin, destination in [
        (29, (31973, 32905, 11), (31971, 32904, 10)),
        (31, (31973, 32905, 10), (31975, 32911, 9)),
        (33, (32009, 32928, 9), (32009, 32929, 10)),
        (35, (32009, 32928, 10), (32009, 32929, 9)),
    ]:
        rows.append(callback(quest, f"crystal-stairs-{line}", "step_in",
                             {"position": position(*origin), "action": 25018}, position(*destination), [],
                             "movements_StepIn_CursedCrystal.lua", line))
        rows[-1]["sources"].append(f"{PIN}data-global/scripts/quests/{quest}/movements_StepIn_CursedCrystal.lua:25")
    return {"schema": 1, "callbacks": rows}
