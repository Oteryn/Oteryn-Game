"""Bind the chosen Bewitched contribution and source Raven Herb collection to World clocks."""

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = "content/quests/world-callbacks/qready1-clocks.json"
LANDMARK = "tools/content-schema/world-authoring/samples/qready1-clock-landmarks.json"
CAULDRON = (32769, 32026, -8)
RAVEN = (33497, 32196, -7)
BEWITCHED = "oteryn:quest-callback/qready1-bewitched-contribution"
HERB = "oteryn:quest-callback/qready1-threatened-raven-herb"


def build(root=ROOT):
    definitions = [
        r["definition"]
        for p in (root / "content/quests/definitions").glob("quests-*.json")
        for r in json.loads(p.read_bytes())["records"]
    ]
    recipe = next(
        d["recipe"]
        for d in definitions
        if d["identity"]["key"] == "oteryn:quest.authored.bewitched"
    )
    stages = {s["key"]: s for s in recipe["stages"]}
    if (
        recipe["repeat"]["kind"] != "daily"
        or stages["s2"]["count"] != 3
        or stages["s3"]["count"] != 3
        or stages["s4"]["kind"] != "use"
    ):
        raise ValueError("M19_BEWITCHED_RECIPE_CHANGED")

    def callback(key, quest, position, appearance, seconds, source):
        clock = "oteryn:quest-progress/qready1-clock/" + key.rsplit("/", 1)[1]
        x, y, floor = position
        return {
            "key": key,
            "quest": quest,
            "trigger": "use",
            "selector": {
                "position": {"x": x, "y": y, "floor": floor},
                "appearance": appearance,
                "unique": None,
                "action": None,
            },
            "lever": None,
            "reversible": False,
            "passages": [],
            "teleport": None,
            "storage": [],
            "state_ops": [
                {
                    "track": clock,
                    "from": {"kind": "elapsed_at_least", "seconds": seconds},
                    "effect": {"kind": "set_now"},
                }
            ],
            "offerings": [],
            "item_on_target": None,
            "return_on_failure": False,
            "failure_message": "You cannot do that again yet, or the required items are missing.",
            "sources": [source],
        }

    cauldron = callback(
        BEWITCHED,
        "oteryn:quest.authored.bewitched",
        CAULDRON,
        12023,
        86400,
        "oteryn:authored:bewitched/s4",
    )
    cauldron["selector"]["native"] = {
        "position": cauldron["selector"]["position"],
        "ordinal": 1,
        "appearance": 12023,
    }
    cauldron["offerings"] = [
        {
            "position": {"x": 32769, "y": 32025, "floor": -8},
            "item": "oteryn:item.tibia.i8031",
            "count": 3,
        },
        {
            "position": {"x": 32769, "y": 32027, "floor": -8},
            "item": "oteryn:item.tibia.i11982",
            "count": 3,
        },
    ]
    herb = callback(
        HERB,
        "oteryn:quest.threatened_dreams_quest",
        RAVEN,
        25783,
        60 * 30 * 1000,
        "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/threatened_dreams_quest/event_raven_herb_bush.lua:36-52",
    )
    herb["effects"] = {
        "item_reward": {"item": "oteryn:item.tibia.i5953", "count": 1, "contents": []},
        "item_reward_cycle": herb["state_ops"][0]["track"],
    }
    landmark = {
        "schema": "OTERYN_AUTHORED_QUEST_LANDMARK/v1",
        "basis": "CHOSEN_OTERYN_APPROXIMATION",
        "quest": cauldron["quest"],
        "position": list(CAULDRON),
        "source_item_id": 12023,
        "base_ground": 1210,
        "native_ordinal": 1,
        "area": "oteryn:content.area.subregion.green_claw_swamp",
        "note": "Chosen walkable room one floor beneath Green Claw Swamp; source map and Item are retained, placement is Oteryn-authored.",
    }
    return {OUTPUT: {"schema": 1, "callbacks": [cauldron, herb]}, LANDMARK: landmark}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for path, document in build().items():
        data = (json.dumps(document, indent=2, ensure_ascii=False) + "\n").encode()
        if args.check:
            if (ROOT / path).read_bytes() != data:
                raise SystemExit("M19_CLOCK_BINDING_DRIFT: " + path)
        else:
            (ROOT / path).write_bytes(data)
