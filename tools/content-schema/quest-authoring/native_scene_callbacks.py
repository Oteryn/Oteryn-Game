"""Lower pinned source rooms' finite edits into the shared scene projection."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
SOURCE = "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua:1-23"


def position(x, y, floor=-14):
    return {"x": x, "y": y, "floor": floor}


def katana():
    door = position(32177, 32148, -11)
    landing = position(32178, 32148, -11)
    return {
        "key": "oteryn:quest-callback/katana-hidden-lever",
        "quest": "oteryn:quest.katana_quest",
        "trigger": "use",
        "selector": {"position": position(32182, 32145, -11), "unique": 30029,
                     "action": None, "appearance": 2772},
        "lever": [2772, 2773], "reversible": True, "passages": [],
        "teleport": None, "storage": [], "offerings": [],
        "return_on_failure": False, "failure_message": None,
        "effects": {"scene": {
            "open": [{"position": door, "remove": 5107, "create": 5108, "passable": True}],
            "close": [{"position": door, "remove": 5108, "create": 5107, "passable": False}],
            "evacuate_close": [{"players_to": landing, "creatures_to": landing,
                                "all": True, "from": door}],
            "relocate_items_close": True,
        }},
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:"
                    "data-global/scripts/actions/rookgaard/katana_quest/katana_quest_lever.lua:6-28"],
    }


def serpentine():
    walls = [(x, y) for y in (32867, 32868) for x in (33148, 33149)]
    crystal = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/"
    return {
        "key": "oteryn:quest-callback/serpentine-tower-lever",
        "quest": "oteryn:quest.serpentine_tower_quest",
        "trigger": "use",
        "selector": {"position": position(33152, 32866, -8), "unique": None,
                     "action": 5633, "appearance": 2772},
        "lever": [2772, 2773], "reversible": True, "passages": [],
        "teleport": None, "storage": [], "offerings": [],
        "return_on_failure": False, "failure_message": None,
        "effects": {"scene": {
            "open": [
                {"position": position(x, y, -9), "remove": 2129, "create": None,
                 "passable": None, "allow_missing": True}
                for x, y in walls
            ],
            "close": [
                {"position": position(x, y, -9), "remove": None, "create": 2129,
                 "passable": False}
                for x, y in walls
            ],
            "evacuate_close": [
                {"from": position(x, y, -9), "players_to": position(x, 32869, -9),
                 "creatures_to": position(x, 32869, -9), "all": True}
                for x, y in walls
            ],
            "relocate_items_close": True,
        }},
        "sources": [
            crystal + "scripts/quests/others/actions_serpentine_tower_lever.lua:1-34",
            crystal + "startup/tables/lever.lua:79-83",
        ],
    }


def generate():
    edits = [{"position": position(32592, y), "remove": None, "create": 1270, "passable": False}
             for y in (32105, 32106)]
    edits += [{"position": position(x, 32104), "remove": 1271, "create": None, "passable": None}
              for x in range(32593, 32602)]
    return {"schema": 1, "callbacks": [katana(), {
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
    }, serpentine()]}


if __name__ == "__main__":
    target = ROOT / "content/quests/native-scene-callbacks.json"
    raw = json.dumps(generate(), indent=2) + "\n"
    if "--check" in sys.argv:
        if target.read_text() != raw:
            raise SystemExit("stale native scene callbacks")
    else:
        target.write_text(raw)
