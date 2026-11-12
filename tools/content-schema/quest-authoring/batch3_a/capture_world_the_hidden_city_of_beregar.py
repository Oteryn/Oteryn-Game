"""Pinned source world callbacks for the_hidden_city_of_beregar; publication is a separate gate."""

from build_dialogues import PIN
from world_common import callback, position, storage


def document():
    quest = "the_hidden_city_of_beregar"
    prefix = "oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/"
    rows = []
    for aid, z, line in [(50092, 15, 2), (50093, 14, 3)]:
        rows.append(callback(quest, f"elevator-{aid}", "step_in", {"action": aid}, position(32612, 31499, z),
                             [storage(prefix + "going_down", 2, 2)], "moviments_elevator.lua", line, "You don't know how to use this yet."))
    rows.append(callback(quest, "gap", "step_in", {"action": 50111}, position(32569, 31507, 9), [], "moviments_gap.lua", 9))
    rows.append(callback(quest, "tunnel", "step_in", {"action": 40029}, position(32616, 31514, 9), [], "moviments_tunnel.lua", 9))
    rows.append(callback(quest, "discover-ore-wagon", "step_in", {"action": 50091}, None,
                         [storage(prefix + "ore_wagon", -1, 0, 1, False)], "moviments_ore_wagon.lua", 9))
    rows[-1]["success_message"] = "You have found the entrance to the hidden city of Beregar and may now use the ore wagon."
    rows[-1]["sources"].append(f"{PIN}data-global/scripts/quests/{quest}/moviments_ore_wagon.lua:11")
    return {"schema": 1, "callbacks": rows}
