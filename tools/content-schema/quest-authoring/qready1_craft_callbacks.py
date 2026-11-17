"""Bind Cursed Crystal's source grinder to the native immovable station."""

import argparse
import copy
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "content/quests/world-callbacks/qready1-crafts.json"
KEY = "oteryn:quest-callback/qready1-cursed-grind"
TRACK = "oteryn:quest-progress/qready1-craft/grind"
POSITION = {"x": 32012, "y": 32908, "floor": -7}


def build():
    document = {
        "schema": 1,
        "callbacks": [
            {
                "key": KEY,
                "quest": "oteryn:quest.the_cursed_crystal",
                "trigger": "use",
                "selector": {
                    "position": POSITION,
                    "appearance": 21573,
                    "unique": None,
                    "action": None,
                    "native": {"position": POSITION, "ordinal": 3, "appearance": 21573},
                },
                "lever": None,
                "reversible": False,
                "passages": [],
                "teleport": None,
                "storage": [
                    {
                        "track": TRACK,
                        "minimum": 0,
                        "maximum": 1000000,
                        "set": 1,
                        "preserve": False,
                        "at_least": False,
                        "increment": True,
                    }
                ],
                "offerings": [],
                "item_on_target": {
                    "item": "oteryn:item.tibia.i16122",
                    "count": 1,
                    "replacement": None,
                    "target_replacement": None,
                    "reusable": False,
                },
                "return_on_failure": False,
                "failure_message": "You need a green crystal splinter to use the grinder.",
                "effects": {
                    "item_reward": {
                        "item": "oteryn:item.tibia.i21507",
                        "count": 1,
                        "contents": [],
                    },
                    "item_reward_cycle": TRACK,
                },
                "sources": [
                    "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/scripts/lib/register_actions.lua:1156-1182",
                    "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:data-otservbr-global/scripts/actions/tools/grinder.lua:1-10",
                ],
            }
        ],
    }


    base = document["callbacks"][0]
    def station(name, quest, position, ordinal, appearance, output, materials, carried):
        row = copy.deepcopy(base)
        row["key"] = "oteryn:quest-callback/" + name
        row["quest"] = "oteryn:quest." + quest
        row["selector"] = {
            "position": position, "appearance": appearance, "unique": None, "action": None,
            "native": {"position": position, "ordinal": ordinal, "appearance": appearance},
        }
        track = "oteryn:quest-progress/qready1-craft/" + name
        row["storage"][0]["track"] = track
        row["effects"]["item_reward_cycle"] = track
        row["effects"]["item_reward"]["item"] = f"oteryn:item.tibia.i{output}"
        row["failure_message"] = "You need all of the recipe's ingredients."
        if carried:
            primary, *others = materials
            row["item_on_target"]["item"] = f"oteryn:item.tibia.i{primary}"
            row["item_on_target"]["ingredients"] = [
                {"item": f"oteryn:item.tibia.i{i}", "count": 1} for i in others
            ]
        else:
            row["item_on_target"] = None
            row["offerings"] = [
                {"position": position, "item": f"oteryn:item.tibia.i{i}", "count": 1}
                for i in materials
            ]
        row["sources"] = [
            f"crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data/items/items.xml:item-{appearance}",
            "OTERYN_CHOSEN_WORKBENCH:all-declared-materials;one-atomic-use;one-output",
            "https://www.tibiawiki.com.br/wiki/" + (
                "Cradle_of_Monsters_Quest#Heal_with_Anti-Mutagen" if carried
                else "The_Scatterbrained_Sorcerer_Quest#Feed"
            ),
        ]
        return row
    document["callbacks"].extend([
        station("qready1-cradle-brew", "cradle_of_monsters",
                {"x":33786,"y":32570,"floor":-12}, 2, 8997, 40603,
                [40601, 40598, 40016], True),
        station("qready1-scatter-blend", "the_scatterbrained_sorcerer_quest",
                {"x":32293,"y":32293,"floor":-8}, 2, 9108, 9109,
                [3603, 3492, 3723, 3606, 3738], False),
    ])
    return document


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(), indent=2, ensure_ascii=False) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("QREADY1_CRAFT_BINDING_DRIFT")
    else:
        OUTPUT.write_bytes(raw)
