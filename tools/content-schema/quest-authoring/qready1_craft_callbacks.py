"""Bind Cursed Crystal's source grinder to the native immovable station."""

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "content/quests/world-callbacks/qready1-crafts.json"
KEY = "oteryn:quest-callback/qready1-cursed-grind"
TRACK = "oteryn:quest-progress/qready1-craft/grind"
POSITION = {"x": 32012, "y": 32908, "floor": -7}


def build():
    return {
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
