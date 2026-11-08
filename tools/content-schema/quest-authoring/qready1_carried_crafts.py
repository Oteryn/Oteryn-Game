"""Lower Cursed Crystal's first, storage-free two-operand mixture."""

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "content/quests/npc-callbacks/qready1-carried-crafts.json"


def build():
    callbacks = []
    for source, target in [(11466, 21504), (21504, 11466)]:
        callbacks.append(
            {
                "key": f"oteryn:quest-reward/qready1/cursed-crystal-first-{source}",
                "quest": "oteryn:quest.tibia_tales",
                "npc": "oteryn:npc.one_eyed_joe",
                "carried_item": f"oteryn:item.tibia.i{source}",
                "carried_target": f"oteryn:item.tibia.i{target}",
                "keywords": ["mix"],
                "topic": 0,
                "next_topic": 0,
                "storage": [],
                "take": [
                    {"item": f"oteryn:item.tibia.i{source}", "count": 1},
                    {"item": f"oteryn:item.tibia.i{target}", "count": 1},
                ],
                "give": {
                    "item": "oteryn:item.tibia.i21505",
                    "count": 1,
                    "contents": [],
                },
                "outfit": None,
                "achievement": None,
                "reply": [
                    ("You mixed the first ingredients to create a special ointment. "
                    "But it isn't complete yet.")
                ],
                "sources": [
                    ("crystal:201189f871d23597b82caf88fd611454a2ddefb8:"
                    "data-global/scripts/quests/the_cursed_crystal/actions_MedusaOil.lua:3-4,50-61")
                ],
            }
        )
    return {"schema": 1, "tracks": [], "callbacks": callbacks}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    raw = (json.dumps(build(), indent=2) + "\n").encode()
    if args.check:
        if OUTPUT.read_bytes() != raw:
            raise SystemExit("QREADY1_CARRIED_CRAFT_BINDING_DRIFT")
    else:
        OUTPUT.write_bytes(raw)
