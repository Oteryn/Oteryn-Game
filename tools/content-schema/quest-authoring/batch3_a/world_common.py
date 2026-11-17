"""Shared shape constructors; no quest is published by importing these helpers."""

from build_dialogues import PIN, QUESTS


def position(x, y, z):
    return {"x": x, "y": y, "floor": -z}


def storage(track, minimum, maximum, value=0, read_only=True, failure=None):
    result = {"track": track, "minimum": minimum, "maximum": maximum, "set": value, "read_only": read_only}
    if failure is not None:
        result["failure_message"] = failure
    return result


def callback(quest, label, trigger, selector, teleport, states, source, line, failure=None, reject=False):
    return {
        "key": f"oteryn:quest-callback/batch3/{quest}/{label}", "quest": QUESTS[quest],
        "trigger": trigger, "selector": {"position": None, "unique": None, "action": None, "appearance": None, **selector},
        "lever": None, "reversible": False, "passages": [], "teleport": teleport,
        "storage": states, "offerings": [], "item_on_target": None,
        "return_on_failure": reject, "failure_message": failure,
        "sources": [f"{PIN}data-global/scripts/quests/{quest}/{source}:{line}"],
    }
