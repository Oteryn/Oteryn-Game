"""Lower the pinned source room's finite edits into the shared scene projection."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
SOURCE = "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua:1-23"


def position(x, y):
    return {"x": x, "y": y, "floor": -14}


def generate():
    edits = [{"position": position(32592, y), "remove": None, "create": 1270, "passable": False}
             for y in (32105, 32106)]
    edits += [{"position": position(x, 32104), "remove": 1271, "create": None, "passable": None}
              for x in range(32593, 32602)]
    return {"schema": 1, "callbacks": [{
        "key": "oteryn:quest-callback/mintwallin-cyclops-scene",
        "quest": "oteryn:quest.mintwallin_cyclops_quest",
        "trigger": "use",
        "selector": {"position": position(32600, 32105), "unique": None,
                     "action": None, "appearance": 2772, "ordinal": 1},
        "lever": [2772, 2773], "reversible": False, "passages": [],
        "teleport": None, "storage": [], "offerings": [],
        "return_on_failure": False, "failure_message": None,
        "effects": {"scene": {"open": edits, "close": [], "remove_self": True}},
        "sources": [SOURCE,
            "https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua",
            "Committed native region-z14-x127-y125.b3: lever32600,32105,14 appearance2772 ordinal1; action30008 is not retained, so no legacy tag is invented."]
    }]}


if __name__ == "__main__":
    target = ROOT / "content/quests/native-scene-callbacks.json"
    raw = json.dumps(generate(), indent=2) + "\n"
    if "--check" in sys.argv:
        if target.read_text() != raw:
            raise SystemExit("stale native scene callbacks")
    else:
        target.write_text(raw)
