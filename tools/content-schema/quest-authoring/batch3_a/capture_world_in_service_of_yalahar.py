"""Pinned source world callbacks for in_service_of_yalahar; publication is a separate gate."""

from world_common import callback, position, storage


def document():
    quest = "in_service_of_yalahar"
    track = "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline"
    gates = [
        (3091, 32744, 31161, 5, 21), (3092, 32744, 31164, 5, 21),
        (3093, 32833, 31269, 5, 24), (3094, 32833, 31266, 5, 24),
        (3095, 32729, 31200, 5, 29), (3096, 32734, 31200, 5, 29),
        (3097, 32776, 31141, 5, 35), (3098, 32776, 31145, 5, 35),
        (3099, 32874, 31202, 5, 41), (3100, 32869, 31202, 5, 41),
        (3101, 32856, 31251, 5, 45), (3102, 32854, 31248, 5, 45),
    ]
    rows = []
    for index, (uid, x, y, z, minimum) in enumerate(gates, 2):
        rows.append(callback(quest, f"mechanism-{uid}", "use", {"unique": uid}, position(x, y, z),
                             [storage(track, minimum, 2147483647)], "actions_mechanism.lua", index,
                             "The gate mechanism won't move. You probably have to find a way around until you figure out how to operate the gate."))
    for uid, x, y, z, line in [(1018, 32773, 31116, 7, 17), (1019, 32780, 31115, 7, 18)]:
        rows.append(callback(quest, f"mechanism-{uid}", "use", {"unique": uid}, position(x, y, z), [], "actions_mechanism.lua", line))
    return {"schema": 1, "callbacks": rows}
